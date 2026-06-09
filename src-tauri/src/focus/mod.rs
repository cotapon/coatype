#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{capture_frontmost_pid, restore_frontmost, show_panel};
#[cfg(target_os = "windows")]
pub use windows::{capture_frontmost_pid, restore_frontmost, show_panel};
