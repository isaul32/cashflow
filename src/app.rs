//! Main window: entry form and lists on the left, Sankey chart on the right.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::chart::SankeyChart;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::plot::shape::{SankeyAlign, SankeyLink};
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, Theme, ThemeMode, TitleBar, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use std::path::PathBuf;
use std::sync::Arc;

use crate::backup;
use crate::data::DataFile;
use crate::i18n::{Currency, Lang, Locale, Msg, Plural, SystemLocale};
use crate::import::{self, ImportPlan};
use crate::recurring::{self, RecurringStore};
use crate::model::{Categories, CategoryId, Entry, Kind, MonthKey, parse_amount_cents};
use crate::sankey::{self, FlowNode, SankeyData};
use crate::settings::{Labels, Settings, ThemeChoice};
use crate::storage::{MonthData, Storage};
use crate::year::{self, YearSummary};

/// Category colours: vivid ones (used for new categories by default), plus a
/// darker and a muted (softer) set of the same hues. The picker shows all three.
pub(crate) const PALETTE: [u32; 10] = [
    0x3F51B5, 0xFF9800, 0x03A9F4, 0x9C27B0, 0xE91E63, 0x4CAF50, 0x795548, 0x00BCD4, 0xCDDC39,
    0xFF5722,
];
const PALETTE_DARK: [u32; 10] = [
    0x1A237E, 0xE65100, 0x01579B, 0x4A148C, 0x880E4F, 0x1B5E20, 0x3E2723, 0x006064, 0x827717,
    0xBF360C,
];
const PALETTE_MUTED: [u32; 10] = [
    0x8E99C4, 0xD9A566, 0x7FB0CF, 0xA986B5, 0xC98BA0, 0x86AD88, 0x9C8578, 0x7FB5BB, 0xBBBD86,
    0xCF9482,
];

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Overview,
    Year,
    Categories,
    Recurring,
    Settings,
    About,
}

pub struct CashFlowApp {
    storage: Option<Storage>,
    settings: Settings,
    /// Templates for entries that repeat every month.
    recurring: RecurringStore,
    /// "Repeat every month" is ticked in the entry form.
    repeat: bool,
    /// Language of the operating system, used when the language is "automatic".
    system: SystemLocale,
    /// The resolved language and currency.
    lc: Locale,
    /// App logo shown in the sidebar (same artwork as the window icon).
    logo: Arc<Image>,
    /// Same logo on a dark tile, shown when the theme is dark.
    logo_dark: Arc<Image>,
    total_in: Entity<InputState>,
    savings_in: Entity<InputState>,
    deficit_in: Entity<InputState>,
    month: MonthKey,
    cats: Categories,
    entries: Vec<Entry>,
    next_entry_id: u64,
    data: SankeyData,
    page: Page,

    // Entry form.
    kind: Kind,
    category: Option<CategoryId>,
    editing: Option<u64>,
    name_in: Entity<InputState>,
    amount_in: Entity<InputState>,

    // Category form.
    editing_cat: Option<CategoryId>,
    cat_kind: Kind,
    cat_parent: Option<CategoryId>,
    cat_color: u32,
    cat_name_in: Entity<InputState>,

    error: Option<SharedString>,
    /// True right after the chart text was copied; cleared when data changes.
    copied: bool,
    /// The year shown on the Year page and its aggregated data.
    year: i32,
    year_summary: YearSummary,
    /// A parsed SankeyMATIC import waiting for the user to confirm.
    pending_import: Option<ImportPlan>,
    /// A neutral success message (saved a file, imported entries).
    notice: Option<SharedString>,
    /// A newer release found by the opt-in update check.
    update: Option<crate::update::Release>,
    /// A parsed backup file waiting for the user to confirm the restore.
    pending_restore: Option<DataFile>,
    _subs: Vec<Subscription>,
}

fn input(
    window: &mut Window,
    cx: &mut Context<CashFlowApp>,
    placeholder: impl Into<SharedString>,
) -> Entity<InputState> {
    let placeholder: SharedString = placeholder.into();
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}

