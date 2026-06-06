use crate::api::error::ApiError;
use reqwest::multipart::{Form, Part};

pub struct WhisperClient {
    base: String,
    api_key: std::sync::Arc<std::sync::Mutex<String>>,
    http: reqwest::Client,
}

#[derive(serde::Deserialize)]
struct WhisperResponse {
    text: String,
}

impl WhisperClient {
    pub fn new(base: String, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client");
        Self {
            base,
            api_key: std::sync::Arc::new(std::sync::Mutex::new(api_key)),
            http,
        }
    }

    pub fn set_api_key(&self, key: String) {
        *self.api_key.lock().unwrap() = key;
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
            .text("model", "whisper-large-v3")
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
            .text("model", "whisper-large-v3");
        self.post("/v1/audio/translations", form).await
    }

    async fn post(&self, path: &str, form: Form) -> Result<String, ApiError> {
        let url = format!("{}{}", self.base, path);
        let key = self.api_key.lock().unwrap().clone();
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&key)
            .multipart(form)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Status { status, body });
        }
        let parsed: WhisperResponse = resp.json().await?;
        Ok(parsed.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn transcribe_returns_text() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/audio/transcriptions")
            .with_status(200)
            .with_body(r#"{"text":"こんにちは","usage":{"type":"duration","seconds":1}}"#)
            .create_async()
            .await;

        let client = WhisperClient::new(server.url(), "test-key".into());
        let bytes = b"RIFF....fake".to_vec();
        let text = client.transcribe(&bytes, "ja", None).await.unwrap();
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

        let client = WhisperClient::new(server.url(), "test-key".into());
        let bytes = b"RIFF....fake".to_vec();
        let text = client.translate(&bytes).await.unwrap();
        assert_eq!(text, "Hello");
        mock.assert_async().await;
    }
}
