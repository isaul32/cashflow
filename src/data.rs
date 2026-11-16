//! The data file: everything the user has recorded (categories, recurring
//! entries and all months) in one JSON document, `data.json`.
//!
//! The document carries a schema `version`. The version only changes when the
//! layout changes in a way an older app could not read; it is not the release
//! number of the app. Files with an older version are upgraded step by step by
//! the functions in [`MIGRATIONS`]; files with a newer version are refused so an
//! older app never overwrites data it does not understand.
//!
//! Adding a migration: append a function that turns a version N document into a
//! version N+1 document. [`VERSION`] follows from the number of functions.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{Categories, MonthKey};
use crate::recurring::RecurringStore;
use crate::storage::MonthData;

/// Marker that identifies a CashFlow data file.
const APP: &str = "cashflow";

/// Upgrades a document by one schema version.
pub(crate) type Migration = fn(Value) -> Result<Value, String>;

/// `MIGRATIONS[n]` turns a version `n + 1` document into version `n + 2`.
/// Version 1 is the first layout, so there is nothing to migrate yet.
pub(crate) const MIGRATIONS: &[Migration] = &[];

/// The schema version this app reads and writes.
pub const VERSION: u32 = MIGRATIONS.len() as u32 + 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DataFile {
    app: String,
    /// Schema version of the layout below.
    pub version: u32,
    /// App release that last wrote the file. Information only; never used for migrations.
    pub written_by: String,
    /// Local time of the last save.
    pub saved_at: String,
    pub categories: Categories,
    pub recurring: RecurringStore,
    /// Entries per month, keyed by `YYYY-MM`.
    pub months: BTreeMap<String, MonthData>,
}

impl Default for DataFile {
    fn default() -> Self {
        DataFile {
            app: APP.to_string(),
            version: VERSION,
            written_by: env!("CARGO_PKG_VERSION").to_string(),
            saved_at: String::new(),
            categories: Categories::default(),
            recurring: RecurringStore::default(),
            months: BTreeMap::new(),
        }
    }
}

/// Why a data file could not be used.
#[derive(Debug, PartialEq)]
pub enum DataError {
    /// Not readable as a CashFlow data file.
    Invalid(String),
    /// Written by a newer app; the number is the file's schema version.
    Newer(u32),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Invalid(why) => f.write_str(why),
            DataError::Newer(v) => write!(
                f,
                "the file was made by a newer version of CashFlow (data version {v}, this app reads up to {VERSION})"
            ),
        }
    }
}

impl DataFile {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Parses a data file, upgrading old versions. The second value is the
    /// version the file had, so callers can tell that an upgrade happened.
    pub fn parse(text: &str) -> Result<(DataFile, u32), DataError> {
        parse_with(text, MIGRATIONS)
    }
}

pub(crate) fn parse_with(text: &str, steps: &[Migration]) -> Result<(DataFile, u32), DataError> {
    // Editors such as Notepad may save JSON with a UTF-8 byte order mark.
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| DataError::Invalid(e.to_string()))?;
    if value.get("app").and_then(Value::as_str) != Some(APP) {
        return Err(DataError::Invalid("the file is not a CashFlow data file".to_string()));
    }
    let (value, from) = upgrade(value, steps)?;
    let data: DataFile = serde_json::from_value(value).map_err(|e| DataError::Invalid(e.to_string()))?;
    if let Some(bad) = data.months.keys().find(|k| k.parse::<MonthKey>().is_err()) {
        return Err(DataError::Invalid(format!("unknown month \"{bad}\"")));
    }
    Ok((data, from))
}

/// Runs the migrations from the document's version up to the newest one.
fn upgrade(mut value: Value, steps: &[Migration]) -> Result<(Value, u32), DataError> {
    let newest = steps.len() as u32 + 1;
    let found = value
        .get("version")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .filter(|&v| v >= 1)
        .ok_or_else(|| DataError::Invalid("the file has no data version".to_string()))?;
    if found > newest {
        return Err(DataError::Newer(found));
    }
    for version in found..newest {
        value = steps[(version - 1) as usize](value)
            .map_err(|e| DataError::Invalid(format!("upgrading from data version {version} failed: {e}")))?;
        value["version"] = Value::from(version + 1);
    }
    Ok((value, found))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn v1_to_v2(mut v: Value) -> Result<Value, String> {
        // Test migration: "months" entries gain a "currency" marker.
        v["currency_note"] = json!("eur");
        Ok(v)
    }

    fn v2_to_v3(mut v: Value) -> Result<Value, String> {
        let note = v["currency_note"].take();
        v["moved_note"] = note;
        Ok(v)
    }

    fn failing(_: Value) -> Result<Value, String> {
        Err("boom".to_string())
    }

    #[test]
    fn the_current_version_follows_the_migration_list() {
        assert_eq!(VERSION, MIGRATIONS.len() as u32 + 1);
        let json = DataFile::default().to_json().unwrap();
        let (parsed, from) = DataFile::parse(&json).unwrap();
        assert_eq!(from, VERSION);
        assert_eq!(parsed, DataFile::default());
    }

    #[test]
    fn migrations_run_in_order_from_the_files_version() {
        let doc = json!({"app": "cashflow", "version": 1});
        let (up, from) = upgrade(doc, &[v1_to_v2 as Migration, v2_to_v3]).unwrap();
        assert_eq!(from, 1);
        assert_eq!(up["version"], 3);
        assert_eq!(up["moved_note"], "eur");
        // A version 2 file skips the first step.
        let doc = json!({"app": "cashflow", "version": 2, "currency_note": "sek"});
        let (up, from) = upgrade(doc, &[v1_to_v2 as Migration, v2_to_v3]).unwrap();
        assert_eq!((from, up["version"].as_u64(), up["moved_note"].as_str()), (2, Some(3), Some("sek")));
    }

    #[test]
    fn newer_files_and_failing_migrations_are_reported() {
        let newer = json!({"app": "cashflow", "version": 5});
        assert_eq!(upgrade(newer, &[v1_to_v2 as Migration]).unwrap_err(), DataError::Newer(5));
        let doc = json!({"app": "cashflow", "version": 1});
        let err = upgrade(doc, &[failing as Migration]).unwrap_err();
        assert!(matches!(err, DataError::Invalid(ref m) if m.contains("boom")), "{err}");
        let no_version = json!({"app": "cashflow"});
        assert!(upgrade(no_version, &[]).is_err());
    }

    #[test]
    fn rejects_other_files_and_bad_months() {
        assert!(DataFile::parse("not json").is_err());
        assert!(DataFile::parse("{}").is_err());
        assert!(DataFile::parse(r#"{"app":"other","version":1}"#).is_err());
        let mut data = DataFile::default();
        data.months.insert("october".to_string(), MonthData::default());
        assert!(DataFile::parse(&data.to_json().unwrap()).is_err());
        let newer = DataFile::default().to_json().unwrap().replace("\"version\": 1", "\"version\": 99");
        assert_eq!(DataFile::parse(&newer).unwrap_err(), DataError::Newer(99));
    }

    #[test]
    fn a_byte_order_mark_is_ignored() {
        let json = format!("\u{feff}{}", DataFile::default().to_json().unwrap());
        assert!(DataFile::parse(&json).is_ok());
    }
}
