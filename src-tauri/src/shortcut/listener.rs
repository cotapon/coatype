use crate::commands::ActiveShortcut;
use crate::config::settings::TriggerMode;
use rdev::{grab, Event, EventType, Key};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy)]
pub enum ShortcutEvent {
    StartRecording,
    StopRecording,
}

#[derive(Debug, Clone, Copy)]
enum ShortcutKey {
    Single(Key),
    Combo { modifier: Key, key: Key },
}

pub fn start(
    trigger: TriggerMode,
    shortcut_key: String,
    tx: Sender<ShortcutEvent>,
    active: Arc<Mutex<ActiveShortcut>>,
    handle: AppHandle,
) {
    std::thread::spawn(move || {
        let Some(shortcut) = parse_shortcut(&shortcut_key) else {
            let msg = format!("サポートされていないキー: {shortcut_key}");
            tracing::warn!("{msg}");
            set_active(&active, "parse_error", Some(msg.clone()));
            let _ = handle.emit("shortcut-error", msg);
            return;
        };

        set_active(&active, "ok", None);

        let mut is_recording = false;
        let mut is_held = false;
        let mut modifier_held = false;

        if let Err(e) = grab(move |event: Event| {
            match shortcut {
                ShortcutKey::Single(target) => handle_single(
                    event.event_type,
                    target,
                    &trigger,
                    &tx,
                    &mut is_recording,
                    &mut is_held,
                ),
                ShortcutKey::Combo { modifier, key } => handle_combo(
                    event.event_type,
                    modifier,
                    key,
                    &trigger,
                    &tx,
                    &mut is_recording,
                    &mut is_held,
                    &mut modifier_held,
                ),
            }

            Some(event) // すべてパススルー（消費しない）
        }) {
            let msg = format!("イベントタップ作成失敗: {e:?}");
            tracing::error!("{msg}");
            set_active(&active, "tap_failed", Some(msg.clone()));
            let _ = handle.emit("shortcut-error", msg);
        }
    });
}

fn set_active(active: &Arc<Mutex<ActiveShortcut>>, status: &str, error: Option<String>) {
    if let Ok(mut a) = active.lock() {
        a.status = status.into();
        a.error = error;
    }
}

fn handle_single(
    ev: EventType,
    target: Key,
    trigger: &TriggerMode,
    tx: &Sender<ShortcutEvent>,
    is_recording: &mut bool,
    is_held: &mut bool,
) {
    match (ev, trigger) {
        (EventType::KeyPress(k), TriggerMode::PushToTalk) if k == target && !*is_held => {
            *is_held = true;
            let _ = tx.send(ShortcutEvent::StartRecording);
        }
        (EventType::KeyRelease(k), TriggerMode::PushToTalk) if k == target => {
            *is_held = false;
            let _ = tx.send(ShortcutEvent::StopRecording);
        }
        (EventType::KeyPress(k), TriggerMode::Toggle) if k == target => {
            *is_recording = !*is_recording;
            let _ = tx.send(if *is_recording {
                ShortcutEvent::StartRecording
            } else {
                ShortcutEvent::StopRecording
            });
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_combo(
    ev: EventType,
    modifier: Key,
    key: Key,
    trigger: &TriggerMode,
    tx: &Sender<ShortcutEvent>,
    is_recording: &mut bool,
    is_held: &mut bool,
    modifier_held: &mut bool,
) {
    match ev {
        EventType::KeyPress(k) if k == modifier => {
            *modifier_held = true;
        }
        EventType::KeyRelease(k) if k == modifier => {
            *modifier_held = false;
            if *is_held {
                *is_held = false;
                let _ = tx.send(ShortcutEvent::StopRecording);
            }
        }
        EventType::KeyPress(k) if k == key && *modifier_held => match trigger {
            TriggerMode::PushToTalk if !*is_held => {
                *is_held = true;
                let _ = tx.send(ShortcutEvent::StartRecording);
            }
            TriggerMode::Toggle => {
                *is_recording = !*is_recording;
                let _ = tx.send(if *is_recording {
                    ShortcutEvent::StartRecording
                } else {
                    ShortcutEvent::StopRecording
                });
            }
            _ => {}
        },
        EventType::KeyRelease(k) if k == key && *is_held => {
            if matches!(trigger, TriggerMode::PushToTalk) {
                *is_held = false;
                let _ = tx.send(ShortcutEvent::StopRecording);
            }
        }
        _ => {}
    }
}

fn parse_shortcut(s: &str) -> Option<ShortcutKey> {
    let s = s.to_lowercase();
    if let Some(idx) = s.rfind('+') {
        let modifier_str = s[..idx].trim();
        let key_str = s[idx + 1..].trim();
        let modifier = parse_single_key(modifier_str)?;
        let key = parse_single_key(key_str)?;
        return Some(ShortcutKey::Combo { modifier, key });
    }
    parse_single_key(&s).map(ShortcutKey::Single)
}

fn parse_single_key(s: &str) -> Option<Key> {
    match s {
        "fn" | "function" => Some(Key::Function),
        "rightoption" | "ralt" => Some(Key::AltGr),
        "leftoption" | "lalt" | "alt" => Some(Key::Alt),
        "rightcontrol" | "rctrl" => Some(Key::ControlRight),
        "leftcontrol" | "lctrl" | "ctrl" => Some(Key::ControlLeft),
        "leftshift" | "lshift" | "shift" => Some(Key::ShiftLeft),
        "rightshift" | "rshift" => Some(Key::ShiftRight),
        "space" => Some(Key::Space),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        _ => None,
    }
}
