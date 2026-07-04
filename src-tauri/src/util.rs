use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// poisoned lock でも中身を取り出して続行する。
/// 音声コールバックなど別スレッドの panic が poison となって
/// 以後の録音・イベントループ全体を巻き込むのを防ぐ。
pub fn lock_or_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 同一ディレクトリの一時ファイルへ書いてから rename で置き換える。
/// 書き込み途中にクラッシュしても既存ファイルが破損しない。
///
/// Windows では宛先を別プロセス (AV・インデクサ等) が掴んでいると rename が
/// 失敗することがある。その場合も「宛先を削除してリトライ」はしない (削除後に
/// 失敗すると新旧両方を失う)。旧ファイルを退避 → 差し替え → 退避を削除、の順で
/// 置き換え、どの時点で失敗しても新旧いずれかのデータが必ずディスクに残るようにする。
pub fn atomic_write(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let tmp = sibling_with_suffix(path, ".tmp");
    std::fs::write(&tmp, contents)?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(first) => {
            let aside = sibling_with_suffix(path, ".old");
            // 旧ファイルの退避にも失敗した場合、新しい内容は .tmp に残る
            std::fs::rename(path, &aside)
                .map_err(|e| anyhow::anyhow!("atomic write failed: {first} (退避も失敗: {e})"))?;
            match std::fs::rename(&tmp, path) {
                Ok(()) => {
                    let _ = std::fs::remove_file(&aside);
                    Ok(())
                }
                Err(second) => {
                    // 旧ファイルを戻す。新しい内容は .tmp に残す
                    let _ = std::fs::rename(&aside, path);
                    Err(anyhow::anyhow!(
                        "atomic write failed: {first} / retry: {second}"
                    ))
                }
            }
        }
    }
}

/// JSON ファイルを読み込んでデシリアライズする。ファイルが無ければ None。
/// パースに失敗した場合は破損ファイルを `<name>.bak` に退避して None を返す。
/// 退避しないと、次の保存 (atomic_write) が破損原本をバックアップなしに上書きし、
/// 復旧余地のあったユーザーデータごと消えてしまう (settings と辞書で共通の対策)。
pub fn load_json_with_corruption_backup<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&raw) {
        Ok(v) => Some(v),
        Err(e) => {
            let backup = sibling_with_suffix(path, ".bak");
            tracing::warn!(
                "{} のパースに失敗 ({e})。{} に退避してデフォルトで再作成します",
                path.display(),
                backup.display()
            );
            let _ = std::fs::rename(path, &backup);
            None
        }
    }
}

pub(crate) fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("file"));
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        std::fs::write(&path, "old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn atomic_write_leaves_no_tmp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        atomic_write(&path, b"data").unwrap();
        assert!(!sibling_with_suffix(&path, ".tmp").exists());
        assert!(!sibling_with_suffix(&path, ".old").exists());
    }

    #[test]
    fn lock_or_recover_returns_inner_after_poison() {
        let m = std::sync::Arc::new(Mutex::new(1));
        let m2 = m.clone();
        let _ = std::thread::spawn(move || {
            let _guard = m2.lock().unwrap();
            panic!("poison the lock");
        })
        .join();
        assert_eq!(*lock_or_recover(&m), 1);
    }
}
