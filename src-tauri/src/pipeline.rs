use crate::api::whisper::WhisperClient;
use crate::audio::recorder::{LevelCb, Recorder};
use crate::config::settings::ProviderConfig;
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
    pub dict: Arc<Mutex<Dictionary>>,
    pub history: Arc<HistoryStore>,
    pub translate: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<String>>,
    started_at: Mutex<Option<Instant>>,
    pub current_task: Mutex<Option<CurrentTask>>,
}

impl Pipeline {
    pub fn new(
        client: WhisperClient,
        dict: Dictionary,
        history: Arc<HistoryStore>,
        language: String,
        translate: bool,
    ) -> Self {
        Self {
            recorder: Mutex::new(Recorder::new()),
            client: Arc::new(Mutex::new(Arc::new(client))),
            dict: Arc::new(Mutex::new(dict)),
            history,
            translate: Arc::new(Mutex::new(translate)),
            language: Arc::new(Mutex::new(language)),
            started_at: Mutex::new(None),
            current_task: Mutex::new(None),
        }
    }

    // ── API キー更新 ──────────────────────────────────────────────

    pub fn update_api_key(&self, key: String) {
        self.client.lock().unwrap().set_api_key(key);
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

    // ── 録音 / 処理 ───────────────────────────────────────────────

    pub fn start(&self, on_level: Option<LevelCb>) -> anyhow::Result<()> {
        tracing::info!("recording: start");
        self.recorder.lock().unwrap().start(on_level)?;
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
        let (rms, peak) = wav_stats(&wav);
        tracing::info!(
            "recording: rms={:.1}, peak={} (skip if rms<{} & peak<{})",
            rms,
            peak,
            SILENCE_THRESHOLD_RMS,
            SILENCE_THRESHOLD_PEAK
        );
        // 無音スキップは RMS とピーク振幅の両方が低いときだけ行う。
        // 低ゲインのマイクで音量が小さい実発話を、RMS だけで弾いてしまうのを防ぐ。
        // 発話があれば子音・破裂音などでピークが立つため、ピークが閾値を超えていれば通す。
        if rms < SILENCE_THRESHOLD_RMS && (peak as f64) < SILENCE_THRESHOLD_PEAK {
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

        let final_text = after_dict;

        self.history.insert(&final_text, &language, translate, elapsed)?;
        Ok(final_text)
    }

    /// 録音テスト用: 録音を停止して文字起こしのみ行う。
    /// 履歴への保存・LLM 補正・テキスト挿入は行わず、辞書置換のみ適用して結果を返す。
    /// 設定画面の「録音テスト」ボタンから呼ばれる。
    ///
    /// 本番の `stop_and_process` と違い、無音スキップは行わない。
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

const WAV_HEADER: usize = 44;
/// 無音判定の RMS 閾値。i16::MAX (32767) の約 1%。
const SILENCE_THRESHOLD_RMS: f64 = 300.0;
/// 無音判定のピーク振幅閾値。実発話なら子音・破裂音でこの値を超える。
/// RMS が低くてもピークがこれを超えていれば「無音ではない」と判断する。
const SILENCE_THRESHOLD_PEAK: f64 = 1200.0;

/// WAV の i16 サンプルから RMS とピーク絶対値を計算する。
/// Whisper は無音・環境音のみの音声に対して「ありがとうございます」などを幻覚するため、
/// 送信前にこの統計値で無音を弾く。
fn wav_stats(wav: &[u8]) -> (f64, i32) {
    let header = WAV_HEADER.min(wav.len());
    let sample_bytes = &wav[header..];
    if sample_bytes.len() < 2 {
        return (0.0, 0);
    }
    let mut sum_sq = 0.0_f64;
    let mut peak = 0_i32;
    for c in sample_bytes.chunks_exact(2) {
        let s = i16::from_le_bytes([c[0], c[1]]) as i32;
        sum_sq += (s as f64) * (s as f64);
        let a = s.abs();
        if a > peak {
            peak = a;
        }
    }
    let count = (sample_bytes.len() / 2) as f64;
    ((sum_sq / count).sqrt(), peak)
}
