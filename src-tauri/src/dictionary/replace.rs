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
                Entry { from: "さいば".into(), to: "CyberAgent".into() },
                Entry { from: "あい えーじぇんと".into(), to: "AI Agent".into() },
            ],
        };
        let out = dict.apply("さいばはあい えーじぇんとを推進する");
        assert_eq!(out, "CyberAgentはAI Agentを推進する");
    }

    #[test]
    fn empty_dict_is_identity() {
        let dict = Dictionary { entries: vec![] };
        assert_eq!(dict.apply("変わらない"), "変わらない");
    }
}
