//! Persistence in the data folder.
//!
//! - `data.json`: everything the user recorded (categories, recurring entries,
//!   months), see [`crate::data`]. The whole document is kept in memory and
//!   written as one unit, so a change that touches several months can never be
//!   saved half way.
//! - `settings.json`: preferences of this computer (language, theme, ...). It is
//!   not part of the data, so restoring a backup does not change it.
//!
//! Writes are atomic (temp file + rename) and the previous save is kept as
//! `data.json.bak`. A data file that fails to parse is renamed to `*.corrupt`
//! (and the previous save is used instead) so nothing is silently overwritten.
//! A file from a newer app is never written to. Data folders from before
//! `data.json` existed (`categories.json` and one `YYYY-MM.json` per month) are
//! imported once; the old files are moved to `legacy/`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use crate::data::{DataError, DataFile, Migration};
use crate::model::{Categories, Entry, MonthKey};
use crate::recurring::RecurringStore;
use crate::settings::Settings;

const DATA_FILE: &str = "data.json";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MonthData {
    pub entries: Vec<Entry>,
}

impl MonthData {
    pub fn next_entry_id(&self) -> u64 {
        self.entries.iter().map(|e| e.id).max().map_or(1, |m| m + 1)
    }
}

#[derive(Debug)]
struct State {
    data: DataFile,
    /// Why saving is refused (the file on disk could not be read safely), if it is.
    locked: Option<String>,
    /// Problems found while opening, for [`Storage::take_warnings`].
    warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Storage {
    dir: PathBuf,
    state: Arc<Mutex<State>>,
}

/// Result of loading the settings file: the value plus a note if a corrupt file was set aside.
pub struct Loaded<T> {
    pub value: T,
    pub warning: Option<String>,
}

impl Storage {
    /// Opens the data folder: reads `data.json`, upgrades or imports older layouts.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::open_with(dir, crate::data::MIGRATIONS)
    }

