use crate::api::error::ApiError;
use serde::{Deserialize, Serialize};

pub struct LlmCorrectClient {
    base: String,
    api_key: String,
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
    pub fn new(base: String, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap();
        Self { base, api_key, http }
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
            model: "gpt-4o-mini".into(),
            messages: vec![
                ChatMsg { role: "system".into(), content: system },
                ChatMsg { role: "user".into(), content: text.to_string() },
            ],
            temperature: 0.0,
            max_tokens: 512,
        };
        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base))
            .bearer_auth(&self.api_key)
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

        let client = LlmCorrectClient::new(server.url(), "test-key".into());
        let dict = vec![("さいば".to_string(), "CyberAgent".to_string())];
        let out = client.correct("さいばの音声入力テスト", &dict).await.unwrap();
        assert_eq!(out, "CyberAgentの音声入力テスト");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn correct_skips_empty_dict() {
        let client = LlmCorrectClient::new("http://unused".into(), "key".into());
        let out = client.correct("そのまま", &[]).await.unwrap();
        assert_eq!(out, "そのまま");
    }
}
