use crate::api::auth::{apply_auth, AuthKind};
use crate::api::error::ApiError;
use crate::util::lock_or_recover;
use reqwest::multipart::{Form, Part};
use std::sync::{Arc, Mutex};

pub struct WhisperClient {
    base: String,
    model: String,
    auth_kind: AuthKind,
    api_key: Arc<Mutex<String>>,
    http: reqwest::Client,
}

#[derive(serde::Deserialize)]
struct WhisperResponse {
    text: String,
}

impl WhisperClient {
    pub fn new(base: String, model: String, auth_kind: AuthKind, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client");
        Self {
            base,
            model,
            auth_kind,
            api_key: Arc::new(Mutex::new(api_key)),
            http,
        }
    }

    pub fn set_api_key(&self, key: String) {
        *lock_or_recover(&self.api_key) = key;
    }

    /// STT が使える状態かを返す。録音前の事前チェックで弾くために使う。
    /// base URL は常に必須。API キーは認証が必要な auth_kind の場合のみ必須
    /// (`AuthKind::None` の認証なしローカルエンドポイントはキーなしで使える)。
    pub fn is_configured(&self) -> bool {
        if self.base.trim().is_empty() {
            return false;
        }
        match self.auth_kind {
            AuthKind::None => true,
            _ => !lock_or_recover(&self.api_key).trim().is_empty(),
        }
    }

    pub async fn transcribe(
        &self,
        wav: &[u8],
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, ApiError> {
        let mut form = Form::new()
            .part(
                "file",
                Part::bytes(wav.to_vec())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .unwrap(),
            )
            .text("model", self.model.clone())
            .text("language", language.to_string());
        if let Some(p) = prompt {
            form = form.text("prompt", p.to_string());
        }
        self.post("/v1/audio/transcriptions", form).await
    }

    pub async fn translate(&self, wav: &[u8]) -> Result<String, ApiError> {
        let form = Form::new()
            .part(
                "file",
                Part::bytes(wav.to_vec())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .unwrap(),
            )
            .text("model", self.model.clone());
        self.post("/v1/audio/translations", form).await
    }

    async fn post(&self, path: &str, form: Form) -> Result<String, ApiError> {
        let url = format!("{}{}", self.base, path);
        tracing::debug!("whisper: POST {url}");
        let key = lock_or_recover(&self.api_key).clone();
        let resp = apply_auth(self.http.post(&url), &self.auth_kind, &key)
            .multipart(form)
            .send()
            .await?;
        let status = resp.status();
        tracing::debug!("whisper: response status {status}");
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("whisper: error {}: {body}", status.as_u16());
            return Err(ApiError::Status {
                status: status.as_u16(),
                body,
            });
        }
        let parsed: WhisperResponse = resp.json().await?;
        Ok(parsed.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(url: &str) -> WhisperClient {
        WhisperClient::new(
            url.into(),
            "whisper-large-v3".into(),
            AuthKind::Bearer,
            "test-key".into(),
        )
    }

    #[test]
    fn is_configured_requires_base_url() {
        let c = WhisperClient::new(String::new(), "m".into(), AuthKind::Bearer, "key".into());
        assert!(!c.is_configured());
    }

    #[test]
    fn is_configured_requires_key_when_auth_needed() {
        let c = WhisperClient::new(
            "http://localhost:9000".into(),
            "m".into(),
            AuthKind::Bearer,
            String::new(),
        );
        assert!(!c.is_configured());
        c.set_api_key("k".into());
        assert!(c.is_configured());
    }

    #[test]
    fn is_configured_allows_empty_key_for_auth_none() {
        // 認証なしのローカル STT エンドポイントは API キーなしで使える
        let c = WhisperClient::new(
            "http://localhost:9000".into(),
            "m".into(),
            AuthKind::None,
            String::new(),
        );
        assert!(c.is_configured());
    }

    #[tokio::test]
    async fn transcribe_returns_text() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/audio/transcriptions")
            .with_status(200)
            .with_body(r#"{"text":"こんにちは","usage":{"type":"duration","seconds":1}}"#)
            .create_async()
            .await;

        let text = client(&server.url())
            .transcribe(b"RIFF....fake", "ja", None)
            .await
            .unwrap();
        assert_eq!(text, "こんにちは");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn translate_returns_text() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/audio/translations")
            .with_status(200)
            .with_body(r#"{"text":"Hello"}"#)
            .create_async()
            .await;

        let text = client(&server.url())
            .translate(b"RIFF....fake")
            .await
            .unwrap();
        assert_eq!(text, "Hello");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn custom_model_name_sent_in_request() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/audio/transcriptions")
            .match_body(mockito::Matcher::Regex("custom-model".to_string()))
            .with_status(200)
            .with_body(r#"{"text":"ok"}"#)
            .create_async()
            .await;

        let c = WhisperClient::new(
            server.url(),
            "custom-model".into(),
            AuthKind::Bearer,
            "key".into(),
        );
        c.transcribe(b"RIFF....fake", "ja", None).await.unwrap();
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn api_key_header_auth_used() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/audio/transcriptions")
            .match_header("x-api-key", "my-secret")
            .with_status(200)
            .with_body(r#"{"text":"ok"}"#)
            .create_async()
            .await;

        let c = WhisperClient::new(
            server.url(),
            "whisper-1".into(),
            AuthKind::ApiKeyHeader {
                header_name: "x-api-key".into(),
            },
            "my-secret".into(),
        );
        c.transcribe(b"RIFF....fake", "ja", None).await.unwrap();
        mock.assert_async().await;
    }
}
