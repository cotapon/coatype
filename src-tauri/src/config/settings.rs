use crate::api::auth::AuthKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerMode {
    PushToTalk,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub auth_kind: AuthKind,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self { base_url: String::new(), model: String::new(), auth_kind: AuthKind::Bearer }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub language: String,
    pub shortcut: String,
    pub trigger_mode: TriggerMode,
    pub translate_mode: bool,
    pub translate_model: Option<String>,
    pub llm_correct: bool,
    #[serde(default)]
    pub stt: ProviderConfig,
    #[serde(default)]
    pub llm: ProviderConfig,
    #[serde(default)]
    pub separate_api_keys: bool,
    // 旧 api_base フィールドの読み取り専用エイリアス。load 後に stt/llm へ転写され None になる。
    #[serde(default, rename = "api_base", skip_serializing_if = "Option::is_none")]
    pub legacy_api_base: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        let base = Self::default_base();
        Self {
            language: "ja".into(),
            shortcut: "rightoption".into(),
            trigger_mode: TriggerMode::PushToTalk,
            translate_mode: false,
            translate_model: None,
            llm_correct: false,
            stt: ProviderConfig {
                base_url: base.clone(),
                model: "whisper-large-v3".into(),
                auth_kind: AuthKind::Bearer,
            },
            llm: ProviderConfig {
                base_url: base,
                model: "gpt-4o-mini".into(),
                auth_kind: AuthKind::Bearer,
            },
            separate_api_keys: false,
            legacy_api_base: None,
        }
    }
}

impl Settings {
    fn default_base() -> String {
        "https://genai.mlplatform.apis.platform.cycloud.jp".to_string()
    }

    /// 旧 api_base フィールドを stt/llm に転写し、空のモデル名にデフォルトを充填する。
    fn migrate_legacy(&mut self) {
        let base = Self::default_base();
        if let Some(old) = self.legacy_api_base.take() {
            if self.stt.base_url.is_empty() {
                self.stt.base_url = old.clone();
            }
            if self.llm.base_url.is_empty() {
                self.llm.base_url = old;
            }
        }
        if self.stt.base_url.is_empty() {
            self.stt.base_url = base.clone();
        }
        if self.llm.base_url.is_empty() {
            self.llm.base_url = base;
        }
        if self.stt.model.is_empty() {
            self.stt.model = "whisper-large-v3".into();
        }
        if self.llm.model.is_empty() {
            self.llm.model = "gpt-4o-mini".into();
        }
    }

    pub fn load(path: &PathBuf) -> Self {
        let mut s: Settings = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        s.migrate_legacy();
        s
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
        assert_eq!(s.shortcut, "rightoption");
        assert_eq!(s.stt.model, "whisper-large-v3");
        assert_eq!(s.llm.model, "gpt-4o-mini");
        assert!(!s.separate_api_keys);
    }

    #[test]
    fn settings_roundtrip_json() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, parsed);
    }

    #[test]
    fn legacy_api_base_migrates_to_stt_and_llm() {
        let json = r#"{
            "language": "ja",
            "shortcut": "rightoption",
            "trigger_mode": "push_to_talk",
            "translate_mode": false,
            "llm_correct": false,
            "api_base": "https://custom.endpoint.example"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.stt.base_url, "https://custom.endpoint.example");
        assert_eq!(s.llm.base_url, "https://custom.endpoint.example");
        assert_eq!(s.stt.model, "whisper-large-v3");
        assert_eq!(s.llm.model, "gpt-4o-mini");
        assert!(s.legacy_api_base.is_none());
    }

    #[test]
    fn legacy_api_base_not_serialized_after_migration() {
        let json = r#"{
            "language": "ja",
            "shortcut": "rightoption",
            "trigger_mode": "push_to_talk",
            "translate_mode": false,
            "llm_correct": false,
            "api_base": "https://custom.endpoint.example"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        let saved = serde_json::to_string(&s).unwrap();
        assert!(!saved.contains("api_base"), "api_base must not be written after migration");
        assert!(saved.contains("stt"), "stt block must be present");
        assert!(saved.contains("llm"), "llm block must be present");
    }

    #[test]
    fn new_format_json_loads_without_migration() {
        let json = r#"{
            "language": "en",
            "shortcut": "f5",
            "trigger_mode": "toggle",
            "translate_mode": true,
            "llm_correct": true,
            "stt": {"base_url": "https://api.openai.com", "model": "whisper-1", "auth_kind": {"kind": "bearer"}},
            "llm": {"base_url": "https://api.openai.com", "model": "gpt-4", "auth_kind": {"kind": "bearer"}},
            "separate_api_keys": true
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.stt.base_url, "https://api.openai.com");
        assert_eq!(s.stt.model, "whisper-1");
        assert_eq!(s.llm.model, "gpt-4");
        assert!(s.separate_api_keys);
    }
}
