use crate::api::auth::{apply_auth, AuthKind};
use crate::api::error::ApiError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

pub struct LlmCorrectClient {
    base: String,
    model: String,
    auth_kind: AuthKind,
    api_key: Arc<Mutex<String>>,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMsg>,
    temperature: f32,
    max_tokens: u32,
}
#[derive(Serialize)]
struct ChatMsg {
    role: String,
    content: String,
}
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: MsgContent,
}
#[derive(Deserialize)]
struct MsgContent {
    content: String,
}

impl LlmCorrectClient {
    pub fn new(base: String, model: String, auth_kind: AuthKind, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap();
        Self {
            base,
            model,
            auth_kind,
            api_key: Arc::new(Mutex::new(api_key)),
            http,
        }
    }

    pub fn set_api_key(&self, key: String) {
        *self.api_key.lock().unwrap() = key;
    }

    pub async fn correct(
        &self,
        text: &str,
        entries: &[(String, String)],
    ) -> Result<String, ApiError> {
        if entries.is_empty() {
            return Ok(text.to_string());
        }
        let dict_str = entries
            .iter()
            .map(|(f, t)| format!("\"{}\" → \"{}\"", f, t))
            .collect::<Vec<_>>()
            .join("\n");
        let system = format!(
            "あなたは音声転写の校正者です。以下の辞書にある用語の表記ゆれのみ修正してください。\n辞書:\n{}\nルール:\n- 辞書にない単語の言い換えや要約は禁止\n- 文体・語尾・助詞は変更しない\n- 自信がない場合は原文のまま返す\n- 修正後のテキストのみ返す（説明不要）",
            dict_str
        );
        let req = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMsg { role: "system".into(), content: system },
                ChatMsg { role: "user".into(), content: text.to_string() },
            ],
            temperature: 0.0,
            max_tokens: 512,
        };
        let key = self.api_key.lock().unwrap().clone();
        let resp = apply_auth(
            self.http.post(format!("{}/v1/chat/completions", self.base)),
            &self.auth_kind,
            &key,
        )
        .json(&req)
        .send()
        .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Status { status, body });
        }
        let parsed: ChatResponse = resp.json().await?;
        Ok(parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_else(|| text.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(url: &str) -> LlmCorrectClient {
        LlmCorrectClient::new(url.into(), "gpt-4o-mini".into(), AuthKind::Bearer, "test-key".into())
    }

    #[tokio::test]
    async fn correct_fixes_known_term() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_body(
                r#"{"choices":[{"message":{"content":"CyberAgentの音声入力テスト"}}]}"#,
            )
            .create_async()
            .await;

        let dict = vec![("さいば".to_string(), "CyberAgent".to_string())];
        let out = client(&server.url()).correct("さいばの音声入力テスト", &dict).await.unwrap();
        assert_eq!(out, "CyberAgentの音声入力テスト");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn correct_skips_empty_dict() {
        let c = LlmCorrectClient::new("http://unused".into(), "gpt-4o-mini".into(), AuthKind::Bearer, "key".into());
        let out = c.correct("そのまま", &[]).await.unwrap();
        assert_eq!(out, "そのまま");
    }

    #[tokio::test]
    async fn set_api_key_reflected_in_next_request() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", "Bearer new-key")
            .with_status(200)
            .with_body(r#"{"choices":[{"message":{"content":"ok"}}]}"#)
            .create_async()
            .await;

        let c = client(&server.url());
        c.set_api_key("new-key".into());
        let dict = vec![("x".to_string(), "y".to_string())];
        c.correct("x", &dict).await.unwrap();
        mock.assert_async().await;
    }
}