    fn open_with(dir: impl Into<PathBuf>, steps: &[Migration]) -> Self {
        let dir = dir.into();
        let state = State::open(&dir, steps);
        Storage { dir, state: Arc::new(Mutex::new(state)) }
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

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Problems found when the data was opened (corrupt or unreadable files).
    /// Returned once.
    pub fn take_warnings(&self) -> Vec<String> {
        std::mem::take(&mut self.lock().warnings)
    }

    /// Applies a change and writes the whole data file.
    fn update(&self, change: impl FnOnce(&mut DataFile)) -> io::Result<()> {
        let mut state = self.lock();
        if let Some(why) = &state.locked {
            return Err(io::Error::other(why.clone()));
        }
        change(&mut state.data);
        write_data(&self.dir, &mut state.data)
    }

    /// A copy of everything that is recorded.
    pub fn snapshot(&self) -> DataFile {
        self.lock().data.clone()
    }

    /// Replaces everything that is recorded, e.g. when restoring a backup.
    pub fn replace_all(&self, data: &DataFile) -> io::Result<()> {
        self.update(|d| *d = data.clone())
    }

    // ---- settings (their own file) ----

    fn settings_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    pub fn load_settings(&self) -> Loaded<Settings> {
        load_settings_file(&self.settings_path())
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        write_atomic(&self.settings_path(), &serde_json::to_string_pretty(settings).map_err(io::Error::other)?)
    }

    // ---- data ----

    /// False on a first run, before anything was ever saved.
    pub fn has_categories(&self) -> bool {
        self.dir.join(DATA_FILE).is_file()
    }

    pub fn load_categories(&self) -> Categories {
        self.lock().data.categories.clone()
    }

    #[cfg(test)]
    pub fn save_categories(&self, cats: &Categories) -> io::Result<()> {
        self.update(|d| d.categories = cats.clone())
    }

    pub fn load_recurring(&self) -> RecurringStore {
        self.lock().data.recurring.clone()
    }

    pub fn save_recurring(&self, store: &RecurringStore) -> io::Result<()> {
        self.update(|d| d.recurring = store.clone())
    }

    /// A month that has no entries is the same as a month that was never saved.
    pub fn load_month(&self, key: MonthKey) -> MonthData {
        self.lock().data.months.get(&key.to_string()).cloned().unwrap_or_default()
    }

    pub fn save_month(&self, key: MonthKey, data: &MonthData) -> io::Result<()> {
        self.update(|d| set_month(d, key, data))
    }

    /// Saves the categories and one month in a single write.
    pub fn save_categories_and_month(&self, cats: &Categories, key: MonthKey, data: &MonthData) -> io::Result<()> {
        self.update(|d| {
            d.categories = cats.clone();
            set_month(d, key, data);
        })
    }

    /// Months that have entries, oldest first.
    pub fn list_months(&self) -> Vec<MonthKey> {
        let mut months: Vec<MonthKey> = self.lock().data.months.keys().filter_map(|k| k.parse().ok()).collect();
        months.sort();
        months
    }
}

fn set_month(data: &mut DataFile, key: MonthKey, month: &MonthData) {
    if month.entries.is_empty() {
        data.months.remove(&key.to_string());
    } else {
        data.months.insert(key.to_string(), month.clone());
    }
}

impl State {
    fn open(dir: &Path, steps: &[Migration]) -> State {
        let mut st = State { data: DataFile::default(), locked: None, warnings: Vec::new() };
        let path = dir.join(DATA_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => match crate::data::parse_with(&text, steps) {
                Ok((data, from)) => {
                    st.data = data;
                    if from < steps.len() as u32 + 1 {
                        st.save_upgraded(dir, &text, from);
                    }
                }
                Err(DataError::Newer(version)) => {
                    let why = format!("{}: {}", path.display(), DataError::Newer(version));
                    st.locked = Some(why.clone());
                    st.warnings.push(why);
                }
                Err(DataError::Invalid(why)) => st.recover(dir, &path, &why, steps),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => st.import_legacy(dir),
            Err(e) => {
                let why = format!("Could not read {}: {e}", path.display());
                st.locked = Some(why.clone());
                st.warnings.push(why);
            }
        }
        st
    }

    /// After an upgrade: keep the old file as `data.v<N>.json.bak`, then save the new layout.
    fn save_upgraded(&mut self, dir: &Path, original: &str, from: u32) {
        let keep = dir.join(format!("data.v{from}.json.bak"));
        if let Err(e) = fs::write(&keep, original) {
            // Without a copy of the old file the upgraded data must not replace it.
            let why = format!("Could not save {} before upgrading the data: {e}", keep.display());
            self.locked = Some(why.clone());
            self.warnings.push(why);
            return;
        }
        if let Err(e) = write_data(dir, &mut self.data) {
            self.warnings.push(format!("Could not save the upgraded data file: {e}"));
        }
    }

    /// A corrupt data file is set aside; the previous save is used if it is readable.
    fn recover(&mut self, dir: &Path, path: &Path, why: &str, steps: &[Migration]) {
        let aside = path.with_extension("json.corrupt");
        let moved = fs::rename(path, &aside).is_ok();
        let note = format!(
            "{} is corrupt ({why}){}",
            path.display(),
            if moved { format!("; saved as {}", aside.display()) } else { String::new() }
        );
        let bak = dir.join("data.json.bak");
        let previous = fs::read_to_string(&bak).ok().and_then(|t| crate::data::parse_with(&t, steps).ok());
        match previous {
            Some((data, _)) => {
                self.data = data;
                self.warnings.push(format!("{note}. Restored the previous save from {}", bak.display()));
                if let Err(e) = write_data(dir, &mut self.data) {
                    self.warnings.push(format!("Could not save the restored data file: {e}"));
                }
            }
            None => self.warnings.push(note),
        }
    }

    /// Imports the layout used before `data.json`: `categories.json` and `YYYY-MM.json` files.
    /// Imported files are moved to `legacy/`; one that cannot be read stays where it is.
    fn import_legacy(&mut self, dir: &Path) {
        let months: Vec<(MonthKey, PathBuf)> = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                Some((name.strip_suffix(".json")?.parse().ok()?, e.path()))
            })
            .collect();
        let categories_path = dir.join("categories.json");
        if !categories_path.is_file() && months.is_empty() {
            return; // a first run
        }

        let mut imported: Vec<PathBuf> = Vec::new();
        if categories_path.is_file() {
            match read_json::<Categories>(&categories_path) {
                Ok(cats) => {
                    self.data.categories = cats;
                    imported.push(categories_path);
                }
                Err(e) => self.warnings.push(e),
            }
        }
        for (key, path) in months {
            match read_json::<MonthData>(&path) {
                Ok(month) => {
                    set_month(&mut self.data, key, &month);
                    imported.push(path);
                }
                Err(e) => self.warnings.push(e),
            }
        }
        if let Err(e) = write_data(dir, &mut self.data) {
            // The old files stay where they are, so nothing is lost.
            self.warnings.push(format!("Could not save {DATA_FILE}: {e}"));
            return;
        }
        let legacy = dir.join("legacy");
        if let Err(e) = fs::create_dir_all(&legacy) {
            self.warnings.push(format!("Could not create {}: {e}", legacy.display()));
            return;
        }
        for path in imported {
            let Some(name) = path.file_name() else { continue };
            if let Err(e) = fs::rename(&path, legacy.join(name)) {
                self.warnings.push(format!("Could not move {} to {}: {e}", path.display(), legacy.display()));
            }
        }
    }
}

