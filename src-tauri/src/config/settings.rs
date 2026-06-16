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
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    StartRecord,
    HandsFree,
    Cancel,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyBinding {
    pub id: String,
    pub action: ActionKind,
    pub combo: String,
    pub enabled: bool,
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
    #[serde(default)]
    pub bindings: Vec<KeyBinding>,
    pub translate_mode: bool,
    pub translate_model: Option<String>,
    pub llm_correct: bool,
    #[serde(default)]
    pub stt: ProviderConfig,
    #[serde(default)]
    pub llm: ProviderConfig,
    #[serde(default)]
    pub separate_api_keys: bool,
    #[serde(default = "default_true")]
    pub show_overlay: bool,
    // 旧 api_base フィールドの読み取り専用エイリアス
    #[serde(default, rename = "api_base", skip_serializing_if = "Option::is_none")]
    pub legacy_api_base: Option<String>,
    // 旧 shortcut/trigger_mode: マイグレーション用 (読み取りのみ、書き込み時は省略)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_mode: Option<TriggerMode>,
}

fn default_true() -> bool {
    true
}

/// ロケール文字列から Whisper 用の言語コード(先頭サブタグ)を取り出す。
/// 例: "ja-JP" -> "ja", "zh-Hans-CN" -> "zh"。空なら None。
fn language_from_locale(locale: &str) -> Option<String> {
    let tag = locale
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    if tag.is_empty() {
        None
    } else {
        Some(tag)
    }
}

/// システムロケールから言語コードを推定する。取得・解析に失敗した場合は "ja"。
fn detect_system_language() -> String {
    sys_locale::get_locale()
        .as_deref()
        .and_then(language_from_locale)
        .unwrap_or_else(|| "ja".to_string())
}

impl Default for Settings {
    fn default() -> Self {
        let base = Self::default_base();
        Self {
            language: "ja".into(),
            bindings: vec![KeyBinding {
                id: "default-0".to_string(),
                action: ActionKind::StartRecord,
                combo: "rightoption".to_string(),
                enabled: true,
            }],
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
            show_overlay: true,
            legacy_api_base: None,
            shortcut: None,
            trigger_mode: None,
        }
    }
}

impl Settings {
    fn default_base() -> String {
        "https://genai.mlplatform.apis.platform.cycloud.jp".to_string()
    }

    fn migrate_legacy(&mut self) {
        let base = Self::default_base();

        // 旧 api_base を stt/llm に転写
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

        // 削除されたアクション (paste_last 等) を持つバインドを除去
        self.bindings.retain(|b| b.action != ActionKind::Unknown);

        // 旧 shortcut/trigger_mode を bindings に変換
        if self.bindings.is_empty() {
            let combo = self.shortcut.take().unwrap_or_else(|| "rightoption".to_string());
            let action = match self.trigger_mode.take().unwrap_or(TriggerMode::PushToTalk) {
                TriggerMode::PushToTalk => ActionKind::StartRecord,
                TriggerMode::Toggle => ActionKind::HandsFree,
            };
            self.bindings.push(KeyBinding {
                id: "migrated-0".to_string(),
                action,
                combo,
                enabled: true,
            });
        } else {
            // bindings が既にある場合は旧フィールドを消すだけ
            self.shortcut = None;
            self.trigger_mode = None;
        }
    }

    pub fn load(path: &PathBuf) -> Self {
        let existing: Option<Settings> = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok());
        let mut s = match existing {
            Some(s) => s,
            None => {
                // 初回起動 (設定ファイルなし): システム言語を初期デフォルトにする
                let mut d = Settings::default();
                d.language = detect_system_language();
                d
            }
        };
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
    fn language_from_locale_extracts_primary_subtag() {
        assert_eq!(language_from_locale("ja-JP").as_deref(), Some("ja"));
        assert_eq!(language_from_locale("en_US").as_deref(), Some("en"));
        assert_eq!(language_from_locale("zh-Hans-CN").as_deref(), Some("zh"));
        assert_eq!(language_from_locale("EN").as_deref(), Some("en"));
        assert_eq!(language_from_locale("fr").as_deref(), Some("fr"));
        assert_eq!(language_from_locale(""), None);
        assert_eq!(language_from_locale("  "), None);
    }

