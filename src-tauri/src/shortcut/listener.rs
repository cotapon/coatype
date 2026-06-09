use crate::commands::ActiveShortcut;
use crate::config::settings::ActionKind;
use rdev::{grab, Event, EventType, Key};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub enum RecordMode {
    PushToTalk,
    HandsFree,
}

#[derive(Debug, Clone)]
pub enum ShortcutEvent {
    StartRecording { binding_id: String, mode: RecordMode },
    StopRecording { binding_id: String },
    Cancel,
    PasteLast,
}

#[derive(Debug, Clone)]
pub enum ShortcutKey {
    Single(Key),
    Combo { modifiers: Vec<Key>, key: Key },
}

impl ShortcutKey {
    fn matches_press(&self, pressed: Key, held: &HashSet<Key>) -> bool {
        match self {
            ShortcutKey::Single(k) => *k == pressed,
            ShortcutKey::Combo { modifiers, key } => {
                *key == pressed && modifiers.iter().all(|m| held.contains(m))
            }
        }
    }

    fn is_part_of(&self, k: Key) -> bool {
        match self {
            ShortcutKey::Single(sk) => *sk == k,
            ShortcutKey::Combo { modifiers, key } => *key == k || modifiers.contains(&k),
        }
    }
}

pub struct RegisteredBinding {
    pub id: String,
    pub action: ActionKind,
    pub shortcut: ShortcutKey,
    pub enabled: bool,
}

