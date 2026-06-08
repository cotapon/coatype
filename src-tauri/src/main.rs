#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use coatype_lib::api::whisper::WhisperClient;
use coatype_lib::commands::{ActiveShortcut, DictPath, ListenerState, SettingsPath};
use coatype_lib::config::settings::{Settings, TriggerMode};
use coatype_lib::dictionary::replace::Dictionary;
use coatype_lib::history::store::HistoryStore;
use coatype_lib::pipeline::Pipeline;
use coatype_lib::secrets::keychain;
use coatype_lib::shortcut::listener::{self, ShortcutEvent};
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

            let api_key = keychain::resolve_api_key().unwrap_or_default();
            let whisper = WhisperClient::new(settings.stt.base_url.clone(), api_key);

            let pipeline = Arc::new(Pipeline::new(
                whisper,
                None, // LLM クライアントは設定で ON にしたときに初期化 (Task 11 以降)
                dict,
                history,
                settings.language.clone(),
                settings.translate_mode,
                settings.llm_correct,
            ));

            let trigger_str = match settings.trigger_mode {
                TriggerMode::PushToTalk => "push_to_talk",
                TriggerMode::Toggle => "toggle",
            };
            let active_shortcut_state = Arc::new(std::sync::Mutex::new(ActiveShortcut {
                shortcut: settings.shortcut.clone(),
                trigger_mode: trigger_str.to_string(),
                status: "starting".to_string(),
                error: None,
            }));

            app.manage(pipeline.clone());
            app.manage(SettingsPath(settings_path));
            app.manage(DictPath(dict_path));
            app.manage(ListenerState(active_shortcut_state.clone()));

            // 設定ウィンドウの × は閉じる代わりに隠す（再表示できなくなるのを防ぐ）
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
            listener::start(
                settings.trigger_mode,
                settings.shortcut.clone(),
                tx,
                active_shortcut_state,
                app.handle().clone(),
            );

            let handle = app.handle().clone();
            let pipeline_clone = pipeline.clone();
            // キー押下時点のフロントアプリ PID を保持し、注入前に復元するための状態
            let prev_app_pid: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
            let prev_pid = prev_app_pid.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let Ok(ev) = rx.recv() else { break };
                    match ev {
                        ShortcutEvent::StartRecording => {
                            // overlay を表示する前に現在のフロントアプリを記録する
                            #[cfg(target_os = "macos")]
                            {
                                *prev_pid.lock().unwrap() =
                                    coatype_lib::focus::capture_frontmost_pid();
                            }

                            match pipeline_clone.start() {
                                Ok(()) => {
                                    let _ = handle.emit("recording-state", "started");
                                    // フォーカスを奪わずに overlay を表示する
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
                                Err(e) => {
                                    tracing::error!("record start error: {e}");
                                    let _ = handle.emit("error", format!("録音開始失敗: {e}"));
                                }
                            }
                        }
                        ShortcutEvent::StopRecording => {
                            let _ = handle.emit("recording-state", "processing");
                            // processing 中も overlay をフォーカス奪取なしで表示
                            let h2 = handle.clone();
                            let _ = handle.run_on_main_thread(move || {
                                if let Some(w) = h2.get_webview_window("overlay") {
                                    #[cfg(target_os = "macos")]
                                    if let Ok(ns_win) = w.ns_window() {
                                        coatype_lib::focus::show_panel(ns_win);
                                        return;
                                    }
                                    let _ = w.show();
                                }
                            });

                            let pid = prev_pid.lock().unwrap().take();
                            match pipeline_clone.stop_and_process().await {
                                Ok(text) => {
                                    let _ = handle.emit("transcribed", text.clone());
                                    let t = text.clone();
                                    // enigo は TSM API (HIToolbox) を呼ぶため main thread 必須。
                                    // 注入前に元のアプリのフォーカスを復元する。
                                    let _ = handle.run_on_main_thread(move || {
                                        #[cfg(target_os = "macos")]
                                        if let Some(p) = pid {
                                            coatype_lib::focus::restore_frontmost(p);
                                            std::thread::sleep(
                                                std::time::Duration::from_millis(80),
                                            );
                                        }
                                        if let Err(e) = coatype_lib::injector::insert(&t) {
                                            tracing::error!("inject error: {e}");
                                        }
                                    });
                                }
                                Err(e) => {
                                    tracing::error!("pipeline error: {e}");
                                    let _ = handle.emit("error", e.to_string());
                                }
                            }
                            let _ = handle.emit("recording-state", "idle");

                            if let Some(w) = handle.get_webview_window("overlay") {
                                let _ = w.hide();
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoAType");
}
