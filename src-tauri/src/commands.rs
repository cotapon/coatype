use crate::config::settings::{KeyBinding, Settings};
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryItem;
use crate::pipeline::Pipeline;
use crate::secrets::keychain::{self, ACCOUNT_COMMON, ACCOUNT_LLM, ACCOUNT_STT};
use crate::shortcut::listener::{parse_shortcut, RegisteredBinding};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveShortcut {
    pub status: String, // "starting" | "ok" | "tap_failed"
    pub error: Option<String>,
}

pub struct ListenerState(pub Arc<Mutex<ActiveShortcut>>);
pub struct ListenerBindings(pub Arc<Mutex<Vec<RegisteredBinding>>>);
pub struct ListenerPaused(pub Arc<AtomicBool>);
pub struct ShowOverlay(pub Arc<AtomicBool>);

pub struct SettingsPath(pub PathBuf);
pub struct DictPath(pub PathBuf);

#[tauri::command]
pub async fn get_settings(path: State<'_, SettingsPath>) -> Result<Settings, String> {
    Ok(Settings::load(&path.0))
}

#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    path: State<'_, SettingsPath>,
    pipeline: State<'_, Arc<Pipeline>>,
    bindings_state: State<'_, ListenerBindings>,
    show_overlay_state: State<'_, ShowOverlay>,
) -> Result<(), String> {
    settings.save(&path.0).map_err(|e| e.to_string())?;
    let p: &Pipeline = &**pipeline;
    *p.translate.lock().unwrap() = settings.translate_mode;
    *p.language.lock().unwrap() = settings.language.clone();
    *p.llm_correct.lock().unwrap() = settings.llm_correct;

    let stt_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_STT } else { ACCOUNT_COMMON },
    )
    .unwrap_or_default();
    let llm_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_LLM } else { ACCOUNT_COMMON },
    )
    .unwrap_or_default();
    p.rebuild_stt_client(&settings.stt, stt_key);
    p.rebuild_llm_client(&settings.llm, llm_key);

    // キーバインドをホットリロード (リスナー再起動なし)
    let new_registered = bindings_to_registered(&settings.bindings);
    *bindings_state.0.lock().unwrap() = new_registered;

    show_overlay_state.0.store(settings.show_overlay, Ordering::Relaxed);

    Ok(())
}

