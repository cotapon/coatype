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
    #[serde(default)]
    pub stt: ProviderConfig,
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

/// OS ごとのデフォルトショートカット。
/// macOS: 右Option (Key::AltGr)。Windows: 右Ctrl (Key::ControlRight)。
/// いずれも「右側の、単独では滅多に使わない修飾キー」を Push-to-Talk トリガーにする思想で選定。
/// Windows で左右非対称の Meta(Winキー) を避けるのは、単独押下でスタートメニューが開き
/// Win+V 等の OS 予約コンボと衝突するため。
#[cfg(target_os = "windows")]
fn default_combo() -> &'static str {
    "rightcontrol"
}
#[cfg(not(target_os = "windows"))]
fn default_combo() -> &'static str {
    "rightoption"
}

/// combo 文字列を実行 OS に合わせて正規化する。
///
/// Mac で作成された設定 (leftmeta/rightmeta = Command, fn) が Windows に持ち込まれた場合、
/// rdev の Windows バックエンドには MetaRight/Function の keycode が存在せず、
/// また MetaLeft (Winキー) は単独押下でスタートメニューを開いてしまい実用にならない。
/// そのため Windows ロード時のみ Meta 系を Ctrl 系へ寄せ、fn は除去する。
/// 逆方向 (Windows → macOS) は変換しない: Ctrl/Alt はどちらの OS でも意図通りに機能するため、
/// ユーザーが明示的に選んだキーを強制的に書き換える必要がない。
///
/// テスト容易性のため OS 判定 (`is_windows`) を引数として受け取る純粋関数にしている。
fn normalize_combo_for(combo: &str, is_windows: bool) -> String {
    if !is_windows {
        return combo.to_string();
    }
    combo
        .split('+')
        .map(|p| p.trim().to_lowercase())
        .map(|p| match p.as_str() {
            "leftmeta" | "lmeta" | "lcmd" | "leftcmd" | "metacmd" => "leftcontrol".to_string(),
            "rightmeta" | "rmeta" | "rcmd" | "rightcmd" => "rightcontrol".to_string(),
            "fn" | "function" => String::new(),
            other => other.to_string(),
        })
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

impl Default for Settings {
    fn default() -> Self {
        let base = Self::default_base();
        Self {
            language: "ja".into(),
            bindings: vec![KeyBinding {
                id: "default-0".to_string(),
                action: ActionKind::StartRecord,
                combo: default_combo().to_string(),
                enabled: true,
            }],
            translate_mode: false,
            translate_model: None,
            stt: ProviderConfig {
                base_url: base,
                model: String::new(),
                auth_kind: AuthKind::Bearer,
            },
            show_overlay: true,
            legacy_api_base: None,
            shortcut: None,
            trigger_mode: None,
        }
    }
}

impl Settings {
    fn default_base() -> String {
        String::new()
    }

    fn migrate_legacy(&mut self) {
        let base = Self::default_base();

        // 旧 api_base を stt に転写
        if let Some(old) = self.legacy_api_base.take() {
            if self.stt.base_url.is_empty() {
                self.stt.base_url = old;
            }
        }
        if self.stt.base_url.is_empty() {
            self.stt.base_url = base;
        }

        // 削除されたアクション (paste_last 等) を持つバインドを除去
        self.bindings.retain(|b| b.action != ActionKind::Unknown);

        // 旧 shortcut/trigger_mode を bindings に変換
        if self.bindings.is_empty() {
            let combo = self.shortcut.take().unwrap_or_else(|| default_combo().to_string());
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

        // OS 間で持ち込まれた combo (Mac の Meta/fn 等) を実行 OS に合わせて正規化する。
        let is_windows = cfg!(target_os = "windows");
        for b in &mut self.bindings {
            let normalized = normalize_combo_for(&b.combo, is_windows);
            if normalized != b.combo {
                tracing::warn!(
                    old = %b.combo,
                    new = %normalized,
                    "combo を実行 OS 向けに正規化しました"
                );
                b.combo = normalized;
            }
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
        assert_eq!(s.bindings[0].combo, default_combo());
        assert!(matches!(s.bindings[0].action, ActionKind::StartRecord));
        assert_eq!(s.stt.model, "");
    }

    #[test]
    fn default_combo_matches_current_platform() {
        // macOS: 右Option、Windows: 右Ctrl。ビルド対象 OS に応じて分岐すること。
        if cfg!(target_os = "windows") {
            assert_eq!(default_combo(), "rightcontrol");
        } else {
            assert_eq!(default_combo(), "rightoption");
        }
    }

    #[test]
    fn normalize_combo_for_windows_maps_meta_to_control_and_drops_fn() {
        assert_eq!(normalize_combo_for("leftmeta+v", true), "leftcontrol+v");
        assert_eq!(normalize_combo_for("rightmeta", true), "rightcontrol");
        assert_eq!(normalize_combo_for("fn+space", true), "space");
        // Windows でも既に対応済みのキーはそのまま
        assert_eq!(normalize_combo_for("rightcontrol", true), "rightcontrol");
        assert_eq!(normalize_combo_for("leftoption", true), "leftoption");
    }

    #[test]
    fn normalize_combo_for_non_windows_is_identity() {
        assert_eq!(normalize_combo_for("leftmeta+v", false), "leftmeta+v");
        assert_eq!(normalize_combo_for("fn", false), "fn");
        assert_eq!(normalize_combo_for("rightoption", false), "rightoption");
    }

    #[test]
    fn migrate_legacy_normalizes_combo_for_current_platform() {
        let json = r#"{
            "language": "ja",
            "translate_mode": false,
            "bindings": [
                {"id": "b1", "action": "start_record", "combo": "leftmeta+v", "enabled": true}
            ]
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        if cfg!(target_os = "windows") {
            assert_eq!(s.bindings[0].combo, "leftcontrol+v");
        } else {
            assert_eq!(s.bindings[0].combo, "leftmeta+v");
        }
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
            "translate_mode": false
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
            "translate_mode": false
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
    fn legacy_api_base_migrates_to_stt() {
        let json = r#"{
            "language": "ja",
            "shortcut": "rightoption",
            "trigger_mode": "push_to_talk",
            "translate_mode": false,
            "api_base": "https://custom.endpoint.example"
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.stt.base_url, "https://custom.endpoint.example");
        assert_eq!(s.stt.model, "");
        assert!(s.legacy_api_base.is_none());
    }

    #[test]
    fn legacy_api_base_not_serialized_after_migration() {
        let json = r#"{
            "language": "ja",
            "shortcut": "rightoption",
            "trigger_mode": "push_to_talk",
            "translate_mode": false,
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
            "stt": {"base_url": "https://api.openai.com", "model": "whisper-1", "auth_kind": {"kind": "bearer"}}
        }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.migrate_legacy();
        assert_eq!(s.bindings.len(), 2);
        assert_eq!(s.bindings[0].id, "b1");
        assert_eq!(s.bindings[1].action, ActionKind::Cancel);
        assert_eq!(s.stt.model, "whisper-1");
    }

    #[test]
    fn show_overlay_defaults_to_true_when_missing() {
        let json = r#"{
            "language": "ja",
            "bindings": [],
            "translate_mode": false,
            "stt": {"base_url": "https://example.com", "model": "m", "auth_kind": {"kind": "bearer"}}
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
