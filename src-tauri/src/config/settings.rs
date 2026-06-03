use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    PushToTalk,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub language: String,
    pub shortcut: String,
    pub trigger_mode: TriggerMode,
    pub translate_mode: bool,
    pub translate_model: Option<String>,
    pub llm_correct: bool,
    pub api_base: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "ja".into(),
            shortcut: "RightOption".into(),
            trigger_mode: TriggerMode::PushToTalk,
            translate_mode: false,
            translate_model: None,
            llm_correct: false,
            api_base: "https://genai.mlplatform.apis.platform.cycloud.jp".into(),
        }
    }
}

impl Settings {
    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_sane() {
        let s = Settings::default();
        assert_eq!(s.language, "ja");
        assert!(matches!(s.trigger_mode, TriggerMode::PushToTalk));
        assert_eq!(s.shortcut, "RightOption");
    }

    #[test]
    fn settings_roundtrip_json() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }
}
