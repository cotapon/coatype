use anyhow::{anyhow, Result};

const SERVICE: &str = "jp.co.cyberagent.coatype";
pub const ACCOUNT_COMMON: &str = "api-key";
pub const ACCOUNT_STT: &str = "api-key-stt";
pub const ACCOUNT_LLM: &str = "api-key-llm";

/// `account` に対応する環境変数名を返す。
fn env_var_for(account: &str) -> &'static str {
    match account {
        ACCOUNT_STT => "COATYPE_STT_API_KEY",
        ACCOUNT_LLM => "COATYPE_LLM_API_KEY",
        _ => "COATYPE_API_KEY",
    }
}

/// 環境変数 → Keychain の順で API キーを解決する。
/// per-provider アカウントの場合、provider 固有の env が未設定なら COATYPE_API_KEY にフォールバックする。
pub fn resolve_api_key_for(account: &str) -> Option<String> {
    let env = env_var_for(account);
    if let Ok(v) = std::env::var(env) {
        if !v.is_empty() {
            return Some(v);
        }
    }
    // per-provider env が未設定の場合は共通 env にフォールバック
    if account != ACCOUNT_COMMON {
        if let Ok(v) = std::env::var("COATYPE_API_KEY") {
            if !v.is_empty() {
                return Some(v);
            }
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

pub fn has_api_key_for(account: &str) -> bool {
    resolve_api_key_for(account).is_some()
}

// ── 後方互換ラッパー ──────────────────────────────────────────

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

    #[test]
    fn stt_env_takes_priority_over_common() {
        let _g = env_lock();
        std::env::set_var("COATYPE_API_KEY", "common-key");
        std::env::set_var("COATYPE_STT_API_KEY", "stt-key");
        let key = resolve_api_key_for(ACCOUNT_STT).unwrap();
        assert_eq!(key, "stt-key");
        std::env::remove_var("COATYPE_API_KEY");
        std::env::remove_var("COATYPE_STT_API_KEY");
    }

    #[test]
    fn stt_falls_back_to_common_env() {
        let _g = env_lock();
        std::env::set_var("COATYPE_API_KEY", "common-key");
        std::env::remove_var("COATYPE_STT_API_KEY");
        let key = resolve_api_key_for(ACCOUNT_STT).unwrap();
        assert_eq!(key, "common-key");
        std::env::remove_var("COATYPE_API_KEY");
    }
}
