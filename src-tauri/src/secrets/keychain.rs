use anyhow::{anyhow, Result};

const SERVICE: &str = "jp.co.cyberagent.coatype";
const ACCOUNT: &str = "api-key";

pub fn resolve_api_key() -> Result<String> {
    if let Ok(v) = std::env::var("COATYPE_API_KEY") {
        if !v.is_empty() {
            return Ok(v);
        }
    }
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    entry
        .get_password()
        .map_err(|e| anyhow!("API key not set. Please configure it in Settings: {e}"))
}

pub fn save_api_key(key: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    entry.set_password(key)?;
    Ok(())
}

pub fn delete_api_key() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, ACCOUNT)?;
    entry.delete_credential().ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_env_when_set() {
        std::env::set_var("COATYPE_API_KEY", "test-from-env");
        let key = resolve_api_key().expect("should find key");
        assert_eq!(key, "test-from-env");
        std::env::remove_var("COATYPE_API_KEY");
    }
}
