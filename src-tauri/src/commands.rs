use crate::audio::recorder::Recorder;
use crate::config::settings::{KeyBinding, Settings};
use crate::dictionary::replace::Dictionary;
use crate::history::store::HistoryItem;
use crate::pipeline::Pipeline;
use crate::secrets::keychain;
use crate::shortcut::listener::{parse_shortcut, RegisteredBinding};
use crate::util::lock_or_recover;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, State};

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

/// マイクテスト専用の Recorder。Pipeline の本番用 recorder とは状態を共有しない。
/// これにより「設定画面でマイクテスト中」と「グローバルショートカットでの本番録音」が
/// バッファ/ストリームを奪い合わずに共存できる。
pub struct MicTest(pub Mutex<Recorder>);

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
    let p: &Pipeline = &pipeline;
    *lock_or_recover(&p.translate) = settings.translate_mode;
    *lock_or_recover(&p.language) = settings.language.clone();
    p.update_input_device(settings.input_device.clone());

    let stt_key = keychain::resolve_api_key().unwrap_or_default();
    p.rebuild_stt_client(&settings.stt, stt_key);

    // キーバインドをホットリロード (リスナー再起動なし)
    let new_registered = bindings_to_registered(&settings.bindings);
    *lock_or_recover(&bindings_state.0) = new_registered;

    show_overlay_state
        .0
        .store(settings.show_overlay, Ordering::Relaxed);

    Ok(())
}

#[tauri::command]
pub async fn get_dictionary(pipeline: State<'_, Arc<Pipeline>>) -> Result<Dictionary, String> {
    let p: &Pipeline = &pipeline;
    Ok(lock_or_recover(&p.dict).clone())
}

#[tauri::command]
pub async fn save_dictionary(
    dict: Dictionary,
    pipeline: State<'_, Arc<Pipeline>>,
    dict_path: State<'_, DictPath>,
) -> Result<(), String> {
    dict.save(&dict_path.0).map_err(|e| e.to_string())?;
    let p: &Pipeline = &pipeline;
    *lock_or_recover(&p.dict) = dict;
    Ok(())
}

#[tauri::command]
pub async fn import_dictionary(
    path: String,
    pipeline: State<'_, Arc<Pipeline>>,
    dict_path: State<'_, DictPath>,
) -> Result<Dictionary, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let incoming = crate::dictionary::io::detect_and_parse(&content).map_err(|e| e.to_string())?;

    let p: &Pipeline = &pipeline;
    let mut dict = lock_or_recover(&p.dict).clone();
    dict.merge(incoming);

    dict.save(&dict_path.0).map_err(|e| e.to_string())?;
    *lock_or_recover(&p.dict) = dict.clone();

    Ok(dict)
}

#[tauri::command]
pub async fn export_dictionary(
    path: String,
    pipeline: State<'_, Arc<Pipeline>>,
) -> Result<(), String> {
    let p: &Pipeline = &pipeline;
    let dict = lock_or_recover(&p.dict).clone();
    let csv = crate::dictionary::io::to_csv(&dict).map_err(|e| e.to_string())?;
    std::fs::write(&path, csv).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn list_history(
    pipeline: State<'_, Arc<Pipeline>>,
    limit: i64,
) -> Result<Vec<HistoryItem>, String> {
    let p: &Pipeline = &pipeline;
    p.history.list(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(pipeline: State<'_, Arc<Pipeline>>) -> Result<(), String> {
    let p: &Pipeline = &pipeline;
    p.history.clear().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_api_key(key: String, pipeline: State<'_, Arc<Pipeline>>) -> Result<(), String> {
    keychain::save_api_key(&key).map_err(|e| e.to_string())?;
    pipeline.update_api_key(key);
    Ok(())
}

#[tauri::command]
pub async fn has_api_key() -> bool {
    keychain::resolve_api_key().is_ok()
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
    Ok(lock_or_recover(&listener_state.0).clone())
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

/// 利用可能な入力デバイス名の一覧を返す (マイク選択セレクト用)。
#[tauri::command]
pub async fn list_input_devices() -> Result<Vec<String>, String> {
    crate::audio::recorder::list_input_device_names().map_err(|e| e.to_string())
}

/// マイクテスト開始: 文字起こしをせず、レベルメーター表示専用に録音を開始する。
/// Pipeline 本番の recorder とは独立した専用 recorder (MicTest state) を使うため、
/// 本番録音 (グローバルショートカット) やハンズフリー中でも競合しない。
/// `device` はまだ保存されていない選択中のデバイス名を直接渡せる (保存前でもテスト可能)。
#[tauri::command]
pub async fn start_mic_test(
    device: Option<String>,
    mic_test: State<'_, MicTest>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 冪等化: 既存のテストストリームが残っていれば先に破棄する。
    let _ = lock_or_recover(&mic_test.0).stop();

    let (level_tx, level_rx) = std::sync::mpsc::channel::<f32>();
    let on_level: crate::audio::recorder::LevelCb = Box::new(move |lvl: f32| {
        let _ = level_tx.send(lvl);
    });
    std::thread::spawn(move || {
        while let Ok(lvl) = level_rx.recv() {
            let _ = app.emit("mic-test-level", lvl);
        }
    });

    mic_test
        .0
        .lock()
        .unwrap()
        .start(device.as_deref(), Some(on_level))
        .map_err(|e| e.to_string())
}

/// マイクテスト停止: 録音を停止して WAV データは破棄する (文字起こしは行わない)。
/// ストリームの drop で on_level クロージャも drop され、level_tx が切れて
/// start_mic_test 内の emit スレッドは自然終了する。
#[tauri::command]
pub async fn stop_mic_test(mic_test: State<'_, MicTest>) -> Result<(), String> {
    let _ = lock_or_recover(&mic_test.0).stop();
    Ok(())
}

/// 外部 URL をデフォルトブラウザで開く。
#[tauri::command]
pub async fn open_url(url: String) {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&url).spawn().ok();
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/c", "start", &url])
        .spawn()
        .ok();
}

/// 実行中の OS を返す ("macos" | "windows" | "linux")。
/// フロントエンドでの修飾キー表示ラベル切り替え専用。デフォルト値や combo の正規化は
/// Rust 側の #[cfg(target_os)] で完結させており、これは表示目的のみ。
#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
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
        .filter_map(|b| match parse_shortcut(&b.combo) {
            Some(sk) => Some(RegisteredBinding {
                id: b.id.clone(),
                action: b.action.clone(),
                shortcut: sk,
                enabled: b.enabled,
            }),
            None => {
                tracing::warn!(
                    "キーバインド '{}' をパースできませんでした (id={})",
                    b.combo,
                    b.id
                );
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::ActionKind;

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
