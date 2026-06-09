#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub fn insert(text: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    return macos::insert_text(text);
    #[cfg(target_os = "windows")]
    return windows::insert_text(text);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    anyhow::bail!("unsupported platform");
}
