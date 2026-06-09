use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication, NSWindow, NSWorkspace};
use std::ffi::c_void;

/// ショートカットキー押下時点のフロントアクティブアプリの PID を返す。
/// 自プロセスの場合は None。どのスレッドからも呼び出し可能。
pub fn capture_frontmost_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    let our_pid = std::process::id() as i32;
    if pid == our_pid {
        None
    } else {
        Some(pid)
    }
}

/// 指定 PID のアプリをフォアグラウンドに戻す。メインスレッドから呼ぶこと。
pub fn restore_frontmost(pid: i32) {
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return;
    };
    // macOS 14 以降は ActivateIgnoringOtherApps が deprecated だが、
    // 13 系ターゲットでは代替 API がないため使用する。
    #[allow(deprecated)]
    app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
}

/// NSWindow をフォーカスを奪わずに表示する (orderFront)。メインスレッドから呼ぶこと。
/// `ns_window` は `WebviewWindow::ns_window()` が返す `*mut c_void`。
pub fn show_panel(ns_window: *mut c_void) {
    if ns_window.is_null() {
        return;
    }
    unsafe {
        let window = &*(ns_window as *const NSWindow);
        window.orderFront(None);
    }
}
