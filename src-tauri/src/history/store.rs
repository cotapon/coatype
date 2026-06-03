use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Serialize, Clone)]
pub struct HistoryItem {
    pub id: i64,
    pub text: String,
    pub language: String,
    pub translated: bool,
    pub duration_ms: i64,
    pub created_at: String,
}

pub struct HistoryStore {
    conn: Mutex<Connection>,
}

impl HistoryStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn init(conn: &Connection) -> anyhow::Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                language TEXT NOT NULL,
                translated INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(())
    }

    pub fn insert(
        &self,
        text: &str,
        language: &str,
        translated: bool,
        duration_ms: i64,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (text, language, translated, duration_ms) VALUES (?1, ?2, ?3, ?4)",
            params![text, language, translated as i64, duration_ms],
        )?;
        Ok(())
    }

    pub fn list(&self, limit: i64) -> anyhow::Result<Vec<HistoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, text, language, translated, duration_ms, created_at FROM history ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok(HistoryItem {
                id: r.get(0)?,
                text: r.get(1)?,
                language: r.get(2)?,
                translated: r.get::<_, i64>(3)? != 0,
                duration_ms: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        self.conn.lock().unwrap().execute("DELETE FROM history", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_list() {
        let store = HistoryStore::open_in_memory().unwrap();
        store.insert("hello", "ja", false, 1500).unwrap();
        store.insert("world", "ja", false, 800).unwrap();
        let items = store.list(10).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "world");
    }
}