#[tauri::command]
pub async fn get_dictionary(pipeline: State<'_, Arc<Pipeline>>) -> Result<Dictionary, String> {
    let p: &Pipeline = &**pipeline;
    Ok(p.dict.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_dictionary(
    dict: Dictionary,
    pipeline: State<'_, Arc<Pipeline>>,
    dict_path: State<'_, DictPath>,
) -> Result<(), String> {
    dict.save(&dict_path.0).map_err(|e| e.to_string())?;
    let p: &Pipeline = &**pipeline;
    *p.dict.lock().unwrap() = dict;
    Ok(())
}

#[tauri::command]
pub async fn list_history(
    pipeline: State<'_, Arc<Pipeline>>,
    limit: i64,
) -> Result<Vec<HistoryItem>, String> {
    let p: &Pipeline = &**pipeline;
    p.history.list(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(pipeline: State<'_, Arc<Pipeline>>) -> Result<(), String> {
    let p: &Pipeline = &**pipeline;
    p.history.clear().map_err(|e| e.to_string())
}

/// provider: "stt" | "llm" | "common" (デフォルト: common)
#[tauri::command]
pub async fn save_api_key(
    provider: String,
    key: String,
    pipeline: State<'_, Arc<Pipeline>>,
) -> Result<(), String> {
    let account = provider_to_account(&provider);
    keychain::save_api_key_for(account, &key).map_err(|e| e.to_string())?;
    match provider.as_str() {
        "stt" => pipeline.update_stt_api_key(key),
        "llm" => pipeline.update_llm_api_key(key),
        _ => pipeline.update_api_key(key),
    }
    Ok(())
}

/// provider: "stt" | "llm" | "common" (デフォルト: common)
#[tauri::command]
pub async fn has_api_key(provider: String) -> bool {
    keychain::has_api_key_for(provider_to_account(&provider))
}

fn provider_to_account(provider: &str) -> &'static str {
    match provider {
        "stt" => ACCOUNT_STT,
        "llm" => ACCOUNT_LLM,
        _ => ACCOUNT_COMMON,
    }
}

#[tauri::command]
pub async fn check_accessibility() -> bool {
    crate::permissions::is_accessibility_trusted()
}

#[tauri::command]
pub async fn open_accessibility_settings() {
    crate::permissions::open_accessibility_prefs();
}

#[tauri::command]
pub async fn active_shortcut(
    listener_state: State<'_, ListenerState>,
) -> Result<ActiveShortcut, String> {
    Ok(listener_state.0.lock().unwrap().clone())
}

/// 録音テスト開始: マイク録音のみ開始する (グローバルショートカットの録音とは独立)。
/// 呼び出し側 (フロント) でリスナーを一時停止してから使うこと。
/// 本番の録音と同じく非同期ランタイム上で `pipeline.start()` を呼ぶ。
#[tauri::command]
pub async fn start_test_recording(pipeline: State<'_, Arc<Pipeline>>) -> Result<(), String> {
    pipeline.start(None).map_err(|e| e.to_string())
}

/// 録音テスト停止: 録音を停止して文字起こし結果を返す (履歴・挿入なし)。
#[tauri::command]
pub async fn stop_test_recording(pipeline: State<'_, Arc<Pipeline>>) -> Result<String, String> {
    pipeline
        .stop_and_transcribe_test()
        .await
        .map_err(|e| e.to_string())
}

/// キーバインド設定中 (CaptureModal 表示中) はリスナーを一時停止する。
#[tauri::command]
pub async fn set_listener_paused(
    paused: bool,
    listener_paused: State<'_, ListenerPaused>,
) -> Result<(), String> {
    listener_paused.0.store(paused, Ordering::Relaxed);
    Ok(())
}

/// KeyBinding リストを RegisteredBinding リストに変換する。
/// parse_shortcut に失敗したバインドは警告ログを出してスキップ。
pub fn bindings_to_registered(bindings: &[KeyBinding]) -> Vec<RegisteredBinding> {
    bindings
        .iter()
        .filter_map(|b| {
            match parse_shortcut(&b.combo) {
                Some(sk) => Some(RegisteredBinding {
                    id: b.id.clone(),
                    action: b.action.clone(),
                    shortcut: sk,
                    enabled: b.enabled,
                }),
                None => {
                    tracing::warn!("キーバインド '{}' をパースできませんでした (id={})", b.combo, b.id);
                    None
                }
            }
        })
        .collect()
}

/// IME と衝突する可能性があるコンボかどうかを判定する。
/// Shift + 文字キー / Shift + Space は日本語 IME に消費されることがある。
pub fn detect_ime_conflict(combo: &str) -> bool {
    let lower = combo.to_lowercase();
    let parts: Vec<&str> = lower.split('+').map(str::trim).collect();
    if parts.len() < 2 {
        return false;
    }
    let has_shift = parts.iter().any(|p| *p == "leftshift" || *p == "rightshift" || *p == "lshift" || *p == "rshift" || *p == "shift");
    if !has_shift {
        return false;
    }
    // Shift + 文字キー (a-z) または Shift + Space
    let last = *parts.last().unwrap();
    let is_letter = last.len() == 1 && last.chars().all(|c| c.is_ascii_alphabetic());
    let is_space = last == "space";
    is_letter || is_space
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::ActionKind;

    #[test]
    fn ime_conflict_detection() {
        assert!(detect_ime_conflict("leftshift+a"));
        assert!(detect_ime_conflict("rightshift+space"));
        assert!(detect_ime_conflict("shift+z"));
        assert!(!detect_ime_conflict("leftcontrol+r"));
        assert!(!detect_ime_conflict("leftmeta+leftcontrol+v"));
        assert!(!detect_ime_conflict("escape"));
        assert!(!detect_ime_conflict("leftoption"));
    }

    #[test]
    fn bindings_to_registered_skips_invalid() {
        let bindings = vec![
            KeyBinding {
                id: "b1".to_string(),
                action: ActionKind::StartRecord,
                combo: "rightoption".to_string(),
                enabled: true,
            },
            KeyBinding {
                id: "b2".to_string(),
                action: ActionKind::Cancel,
                combo: "invalid_key_xyz".to_string(),
                enabled: true,
            },
        ];
        let registered = bindings_to_registered(&bindings);
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].id, "b1");
    }
}
