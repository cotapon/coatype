#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use coatype_lib::api::whisper::WhisperClient;
use coatype_lib::commands::{DictPath, SettingsPath};
use coatype_lib::config::settings::Settings;
use coatype_lib::dictionary::replace::Dictionary;
use coatype_lib::history::store::HistoryStore;
use coatype_lib::pipeline::Pipeline;
use coatype_lib::secrets::keychain;
use coatype_lib::shortcut::listener::{self, ShortcutEvent};
use std::sync::{mpsc, Arc};
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
            let whisper = WhisperClient::new(settings.api_base.clone(), api_key);

            let pipeline = Arc::new(Pipeline::new(
                whisper,
                None, // LLM クライアントは設定で ON にしたときに初期化 (Task 11 以降)
                dict,
                history,
                settings.language.clone(),
                settings.translate_mode,
                settings.llm_correct,
            ));

            app.manage(pipeline.clone());
            app.manage(SettingsPath(settings_path));
            app.manage(DictPath(dict_path));

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
            listener::start(settings.trigger_mode, settings.shortcut.clone(), tx);

            let handle = app.handle().clone();
            let pipeline_clone = pipeline.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let Ok(ev) = rx.recv() else { break };
                    match ev {
                        ShortcutEvent::StartRecording => {
                            if let Err(e) = pipeline_clone.start() {
                                tracing::error!("record start error: {e}");
                            }
                            let _ = handle.emit("recording-state", "started");
                        }
                        ShortcutEvent::StopRecording => {
                            let _ = handle.emit("recording-state", "processing");

                            // オーバーレイウィンドウを表示
                            if let Some(w) = handle.get_webview_window("overlay") {
                                let _ = w.show();
                            }

                            match pipeline_clone.stop_and_process().await {
                                Ok(text) => {
                                    let _ = handle.emit("transcribed", text);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoAType");
}