fn input_with(
    window: &mut Window,
    cx: &mut Context<CashFlowApp>,
    placeholder: impl Into<SharedString>,
    value: &str,
) -> Entity<InputState> {
    let placeholder: SharedString = placeholder.into();
    let value = value.to_string();
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder).default_value(value))
}

fn theme_mode(choice: ThemeChoice) -> ThemeMode {
    match choice {
        ThemeChoice::Light => ThemeMode::Light,
        ThemeChoice::Dark => ThemeMode::Dark,
    }
}

fn id_of(prefix: &str, id: u64) -> SharedString {
    format!("{prefix}-{id}").into()
}

impl CashFlowApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_storage(Storage::default_location(), window, cx)
    }

    /// Like [`Self::new`] with an explicit data folder (`None`: no folder available).
    pub fn with_storage(storage: Option<Storage>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Developer overrides, used by site/tools/screenshots.ps1 to capture screenshots.
        let month = std::env::var("CASHFLOW_MONTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(MonthKey::current);
        let system = SystemLocale::detect();
        let mut warnings: Vec<String> = Vec::new();
        let mut settings = Settings::default();
        let mut first_run = true;
        let mut recurring = RecurringStore::default();
        let (mut cats, entries) = match &storage {
            Some(st) => {
                warnings.extend(st.take_warnings());
                first_run = !st.has_categories();
                let c = st.load_categories();
                recurring = st.load_recurring();
                // The shown month gets its recurring entries if it is still empty.
                if let Err(e) = recurring::apply_month(st, &mut recurring, month) {
                    warnings.push(format!("Could not add recurring entries: {e}"));
                }
                let s = st.load_settings();
                warnings.extend(s.warning);
                settings = s.value;
                (c, st.load_month(month).entries)
            }
            None => (Categories::default(), Vec::new()),
        };
        settings.labels = settings.labels.sanitized();
        let lc = Locale::resolve(settings.language, settings.currency, &system);
        if storage.is_none() {
            warnings.push(lc.t(Msg::ErrNoDataDir).to_string());
        }
        // The starter categories are named in the language of the first run.
        if first_run {
            cats = Categories::defaults_for(lc.lang);
        }

        let name_in = input(window, cx, lc.t(Msg::PlaceholderNote));
        let amount_in = input(window, cx, lc.plain_amount(0));
        let cat_name_in = input(window, cx, lc.t(Msg::CatNamePlaceholder));
        let labels = settings.labels.resolved(lc.lang);
        let total_in = input_with(window, cx, lc.t(Msg::DefLabelTotal), &labels.total);
        let savings_in = input_with(window, cx, lc.t(Msg::DefLabelSavings), &labels.savings);
        let deficit_in = input_with(window, cx, lc.t(Msg::DefLabelDeficit), &labels.deficit);

        let subs = vec![
            cx.subscribe_in(&amount_in, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.submit_entry(window, cx);
                }
            }),
            cx.subscribe_in(&cat_name_in, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.submit_category(window, cx);
                }
            }),
        ]
        .into_iter()
        // Keep the brand colors on top of whatever theme the library switches to.
        .chain([cx.observe_global::<Theme>(|_, cx| crate::brand::apply(cx))])
        // The chart label fields save by themselves when left or confirmed with Enter.
        .chain([&total_in, &savings_in, &deficit_in].map(|field| {
            cx.subscribe_in(field, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.autosave_labels(window, cx);
                }
            })
        }))
        .collect::<Vec<_>>();

        let next_entry_id = MonthData { entries: entries.clone() }.next_entry_id();
        let category = cats.flatten_sorted(Kind::Expense).first().map(|(_, c)| c.id);
        let mut app = CashFlowApp {
            data: SankeyData::default(),
            storage,
            settings,
            recurring,
            repeat: false,
            system,
            lc,
            logo: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../assets/icon.png").to_vec(),
            )),
            logo_dark: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../assets/icon-dark.png").to_vec(),
            )),
            total_in,
            savings_in,
            deficit_in,
            month,
            cats,
            entries,
            next_entry_id,
            // Developer override, like CASHFLOW_MONTH above.
            page: match std::env::var("CASHFLOW_PAGE").as_deref() {
                Ok("year") => Page::Year,
                Ok("categories") => Page::Categories,
                Ok("recurring") => Page::Recurring,
                Ok("settings") => Page::Settings,
                Ok("about") => Page::About,
                _ => Page::Overview,
            },
            kind: Kind::Expense,
            category,
            editing: None,
            name_in,
            amount_in,
            editing_cat: None,
            cat_kind: Kind::Expense,
            cat_parent: None,
            cat_color: PALETTE[0],
            cat_name_in,
            error: (!warnings.is_empty()).then(|| warnings.join("\n").into()),
            copied: false,
            year: month.year,
            year_summary: YearSummary::default(),
            pending_import: None,
            notice: None,
            update: None,
            pending_restore: None,
            _subs: subs,
        };
        app.refresh();
        crate::brand::apply(cx);
        if let Some(choice) = app.settings.theme {
            Theme::change(theme_mode(choice), Some(window), cx);
        }
        if let Ok(theme) = std::env::var("CASHFLOW_THEME") {
            let mode = if theme == "dark" { ThemeMode::Dark } else { ThemeMode::Light };
            Theme::change(mode, Some(window), cx);
        }
        crate::brand::apply(cx);
        crate::menu::set(cx, &app.lc);
        // The system language may have changed since last time (language "automatic").
        if app.cats.translate_defaults(app.lc.lang) {
            app.commit();
        }
        app.watch_storage(window, cx);
        if app.settings.check_updates {
            app.check_for_updates(cx);
        }
        app
    }

    /// Looks for a newer release in the background; failures (e.g. offline) are silent.
    fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let found = cx
                .background_spawn(async { crate::update::check(env!("CARGO_PKG_VERSION")) })
                .await;
            if let Ok(Some(release)) = found {
                this.update(cx, |this, cx| {
                    this.update = Some(release);
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn set_check_updates(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.check_updates = on;
        self.persist_settings();
        if on {
            self.check_for_updates(cx);
        } else {
            self.update = None;
        }
        cx.notify();
    }

    /// The logo variant that suits the current theme.
    fn logo_for(&self, cx: &App) -> Arc<Image> {
        if cx.theme().is_dark() { self.logo_dark.clone() } else { self.logo.clone() }
    }

    // ---- language ----

    fn t(&self, msg: Msg) -> &'static str {
        self.lc.t(msg)
    }

    fn tf(&self, msg: Msg, args: &[(&str, &str)]) -> String {
        self.lc.tf(msg, args)
    }

    /// The chart labels in the current language.
    fn labels(&self) -> Labels {
        self.settings.labels.resolved(self.lc.lang)
    }

    /// Recomputes the locale from the settings and refreshes everything that
    /// depends on it: placeholders, label fields and the charts.
    fn apply_locale(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.lc = Locale::resolve(self.settings.language, self.settings.currency, &self.system);
        let lc = self.lc;
        // Starter categories that were never edited follow the language.
        if self.cats.translate_defaults(lc.lang) {
            self.commit();
        }
        crate::menu::set(cx, &lc);
        let amount_placeholder = lc.plain_amount(0);
        self.name_in.update(cx, |s, cx| s.set_placeholder(lc.t(Msg::PlaceholderNote), window, cx));
        self.amount_in.update(cx, |s, cx| s.set_placeholder(amount_placeholder, window, cx));
        self.cat_name_in.update(cx, |s, cx| s.set_placeholder(lc.t(Msg::CatNamePlaceholder), window, cx));
        let labels = self.labels();
        for (input, placeholder, text) in [
            (&self.total_in, Msg::DefLabelTotal, &labels.total),
            (&self.savings_in, Msg::DefLabelSavings, &labels.savings),
            (&self.deficit_in, Msg::DefLabelDeficit, &labels.deficit),
        ] {
            input.update(cx, |s, cx| {
                s.set_placeholder(lc.t(placeholder), window, cx);
                s.set_value(text.as_str(), window, cx);
            });
        }
        self.refresh();
        cx.notify();
    }

    fn set_language(&mut self, language: Option<Lang>, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.language = language;
        self.persist_settings();
        self.apply_locale(window, cx);
    }

    /// Applies and stores the theme; `None` follows the system appearance.
    fn set_theme(&mut self, theme: Option<ThemeChoice>, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.theme = theme;
        self.persist_settings();
        match theme {
            Some(choice) => Theme::change(theme_mode(choice), Some(window), cx),
            None => Theme::sync_system_appearance(Some(window), cx),
        }
        cx.notify();
    }

    fn set_currency(&mut self, currency: Option<Currency>, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.currency = currency;
        self.persist_settings();
        self.apply_locale(window, cx);
    }

    /// Writes `settings.json`; a failure is shown as an error.
    fn persist_settings(&mut self) -> bool {
        let Some(st) = &self.storage else { return false };
        match st.save_settings(&self.settings) {
            Ok(()) => true,
            Err(e) => {
                self.error = Some(self.tf(Msg::ErrSaveSettingsFailed, &[("error", &e.to_string())]).into());
                false
            }
        }
    }

    fn refresh(&mut self) {
        self.copied = false;
        self.data = sankey::build_with(&self.cats, &self.entries, &self.labels());
        self.refresh_year();
    }

    /// Re-reads the months of the shown year. The current month comes from
    /// memory, the others from disk.
    fn refresh_year(&mut self) {
        let months: Vec<Vec<Entry>> = (1..=12u8)
            .map(|m| {
                let key = MonthKey { year: self.year, month: m };
                if key == self.month {
                    self.entries.clone()
                } else {
                    self.storage
                        .as_ref()
                        .map(|st| st.load_month(key).entries)
                        .unwrap_or_default()
                }
            })
            .collect();
        self.year_summary = year::summarize(self.year, &self.cats, &months, &self.labels());
    }

    fn switch_year(&mut self, year: i32, cx: &mut Context<Self>) {
        self.year = year;
        self.refresh_year();
        self.copied = false;
        cx.notify();
    }

    /// Jumps from the Year page to the Overview of one month.
    fn open_month(&mut self, month: MonthKey, window: &mut Window, cx: &mut Context<Self>) {
        self.page = Page::Overview;
        self.switch_month(month, window, cx);
        cx.notify();
    }

    /// SankeyMATIC text and suggested file name for whatever chart is on screen.
    fn export_payload(&self) -> (String, String) {
        match self.page {
            Page::Year => (
                self.year_summary.data.to_sankeymatic(&self.year.to_string()),
                format!("cashflow-{}-sankeymatic.txt", self.year),
            ),
            _ => (
                self.data.to_sankeymatic(&self.month.to_string()),
                format!("cashflow-{}-sankeymatic.txt", self.month),
            ),
        }
    }

    /// Rebuilds the chart and writes the current state to disk.
    fn commit(&mut self) {
        self.refresh();
        let Some(st) = &self.storage else { return };
        let res = st.save_categories_and_month(
            &self.cats,
            self.month,
            &MonthData { entries: self.entries.clone() },
        );
        if let Err(e) = res {
            self.error = Some(self.tf(Msg::ErrSaveFailed, &[("error", &e.to_string())]).into());
        }
    }

    /// Re-points entries of every other saved month away from a deleted category.
    fn reassign_in_other_months(&mut self, from: CategoryId, to: CategoryId) {
        let Some(st) = &self.storage else { return };
        for key in st.list_months().into_iter().filter(|k| *k != self.month) {
            let mut data = st.load_month(key);
            let mut changed = false;
            for e in data.entries.iter_mut().filter(|e| e.category_id == from) {
                e.category_id = to;
                changed = true;
            }
            if changed && let Err(e) = st.save_month(key, &data) {
                self.error = Some(
                    self.tf(Msg::ErrSaveMonthFailed, &[("month", &key.to_string()), ("error", &e.to_string())])
                        .into(),
                );
            }
        }
    }

    fn copy_sankeymatic(&mut self, cx: &mut Context<Self>) {
        let (text, _) = self.export_payload();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.copied = true;
        cx.notify();
    }

    // ---- chart labels ----

    /// Saves the label fields (after cleaning them up) and redraws the chart.
    /// Saves the chart labels if they differ from the stored ones. Otherwise only tidies the
    /// fields (e.g. trailing spaces) without touching the settings file.
    fn autosave_labels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = Labels {
            total: self.total_in.read(cx).value().to_string(),
            savings: self.savings_in.read(cx).value().to_string(),
            deficit: self.deficit_in.read(cx).value().to_string(),
        }
        .sanitized();
        if typed != self.settings.labels {
            self.save_labels(window, cx);
        }
    }

    fn save_labels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let labels = Labels {
            total: self.total_in.read(cx).value().to_string(),
            savings: self.savings_in.read(cx).value().to_string(),
            deficit: self.deficit_in.read(cx).value().to_string(),
        }
        .sanitized();
        // Show the cleaned-up values (e.g. an emptied field returns to its default).
        for (input, text) in [
            (&self.total_in, &labels.total),
            (&self.savings_in, &labels.savings),
            (&self.deficit_in, &labels.deficit),
        ] {
            input.update(cx, |s, cx| s.set_value(text.as_str(), window, cx));
        }
        self.settings.labels = labels;
        self.refresh();
        self.error = None;
        self.notice = None;
        if self.persist_settings() {
            self.notice = Some(self.t(Msg::NoticeLabelsSaved).into());
        }
        cx.notify();
    }

    fn reset_labels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let d = Labels::default().resolved(self.lc.lang);
        for (input, text) in [
            (&self.total_in, &d.total),
            (&self.savings_in, &d.savings),
            (&self.deficit_in, &d.deficit),
        ] {
            input.update(cx, |s, cx| s.set_value(text.as_str(), window, cx));
        }
        self.save_labels(window, cx);
    }

    // ---- SankeyMATIC export / import ----

    fn save_sankeymatic_file(&mut self, cx: &mut Context<Self>) {
        let (text, name) = self.export_payload();
        self.save_text_file(text, name, cx);
    }

    /// Asks where to save and writes `text` there.
    fn save_text_file(&mut self, text: String, name: String, cx: &mut Context<Self>) {
        let dir: PathBuf = directories::UserDirs::new()
            .and_then(|d| d.document_dir().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let res = std::fs::write(&path, text);
            this.update(cx, |this, cx| {
                match res {
                    Ok(()) => {
                        let m = this.tf(Msg::NoticeSaved, &[("path", &path.display().to_string())]);
                        this.notice = Some(m.into());
                    }
                    Err(e) => {
                        let m = this.tf(Msg::ErrSaveFileFailed, &[("error", &e.to_string())]);
                        this.error = Some(m.into());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ---- recurring entries ----

    /// Writes `recurring.json`; a failure is shown as an error.
    fn persist_recurring(&mut self) {
        let Some(st) = &self.storage else { return };
        if let Err(e) = st.save_recurring(&self.recurring) {
            self.error = Some(self.tf(Msg::ErrSaveFailed, &[("error", &e.to_string())]).into());
        }
    }

    fn toggle_repeat(&mut self, cx: &mut Context<Self>) {
        self.repeat = !self.repeat;
        cx.notify();
    }

    /// Ends a recurring entry after the current month; months that have it keep it.
    fn stop_recurring(&mut self, id: u64, cx: &mut Context<Self>) {
        self.recurring.stop(id, MonthKey::current());
        self.persist_recurring();
        cx.notify();
    }

    /// Deletes the template only; entries it already added stay in their months.
    fn delete_recurring(&mut self, id: u64, cx: &mut Context<Self>) {
        self.recurring.remove(id);
        self.persist_recurring();
        cx.notify();
    }

    // ---- backup and restore ----

    fn save_backup(&mut self, cx: &mut Context<Self>) {
        let Some(st) = &self.storage else { return };
        match st.snapshot().to_json() {
            Ok(json) => self.save_text_file(json, backup::file_name("backup", "json"), cx),
            Err(e) => self.error = Some(self.tf(Msg::ErrSaveFileFailed, &[("error", &e)]).into()),
        }
        cx.notify();
    }

    fn export_csv(&mut self, cx: &mut Context<Self>) {
        let Some(st) = &self.storage else { return };
        let csv = st.snapshot().to_csv();
        self.save_text_file(csv, backup::file_name("entries", "csv"), cx);
    }

    /// Lets the user pick a backup file and shows the confirmation; nothing changes yet.
    fn open_backup_file(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(self.t(Msg::PromptOpenBackup).into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let parsed = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| backup::parse(&text));
            this.update(cx, |this, cx| {
                match parsed {
                    Ok(backup) => {
                        this.pending_restore = Some(backup);
                        this.error = None;
                        this.notice = None;
                    }
                    Err(e) => {
                        this.pending_restore = None;
                        this.error = Some(this.tf(Msg::ErrBackupInvalid, &[("error", &e)]).into());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn cancel_restore(&mut self, cx: &mut Context<Self>) {
        self.pending_restore = None;
        cx.notify();
    }

    /// Replaces all data with the chosen backup after saving a copy of the current data.
    fn confirm_restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(backup) = self.pending_restore.take() else { return };
        let Some(st) = self.storage.clone() else { return };
        let result = backup::save_safety_copy(&st).and_then(|copy| st.replace_all(&backup).map(|()| copy));
        match result {
            Ok(copy) => {
                self.reload_from_disk(window, cx);
                let m = self.tf(Msg::NoticeRestored, &[("path", &copy.display().to_string())]);
                self.error = None;
                self.notice = Some(m.into());
            }
            Err(e) => {
                let m = self.tf(Msg::ErrRestoreFailed, &[("error", &e.to_string())]);
                self.error = Some(m.into());
            }
        }
        cx.notify();
    }

    /// Reloads categories, recurring entries and the shown month after the data was replaced.
    fn reload_from_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.storage.clone() else { return };
        self.cats = st.load_categories();
        self.recurring = st.load_recurring();
        if let Err(e) = recurring::apply_month(&st, &mut self.recurring, self.month) {
            self.error = Some(self.tf(Msg::ErrSaveFailed, &[("error", &e.to_string())]).into());
        }
        self.entries = st.load_month(self.month).entries;
        self.next_entry_id = MonthData { entries: self.entries.clone() }.next_entry_id();
        self.category = self.cats.flatten_sorted(self.kind).first().map(|(_, c)| c.id);
        self.editing = None;
        self.editing_cat = None;
        self.pending_import = None;
        self.clear_inputs(window, cx);
        self.refresh();
    }

    fn paste_import(&mut self, cx: &mut Context<Self>) {
        match cx.read_from_clipboard().and_then(|item| item.text()) {
            Some(text) => self.start_import(&text, cx),
            None => {
                self.set_error(self.t(Msg::ErrClipboardEmpty));
                cx.notify();
            }
        }
    }

    fn open_import_file(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(self.t(Msg::PromptOpenFile).into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let read = std::fs::read_to_string(&path);
            this.update(cx, |this, cx| {
                match read {
                    Ok(text) => this.start_import(&text, cx),
                    Err(e) => {
                        let m = this.tf(Msg::ErrReadFileFailed, &[("error", &e.to_string())]);
                        this.error = Some(m.into());
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Parses text and shows the confirmation panel; nothing changes yet.
    fn start_import(&mut self, text: &str, cx: &mut Context<Self>) {
        match import::parse_with(text, &self.labels(), self.lc.lang) {
            Ok(plan) => {
                self.pending_import = Some(plan);
                self.error = None;
                self.notice = None;
            }
            Err(e) => {
                self.pending_import = None;
                self.error = Some(self.tf(Msg::ErrImportFailed, &[("error", &e)]).into());
            }
        }
        cx.notify();
    }

    fn confirm_import(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(plan) = self.pending_import.take() else { return };
        let stats = import::apply(
            &plan,
            &mut self.cats,
            &mut self.entries,
            &mut self.next_entry_id,
            replace,
            &PALETTE,
        );
        self.clear_inputs(window, cx);
        self.category = self.cats.flatten_sorted(self.kind).first().map(|(_, c)| c.id);
        self.error = None;
        self.commit();
        let period = self.lc.month_inline(self.month);
        let entries = self.lc.count(Plural::CountEntries, stats.entries_added);
        let new_cats = self.lc.count(Plural::CountNewCategories, stats.categories_added);
        self.notice = Some(
            self.tf(
                Msg::NoticeImported,
                &[("entries", &entries), ("period", &period), ("newcats", &new_cats)],
            )
            .into(),
        );
        cx.notify();
    }

    fn cancel_import(&mut self, cx: &mut Context<Self>) {
        self.pending_import = None;
        cx.notify();
    }

    fn set_error(&mut self, msg: impl Into<SharedString>) {
        self.error = Some(msg.into());
    }

    fn clear_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.name_in.update(cx, |s, cx| s.set_value("", window, cx));
        self.amount_in.update(cx, |s, cx| s.set_value("", window, cx));
        self.editing = None;
        self.repeat = false;
    }

    // ---- entries ----

    fn submit_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(category_id) = self.category else {
            self.set_error(self.t(Msg::ErrPickCategory));
            cx.notify();
            return;
        };
        let Some(amount_cents) = parse_amount_cents(self.amount_in.read(cx).value().as_ref(), self.lc.decimal_mark())
        else {
            let example = self.lc.plain_amount(1250);
            self.set_error(self.tf(Msg::ErrPositiveAmount, &[("example", &example)]));
            cx.notify();
            return;
        };
        let name = self.name_in.read(cx).value().trim().to_string();

        match self.editing {
            Some(id) => {
                if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                    e.category_id = category_id;
                    e.name = name.clone();
                    e.amount_cents = amount_cents;
                }
            }
            None => {
                self.entries.push(Entry {
                    id: self.next_entry_id,
                    category_id,
                    name: name.clone(),
                    amount_cents,
                });
                self.next_entry_id += 1;
            }
        }
        // Works for a new entry and for an edited one. An entry that already repeats with the
        // same values (e.g. one a recurring entry added) does not get a second template.
        let repeated = self.repeat
            && !self.recurring.repeats(category_id, &name, amount_cents, self.month)
            && {
                self.recurring.add(category_id, name, amount_cents, self.month);
                true
            };
        self.error = None;
        self.clear_inputs(window, cx);
        self.commit();
        if repeated {
            // The shown month already has its entry; later months get theirs when they are opened.
            self.persist_recurring();
        }
        cx.notify();
    }

    fn edit_entry(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.entries.iter().find(|e| e.id == id).cloned() else {
            return;
        };
        if let Some(cat) = self.cats.get(e.category_id) {
            self.kind = cat.kind;
        }
        self.category = Some(e.category_id);
        self.editing = Some(id);
        self.repeat = false;
        self.name_in.update(cx, |s, cx| s.set_value(e.name.as_str(), window, cx));
        let amount = self.lc.plain_amount(e.amount_cents);
        self.amount_in.update(cx, |s, cx| s.set_value(amount.as_str(), window, cx));
        self.error = None;
        cx.notify();
    }

    fn delete_entry(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.entries.retain(|e| e.id != id);
        if self.editing == Some(id) {
            self.clear_inputs(window, cx);
        }
        self.commit();
        cx.notify();
    }

    fn set_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        self.kind = kind;
        self.category = self.cats.flatten_sorted(kind).first().map(|(_, c)| c.id);
        cx.notify();
    }

    // ---- categories ----

    fn submit_category(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.cat_name_in.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.set_error(self.t(Msg::ErrCatNameEmpty));
            cx.notify();
            return;
        }
        let editing = self.editing_cat;
        if self.cats.items.iter().any(|c| {
            c.kind == self.cat_kind && Some(c.id) != editing && c.name.eq_ignore_ascii_case(&name)
        }) {
            self.set_error(self.t(Msg::ErrCatExists));
            cx.notify();
            return;
        }
        match editing {
            Some(id) => {
                if !self.cats.update(id, name, self.cat_parent, self.cat_color) {
                    self.set_error(self.t(Msg::ErrParent));
                    cx.notify();
                    return;
                }
            }
            None => {
                let id = self.cats.add(name, self.cat_kind, self.cat_parent, self.cat_color);
                if self.kind == self.cat_kind && self.category.is_none() {
                    self.category = Some(id);
                }
            }
        }
        self.error = None;
        self.clear_category_form(window, cx);
        self.commit();
        cx.notify();
    }

    fn clear_category_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cat_name_in.update(cx, |s, cx| s.set_value("", window, cx));
        self.editing_cat = None;
        self.cat_parent = None;
        self.cat_color = PALETTE[self.cats.items.len() % PALETTE.len()];
    }

    fn edit_category(&mut self, id: CategoryId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(c) = self.cats.get(id).cloned() else { return };
        self.editing_cat = Some(id);
        self.cat_kind = c.kind;
        self.cat_parent = c.parent;
        self.cat_color = c.color;
        self.cat_name_in.update(cx, |s, cx| s.set_value(c.name.as_str(), window, cx));
        self.error = None;
        cx.notify();
    }

    fn switch_month(&mut self, month: MonthKey, window: &mut Window, cx: &mut Context<Self>) {
        if month == self.month {
            return;
        }
        self.month = month;
        self.pending_import = None;
        if let Some(st) = &self.storage {
            let filled = recurring::apply_month(st, &mut self.recurring, month);
            if let Err(e) = filled {
                self.error = Some(self.tf(Msg::ErrSaveFailed, &[("error", &e.to_string())]).into());
            }
            self.entries = st.load_month(month).entries;
        } else {
            self.entries = Vec::new();
        }
        self.next_entry_id = MonthData { entries: self.entries.clone() }.next_entry_id();
        self.clear_inputs(window, cx);
        self.refresh();
        cx.notify();
    }

    fn category_used(&self, id: CategoryId) -> bool {
        self.recurring.uses_category(id)
            || self.entries.iter().any(|e| e.category_id == id)
            || self.storage.as_ref().is_some_and(|st| {
                st.list_months()
                    .into_iter()
                    .filter(|k| *k != self.month)
                    .any(|k| st.load_month(k).entries.iter().any(|e| e.category_id == id))
            })
    }

    fn delete_category(&mut self, id: CategoryId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cat) = self.cats.get(id) else { return };
        let kind = cat.kind;
        // Entries move to the parent, or to another top-level category of the same kind.
        let target = cat.parent.or_else(|| {
            self.cats
                .flatten(kind)
                .iter()
                .map(|(_, c)| c.id)
                .find(|&other| other != id)
        });
        let target = match target {
            Some(t) => t,
            None if self.category_used(id) => {
                self.set_error(self.t(Msg::ErrCannotDelete));
                cx.notify();
                return;
            }
            None => id, // unused: nothing to move
        };
        if target != id {
            self.reassign_in_other_months(id, target);
            if self.recurring.reassign_category(id, target) {
                self.persist_recurring();
            }
        }
        self.cats.remove(id, target, &mut self.entries);
        if self.cat_parent == Some(id) {
            self.cat_parent = None;
        }
        if self.editing_cat == Some(id) {
            self.clear_category_form(window, cx);
        }
        if self.category == Some(id) {
            self.category = Some(target).filter(|t| self.cats.get(*t).is_some());
        }
        self.error = None;
        self.commit();
        cx.notify();
    }

    fn set_cat_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        if self.editing_cat.is_some() {
            return; // the kind of an existing category cannot change
        }
        self.cat_kind = kind;
        self.cat_parent = None;
        cx.notify();
    }
}

mod mcp;
mod sync;
mod view;

#[cfg(test)]
mod tests;
