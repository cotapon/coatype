use anyhow::{anyhow, Result};

const SERVICE: &str = "jp.co.cotapon.coatype";
pub const ACCOUNT_COMMON: &str = "api-key";

/// 環境変数 (COATYPE_API_KEY) → Keychain の順で API キーを解決する。
pub fn resolve_api_key_for(account: &str) -> Option<String> {
    if let Ok(v) = std::env::var("COATYPE_API_KEY") {
        if !v.is_empty() {
            return Some(v);
        }
    }
    let entry = keyring::Entry::new(SERVICE, account).ok()?;
    entry.get_password().ok()
}

pub fn save_api_key_for(account: &str, key: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, account)?;
    entry.set_password(key)?;
    Ok(())
}

pub fn delete_api_key_for(account: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, account)?;
    entry.delete_credential().ok();
    Ok(())
}

// ── 共通アカウント向けラッパー ────────────────────────────────

pub fn resolve_api_key() -> Result<String> {
    resolve_api_key_for(ACCOUNT_COMMON)
        .ok_or_else(|| anyhow!("API key not set. Please configure it in Settings."))
}

pub fn save_api_key(key: &str) -> Result<()> {
    save_api_key_for(ACCOUNT_COMMON, key)
}

pub fn delete_api_key() -> Result<()> {
    delete_api_key_for(ACCOUNT_COMMON)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    // 環境変数はグローバル資源なので並行テストを直列化する
    static ENV_LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(Default::default).lock().unwrap()
    }

    #[test]
    fn resolve_returns_env_when_set() {
        let _g = env_lock();
        std::env::set_var("COATYPE_API_KEY", "test-from-env");
        let key = resolve_api_key().expect("should find key");
        assert_eq!(key, "test-from-env");
        std::env::remove_var("COATYPE_API_KEY");
    }
}
