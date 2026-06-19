use super::replace::{Dictionary, Entry};
use anyhow::Result;

// 既知のヘッダ行(1行目がこれらに一致した場合スキップ)
const KNOWN_HEADERS: &[(&str, &str)] = &[
    ("from", "to"),
    ("変換前", "変換後"),
    ("replacement", "original"),  // Talon words_to_replace.csv
    ("original", "replacement"),
    ("incorrect", "correct"),
    ("wrong", "correct"),
    ("spoken form", "output"),    // Talon additional_words.csv
    ("spoken", "written"),
];

fn is_header_row(col1: &str, col2: &str) -> bool {
    let c1 = col1.trim().to_lowercase();
    let c2 = col2.trim().to_lowercase();
    KNOWN_HEADERS.iter().any(|(h1, h2)| c1 == *h1 && c2 == *h2)
}

/// CSV コンテンツをパースして Entry リストに変換する。
/// - RFC 4180 準拠 (クォート内カンマ・改行に対応)
/// - 2カラム: col1=from, col2=to
/// - 1カラム: from == to (語彙登録のみ)
/// - 既知ヘッダ行は自動スキップ
/// - from が空の行はスキップ
pub fn parse_csv(content: &str) -> Result<Vec<Entry>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut entries = Vec::new();
    let mut first = true;

    for result in rdr.records() {
        let record = result?;
        let col1 = record.get(0).unwrap_or("").trim();
        let col2 = record.get(1).map(|s| s.trim()).unwrap_or("");

        // 1行目が既知ヘッダならスキップ
        if first {
            first = false;
            if is_header_row(col1, col2) {
                continue;
            }
        }

        if col1.is_empty() {
            continue;
        }

        let from = col1.to_string();
        let to = if col2.is_empty() {
            // 1カラムのみ: 語彙として from == to
            from.clone()
        } else {
            col2.to_string()
        };

        entries.push(Entry { from, to });
    }

    Ok(entries)
}

/// Dragon TXT コンテンツをパースして Entry リストに変換する。
/// - 1行目 `@version=Plato-UTF8` をスキップ
/// - 各行 `writtenform\\spokenform` (バックスラッシュ2つで区切り)
/// - CoAType マッピング: from=spokenform, to=writtenform
/// - `\\` がない行 (語彙登録のみ) はスキップ
pub fn parse_dragon_txt(content: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut lines = content.lines();

    // 1行目: @version=Plato-UTF8 をスキップ。@version でなければ通常行として処理
    if let Some(first) = lines.next() {
        if !first.trim().starts_with("@version") {
            if let Some(entry) = parse_dragon_line(first) {
                entries.push(entry);
            }
        }
    }

    for line in lines {
        if let Some(entry) = parse_dragon_line(line) {
            entries.push(entry);
        }
    }

    entries
}

fn parse_dragon_line(line: &str) -> Option<Entry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    // Dragon 形式: `writtenform\\spokenform`
    // `\\` = バックスラッシュ2つが区切り文字
    if let Some(pos) = line.find("\\\\") {
        let written = line[..pos].trim();
        let spoken = line[pos + 2..].trim();
        if spoken.is_empty() {
            return None;
        }
        Some(Entry {
            from: spoken.to_string(),
            to: written.to_string(),
        })
    } else {
        // `\\` がない行は語彙登録のみ → スキップ
        None
    }
}

/// ファイル内容を見てフォーマットを自動判定してパースする。
/// - 1行目が `@version=Plato-UTF8` なら Dragon TXT
/// - それ以外は CSV
pub fn detect_and_parse(content: &str) -> Result<Vec<Entry>> {
    let first_line = content.lines().next().unwrap_or("").trim();
    if first_line == "@version=Plato-UTF8" {
        Ok(parse_dragon_txt(content))
    } else {
        parse_csv(content)
    }
}

