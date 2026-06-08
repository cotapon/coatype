use crate::config::settings::Settings;
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryItem;
use crate::pipeline::Pipeline;
use crate::secrets::keychain::{self, ACCOUNT_COMMON, ACCOUNT_LLM, ACCOUNT_STT};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveShortcut {
    pub shortcut: String,
    pub trigger_mode: String,
    pub status: String, // "starting" | "ok" | "parse_error" | "tap_failed"
    pub error: Option<String>,
}

pub struct ListenerState(pub Arc<Mutex<ActiveShortcut>>);

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
) -> Result<(), String> {
    settings.save(&path.0).map_err(|e| e.to_string())?;
    let p: &Pipeline = &**pipeline;
    *p.translate.lock().unwrap() = settings.translate_mode;
    *p.language.lock().unwrap() = settings.language.clone();
    *p.llm_correct.lock().unwrap() = settings.llm_correct;

    // base_url / model / auth_kind が変わった場合はクライアントを再構築する
    let stt_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_STT } else { ACCOUNT_COMMON },
    ).unwrap_or_default();
    let llm_key = keychain::resolve_api_key_for(
        if settings.separate_api_keys { ACCOUNT_LLM } else { ACCOUNT_COMMON },
    ).unwrap_or_default();
    p.rebuild_stt_client(&settings.stt, stt_key);
    p.rebuild_llm_client(&settings.llm, llm_key);

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
