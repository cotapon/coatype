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
        tracing::info!("recording: start");
        self.recorder.lock().unwrap().start()?;
        *self.started_at.lock().unwrap() = Some(Instant::now());
        Ok(())
    }

    pub async fn stop_and_process(&self) -> anyhow::Result<String> {
        tracing::info!("stop_and_process: acquiring recorder lock");
        let wav = {
            let mut rec = self.recorder.lock().unwrap();
            tracing::info!("stop_and_process: lock acquired, calling stop()");
            rec.stop()
        };
        tracing::info!("stop_and_process: recorder.stop() returned");
        let elapsed = self
            .started_at
            .lock()
            .unwrap()
            .take()
            .map(|s| s.elapsed().as_millis() as i64)
            .unwrap_or(0);
        tracing::info!("recording: stop ({elapsed}ms, {} bytes)", wav.len());

        if wav.len() <= 44 {
            tracing::debug!("wav too short, skipping");
            return Ok(String::new());
        }
        if wav_is_silent(&wav) {
            tracing::info!("silent audio, skipping transcription");
            return Ok(String::new());
        }

        let translate = *self.translate.lock().unwrap();
        let language = self.language.lock().unwrap().clone();

        tracing::info!("stt: request (translate={translate}, language={language})");
        let client = self.client.lock().unwrap().clone();
        let raw = if translate {
            client.translate(&wav).await?
        } else {
            client.transcribe(&wav, &language, None).await?
        };
        tracing::info!("stt: response {:?}", raw);

        let after_dict = self.dict.lock().unwrap().apply(&raw);
        if after_dict != raw {
            tracing::debug!("dict: {:?} -> {:?}", raw, after_dict);
        }

        let final_text = if *self.llm_correct.lock().unwrap() {
            let llm_opt = self.llm.lock().unwrap().clone();
            if let Some(llm) = llm_opt {
                tracing::info!("llm: correct request");
                let entries: Vec<(String, String)> = self
                    .dict
                    .lock()
                    .unwrap()
                    .entries
                    .iter()
                    .map(|e| (e.from.clone(), e.to.clone()))
                    .collect();
                let result = llm.correct(&after_dict, &entries).await.unwrap_or_else(|e| {
                    tracing::warn!("llm: correct failed: {e}");
                    after_dict.clone()
                });
                tracing::info!("llm: result {:?}", result);
                result
            } else {
                tracing::debug!("llm: skipped (no client)");
                after_dict
            }
        } else {
            after_dict
        };

        self.history.insert(&final_text, &language, translate, elapsed)?;
        Ok(final_text)
    }

    /// 録音テスト用: 録音を停止して文字起こしのみ行う。
    /// 履歴への保存・LLM 補正・テキスト挿入は行わず、辞書置換のみ適用して結果を返す。
    /// 設定画面の「録音テスト」ボタンから呼ばれる。
    ///
    /// 本番の `stop_and_process` と違い、無音スキップ (`wav_is_silent`) は行わない。
    /// テストはユーザーが明示的に開始するものなので、捕捉した音声はそのまま STT に送る。
    /// マイクからサンプルを 1 件も取得できなかった場合のみエラーを返す
    /// (マイク権限やデバイス選択の問題を切り分けやすくするため)。
    pub async fn stop_and_transcribe_test(&self) -> anyhow::Result<String> {
        let wav = {
            let mut rec = self.recorder.lock().unwrap();
            rec.stop()
        };
        *self.started_at.lock().unwrap() = None;

        let sample_count = wav.len().saturating_sub(44) / 2;
        tracing::info!(
            "test recording: stop ({} bytes, ~{} samples)",
            wav.len(),
            sample_count
        );

        if sample_count == 0 {
            anyhow::bail!(
                "マイクから音声を取得できませんでした。マイクの権限と入力デバイスを確認してください"
            );
        }

        let translate = *self.translate.lock().unwrap();
        let language = self.language.lock().unwrap().clone();
        let client = self.client.lock().unwrap().clone();
        let raw = if translate {
            client.translate(&wav).await?
        } else {
            client.transcribe(&wav, &language, None).await?
        };
        tracing::info!("test recording: stt result {:?}", raw);
        Ok(self.dict.lock().unwrap().apply(&raw))
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