/// Dictionary を CSV 文字列にシリアライズする。
/// - 1行目はヘッダ `from,to`
/// - RFC 4180 準拠 (カンマや改行を含む値は自動クォート)
pub fn to_csv(dict: &Dictionary) -> Result<String> {
    let mut buf = Vec::new();
    {
        let mut wtr = csv::WriterBuilder::new().from_writer(&mut buf);
        wtr.write_record(["from", "to"])?;
        for entry in &dict.entries {
            wtr.write_record([&entry.from, &entry.to])?;
        }
        wtr.flush()?;
    }
    Ok(String::from_utf8(buf)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- parse_csv ----

    #[test]
    fn parse_csv_two_column() {
        let content = "Monvi,Manvi\nずんだもん,ずんだモン";
        let entries = parse_csv(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].from, "Monvi");
        assert_eq!(entries[0].to, "Manvi");
        assert_eq!(entries[1].from, "ずんだもん");
        assert_eq!(entries[1].to, "ずんだモン");
    }

    #[test]
    fn parse_csv_skips_known_header() {
        let content = "from,to\nMonvi,Manvi";
        let entries = parse_csv(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from, "Monvi");
        assert_eq!(entries[0].to, "Manvi");
    }

    #[test]
    fn parse_csv_one_column_vocab() {
        let content = "専門用語\n別の語";
        let entries = parse_csv(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].from, "専門用語");
        assert_eq!(entries[0].to, "専門用語");
    }

    #[test]
    fn parse_csv_skips_empty_from() {
        // col1 が空の行はスキップ
        let content = ",to_only\nfoo,bar";
        let entries = parse_csv(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from, "foo");
    }

    #[test]
    fn parse_csv_handles_quoted_comma() {
        let content = "\"hello, world\",greeting\nfoo,bar";
        let entries = parse_csv(content).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].from, "hello, world");
        assert_eq!(entries[0].to, "greeting");
    }

    // ---- parse_dragon_txt ----

    #[test]
    fn parse_dragon_txt_basic() {
        // ファイル内の \\ は2文字のバックスラッシュ。Rustリテラルでは \\\\
        let content = "@version=Plato-UTF8\nMB\\\\megabyte\nOS\\\\operating system";
        let entries = parse_dragon_txt(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].from, "megabyte");
        assert_eq!(entries[0].to, "MB");
        assert_eq!(entries[1].from, "operating system");
        assert_eq!(entries[1].to, "OS");
    }

    #[test]
    fn parse_dragon_txt_skips_written_only() {
        // \\ がない行 (語彙登録のみ) はスキップ
        let content = "@version=Plato-UTF8\nvocabword\nMB\\\\megabyte";
        let entries = parse_dragon_txt(content);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from, "megabyte");
        assert_eq!(entries[0].to, "MB");
    }

    // ---- detect_and_parse ----

    #[test]
    fn detect_dragon_format() {
        let content = "@version=Plato-UTF8\nMB\\\\megabyte";
        let entries = detect_and_parse(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from, "megabyte");
        assert_eq!(entries[0].to, "MB");
    }

    #[test]
    fn detect_csv_format() {
        let content = "from,to\nMonvi,Manvi";
        let entries = detect_and_parse(content).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from, "Monvi");
        assert_eq!(entries[0].to, "Manvi");
    }

    // ---- to_csv / roundtrip ----

    #[test]
    fn to_csv_has_header() {
        let dict = Dictionary { entries: vec![] };
        let csv_str = to_csv(&dict).unwrap();
        assert!(csv_str.starts_with("from,to"));
    }

    #[test]
    fn to_csv_roundtrip() {
        let dict = Dictionary {
            entries: vec![
                Entry { from: "hello, world".to_string(), to: "greeting".to_string() },
                Entry { from: "foo".to_string(), to: "bar".to_string() },
            ],
        };
        let csv_str = to_csv(&dict).unwrap();
        let entries = parse_csv(&csv_str).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].from, "hello, world");
        assert_eq!(entries[0].to, "greeting");
        assert_eq!(entries[1].from, "foo");
        assert_eq!(entries[1].to, "bar");
    }
}
