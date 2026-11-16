//! Full backups: every category, month and setting in one JSON file, plus a CSV
//! export of all entries for spreadsheets.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::{Categories, CategoryId, Kind, MonthKey};
use crate::settings::Settings;
use crate::storage::{MonthData, Storage};

/// Marker that identifies a CashFlow backup file.
const APP: &str = "cashflow";
/// Version of the backup layout. Files with a higher number come from a newer app.
const FORMAT: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Backup {
    app: String,
    format: u32,
    /// Local time the backup was made, for people reading the file.
    pub created: String,
    pub categories: Categories,
    pub settings: Settings,
    /// Entries per month, keyed by `YYYY-MM`.
    pub months: BTreeMap<String, MonthData>,
}

impl Backup {
    /// Reads everything that is saved on disk right now.
    pub fn collect(st: &Storage) -> Backup {
        let months = st
            .list_months()
            .into_iter()
            .map(|key| (key.to_string(), st.load_month(key).value))
            .collect();
        Backup {
            app: APP.to_string(),
            format: FORMAT,
            created: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            categories: st.load_categories().value,
            settings: st.load_settings().value,
            months,
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Parses and validates a backup file; the error text is meant for the user.
    pub fn parse(text: &str) -> Result<Backup, String> {
        let backup: Backup = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| e.to_string())?;
        if backup.app != APP {
            return Err("the file is not a CashFlow backup".to_string());
        }
        if backup.format > FORMAT {
            return Err("the file was made by a newer version of CashFlow".to_string());
        }
        if let Some(bad) = backup.months.keys().find(|k| k.parse::<MonthKey>().is_err()) {
            return Err(format!("unknown month \"{bad}\""));
        }
        Ok(backup)
    }

    /// Replaces everything in `st` with this backup: months that are not in the
    /// backup are removed. Call [`save_safety_copy`] first.
    pub fn restore(&self, st: &Storage) -> io::Result<()> {
        let keep: Vec<MonthKey> = self.months.keys().filter_map(|k| k.parse().ok()).collect();
        for key in st.list_months() {
            if !keep.contains(&key) {
                st.remove_month(key)?;
            }
        }
        st.save_categories(&self.categories)?;
        for (key, data) in &self.months {
            if let Ok(key) = key.parse() {
                st.save_month(key, data)?;
            }
        }
        st.save_settings(&self.settings)
    }

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

/// Writes the current data to `<data dir>/backups/` before it is replaced.
pub fn save_safety_copy(st: &Storage) -> io::Result<PathBuf> {
    let dir = st.dir().join("backups");
    fs::create_dir_all(&dir)?;
    let name = format!("before-restore-{}.json", chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let path = dir.join(name);
    let json = Backup::collect(st).to_json().map_err(io::Error::other)?;
    fs::write(&path, json)?;
    Ok(path)
}

/// Suggested file name, e.g. `cashflow-backup-2026-11-16.json`.
pub fn file_name(stem: &str, extension: &str) -> String {
    format!("cashflow-{stem}-{}.{extension}", chrono::Local::now().format("%Y-%m-%d"))
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
    use crate::model::Entry;

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
        let mut settings = Settings::default();
        settings.labels.total = "Budget".into();
        st.save_settings(&settings).unwrap();
        (st, cats)
    }

    #[test]
    fn backup_round_trips_through_json_and_restore() {
        let (st, cats) = filled("roundtrip");
        let json = Backup::collect(&st).to_json().unwrap();
        let parsed = Backup::parse(&json).unwrap();
        assert_eq!(parsed.categories, cats);
        assert_eq!(parsed.months.len(), 2);

        // Restoring into another folder reproduces everything and removes extra months.
        let other = temp_storage("restore-target");
        other.save_month(MonthKey::new(2020, 1).unwrap(), &MonthData::default()).unwrap();
        parsed.restore(&other).unwrap();
        assert_eq!(other.list_months(), st.list_months());
        assert_eq!(other.load_categories().value, cats);
        assert_eq!(other.load_settings().value.labels.total, "Budget");
        let _ = fs::remove_dir_all(st.dir());
        let _ = fs::remove_dir_all(other.dir());
    }

    #[test]
    fn rejects_other_files_and_newer_formats() {
        assert!(Backup::parse("not json").is_err());
        assert!(Backup::parse("{}").is_err());
        let (st, _) = filled("reject");
        let json = Backup::collect(&st).to_json().unwrap();
        let foreign = json.replace("\"app\": \"cashflow\"", "\"app\": \"other\"");
        assert!(Backup::parse(&foreign).unwrap_err().contains("not a CashFlow backup"));
        let newer = json.replace("\"format\": 1", "\"format\": 99");
        assert!(Backup::parse(&newer).unwrap_err().contains("newer version"));
        let bad_month = json.replace("\"2026-10\"", "\"october\"");
        assert!(Backup::parse(&bad_month).is_err());
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn safety_copy_is_a_restorable_backup() {
        let (st, _) = filled("safety");
        let path = save_safety_copy(&st).unwrap();
        assert!(path.starts_with(st.dir().join("backups")));
        assert!(Backup::parse(&fs::read_to_string(&path).unwrap()).is_ok());
        // The backups folder is not mistaken for a month.
        assert_eq!(st.list_months().len(), 2);
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn csv_quotes_text_and_defuses_formulas() {
        let (st, _) = filled("csv");
        let csv = Backup::collect(&st).to_csv();
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
