#[cfg(target_os = "macos")]
mod macos;

pub fn insert(text: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    return macos::insert_text(text);
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("unsupported platform");
}