/// Writes `data.json` atomically, keeping the previous save as `data.json.bak`.
fn write_data(dir: &Path, data: &mut DataFile) -> io::Result<()> {
    data.written_by = env!("CARGO_PKG_VERSION").to_string();
    data.saved_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let json = data.to_json().map_err(io::Error::other)?;
    fs::create_dir_all(dir)?;
    let path = dir.join(DATA_FILE);
    if path.is_file() {
        // A failed copy must not block saving; the new file is still written safely.
        let _ = fs::copy(&path, dir.join("data.json.bak"));
    }
    write_atomic(&path, &json)
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    // Editors such as Notepad may save JSON with a UTF-8 byte order mark.
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("{} could not be imported ({e})", path.display()))
}

fn load_settings_file(path: &Path) -> Loaded<Settings> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Loaded { value: Settings::default(), warning: None };
        }
        Err(e) => {
            return Loaded {
                value: Settings::default(),
                warning: Some(format!("Could not read {}: {e}", path.display())),
            };
        }
    };
    match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
        Ok(value) => Loaded { value, warning: None },
        Err(e) => {
            let aside = path.with_extension("json.corrupt");
            let moved = fs::rename(path, &aside).is_ok();
            Loaded {
                value: Settings::default(),
                warning: Some(format!(
                    "{} is corrupt ({e}){}",
                    path.display(),
                    if moved { format!("; saved as {}", aside.display()) } else { String::new() }
                )),
            }
        }
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
    use serde_json::{Value, json};

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

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("budget-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn temp_storage(name: &str) -> Storage {
        Storage::new(temp_dir(name))
    }

    fn month(y: i32, m: u8) -> MonthKey {
        MonthKey::new(y, m).unwrap()
    }

    #[test]
    fn missing_files_give_defaults() {
        let s = temp_storage("missing");
        assert!(s.take_warnings().is_empty());
        assert!(!s.has_categories());
        assert!(s.load_month(month(2026, 1)).entries.is_empty());
        assert!(s.list_months().is_empty());
        assert!(!s.dir().exists(), "opening an empty folder must not create files");
    }

    #[test]
    fn round_trips_through_one_file() {
        let s = temp_storage("roundtrip");
        let mut cats = Categories::default();
        let extra = cats.add("Pets", Kind::Expense, None, 0x123456);
        let key = month(2026, 11);
        let data = MonthData {
            entries: vec![Entry { id: 7, category_id: extra, name: "Food".into(), amount_cents: 1999 }],
        };
        s.save_categories(&cats).unwrap();
        s.save_month(key, &data).unwrap();
        assert_eq!(data.next_entry_id(), 8);
        assert_eq!(s.list_months(), vec![key]);

        // A new Storage on the same folder reads everything back from data.json alone.
        let again = Storage::new(s.dir());
        assert!(again.has_categories());
        assert_eq!(again.load_categories(), cats);
        assert_eq!(again.load_month(key), data);
        assert!(s.dir().join("data.json").is_file());
        assert!(!s.dir().join("categories.json").exists());
        assert!(!s.dir().join("2026-11.json").exists());

        // An emptied month disappears from the file.
        again.save_month(key, &MonthData::default()).unwrap();
        assert!(again.list_months().is_empty());
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn the_previous_save_is_kept_as_a_backup() {
        let s = temp_storage("bak");
        let key = month(2026, 5);
        let one = MonthData { entries: vec![Entry { id: 1, category_id: 1, name: "a".into(), amount_cents: 1 }] };
        s.save_month(key, &one).unwrap();
        s.save_month(key, &MonthData::default()).unwrap();
        let bak = fs::read_to_string(s.dir().join("data.json.bak")).unwrap();
        assert!(DataFile::parse(&bak).unwrap().0.months.contains_key("2026-05"));
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn settings_have_their_own_file_and_are_not_data() {
        let s = temp_storage("settings");
        assert_eq!(s.load_settings().value, Settings::default());
        let mut settings = Settings::default();
        settings.labels.total = "Budget".to_string();
        s.save_settings(&settings).unwrap();
        assert_eq!(s.load_settings().value, settings);
        assert!(s.list_months().is_empty());
        assert!(!s.dir().join("data.json").exists(), "settings must not create the data file");
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn settings_saved_with_a_byte_order_mark_still_load() {
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
    fn a_corrupt_data_file_is_set_aside_not_overwritten() {
        let dir = temp_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("data.json"), "{ not json").unwrap();
        let s = Storage::new(&dir);
        assert_eq!(s.take_warnings().len(), 1);
        assert!(dir.join("data.json.corrupt").exists());
        assert!(!dir.join("data.json").exists());
        assert!(s.list_months().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_corrupt_data_file_falls_back_to_the_previous_save() {
        let s = temp_storage("recover");
        let key = month(2026, 2);
        let data = MonthData { entries: vec![Entry { id: 1, category_id: 1, name: "x".into(), amount_cents: 5 }] };
        s.save_month(key, &data).unwrap();
        s.save_categories(&Categories::default()).unwrap(); // the backup now holds the month
        fs::write(s.dir().join("data.json"), "garbage").unwrap();
        let again = Storage::new(s.dir());
        let warnings = again.take_warnings();
        assert!(warnings.iter().any(|w| w.contains("Restored the previous save")), "{warnings:?}");
        assert_eq!(again.load_month(key), data);
        assert!(s.dir().join("data.json.corrupt").exists());
        assert!(DataFile::parse(&fs::read_to_string(s.dir().join("data.json")).unwrap()).is_ok());
        let _ = fs::remove_dir_all(s.dir());
    }

    #[test]
    fn a_file_from_a_newer_app_is_never_written() {
        let dir = temp_dir("newer");
        fs::create_dir_all(&dir).unwrap();
        let text = DataFile::default().to_json().unwrap().replace("\"version\": 1", "\"version\": 99");
        fs::write(dir.join("data.json"), &text).unwrap();
        let s = Storage::new(&dir);
        assert!(s.take_warnings()[0].contains("newer version"));
        assert!(s.save_month(month(2026, 1), &MonthData::default()).is_err());
        assert!(s.save_categories(&Categories::default()).is_err());
        assert_eq!(fs::read_to_string(dir.join("data.json")).unwrap(), text, "the file must stay untouched");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_layouts_are_upgraded_and_the_old_file_is_kept() {
        fn cats_to_categories(mut v: Value) -> Result<Value, String> {
            let cats = v["cats"].take();
            v["categories"] = cats;
            Ok(v)
        }
        let dir = temp_dir("upgrade");
        fs::create_dir_all(&dir).unwrap();
        let cats = serde_json::to_value(Categories::default()).unwrap();
        let old = json!({"app": "cashflow", "version": 1, "cats": cats}).to_string();
        fs::write(dir.join("data.json"), &old).unwrap();

        let s = Storage::open_with(&dir, &[cats_to_categories as Migration]);
        assert!(s.take_warnings().is_empty());
        assert_eq!(s.load_categories(), Categories::default());
        assert_eq!(fs::read_to_string(dir.join("data.v1.json.bak")).unwrap(), old);
        let saved: Value = serde_json::from_str(&fs::read_to_string(dir.join("data.json")).unwrap()).unwrap();
        assert_eq!(saved["version"], 2);
        assert!(saved.get("categories").is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_old_multi_file_layout_is_imported_once() {
        let dir = temp_dir("legacy");
        fs::create_dir_all(&dir).unwrap();
        let mut cats = Categories::default();
        let rent = cats.add("Rent", Kind::Expense, None, 0xAABBCC);
        fs::write(dir.join("categories.json"), serde_json::to_string(&cats).unwrap()).unwrap();
        let oct = MonthData { entries: vec![Entry { id: 1, category_id: rent, name: "Flat".into(), amount_cents: 80000 }] };
        fs::write(dir.join("2026-10.json"), serde_json::to_string(&oct).unwrap()).unwrap();
        fs::write(dir.join("2026-11.json"), "{ broken").unwrap();
        fs::write(dir.join("settings.json"), "{\"language\":\"fi\"}").unwrap();

        let s = Storage::new(&dir);
        assert_eq!(s.load_categories(), cats);
        assert_eq!(s.load_month(month(2026, 10)), oct);
        assert_eq!(s.list_months(), vec![month(2026, 10)]);
        // The unreadable month is reported and left in place; the others moved to legacy/.
        let warnings = s.take_warnings();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(dir.join("2026-11.json").exists());
        assert!(dir.join("legacy").join("categories.json").exists());
        assert!(dir.join("legacy").join("2026-10.json").exists());
        assert!(!dir.join("categories.json").exists());
        assert!(dir.join("settings.json").exists(), "settings stay where they are");
        assert!(dir.join("data.json").is_file());

        // A second start reads data.json and imports nothing again.
        let again = Storage::new(&dir);
        assert_eq!(again.load_month(month(2026, 10)), oct);
        let _ = fs::remove_dir_all(&dir);
    }
}
