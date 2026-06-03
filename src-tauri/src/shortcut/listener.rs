use crate::config::settings::TriggerMode;
use rdev::{listen, EventType, Key};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy)]
pub enum ShortcutEvent {
    StartRecording,
    StopRecording,
}

pub fn start(trigger: TriggerMode, shortcut_key: String, tx: Sender<ShortcutEvent>) {
    std::thread::spawn(move || {
        let target = parse_key(&shortcut_key);
        let mut is_recording = false;
        let mut is_held = false;
        if let Err(e) = listen(move |event| {
            let Some(t) = target else { return };
            match (event.event_type, &trigger) {
                (EventType::KeyPress(k), TriggerMode::PushToTalk) if k == t && !is_held => {
                    is_held = true;
                    let _ = tx.send(ShortcutEvent::StartRecording);
                }
                (EventType::KeyRelease(k), TriggerMode::PushToTalk) if k == t => {
                    is_held = false;
                    let _ = tx.send(ShortcutEvent::StopRecording);
                }
                (EventType::KeyPress(k), TriggerMode::Toggle) if k == t => {
                    is_recording = !is_recording;
                    let _ = tx.send(if is_recording {
                        ShortcutEvent::StartRecording
                    } else {
                        ShortcutEvent::StopRecording
                    });
                }
                _ => {}
            }
        }) {
            tracing::error!("rdev listener error: {e:?}");
        }
    });
}

fn parse_key(s: &str) -> Option<Key> {
    match s.to_lowercase().as_str() {
        "fn" | "function" => Some(Key::Function),
        "rightoption" | "ralt" => Some(Key::AltGr),
        "leftoption" | "lalt" => Some(Key::Alt),
        "rightcontrol" | "rctrl" => Some(Key::ControlRight),
        "leftcontrol" | "lctrl" => Some(Key::ControlLeft),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        _ => None,
    }
}
