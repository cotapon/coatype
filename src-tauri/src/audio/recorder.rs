use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

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

    pub fn start(&mut self) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device"))?;
        let config = device.default_input_config()?;
        self.sample_rate = config.sample_rate().0;

        let samples = Arc::clone(&self.samples);
        samples.lock().unwrap().clear();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    let mut buf = samples.lock().unwrap();
                    buf.extend(
                        data.iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16),
                    );
                },
                |err| tracing::error!("stream error: {err}"),
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| samples.lock().unwrap().extend_from_slice(data),
                |err| tracing::error!("stream error: {err}"),
                None,
            )?,
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
