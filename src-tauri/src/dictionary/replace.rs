use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dictionary {
    pub entries: Vec<Entry>,
}

impl Dictionary {
    pub fn apply(&self, input: &str) -> String {
        let mut out = input.to_string();
        for e in &self.entries {
            if e.from.is_empty() {
                continue;
            }
            out = out.replace(&e.from, &e.to);
        }
        out
    }

    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// incoming のエントリを追記マージする。
    /// - `from` をキーに既存エントリを上書き(既存の順序を維持)
    /// - 新規 `from` は末尾追加
    /// - `from` が空のエントリはスキップ
    pub fn merge(&mut self, incoming: Vec<Entry>) {
        use std::collections::HashMap;
        // from -> entries 内インデックス のマップを構築
        let mut index: HashMap<String, usize> = self.entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.from.clone(), i))
            .collect();

        for entry in incoming {
            if entry.from.is_empty() {
                continue;
            }
            if let Some(&idx) = index.get(&entry.from) {
                // 既存エントリの to を上書き
                self.entries[idx].to = entry.to;
            } else {
                // 新規エントリを末尾追加
                let idx = self.entries.len();
                index.insert(entry.from.clone(), idx);
                self.entries.push(entry);
            }
        }
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_entries_in_order() {
        let dict = Dictionary {
            entries: vec![
                Entry { from: "おーぷんえーあい".into(), to: "OpenAI".into() },
                Entry { from: "りらいとする".into(), to: "rewrite".into() },
            ],
        };
        let out = dict.apply("おーぷんえーあいはりらいとするのが得意だ");
        assert_eq!(out, "OpenAIはrewriteのが得意だ");
    }

    #[test]
    fn empty_dict_is_identity() {
        let dict = Dictionary { entries: vec![] };
        assert_eq!(dict.apply("変わらない"), "変わらない");
    }

    #[test]
    fn merge_appends_new_entries() {
        let mut dict = Dictionary {
            entries: vec![Entry { from: "a".into(), to: "A".into() }],
        };
        dict.merge(vec![
            Entry { from: "b".into(), to: "B".into() },
        ]);
        assert_eq!(dict.entries.len(), 2);
        assert_eq!(dict.entries[1].from, "b");
        assert_eq!(dict.entries[1].to, "B");
    }

    #[test]
    fn merge_overwrites_existing_from() {
        let mut dict = Dictionary {
            entries: vec![
                Entry { from: "a".into(), to: "OLD".into() },
                Entry { from: "b".into(), to: "B".into() },
            ],
        };
        dict.merge(vec![
            Entry { from: "a".into(), to: "NEW".into() },
        ]);
        // 既存エントリ数は変わらない
        assert_eq!(dict.entries.len(), 2);
        // "a" の to が上書きされている
        assert_eq!(dict.entries[0].to, "NEW");
        // "b" は変わらない
        assert_eq!(dict.entries[1].to, "B");
    }

    #[test]
    fn merge_preserves_order_and_appends() {
        let mut dict = Dictionary {
            entries: vec![
                Entry { from: "x".into(), to: "X".into() },
                Entry { from: "y".into(), to: "Y".into() },
            ],
        };
        dict.merge(vec![
            Entry { from: "y".into(), to: "Y2".into() },
            Entry { from: "z".into(), to: "Z".into() },
        ]);
        assert_eq!(dict.entries.len(), 3);
        assert_eq!(dict.entries[0].from, "x");
        assert_eq!(dict.entries[1].from, "y");
        assert_eq!(dict.entries[1].to, "Y2");
        assert_eq!(dict.entries[2].from, "z");
    }

    #[test]
    fn merge_skips_empty_from() {
        let mut dict = Dictionary { entries: vec![] };
        dict.merge(vec![
            Entry { from: "".into(), to: "something".into() },
            Entry { from: "a".into(), to: "A".into() },
        ]);
        assert_eq!(dict.entries.len(), 1);
        assert_eq!(dict.entries[0].from, "a");
    }
}