    #[test]
    fn detect_system_language_is_non_empty() {
        // get_locale() の戻り値は環境依存だが、失敗時も "ja" にフォールバックするため
        // 常に非空の言語コードを返すことを保証する。
        assert!(!detect_system_language().is_empty());
    }

    #[test]
    fn default_settings_are_sane() {
        let s = Settings::default();
        assert_eq!(s.language, "ja");
        assert_eq!(s.bindings.len(), 1);
        assert_eq!(s.bindings[0].combo, "rightoption");
        assert!(matches!(s.bindings[0].action, ActionKind::StartRecord));
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
    fn legacy_shortcut_trigger_migrates_to_bindings() {
        let json = r#"{
            "language": "ja",
            "shortcut": "rightoption",
            "trigger_mode": "push_to_talk",
            "translate_mode": false,
            "llm_correct": false
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.bindings.len(), 1);
        assert_eq!(s.bindings[0].combo, "rightoption");
        assert!(matches!(s.bindings[0].action, ActionKind::StartRecord));
        assert!(s.shortcut.is_none());
        assert!(s.trigger_mode.is_none());
    }

    #[test]
    fn legacy_toggle_migrates_to_hands_free() {
        let json = r#"{
            "language": "ja",
            "shortcut": "f5",
            "trigger_mode": "toggle",
            "translate_mode": false,
            "llm_correct": false
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.bindings.len(), 1);
        assert_eq!(s.bindings[0].combo, "f5");
        assert!(matches!(s.bindings[0].action, ActionKind::HandsFree));
    }

    #[test]
    fn existing_bindings_not_overwritten_by_migration() {
        let json = r#"{
            "language": "ja",
            "shortcut": "f5",
            "trigger_mode": "toggle",
            "translate_mode": false,
            "llm_correct": false,
            "bindings": [
                {"id": "b1", "action": "start_record", "combo": "rightoption", "enabled": true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.bindings.len(), 1);
        assert_eq!(s.bindings[0].id, "b1");
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
        assert!(!saved.contains("\"shortcut\""), "shortcut must not be written after migration");
        assert!(!saved.contains("trigger_mode"), "trigger_mode must not be written after migration");
        assert!(saved.contains("bindings"), "bindings block must be present");
    }

    #[test]
    fn new_format_json_loads_without_migration() {
        let json = r#"{
            "language": "en",
            "bindings": [
                {"id": "b1", "action": "start_record", "combo": "leftoption", "enabled": true},
                {"id": "b2", "action": "cancel", "combo": "escape", "enabled": true}
            ],
            "translate_mode": true,
            "llm_correct": true,
            "stt": {"base_url": "https://api.openai.com", "model": "whisper-1", "auth_kind": {"kind": "bearer"}},
            "llm": {"base_url": "https://api.openai.com", "model": "gpt-4", "auth_kind": {"kind": "bearer"}},
            "separate_api_keys": true
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.bindings.len(), 2);
        assert_eq!(s.bindings[0].id, "b1");
        assert_eq!(s.bindings[1].action, ActionKind::Cancel);
        assert!(s.separate_api_keys);
    }

    #[test]
    fn show_overlay_defaults_to_true_when_missing() {
        let json = r#"{
            "language": "ja",
            "bindings": [],
            "translate_mode": false,
            "llm_correct": false,
            "stt": {"base_url": "https://example.com", "model": "m", "auth_kind": {"kind": "bearer"}},
            "llm": {"base_url": "https://example.com", "model": "m", "auth_kind": {"kind": "bearer"}},
            "separate_api_keys": false
        }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert!(s.show_overlay, "show_overlay が JSON にない場合は true がデフォルト");
    }

    #[test]
    fn show_overlay_false_roundtrip() {
        let mut s = Settings::default();
        s.show_overlay = false;
        let json = serde_json::to_string(&s).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert!(!parsed.show_overlay);
    }
}
