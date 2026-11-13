//! JSON persistence: `categories.json` (shared) and one `YYYY-MM.json` per month.
//!
//! Writes are atomic (temp file + rename). A file that fails to parse is
//! renamed to `*.corrupt` so it is never silently overwritten.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Categories, Entry, MonthKey};
use crate::settings::Settings;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MonthData {
    pub entries: Vec<Entry>,
}

impl MonthData {
    pub fn next_entry_id(&self) -> u64 {
        self.entries.iter().map(|e| e.id).max().map_or(1, |m| m + 1)
    }
}

#[derive(Clone, Debug)]
pub struct Storage {
    dir: PathBuf,
}

/// Result of loading a file: the value plus a note if a corrupt file was set aside.
pub struct Loaded<T> {
    pub value: T,
    pub warning: Option<String>,
}

impl Storage {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Storage { dir: dir.into() }
    }

    /// Data location, in order of precedence:
    /// 1. `BUDGET_DATA_DIR`,
    /// 2. portable mode: a file named `portable` next to the executable
    ///    keeps everything in `<exe dir>/data`,
    /// 3. the platform data directory.
    pub fn default_location() -> Option<Self> {
        if let Some(dir) = std::env::var_os("BUDGET_DATA_DIR") {
            return Some(Self::new(dir));
        }
        if let Some(dir) = std::env::current_exe().ok().and_then(|exe| portable_dir(&exe)) {
            return Some(Self::new(dir));
        }
        // The folder is still called "budget" so data from earlier versions is kept.
        directories::ProjectDirs::from("", "", "budget").map(|d| Self::new(d.data_dir()))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn categories_path(&self) -> PathBuf {
        self.dir.join("categories.json")
    }

    fn settings_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    pub fn load_settings(&self) -> Loaded<Settings> {
        self.load(&self.settings_path())
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        self.save(&self.settings_path(), settings)
    }

    fn month_path(&self, key: MonthKey) -> PathBuf {
        self.dir.join(format!("{key}.json"))
    }

    /// False on a first run, before categories were ever saved.
    pub fn has_categories(&self) -> bool {
        self.categories_path().is_file()
    }

    pub fn load_categories(&self) -> Loaded<Categories> {
        self.load(&self.categories_path())
    }

    pub fn save_categories(&self, cats: &Categories) -> io::Result<()> {
        self.save(&self.categories_path(), cats)
    }

    pub fn load_month(&self, key: MonthKey) -> Loaded<MonthData> {
        self.load(&self.month_path(key))
    }

    pub fn save_month(&self, key: MonthKey, data: &MonthData) -> io::Result<()> {
        self.save(&self.month_path(key), data)
    }

    /// Months that have a data file, oldest first.
    pub fn list_months(&self) -> Vec<MonthKey> {
        let mut months: Vec<MonthKey> = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                name.strip_suffix(".json")?.parse().ok()
            })
            .collect();
        months.sort();
        months
    }

    fn load<T: for<'de> Deserialize<'de> + Default>(&self, path: &Path) -> Loaded<T> {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Loaded { value: T::default(), warning: None };
            }
            Err(e) => {
                return Loaded {
                    value: T::default(),
                    warning: Some(format!("Could not read {}: {e}", path.display())),
                };
            }
        };
        // Editors such as Notepad may save JSON with a UTF-8 byte order mark.
        match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
            Ok(value) => Loaded { value, warning: None },
            Err(e) => {
                let aside = path.with_extension("json.corrupt");
                let moved = fs::rename(path, &aside).is_ok();
                Loaded {
                    value: T::default(),
                    warning: Some(format!(
                        "{} is corrupt ({e}){}",
                        path.display(),
                        if moved { format!("; saved as {}", aside.display()) } else { String::new() }
                    )),
                }
            }
        }
    }

    fn save<T: Serialize>(&self, path: &Path, value: &T) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, path)
    }
}

/// Data directory for portable mode, if the marker file is present.
fn portable_dir(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    dir.join("portable").is_file().then(|| dir.join("data"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    #[test]
    fn portable_mode_needs_marker_file() {
        let dir = std::env::temp_dir().join(format!("budget-test-{}-portable", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("cashflow.exe");
        assert_eq!(portable_dir(&exe), None);
        fs::write(dir.join("portable"), "").unwrap();
        assert_eq!(portable_dir(&exe), Some(dir.join("data")));
        let _ = fs::remove_dir_all(&dir);
    }

    fn temp_storage(name: &str) -> Storage {
        let dir = std::env::temp_dir().join(format!("budget-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Storage::new(dir)
    }

    #[test]
    fn missing_files_give_defaults() {
        let s = temp_storage("missing");
        let c = s.load_categories();
        assert!(c.warning.is_none());
        assert!(s.load_month(MonthKey::new(2026, 1).unwrap()).value.entries.is_empty());
        assert!(s.list_months().is_empty());
    }

    #[test]
    fn round_trips_categories_and_months() {
        let s = temp_storage("roundtrip");
        let mut cats = Categories::default();
        let extra = cats.add("Pets", Kind::Expense, None, 0x123456);
        s.save_categories(&cats).unwrap();
        assert_eq!(s.load_categories().value, cats);

        let key = MonthKey::new(2026, 11).unwrap();
        let data = MonthData {
            entries: vec![Entry { id: 7, category_id: extra, name: "Food".into(), amount_cents: 1999 }],
        };
        s.save_month(key, &data).unwrap();
        assert_eq!(s.load_month(key).value, data);
        assert_eq!(data.next_entry_id(), 8);
        assert_eq!(s.list_months(), vec![key]);
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn settings_round_trip_and_do_not_count_as_a_month() {
        let s = temp_storage("settings");
        assert_eq!(s.load_settings().value, Settings::default());
        let mut settings = Settings::default();
        settings.labels.total = "Budget".to_string();
        s.save_settings(&settings).unwrap();
        assert_eq!(s.load_settings().value, settings);
        assert!(s.list_months().is_empty());
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn files_saved_with_a_byte_order_mark_still_load() {
        let s = temp_storage("bom");
        fs::create_dir_all(s.dir()).unwrap();
        fs::write(s.dir().join("settings.json"), "\u{feff}{\"language\":\"fi\"}").unwrap();
        let loaded = s.load_settings();
        assert!(loaded.warning.is_none(), "{:?}", loaded.warning);
        assert_eq!(loaded.value.language, Some(crate::i18n::Lang::Fi));
        assert!(s.dir().join("settings.json").exists(), "the file must not be set aside");
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn corrupt_file_is_set_aside_not_overwritten() {
        let s = temp_storage("corrupt");
        fs::create_dir_all(s.dir()).unwrap();
        fs::write(s.dir().join("categories.json"), "{ not json").unwrap();
        let loaded = s.load_categories();
        assert!(loaded.warning.is_some());
        assert!(s.dir().join("categories.json.corrupt").exists());
        assert!(!s.dir().join("categories.json").exists());
        let _ = fs::remove_dir_all(s.dir());
    }
}
