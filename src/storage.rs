//! Persistence in the data folder.
//!
//! - `data.json`: everything the user recorded (categories, recurring entries,
//!   months), see [`crate::data`]. The whole document is kept in memory and
//!   written as one unit, so a change that touches several months can never be
//!   saved half way.
//! - `settings.json`: preferences of this computer (language, theme, ...). It is
//!   not part of the data, so restoring a backup does not change it.
//!
//! Two processes can use the same folder: the app's window and an MCP session started by an
//! AI assistant (`cashflow --mcp`). Every write takes a lock file (`data.lock`), first
//! reads again if the other process changed the file, and only then applies its change, so no
//! change is lost. [`Storage::watch`] tells the window when the other process wrote.
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
use std::time::{Duration, Instant, SystemTime};

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
    /// What `data.json` looked like when this process last read or wrote it.
    stamp: Stamp,
    /// The same for `settings.json`.
    settings_stamp: Stamp,
}

/// Modification time and size of a file: enough to notice that another process changed it.
type Stamp = Option<(SystemTime, u64)>;

fn stamp_of(path: &Path) -> Stamp {
    let meta = fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// What another process changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// `data.json`
    Data,
    /// `settings.json`
    Settings,
}

/// Held while writing `data.json`, so two processes never write at the same time.
struct FileLock(PathBuf);

impl FileLock {
    /// A lock file older than this was left by a crashed process.
    const STALE: Duration = Duration::from_secs(10);
    const GIVE_UP: Duration = Duration::from_secs(5);

