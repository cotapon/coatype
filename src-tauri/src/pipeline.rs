use crate::api::whisper::WhisperClient;
use crate::audio::recorder::Recorder;
use crate::dictionary::llm_correct::LlmCorrectClient;
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryStore;
use crate::injector;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Pipeline {
    pub recorder: Mutex<Recorder>,
    pub client: WhisperClient,
    pub llm: Option<LlmCorrectClient>,
    pub dict: Arc<Mutex<Dictionary>>,
    pub history: Arc<HistoryStore>,
    pub translate: Arc<Mutex<bool>>,
    pub language: Arc<Mutex<String>>,
    pub llm_correct: Arc<Mutex<bool>>,
    started_at: Mutex<Option<Instant>>,
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
            client,
            llm,
            dict: Arc::new(Mutex::new(dict)),
            history,
            translate: Arc::new(Mutex::new(translate)),
            language: Arc::new(Mutex::new(language)),
            llm_correct: Arc::new(Mutex::new(llm_correct)),
            started_at: Mutex::new(None),
        }
    }

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

        let translate = *self.translate.lock().unwrap();
        let language = self.language.lock().unwrap().clone();
        let raw = if translate {
            self.client.translate(&wav).await?
        } else {
            self.client.transcribe(&wav, &language, None).await?
        };

        // 高速パス: 完全一致置換 (常時)
        let after_dict = self.dict.lock().unwrap().apply(&raw);

        // LLM 補正パス (設定 ON かつ llm クライアントあり)
        let final_text = if *self.llm_correct.lock().unwrap() {
            if let Some(llm) = &self.llm {
                let entries: Vec<(String, String)> = self
                    .dict
                    .lock()
                    .unwrap()
                    .entries
                    .iter()
                    .map(|e| (e.from.clone(), e.to.clone()))
                    .collect();
                llm.correct(&after_dict, &entries)
                    .await
                    .unwrap_or(after_dict)
            } else {
                after_dict
            }
        } else {
            after_dict
        };

        self.history
            .insert(&final_text, &language, translate, elapsed)?;
        injector::insert(&final_text)?;
        Ok(final_text)
    }
}
