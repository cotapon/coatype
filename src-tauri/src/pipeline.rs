use crate::api::whisper::WhisperClient;
use crate::audio::recorder::Recorder;
use crate::config::settings::ProviderConfig;
use crate::dictionary::llm_correct::LlmCorrectClient;
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryStore;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinHandle;

pub struct CurrentTask {
    pub join: JoinHandle<()>,
}

pub struct Pipeline {
    pub recorder: Mutex<Recorder>,
    // ダブル Arc パターン: 外側の Mutex で swap、内側の Arc でクローン後に非同期呼び出し
    pub client: Arc<Mutex<Arc<WhisperClient>>>,
    pub llm: Arc<Mutex<Option<Arc<LlmCorrectClient>>>>,
    pub dict: Arc<Mutex<Dictionary>>,
    pub history: Arc<HistoryStore>,
    pub translate: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<String>>,
    pub llm_correct: Arc<Mutex<bool>>,
    started_at: Mutex<Option<Instant>>,
    pub current_task: Mutex<Option<CurrentTask>>,
}

impl Pipeline {
    pub fn new(
        client: WhisperClient,
        llm: Option<LlmCorrectClient>,
        dict: Dictionary,
        history: Arc<HistoryStore>,
        language: String,
        translate: bool,
        llm_correct: bool,
    ) -> Self {
        Self {
            recorder: Mutex::new(Recorder::new()),
            client: Arc::new(Mutex::new(Arc::new(client))),
            llm: Arc::new(Mutex::new(llm.map(Arc::new))),
            dict: Arc::new(Mutex::new(dict)),
            history,
            translate: Arc::new(Mutex::new(translate)),
            language: Arc::new(Mutex::new(language)),
            llm_correct: Arc::new(Mutex::new(llm_correct)),
            started_at: Mutex::new(None),
            current_task: Mutex::new(None),
        }
    }

    // ── API キー更新 ──────────────────────────────────────────────

    pub fn update_api_key(&self, key: String) {
        self.client.lock().unwrap().set_api_key(key.clone());
        if let Some(llm) = &*self.llm.lock().unwrap() {
            llm.set_api_key(key);
        }
    }

    pub fn update_stt_api_key(&self, key: String) {
        self.client.lock().unwrap().set_api_key(key);
    }

    pub fn update_llm_api_key(&self, key: String) {
        if let Some(llm) = &*self.llm.lock().unwrap() {
            llm.set_api_key(key);
        }
    }

    // ── クライアント再構築 ───────────────────────────────────────

    pub fn rebuild_stt_client(&self, config: &ProviderConfig, api_key: String) {
        let new_client = Arc::new(WhisperClient::new(
            config.base_url.clone(),
            config.model.clone(),
            config.auth_kind.clone(),
            api_key,
        ));
        *self.client.lock().unwrap() = new_client;
    }

    pub fn rebuild_llm_client(&self, config: &ProviderConfig, api_key: String) {
        let new_client = Arc::new(LlmCorrectClient::new(
            config.base_url.clone(),
            config.model.clone(),
            config.auth_kind.clone(),
            api_key,
        ));
        *self.llm.lock().unwrap() = Some(new_client);
    }

    // ── 録音 / 処理 ───────────────────────────────────────────────

    pub fn start(&self) -> anyhow::Result<()> {
        self.recorder.lock().unwrap().start()?;
        *self.started_at.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    pub async fn stop_and_process(&self) -> anyhow::Result<String> {
        let wav = self.recorder.lock().unwrap().stop();
        let elapsed = self
            .started_at
            .lock()
            .unwrap()
            .take()
            .map(|s| s.elapsed().as_millis() as i64)
            .unwrap_or(0);
        if wav.len() <= 44 {
            return Ok(String::new());
        }
        if wav_is_silent(&wav) {
            tracing::debug!("silent audio, skipping transcription");
            return Ok(String::new());
        }

        let translate = *self.translate.lock().unwrap();
        let language = self.language.lock().unwrap().clone();

        let client = self.client.lock().unwrap().clone();
        let raw = if translate {
            client.translate(&wav).await?
        } else {
            client.transcribe(&wav, &language, None).await?
        };

        let after_dict = self.dict.lock().unwrap().apply(&raw);

        let final_text = if *self.llm_correct.lock().unwrap() {
            let llm_opt = self.llm.lock().unwrap().clone();
            if let Some(llm) = llm_opt {
                let entries: Vec<(String, String)> = self
                    .dict
                    .lock()
                    .unwrap()
                    .entries
                    .iter()
                    .map(|e| (e.from.clone(), e.to.clone()))
                    .collect();
                llm.correct(&after_dict, &entries).await.unwrap_or(after_dict)
            } else {
                after_dict
            }
        } else {
            after_dict
        };

        self.history.insert(&final_text, &language, translate, elapsed)?;
        Ok(final_text)
    }

    /// 録音を破棄し、処理中タスクを強制終了する。
    pub fn cancel(&self) {
        self.recorder.lock().unwrap().stop();
        *self.started_at.lock().unwrap() = None;
        if let Some(task) = self.current_task.lock().unwrap().take() {
            task.join.abort();
        }
    }

    /// 最後の文字起こし結果を返す。履歴が空なら None。
    pub fn last_transcription(&self) -> Option<String> {
        self.history
            .list(1)
            .ok()
            .and_then(|items| items.into_iter().next().map(|i| i.text))
    }
}

/// WAV の i16 サンプルから RMS を計算し、閾値以下なら true を返す。
/// Whisper は無音・環境音のみの音声に対して「ありがとうございます」などを幻覚するため、
/// 送信前にここで弾く。閾値は i16::MAX (32767) の約 1% = 300。
fn wav_is_silent(wav: &[u8]) -> bool {
    const WAV_HEADER: usize = 44;
    const SILENCE_THRESHOLD_RMS: f64 = 300.0;

    let sample_bytes = &wav[WAV_HEADER..];
    if sample_bytes.len() < 2 {
        return true;
    }
    let sum_sq: f64 = sample_bytes
        .chunks_exact(2)
        .map(|c| {
            let s = i16::from_le_bytes([c[0], c[1]]) as f64;
            s * s
        })
        .sum();
    let count = (sample_bytes.len() / 2) as f64;
    let rms = (sum_sq / count).sqrt();
    rms < SILENCE_THRESHOLD_RMS
}