    fn acquire(dir: &Path) -> io::Result<FileLock> {
        fs::create_dir_all(dir)?;
        let path = dir.join("data.lock");
        let started = Instant::now();
        loop {
            match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(FileLock(path)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    let stale = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age > Self::STALE);
                    if stale {
                        let _ = fs::remove_file(&path);
                    } else if started.elapsed() > Self::GIVE_UP {
                        return Err(io::Error::new(io::ErrorKind::TimedOut, "another CashFlow process is writing the data"));
                    } else {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
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

    /// Applies a change to the latest data and writes the whole data file.
    fn update(&self, change: impl FnOnce(&mut DataFile)) -> io::Result<()> {
        let mut state = self.lock();
        if let Some(why) = &state.locked {
            return Err(io::Error::other(why.clone()));
        }
        let _file_lock = FileLock::acquire(&self.dir)?;
        // The other process may have written since this one last looked. The file is read again
        // whatever its time and size say: two quick writes can leave both the same, and a change
        // built on stale data would wipe out the other process's.
        state.refresh(&self.dir, true)?;
        change(&mut state.data);
        state.write(&self.dir)
    }

    /// Reads `data.json` again if another process changed it. True if it did.
    pub fn reload_if_changed(&self) -> bool {
        let mut state = self.lock();
        state.refresh(&self.dir, false).unwrap_or(false)
    }

    /// Starts watching for changes made by another process (every `interval`). Changes this
    /// storage made itself are not reported. The watching stops when the receiver is dropped.
    pub fn watch(&self, interval: Duration) -> async_channel::Receiver<Change> {
        let (tx, rx) = async_channel::unbounded();
        let storage = self.clone();
        // If no thread can be made there is no watching: the receiver just never gets anything.
        let _ = std::thread::Builder::new().name("data-watch".into()).spawn(move || {
            let (mut told_data, mut told_settings): (Stamp, Stamp) = (None, None);
            loop {
                std::thread::sleep(interval);
                if tx.is_closed() {
                    return;
                }
                let (data_now, settings_now) =
                    (stamp_of(&storage.dir.join(DATA_FILE)), stamp_of(&storage.settings_path()));
                let (known_data, known_settings) = {
                    let state = storage.lock();
                    (state.stamp, state.settings_stamp)
                };
                // Each change is reported once, even if the receiver is slow to act on it.
                if data_now != known_data && data_now != told_data {
                    told_data = data_now;
                    let _ = tx.try_send(Change::Data);
                }
                if settings_now != known_settings && settings_now != told_settings {
                    told_settings = settings_now;
                    let _ = tx.try_send(Change::Settings);
                }
            }
        });
        rx
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
        let loaded = load_settings_file(&self.settings_path());
        self.lock().settings_stamp = stamp_of(&self.settings_path());
        loaded
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.settings_path();
        write_atomic(&path, &serde_json::to_string_pretty(settings).map_err(io::Error::other)?)?;
        self.lock().settings_stamp = stamp_of(&path);
        Ok(())
    }

    // ---- data ----

    /// False on a first run, before anything was ever saved.
    pub fn has_categories(&self) -> bool {
        self.dir.join(DATA_FILE).is_file()
    }

    pub fn load_categories(&self) -> Categories {
        self.lock().data.categories.clone()
    }

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
    /// Writes the data and remembers how the file looks now.
    fn write(&mut self, dir: &Path) -> io::Result<()> {
        self.stamp = write_data(dir, &mut self.data)?;
        Ok(())
    }

    /// Reads `data.json` again if it is not the file this process last saw (always, with `force`).
    /// True if the data was replaced by what was in the file.
    fn refresh(&mut self, dir: &Path, force: bool) -> io::Result<bool> {
        let path = dir.join(DATA_FILE);
        let now = stamp_of(&path);
        if now == self.stamp && !force {
            return Ok(false);
        }
        match fs::read_to_string(&path) {
            Ok(text) => match DataFile::parse(&text) {
                Ok((data, _)) => {
                    self.data = data;
                    self.stamp = now;
                    Ok(true)
                }
                Err(DataError::Newer(v)) => {
                    let why = format!("{}: {}", path.display(), DataError::Newer(v));
                    self.locked = Some(why.clone());
                    Err(io::Error::other(why))
                }
                Err(DataError::Invalid(why)) => Err(io::Error::other(format!("{} is unreadable: {why}", path.display()))),
            },
            // Deleted by hand: keep what is in memory, the next save writes it again.
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.stamp = None;
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    fn open(dir: &Path, steps: &[Migration]) -> State {
        let mut st = State {
            data: DataFile::default(),
            locked: None,
            warnings: Vec::new(),
            stamp: None,
            settings_stamp: None,
        };
        let path = dir.join(DATA_FILE);
        match fs::read_to_string(&path) {
            Ok(text) => match crate::data::parse_with(&text, steps) {
                Ok((data, from)) => {
                    st.data = data;
                    st.stamp = stamp_of(&path);
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
        if let Err(e) = self.write(dir) {
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
                if let Err(e) = self.write(dir) {
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
        if let Err(e) = self.write(dir) {
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
fn write_data(dir: &Path, data: &mut DataFile) -> io::Result<Stamp> {
    data.written_by = env!("CARGO_PKG_VERSION").to_string();
    data.saved_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let json = data.to_json().map_err(io::Error::other)?;
    fs::create_dir_all(dir)?;
    let path = dir.join(DATA_FILE);
    if path.is_file() {
        // A failed copy must not block saving; the new file is still written safely.
        let _ = fs::copy(&path, dir.join("data.json.bak"));
    }
    write_atomic(&path, &json)?;
    Ok(stamp_of(&path))
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

    // ---- two processes on one folder ----

    fn entry_in(id: u64, name: &str) -> MonthData {
        MonthData { entries: vec![Entry { id, category_id: 1, name: name.into(), amount_cents: 100 }] }
    }

    #[test]
    fn a_change_made_by_another_process_is_not_lost() {
        let a = temp_storage("two-a");
        a.save_month(month(2026, 1), &entry_in(1, "from a")).unwrap();
        // A second storage on the same folder stands for the other process.
        let b = Storage::new(a.dir());
        b.save_month(month(2026, 2), &entry_in(1, "from b")).unwrap();

        // A has not looked again yet, but saving something else keeps B's month.
        let cats = Categories::default();
        a.save_categories(&cats).unwrap();
        assert_eq!(a.list_months(), vec![month(2026, 1), month(2026, 2)]);
        assert_eq!(Storage::new(a.dir()).list_months().len(), 2);
        let _ = fs::remove_dir_all(a.dir());
    }

    #[test]
    fn reload_if_changed_sees_the_other_process_but_not_itself() {
        let a = temp_storage("reload-a");
        a.save_month(month(2026, 1), &entry_in(1, "first")).unwrap();
        assert!(!a.reload_if_changed(), "its own write is not a change");
        let b = Storage::new(a.dir());
        assert!(!b.reload_if_changed());

        b.save_month(month(2026, 3), &entry_in(1, "second")).unwrap();
        assert!(a.reload_if_changed());
        assert_eq!(a.load_month(month(2026, 3)).entries[0].name, "second");
        assert!(!a.reload_if_changed(), "reported once");
        let _ = fs::remove_dir_all(a.dir());
    }

    #[test]
    fn the_lock_is_released_after_a_write_and_a_stale_one_is_ignored() {
        let a = temp_storage("lock");
        a.save_month(month(2026, 1), &entry_in(1, "x")).unwrap();
        assert!(!a.dir().join("data.lock").exists(), "the lock is removed after writing");

        // A lock left behind by a crashed process (old enough) does not block saving.
        let lock = a.dir().join("data.lock");
        let file = fs::File::create(&lock).unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(60)).unwrap();
        drop(file);
        a.save_month(month(2026, 2), &entry_in(1, "y")).unwrap();
        assert!(!lock.exists());
        assert_eq!(a.list_months().len(), 2);
        let _ = fs::remove_dir_all(a.dir());
    }

    #[test]
    fn writers_in_several_threads_do_not_lose_each_other() {
        let first = temp_storage("threads");
        first.save_month(month(2026, 1), &entry_in(1, "start")).unwrap();
        let dir = first.dir().to_path_buf();
        let workers: Vec<_> = (2..=9u8)
            .map(|m| {
                let dir = dir.clone();
                std::thread::spawn(move || {
                    // Each thread has its own storage, like a separate process would.
                    let st = Storage::new(&dir);
                    st.save_month(month(2026, m), &entry_in(1, "worker")).unwrap();
                })
            })
            .collect();
        for w in workers {
            w.join().unwrap();
        }
        assert_eq!(Storage::new(&dir).list_months().len(), 9, "every month was kept");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn watching_reports_changes_from_elsewhere_only() {
        let a = temp_storage("watch");
        a.save_month(month(2026, 1), &entry_in(1, "x")).unwrap();
        a.save_settings(&Settings::default()).unwrap();
        let events = a.watch(Duration::from_millis(20));

        // Its own writes are not reported.
        a.save_month(month(2026, 2), &entry_in(1, "own")).unwrap();
        a.save_settings(&Settings { check_updates: true, ..Settings::default() }).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert!(events.try_recv().is_err(), "nothing happened elsewhere");

        let b = Storage::new(a.dir());
        b.save_month(month(2026, 5), &entry_in(1, "other")).unwrap();
        let mut seen = Vec::new();
        for _ in 0..100 {
            if let Ok(change) = events.try_recv() {
                seen.push(change);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(seen, vec![Change::Data]);

        b.save_settings(&Settings { language: Some(crate::i18n::Lang::Fi), ..Settings::default() }).unwrap();
        let mut got_settings = false;
        for _ in 0..100 {
            if events.try_recv() == Ok(Change::Settings) {
                got_settings = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(got_settings);

        // Once the receiver is gone the thread stops by itself (it cannot be observed, so just drop it).
        drop(events);
        let _ = fs::remove_dir_all(a.dir());
    }
}
