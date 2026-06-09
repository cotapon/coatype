#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use coatype_lib::api::whisper::WhisperClient;
use coatype_lib::commands::{
    ActiveShortcut, DictPath, ListenerBindings, ListenerPaused, ListenerState, SettingsPath,
    bindings_to_registered,
};
use coatype_lib::config::settings::Settings;
use coatype_lib::dictionary::llm_correct::LlmCorrectClient;
use coatype_lib::dictionary::replace::Dictionary;
use coatype_lib::history::store::HistoryStore;
use coatype_lib::pipeline::{CurrentTask, Pipeline};
use coatype_lib::secrets::keychain::{self, ACCOUNT_COMMON, ACCOUNT_LLM, ACCOUNT_STT};
use coatype_lib::shortcut::listener::{self, RecordMode, ShortcutEvent};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir().expect("app data dir");
            std::fs::create_dir_all(&app_data)?;

            let settings_path = app_data.join("settings.json");
            let dict_path = app_data.join("dictionary.json");
            let db_path = app_data.join("history.db");

            let settings = Settings::load(&settings_path);
            let dict = Dictionary::load(&dict_path);
            let history = Arc::new(HistoryStore::open(&db_path)?);

            let stt_key = keychain::resolve_api_key_for(
                if settings.separate_api_keys { ACCOUNT_STT } else { ACCOUNT_COMMON },
            )
            .unwrap_or_default();
            let llm_key = keychain::resolve_api_key_for(
                if settings.separate_api_keys { ACCOUNT_LLM } else { ACCOUNT_COMMON },
            )
            .unwrap_or_default();

            let whisper = WhisperClient::new(
                settings.stt.base_url.clone(),
                settings.stt.model.clone(),
                settings.stt.auth_kind.clone(),
                stt_key,
            );
            let llm = LlmCorrectClient::new(
                settings.llm.base_url.clone(),
                settings.llm.model.clone(),
                settings.llm.auth_kind.clone(),
                llm_key,
            );

            let pipeline = Arc::new(Pipeline::new(
                whisper,
                Some(llm),
                dict,
                history,
                settings.language.clone(),
                settings.translate_mode,
                settings.llm_correct,
            ));

            let active_shortcut_state = Arc::new(Mutex::new(ActiveShortcut {
                status: "starting".to_string(),
                error: None,
            }));

            let registered_bindings = bindings_to_registered(&settings.bindings);
            let bindings_arc = Arc::new(Mutex::new(registered_bindings));
            let paused_arc = Arc::new(AtomicBool::new(false));

            app.manage(pipeline.clone());
            app.manage(SettingsPath(settings_path));
            app.manage(DictPath(dict_path));
            app.manage(ListenerState(active_shortcut_state.clone()));
            app.manage(ListenerBindings(bindings_arc.clone()));
            app.manage(ListenerPaused(paused_arc.clone()));

            // 設定ウィンドウの × は閉じる代わりに隠す
            if let Some(settings_win) = app.get_webview_window("settings") {
                let settings_win2 = settings_win.clone();
                settings_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_win2.hide();
                    }
                });
            }

            // メニューバートレイ
            let quit = MenuItem::with_id(app, "quit", "Quit CoAType", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "quit" => app.exit(0),
                    "settings" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // ショートカット監視 → パイプライン
            let (tx, rx) = mpsc::channel::<ShortcutEvent>();
            listener::start(bindings_arc, paused_arc, tx, active_shortcut_state, app.handle().clone());

            let handle = app.handle().clone();
            let pipeline_clone = pipeline.clone();
            let prev_app_pid: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
            let prev_pid = prev_app_pid.clone();

            tauri::async_runtime::spawn(async move {
                // HandsFree バインドのトグル状態を main ループで管理
                let mut is_recording = false;

                loop {
                    let Ok(ev) = rx.recv() else { break };
                    match ev {
                        ShortcutEvent::StartRecording { mode, .. } => {
                            let should_start = match mode {
                                RecordMode::PushToTalk => true,
                                RecordMode::HandsFree => {
                                    if is_recording {
                                        // トグルオフ: 処理を開始
                                        is_recording = false;
                                        spawn_stop_and_process(
                                            &pipeline_clone,
                                            &handle,
                                            prev_pid.lock().unwrap().take(),
                                        );
                                        false
                                    } else {
                                        true
                                    }
                                }
                            };

                            if should_start {
                                #[cfg(target_os = "macos")]
                                {
                                    *prev_pid.lock().unwrap() =
                                        coatype_lib::focus::capture_frontmost_pid();
                                }
                                match pipeline_clone.start() {
                                    Ok(()) => {
                                        is_recording = true;
                                        let _ = handle.emit("recording-state", "started");
                                        show_overlay_panel(&handle);
                                    }
                                    Err(e) => {
                                        tracing::error!("record start error: {e}");
                                        let _ = handle
                                            .emit("error", format!("録音開始失敗: {e}"));
                                    }
                                }
                            }
                        }
                        ShortcutEvent::StopRecording { .. } => {
                            is_recording = false;
                            spawn_stop_and_process(
                                &pipeline_clone,
                                &handle,
                                prev_pid.lock().unwrap().take(),
                            );
                        }
                        ShortcutEvent::Cancel => {
                            is_recording = false;
                            pipeline_clone.cancel();
                            let _ = handle.emit("recording-state", "idle");
                            if let Some(w) = handle.get_webview_window("overlay") {
                                let _ = w.hide();
                            }
                        }
                        ShortcutEvent::PasteLast => {
                            if let Some(text) = pipeline_clone.last_transcription() {
                                let _ = handle.run_on_main_thread(move || {
                                    if let Err(e) = coatype_lib::injector::insert(&text) {
                                        tracing::error!("paste_last inject error: {e}");
                                    }
                                });
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            coatype_lib::commands::get_settings,
            coatype_lib::commands::save_settings,
            coatype_lib::commands::get_dictionary,
            coatype_lib::commands::save_dictionary,
            coatype_lib::commands::list_history,
            coatype_lib::commands::clear_history,
            coatype_lib::commands::save_api_key,
            coatype_lib::commands::has_api_key,
            coatype_lib::commands::check_accessibility,
            coatype_lib::commands::open_accessibility_settings,
            coatype_lib::commands::active_shortcut,
            coatype_lib::commands::set_listener_paused,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoAType");
}

fn show_overlay_panel(handle: &tauri::AppHandle) {
    let h = handle.clone();
    let _ = handle.run_on_main_thread(move || {
        if let Some(w) = h.get_webview_window("overlay") {
            #[cfg(target_os = "macos")]
            if let Ok(ns_win) = w.ns_window() {
                coatype_lib::focus::show_panel(ns_win);
                return;
            }
            let _ = w.show();
        }
    });
}

fn spawn_stop_and_process(
    pipeline: &Arc<Pipeline>,
    handle: &tauri::AppHandle,
    pid: Option<i32>,
) {
    let _ = handle.emit("recording-state", "processing");
    show_overlay_panel(handle);

    let pipeline_task = pipeline.clone();
    let handle_task = handle.clone();

    let join = tokio::spawn(async move {
        match pipeline_task.stop_and_process().await {
            Ok(text) if !text.is_empty() => {
                let _ = handle_task.emit("transcribed", text.clone());
                let _ = handle_task.run_on_main_thread(move || {
                    #[cfg(target_os = "macos")]
                    if let Some(p) = pid {
                        coatype_lib::focus::restore_frontmost(p);
                        std::thread::sleep(std::time::Duration::from_millis(80));
                    }
                    if let Err(e) = coatype_lib::injector::insert(&text) {
                        tracing::error!("inject error: {e}");
                    }
                });
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("pipeline error: {e}");
                let _ = handle_task.emit("error", e.to_string());
            }
        }
        let _ = handle_task.emit("recording-state", "idle");
        if let Some(w) = handle_task.get_webview_window("overlay") {
            let _ = w.hide();
        }
        // タスク完了時に current_task を解放
        *pipeline_task.current_task.lock().unwrap() = None;
    });

    *pipeline.current_task.lock().unwrap() = Some(CurrentTask { join });
}
