//! Backups and CSV export.
//!
//! A backup is simply the data file (`data.json`, see [`crate::data`]): restoring
//! one replaces all recorded data. Settings are not part of it, they belong to
//! the computer. A backup made by an older version of the app is upgraded when it
//! is read.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::data::DataFile;
use crate::model::{Categories, CategoryId, Kind};
use crate::storage::Storage;

/// Parses and validates a backup file; the error text is meant for the user.
pub fn parse(text: &str) -> Result<DataFile, String> {
    DataFile::parse(text).map(|(data, _)| data).map_err(|e| e.to_string())
}

/// Writes the current data to `<data dir>/backups/` before it is replaced.
pub fn save_safety_copy(st: &Storage) -> io::Result<PathBuf> {
    let dir = st.dir().join("backups");
    fs::create_dir_all(&dir)?;
    let name = format!("before-restore-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    let json = st.snapshot().to_json().map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Suggested file name, e.g. `cashflow-backup-2026-11-16.json`.
pub fn file_name(stem: &str, extension: &str) -> String {
    format!("cashflow-{stem}-{}.{extension}", chrono::Local::now().format("%Y-%m-%d"))
}

impl DataFile {
    /// Every entry as CSV: month, type, category (with its parents), note, amount.
    /// Starts with a byte order mark so spreadsheets read the text as UTF-8.
    pub fn to_csv(&self) -> String {
        let mut out = String::from("\u{feff}month,type,category,note,amount\n");
        for (month, data) in &self.months {
            for entry in &data.entries {
                let kind = match self.categories.get(entry.category_id).map(|c| c.kind) {
                    Some(Kind::Income) => "income",
                    Some(Kind::Expense) => "expense",
                    None => "",
                };
                let fields = [
                    month.clone(),
                    kind.to_string(),
                    csv_text(&category_path(&self.categories, entry.category_id)),
                    csv_text(&entry.name),
                    cents_text(entry.amount_cents),
                ];
                out.push_str(&fields.join(","));
                out.push('\n');
            }
        }
        out
    }
}

/// "Parent / Child" for nested categories; an unknown id gives an empty text.
fn category_path(cats: &Categories, id: CategoryId) -> String {
    let mut names = Vec::new();
    let mut next = Some(id);
    // The depth limit protects against a damaged file with a parent cycle.
    while let (Some(current), true) = (next, names.len() < 20) {
        let Some(cat) = cats.get(current) else { break };
        names.push(cat.name.as_str());
        next = cat.parent;
    }
    names.reverse();
    names.join(" / ")
}

/// Quotes a text field and defuses spreadsheet formulas: a text starting with
/// `=`, `+`, `-` or `@` would otherwise be run as a formula when opened.
fn csv_text(text: &str) -> String {
    let safe = if text.starts_with(['=', '+', '-', '@']) { format!("'{text}") } else { text.to_string() };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

fn cents_text(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", cents.abs() / 100, cents.abs() % 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Entry, MonthKey};
    use crate::recurring::RecurringStore;
    use crate::storage::MonthData;

    fn temp_storage(name: &str) -> Storage {
        let dir = std::env::temp_dir().join(format!("budget-test-{}-backup-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Storage::new(dir)
    }

    fn filled(name: &str) -> (Storage, Categories) {
        let st = temp_storage(name);
        let mut cats = Categories::default();
        let rent = cats.add("Rent, \"home\"", Kind::Expense, None, 0x112233);
        st.save_categories(&cats).unwrap();
        st.save_month(
            MonthKey::new(2026, 10).unwrap(),
            &MonthData { entries: vec![Entry { id: 1, category_id: rent, name: "=SUM(A1)".into(), amount_cents: 123456 }] },
        )
        .unwrap();
        st.save_month(
            MonthKey::new(2026, 11).unwrap(),
            &MonthData { entries: vec![Entry { id: 1, category_id: rent, name: "Flat".into(), amount_cents: 5 }] },
        )
        .unwrap();
        let mut recurring = RecurringStore::default();
        recurring.add(rent, "Flat".into(), 5, MonthKey::new(2026, 11).unwrap());
        st.save_recurring(&recurring).unwrap();
        (st, cats)
    }

    #[test]
    fn backup_round_trips_through_json_and_restore() {
        let (st, cats) = filled("roundtrip");
        // Settings are kept apart and stay on the computer that restores.
        let mut settings = crate::settings::Settings::default();
        settings.labels.total = "Mine".into();
        st.save_settings(&settings).unwrap();

        let json = st.snapshot().to_json().unwrap();
        let parsed = parse(&json).unwrap();
        assert_eq!(parsed.categories, cats);
        assert_eq!(parsed.months.len(), 2);
        assert_eq!(parsed.recurring.items.len(), 1);

        let other = temp_storage("restore-target");
        let mut other_settings = crate::settings::Settings::default();
        other_settings.labels.total = "Other computer".into();
        other.save_settings(&other_settings).unwrap();
        other
            .save_month(
                MonthKey::new(2020, 1).unwrap(),
                &MonthData { entries: vec![Entry { id: 1, category_id: 1, name: "old".into(), amount_cents: 1 }] },
            )
            .unwrap();
        other.replace_all(&parsed).unwrap();
        // Everything is replaced, including months that are not in the backup...
        assert_eq!(other.list_months(), st.list_months());
        assert_eq!(other.load_categories(), cats);
        assert_eq!(other.load_recurring(), st.load_recurring());
        // ...but the settings of this computer are not.
        assert_eq!(other.load_settings().value.labels.total, "Other computer");
        let _ = fs::remove_dir_all(st.dir());
        let _ = fs::remove_dir_all(other.dir());
    }

    #[test]
    fn rejects_other_files_and_newer_versions() {
        assert!(parse("not json").is_err());
        assert!(parse("{}").is_err());
        let (st, _) = filled("reject");
        let json = st.snapshot().to_json().unwrap();
        assert!(parse(&json.replace("\"app\": \"cashflow\"", "\"app\": \"other\"")).unwrap_err().contains("not a CashFlow"));
        assert!(parse(&json.replace("\"version\": 1", "\"version\": 99")).unwrap_err().contains("newer version"));
        assert!(parse(&json.replace("\"2026-10\"", "\"october\"")).is_err());
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn safety_copy_is_a_restorable_backup() {
        let (st, _) = filled("safety");
        let path = save_safety_copy(&st).unwrap();
        assert!(path.starts_with(st.dir().join("backups")));
        assert_eq!(parse(&fs::read_to_string(&path).unwrap()).unwrap().months.len(), 2);
        // The backups folder is not mistaken for data.
        assert_eq!(st.list_months().len(), 2);
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn csv_quotes_text_and_defuses_formulas() {
        let (st, _) = filled("csv");
        let csv = st.snapshot().to_csv();
        let lines: Vec<&str> = csv.trim_start_matches('\u{feff}').lines().collect();
        assert_eq!(lines[0], "month,type,category,note,amount");
        assert_eq!(lines[1], "2026-10,expense,\"Rent, \"\"home\"\"\",\"'=SUM(A1)\",1234.56");
        assert_eq!(lines[2], "2026-11,expense,\"Rent, \"\"home\"\"\",\"Flat\",0.05");
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn formats_negative_cents() {
        assert_eq!(cents_text(-1050), "-10.50");
        assert_eq!(cents_text(0), "0.00");
    }
}
