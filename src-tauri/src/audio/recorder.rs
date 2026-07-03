use crate::util::lock_or_recover;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

/// 録音中の音量レベル (RMS, 0.0〜1.0 付近) を受け取るコールバック。
/// オーバーレイの波形表示用に、録音ループから定期的に呼ばれる。
///
/// これは cpal の音声入力コールバックスレッド上で呼ばれるため、
/// 中で重い処理 (IPC など) をしてはならない。値をチャネルに送るなどの
/// 非ブロッキングな処理に留め、実際の emit は別スレッドで行うこと。
pub type LevelCb = Box<dyn FnMut(f32) + Send>;

pub struct Recorder {
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    stream: Option<cpal::Stream>,
}

// cpal::Stream is !Send, but Recorder is always accessed through a Mutex<Recorder>
// and the stream is never sent across threads — only created/dropped on the same thread
// or while holding the lock. This is safe in practice.
unsafe impl Send for Recorder {}

impl Recorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
            stream: None,
        }
    }

    /// `device` に名前を指定すると、そのデバイスを cpal のデバイス一覧から探して使う。
    /// 見つからない場合 (抜線・改名など) はシステムのデフォルト入力デバイスにフォールバックする。
    /// `None` の場合は最初からデフォルトデバイスを使う。
    pub fn start(&mut self, device: Option<&str>, on_level: Option<LevelCb>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let available: Vec<String> = host
            .input_devices()?
            .filter_map(|d| d.name().ok())
            .collect();
        let device = match resolve_device_name(&available, device) {
            Some(name) => host
                .input_devices()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .or_else(|| host.default_input_device()),
            None => {
                if let Some(name) = device {
                    tracing::warn!(
                        "recorder: requested input device '{name}' not found, falling back to default"
                    );
                }
                host.default_input_device()
            }
        };
        let device = device.ok_or_else(|| anyhow::anyhow!("no input device"))?;
        let config = device.default_input_config()?;
        self.sample_rate = config.sample_rate().0;
        tracing::info!(
            "recorder: device={:?}, format={:?}, channels={}, sample_rate={}",
            device.name().unwrap_or_else(|_| "?".into()),
            config.sample_format(),
            config.channels(),
            config.sample_rate().0,
        );

        let samples = Arc::clone(&self.samples);
        lock_or_recover(&samples).clear();

        // ~30fps 相当でレベルを通知するためのサンプル数 (モノラル換算の概算)。
        let level_step = (self.sample_rate as usize / 30).max(1);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut acc = LevelAccumulator::new(level_step, on_level);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        let mut buf = lock_or_recover(&samples);
                        buf.extend(
                            data.iter()
                                .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16),
                        );
                        drop(buf);
                        acc.push_iter(data.iter().map(|&s| s.clamp(-1.0, 1.0) as f64));
                    },
                    |err| tracing::error!("stream error: {err}"),
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let mut acc = LevelAccumulator::new(level_step, on_level);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _| {
                        lock_or_recover(&samples).extend_from_slice(data);
                        acc.push_iter(data.iter().map(|&s| s as f64 / i16::MAX as f64));
                    },
                    |err| tracing::error!("stream error: {err}"),
                    None,
                )?
            }
            other => anyhow::bail!("unsupported sample format: {other:?}"),
        };
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Vec<u8> {
        // サンプルを先に回収してからストリームを解放する。
        // cpal::Stream の drop は CoreAudio スレッドとの同期を取るため、
        // CFRunLoop を持たない tokio ワーカースレッドではデッドロックする。
        // 別スレッドで drop することで回避する。
        tracing::debug!("recorder: locking samples");
        let samples = lock_or_recover(&self.samples).clone();
        let rate = self.sample_rate;
        tracing::debug!(
            "recorder: {} samples collected, releasing stream on background thread",
            samples.len()
        );
        if let Some(stream) = self.stream.take() {
            // cpal::Stream は !Send だが、CoreAudio スレッドから離れた
            // スレッドで drop するだけなので実質安全。
            // フィールドは読まれないが、所有権を移して Drop させるために必要。
            struct SendStream(#[allow(dead_code)] cpal::Stream);
            unsafe impl Send for SendStream {}
            let s = SendStream(stream);
            std::thread::spawn(move || drop(s));
        }
        samples_to_wav(&samples, rate)
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

/// 録音ループのコールバック内でサンプルを溜め、一定数ごとに RMS を計算して
/// `on_level` へ通知する。コールバックは高頻度で呼ばれるため、ここで間引く。
struct LevelAccumulator {
    step: usize,
    on_level: Option<LevelCb>,
    sum_sq: f64,
    count: usize,
}

impl LevelAccumulator {
    fn new(step: usize, on_level: Option<LevelCb>) -> Self {
        Self {
            step,
            on_level,
            sum_sq: 0.0,
            count: 0,
        }
    }

    /// 正規化済みサンプル (-1.0〜1.0) のイテレータを受け取り、必要なら通知する。
    fn push_iter(&mut self, samples: impl Iterator<Item = f64>) {
        if self.on_level.is_none() {
            return;
        }
        for v in samples {
            self.sum_sq += v * v;
            self.count += 1;
            if self.count >= self.step {
                let rms = (self.sum_sq / self.count as f64).sqrt() as f32;
                if let Some(cb) = &mut self.on_level {
                    cb(rms);
                }
                self.sum_sq = 0.0;
                self.count = 0;
            }
        }
    }
}

/// 利用可能な入力デバイス名の一覧を返す (設定画面のマイク選択セレクト用)。
/// 名前取得に失敗したデバイスはスキップし、重複は取り除く。
pub fn list_input_device_names() -> anyhow::Result<Vec<String>> {
    let host = cpal::default_host();
    // Vec::dedup は隣接する重複しか除去しないため、順序を保ちつつ HashSet で重複を落とす
    let mut seen = std::collections::HashSet::new();
    let names: Vec<String> = host
        .input_devices()?
        .filter_map(|d| d.name().ok())
        .filter(|n| seen.insert(n.clone()))
        .collect();
    Ok(names)
}

/// 要求されたデバイス名が利用可能な一覧に含まれていればそれを返し、
/// 含まれていなければ (未検出/未指定) None (= システムのデフォルトを使う) を返す。
/// cpal 実機に依存しない純粋関数として切り出し、フォールバック判定をテスト可能にする。
pub fn resolve_device_name(available: &[String], requested: Option<&str>) -> Option<String> {
    let requested = requested?;
    available.iter().find(|n| n.as_str() == requested).cloned()
}

pub fn samples_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
    }
    cursor.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_to_wav_writes_riff_header() {
        let samples = vec![0i16; 16000];
        let bytes = samples_to_wav(&samples, 16000);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert!(bytes.len() > 44 + samples.len() * 2 - 1);
    }

    #[test]
    fn resolve_device_name_returns_requested_when_available() {
        let available = vec![
            "Built-in Microphone".to_string(),
            "USB Microphone".to_string(),
        ];
        assert_eq!(
            resolve_device_name(&available, Some("USB Microphone")),
            Some("USB Microphone".to_string())
        );
    }

    #[test]
    fn resolve_device_name_falls_back_to_default_when_not_found() {
        let available = vec!["Built-in Microphone".to_string()];
        assert_eq!(resolve_device_name(&available, Some("Missing Mic")), None);
    }

    #[test]
    fn resolve_device_name_returns_none_when_not_requested() {
        let available = vec!["Built-in Microphone".to_string()];
        assert_eq!(resolve_device_name(&available, None), None);
    }
}