pub fn start(
    bindings: Arc<Mutex<Vec<RegisteredBinding>>>,
    paused: Arc<AtomicBool>,
    tx: Sender<ShortcutEvent>,
    active: Arc<Mutex<ActiveShortcut>>,
    handle: AppHandle,
) {
    std::thread::spawn(move || {
        set_active(&active, "ok", None);

        let mut held_keys: HashSet<Key> = HashSet::new();
        // id → is_held: PTT バインドごとに押し下げ状態を追跡
        let mut binding_held: HashMap<String, bool> = HashMap::new();

        if let Err(e) = grab(move |event: Event| {
            // キャプチャモーダルが開いているときはイベントを処理しない
            if paused.load(Ordering::Relaxed) {
                return Some(event);
            }

            let bindings_guard = bindings.lock().unwrap();

            match event.event_type {
                EventType::KeyPress(k) => {
                    let already_held = held_keys.contains(&k);
                    held_keys.insert(k);

                    for binding in bindings_guard.iter().filter(|b| b.enabled) {
                        if !binding.shortcut.matches_press(k, &held_keys) {
                            continue;
                        }
                        match binding.action {
                            ActionKind::StartRecord => {
                                let held =
                                    binding_held.entry(binding.id.clone()).or_insert(false);
                                if !already_held && !*held {
                                    *held = true;
                                    let _ = tx.send(ShortcutEvent::StartRecording {
                                        binding_id: binding.id.clone(),
                                        mode: RecordMode::PushToTalk,
                                    });
                                }
                            }
                            ActionKind::HandsFree => {
                                // トグルロジックは main.rs 側で管理 (is_recording_main)
                                let _ = tx.send(ShortcutEvent::StartRecording {
                                    binding_id: binding.id.clone(),
                                    mode: RecordMode::HandsFree,
                                });
                            }
                            ActionKind::Cancel => {
                                if !already_held {
                                    let _ = tx.send(ShortcutEvent::Cancel);
                                }
                            }
                            ActionKind::PasteLast => {
                                if !already_held {
                                    let _ = tx.send(ShortcutEvent::PasteLast);
                                }
                            }
                        }
                    }
                }
                EventType::KeyRelease(k) => {
                    held_keys.remove(&k);

                    for binding in bindings_guard.iter().filter(|b| b.enabled) {
                        if binding.action == ActionKind::StartRecord {
                            let held =
                                binding_held.entry(binding.id.clone()).or_insert(false);
                            if *held && binding.shortcut.is_part_of(k) {
                                *held = false;
                                let _ = tx.send(ShortcutEvent::StopRecording {
                                    binding_id: binding.id.clone(),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }

            drop(bindings_guard);
            Some(event)
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

pub fn parse_shortcut(s: &str) -> Option<ShortcutKey> {
    let s = s.to_lowercase();
    let parts: Vec<&str> = s.split('+').map(str::trim).collect();
    if parts.len() == 1 {
        return parse_single_key(parts[0]).map(ShortcutKey::Single);
    }
    let key = parse_single_key(parts.last()?)?;
    let modifiers: Vec<Key> = parts[..parts.len() - 1]
        .iter()
        .filter_map(|p| parse_single_key(p))
        .collect();
    // モディファイアのパースに失敗したものがあれば None を返す
    if modifiers.len() != parts.len() - 1 {
        return None;
    }
    Some(ShortcutKey::Combo { modifiers, key })
}

pub fn parse_single_key(s: &str) -> Option<Key> {
    match s {
        "fn" | "function" => Some(Key::Function),
        "rightoption" | "ralt" => Some(Key::AltGr),
        "leftoption" | "lalt" | "alt" => Some(Key::Alt),
        "rightcontrol" | "rctrl" => Some(Key::ControlRight),
        "leftcontrol" | "lctrl" | "ctrl" => Some(Key::ControlLeft),
        "leftshift" | "lshift" | "shift" => Some(Key::ShiftLeft),
        "rightshift" | "rshift" => Some(Key::ShiftRight),
        "space" => Some(Key::Space),
        "leftmeta" | "lmeta" | "lcmd" | "leftcmd" | "metacmd" => Some(Key::MetaLeft),
        "rightmeta" | "rmeta" | "rcmd" | "rightcmd" => Some(Key::MetaRight),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "escape" | "esc" => Some(Key::Escape),
        "enter" | "return" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "a" => Some(Key::KeyA),
        "b" => Some(Key::KeyB),
        "c" => Some(Key::KeyC),
        "d" => Some(Key::KeyD),
        "e" => Some(Key::KeyE),
        "f" => Some(Key::KeyF),
        "g" => Some(Key::KeyG),
        "h" => Some(Key::KeyH),
        "i" => Some(Key::KeyI),
        "j" => Some(Key::KeyJ),
        "k" => Some(Key::KeyK),
        "l" => Some(Key::KeyL),
        "m" => Some(Key::KeyM),
        "n" => Some(Key::KeyN),
        "o" => Some(Key::KeyO),
        "p" => Some(Key::KeyP),
        "q" => Some(Key::KeyQ),
        "r" => Some(Key::KeyR),
        "s" => Some(Key::KeyS),
        "t" => Some(Key::KeyT),
        "u" => Some(Key::KeyU),
        "v" => Some(Key::KeyV),
        "w" => Some(Key::KeyW),
        "x" => Some(Key::KeyX),
        "y" => Some(Key::KeyY),
        "z" => Some(Key::KeyZ),
        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_keys() {
        assert!(matches!(parse_single_key("rightoption"), Some(Key::AltGr)));
        assert!(matches!(parse_single_key("leftoption"), Some(Key::Alt)));
        assert!(matches!(parse_single_key("leftcontrol"), Some(Key::ControlLeft)));
        assert!(matches!(parse_single_key("leftmeta"), Some(Key::MetaLeft)));
        assert!(matches!(parse_single_key("escape"), Some(Key::Escape)));
        assert!(matches!(parse_single_key("f5"), Some(Key::F5)));
        assert!(matches!(parse_single_key("a"), Some(Key::KeyA)));
        assert!(matches!(parse_single_key("z"), Some(Key::KeyZ)));
        assert!(matches!(parse_single_key("0"), Some(Key::Num0)));
        assert!(matches!(parse_single_key("9"), Some(Key::Num9)));
        assert!(parse_single_key("unknown_key").is_none());
    }

    #[test]
    fn parse_shortcut_single() {
        assert!(matches!(parse_shortcut("rightoption"), Some(ShortcutKey::Single(Key::AltGr))));
        assert!(matches!(parse_shortcut("f5"), Some(ShortcutKey::Single(Key::F5))));
        assert!(parse_shortcut("bad_key").is_none());
    }

    #[test]
    fn parse_shortcut_combo_single_modifier() {
        let sk = parse_shortcut("leftcontrol+r").unwrap();
        assert!(matches!(sk, ShortcutKey::Combo { key: Key::KeyR, .. }));
        if let ShortcutKey::Combo { modifiers, .. } = sk {
            assert_eq!(modifiers.len(), 1);
            assert!(matches!(modifiers[0], Key::ControlLeft));
        }
    }

    #[test]
    fn parse_shortcut_combo_multi_modifier() {
        let sk = parse_shortcut("leftmeta+leftcontrol+v").unwrap();
        if let ShortcutKey::Combo { modifiers, key } = sk {
            assert_eq!(key, Key::KeyV);
            assert_eq!(modifiers.len(), 2);
            assert!(modifiers.contains(&Key::MetaLeft));
            assert!(modifiers.contains(&Key::ControlLeft));
        } else {
            panic!("expected Combo");
        }
    }

    #[test]
    fn parse_shortcut_combo_bad_modifier() {
        assert!(parse_shortcut("badmod+r").is_none());
        assert!(parse_shortcut("leftcontrol+badkey").is_none());
    }

    #[test]
    fn parse_shortcut_case_insensitive() {
        assert!(parse_shortcut("RightOption").is_some());
        assert!(parse_shortcut("LeftMeta+LeftControl+V").is_some());
    }
}
