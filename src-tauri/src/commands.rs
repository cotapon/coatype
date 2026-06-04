use crate::config::settings::Settings;
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryItem;
use crate::pipeline::Pipeline;
use crate::secrets;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

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

#[tauri::command]
pub async fn save_api_key(key: String) -> Result<(), String> {
    secrets::keychain::save_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn has_api_key() -> bool {
    secrets::keychain::resolve_api_key().is_ok()
}
