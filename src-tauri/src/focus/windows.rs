use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

/// ショートカットキー押下時点のフロントアクティブアプリの PID を返す。
/// 自プロセスの場合は None。
pub fn capture_frontmost_pid() -> Option<i32> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let our_pid = unsafe { GetCurrentProcessId() };
    if pid == our_pid || pid == 0 {
        None
    } else {
        Some(pid as i32)
    }
}

/// 指定 PID のアプリをフォアグラウンドに戻す。メインスレッドから呼ぶこと。
pub fn restore_frontmost(pid: i32) {
    let target_pid = pid as u32;
    let found = AtomicU32::new(0);
    let found_ptr = &found as *const AtomicU32 as isize;

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let found = unsafe { &*(lparam.0 as *const AtomicU32) };
        let mut win_pid: u32 = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut win_pid)) };
        if win_pid == found.load(Ordering::Relaxed) {
            found.store(hwnd.0 as u32, Ordering::Relaxed);
            return BOOL(0); // stop enumeration
        }
        BOOL(1)
    }

    // AtomicU32 を target_pid の一時置き場として使い回す
    found.store(target_pid, Ordering::Relaxed);
    unsafe { let _ = EnumWindows(Some(enum_proc), LPARAM(found_ptr)); }

    let hwnd_val = found.load(Ordering::Relaxed);
    // target_pid のまま変化していなければ HWND は見つからなかった
    if hwnd_val != target_pid && hwnd_val != 0 {
        unsafe { let _ = SetForegroundWindow(HWND(hwnd_val as *mut _)); }
    }
}

/// Windows ではオーバーレイウィンドウを show するだけ。
/// macOS の show_panel に相当するシグネチャで呼べるようにする。
pub fn show_panel(_ns_window: *mut std::ffi::c_void) {
    // Windows では tauri の w.show() 側で処理するため no-op
}
