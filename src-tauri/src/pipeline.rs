use crate::api::whisper::WhisperClient;
use crate::audio::recorder::{LevelCb, Recorder};
use crate::config::settings::ProviderConfig;
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryStore;
use crate::util::lock_or_recover;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinHandle;

pub struct CurrentTask {
    pub join: JoinHandle<()>,
    /// 採番された世代。完了時のクリーンアップが「自分が登録したタスク」の
    /// ときだけ current_task を解放するための照合キー。これがないと、
    /// 旧タスクの完了処理が後から登録された新タスクのハンドルを握り潰し、
    /// Cancel が新タスクに効かなくなる。
    pub gen: u64,
}

pub struct Pipeline {
    pub recorder: Mutex<Recorder>,
    // ダブル Arc パターン: 外側の Mutex で swap、内側の Arc でクローン後に非同期呼び出し
    pub client: Arc<Mutex<Arc<WhisperClient>>>,
    pub dict: Arc<Mutex<Dictionary>>,
    pub history: Arc<HistoryStore>,
    pub translate: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<String>>,
    /// 選択中の入力デバイス名。None ならシステムのデフォルトマイクを使う。
    /// save_settings から update_input_device 経由で即時反映される (api_key と同じパターン)。
    pub input_device: Arc<Mutex<Option<String>>>,
    started_at: Mutex<Option<Instant>>,
    pub current_task: Mutex<Option<CurrentTask>>,
    task_gen: AtomicU64,
}

impl Pipeline {
    pub fn new(
        client: WhisperClient,
        dict: Dictionary,
        history: Arc<HistoryStore>,
        language: String,
        translate: bool,
        input_device: Option<String>,
    ) -> Self {
        Self {
            recorder: Mutex::new(Recorder::new()),
            client: Arc::new(Mutex::new(Arc::new(client))),
            dict: Arc::new(Mutex::new(dict)),
            history,
            translate: Arc::new(Mutex::new(translate)),
            language: Arc::new(Mutex::new(language)),
            input_device: Arc::new(Mutex::new(input_device)),
            started_at: Mutex::new(None),
            current_task: Mutex::new(None),
            task_gen: AtomicU64::new(0),
        }
    }

    /// current_task 用の世代番号を採番する。
    pub fn next_task_gen(&self) -> u64 {
        self.task_gen.fetch_add(1, Ordering::Relaxed)
    }

    // ── API キー更新 ──────────────────────────────────────────────

    pub fn update_api_key(&self, key: String) {
        lock_or_recover(&self.client).set_api_key(key);
    }

    // ── 入力デバイス更新 ──────────────────────────────────────────

    /// 選択中の入力デバイスを更新する。次回の録音開始から反映される (再起動不要)。
    pub fn update_input_device(&self, device: Option<String>) {
        *lock_or_recover(&self.input_device) = device;
    }

    // ── クライアント再構築 ───────────────────────────────────────

    pub fn rebuild_stt_client(&self, config: &ProviderConfig, api_key: String) {
        let new_client = Arc::new(WhisperClient::new(
            config.base_url.clone(),
            config.model.clone(),
            config.auth_kind.clone(),
            api_key,
        ));
        *lock_or_recover(&self.client) = new_client;
    }

    // ── 録音 / 処理 ───────────────────────────────────────────────

    pub fn start(&self, on_level: Option<LevelCb>) -> anyhow::Result<()> {
        // 未設定のまま録音 → HTTP 401 で初めて失敗、という体験を避けるための事前ガード
        if !lock_or_recover(&self.client).is_configured() {
            anyhow::bail!(
                "STT の設定が未完了です。設定画面でエンドポイントと API キーを確認してください"
            );
        }
        tracing::info!("recording: start");
        let device = lock_or_recover(&self.input_device).clone();
        lock_or_recover(&self.recorder).start(device.as_deref(), on_level)?;
        *lock_or_recover(&self.started_at) = Some(Instant::now());
        Ok(())
    }

    pub async fn stop_and_process(&self) -> anyhow::Result<String> {
        tracing::info!("stop_and_process: acquiring recorder lock");
        let wav = {
            let mut rec = lock_or_recover(&self.recorder);
            tracing::info!("stop_and_process: lock acquired, calling stop()");
            rec.stop()
        };
        tracing::info!("stop_and_process: recorder.stop() returned");
        let elapsed = lock_or_recover(&self.started_at)
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

        let translate = *lock_or_recover(&self.translate);
        let language = lock_or_recover(&self.language).clone();

        tracing::info!("stt: request (translate={translate}, language={language})");
        let client = lock_or_recover(&self.client).clone();
        let raw = if translate {
            client.translate(&wav).await?
        } else {
            client.transcribe(&wav, &language, None).await?
        };
        tracing::info!("stt: response {:?}", raw);

        let after_dict = lock_or_recover(&self.dict).apply(&raw);
        if after_dict != raw {
            tracing::debug!("dict: {:?} -> {:?}", raw, after_dict);
        }

        let final_text = after_dict;

        self.history
            .insert(&final_text, &language, translate, elapsed)?;
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
            let mut rec = lock_or_recover(&self.recorder);
            rec.stop()
        };
        *lock_or_recover(&self.started_at) = None;

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

        let translate = *lock_or_recover(&self.translate);
        let language = lock_or_recover(&self.language).clone();
        let client = lock_or_recover(&self.client).clone();
        let raw = if translate {
            client.translate(&wav).await?
        } else {
            client.transcribe(&wav, &language, None).await?
        };
        tracing::info!("test recording: stt result {:?}", raw);
        Ok(lock_or_recover(&self.dict).apply(&raw))
    }

    /// 録音を破棄し、処理中タスクを強制終了する。
    pub fn cancel(&self) {
        lock_or_recover(&self.recorder).stop();
        *lock_or_recover(&self.started_at) = None;
        if let Some(task) = lock_or_recover(&self.current_task).take() {
            task.join.abort();
        }
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
