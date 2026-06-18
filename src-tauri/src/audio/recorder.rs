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

    pub fn start(&mut self, on_level: Option<LevelCb>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device"))?;
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
        samples.lock().unwrap().clear();

        // ~30fps 相当でレベルを通知するためのサンプル数 (モノラル換算の概算)。
        let level_step = (self.sample_rate as usize / 30).max(1);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut acc = LevelAccumulator::new(level_step, on_level);
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        let mut buf = samples.lock().unwrap();
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
                        samples.lock().unwrap().extend_from_slice(data);
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
        let samples = self.samples.lock().unwrap().clone();
        let rate = self.sample_rate;
        tracing::debug!("recorder: {} samples collected, releasing stream on background thread", samples.len());
        if let Some(stream) = self.stream.take() {
            // cpal::Stream は !Send だが、CoreAudio スレッドから離れた
            // スレッドで drop するだけなので実質安全。
            struct SendStream(cpal::Stream);
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
}
