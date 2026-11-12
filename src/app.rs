//! Main window: entry form and lists on the left, Sankey chart on the right.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::chart::SankeyChart;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::plot::shape::SankeyLink;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use std::path::PathBuf;
use std::sync::Arc;

use crate::import::{self, ImportPlan};
use crate::model::{Categories, CategoryId, Entry, Kind, MonthKey, parse_amount_cents};
use crate::sankey::{self, FlowNode, SankeyData};
use crate::settings::{Labels, Settings};
use crate::storage::{MonthData, Storage};

const PALETTE: [u32; 10] = [
    0x3F51B5, 0xFF9800, 0x03A9F4, 0x9C27B0, 0xE91E63, 0x4CAF50, 0x795548, 0x00BCD4, 0xCDDC39,
    0xFF5722,
];

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Overview,
    Categories,
    Settings,
}

pub struct BudgetApp {
    storage: Option<Storage>,
    settings: Settings,
    /// App logo shown in the sidebar (same artwork as the window icon).
    logo: Arc<Image>,
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
    /// A parsed SankeyMATIC import waiting for the user to confirm.
    pending_import: Option<ImportPlan>,
    /// A neutral success message (saved a file, imported entries).
    notice: Option<SharedString>,
    _subs: Vec<Subscription>,
}

fn input(window: &mut Window, cx: &mut Context<BudgetApp>, placeholder: &'static str) -> Entity<InputState> {
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
}

fn input_with(
    window: &mut Window,
    cx: &mut Context<BudgetApp>,
    placeholder: &'static str,
    value: &str,
) -> Entity<InputState> {
    let value = value.to_string();
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder).default_value(value))
}

fn id_of(prefix: &str, id: u64) -> SharedString {
    format!("{prefix}-{id}").into()
}

/// Groups digits with commas: 1234567 -> "1,234,567".
fn group(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// "€1,234.50"; negative amounts get a leading minus.
fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let c = cents.abs();
    format!("{sign}€{}.{:02}", group(c / 100), c % 100)
}

impl BudgetApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_in = input(window, cx, "Optional note");
        let amount_in = input(window, cx, "0.00");
        let cat_name_in = input(window, cx, "New category name");

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
        ];

        let storage = Storage::default_location();
        let month = MonthKey::current();
        let mut warnings = Vec::new();
        let mut settings = Settings::default();
        let (cats, entries) = match &storage {
            Some(st) => {
                let c = st.load_categories();
                let m = st.load_month(month);
                let s = st.load_settings();
                warnings.extend(s.warning);
                settings = s.value;
                warnings.extend(c.warning);
                warnings.extend(m.warning);
                (c.value, m.value.entries)
            }
            None => {
                warnings.push("No data directory available; changes will not be saved.".to_string());
                (Categories::default(), Vec::new())
            }
        };
        settings.labels = settings.labels.sanitized();
        let total_in = input_with(window, cx, "Total income", &settings.labels.total);
        let savings_in = input_with(window, cx, "Savings", &settings.labels.savings);
        let deficit_in = input_with(window, cx, "Deficit", &settings.labels.deficit);
        let next_entry_id = MonthData { entries: entries.clone() }.next_entry_id();
        let category = cats.flatten(Kind::Expense).first().map(|(_, c)| c.id);
        let mut app = BudgetApp {
            data: SankeyData::default(),
            storage,
            settings,
            logo: Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../assets/icon.png").to_vec(),
            )),
            total_in,
            savings_in,
            deficit_in,
            month,
            cats,
            entries,
            next_entry_id,
            page: Page::Overview,
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
            error: (!warnings.is_empty()).then(|| warnings.join("
").into()),
            copied: false,
            pending_import: None,
            notice: None,
            _subs: subs,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.copied = false;
        self.data = sankey::build_with(&self.cats, &self.entries, &self.settings.labels);
    }

    /// Rebuilds the chart and writes the current state to disk.
    fn commit(&mut self) {
        self.refresh();
        let Some(st) = &self.storage else { return };
        let res = st.save_categories(&self.cats).and_then(|()| {
            st.save_month(self.month, &MonthData { entries: self.entries.clone() })
        });
        if let Err(e) = res {
            self.error = Some(format!("Saving failed: {e}").into());
        }
    }

    /// Re-points entries of every other saved month away from a deleted category.
    fn reassign_in_other_months(&mut self, from: CategoryId, to: CategoryId) {
        let Some(st) = &self.storage else { return };
        for key in st.list_months().into_iter().filter(|k| *k != self.month) {
            let mut data = st.load_month(key).value;
            let mut changed = false;
            for e in data.entries.iter_mut().filter(|e| e.category_id == from) {
                e.category_id = to;
                changed = true;
            }
            if changed && let Err(e) = st.save_month(key, &data) {
                self.error = Some(format!("Saving {key} failed: {e}").into());
            }
        }
    }

    fn copy_sankeymatic(&mut self, cx: &mut Context<Self>) {
        let text = self.data.to_sankeymatic(&self.month.title());
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.copied = true;
        cx.notify();
    }

    // ---- chart labels ----

    /// Saves the label fields (after cleaning them up) and redraws the chart.
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
        if let Some(st) = &self.storage {
            match st.save_settings(&self.settings) {
                Ok(()) => self.notice = Some("Chart labels saved.".into()),
                Err(e) => self.error = Some(format!("Saving settings failed: {e}").into()),
            }
        }
        cx.notify();
    }

    fn reset_labels(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let d = Labels::default();
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
        let text = self.data.to_sankeymatic(&self.month.title());
        let dir: PathBuf = directories::UserDirs::new()
            .and_then(|d| d.document_dir().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let name = format!("budget-{}-sankeymatic.txt", self.month);
        let rx = cx.prompt_for_new_path(&dir, Some(&name));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let res = std::fs::write(&path, text);
            this.update(cx, |this, cx| {
                match res {
                    Ok(()) => this.notice = Some(format!("Saved {}", path.display()).into()),
                    Err(e) => this.error = Some(format!("Could not save the file: {e}").into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn paste_import(&mut self, cx: &mut Context<Self>) {
        match cx.read_from_clipboard().and_then(|item| item.text()) {
            Some(text) => self.start_import(&text, cx),
            None => {
                self.set_error("The clipboard has no text. Copy SankeyMATIC text first.");
                cx.notify();
            }
        }
    }

    fn open_import_file(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select a SankeyMATIC text file".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let read = std::fs::read_to_string(&path);
            this.update(cx, |this, cx| {
                match read {
                    Ok(text) => this.start_import(&text, cx),
                    Err(e) => this.error = Some(format!("Could not read the file: {e}").into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Parses text and shows the confirmation panel; nothing changes yet.
    fn start_import(&mut self, text: &str, cx: &mut Context<Self>) {
        match import::parse_with(text, &self.settings.labels) {
            Ok(plan) => {
                self.pending_import = Some(plan);
                self.error = None;
                self.notice = None;
            }
            Err(e) => {
                self.pending_import = None;
                self.error = Some(format!("Import failed: {e}").into());
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
        self.category = self.cats.flatten(self.kind).first().map(|(_, c)| c.id);
        self.error = None;
        self.commit();
        self.notice = Some(
            format!(
                "Imported {} entries into {} ({} new categories).",
                stats.entries_added,
                self.month.title(),
                stats.categories_added
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
    }

    // ---- entries ----

    fn submit_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(category_id) = self.category else {
            self.set_error("Pick a category first (create one on the Categories page).");
            cx.notify();
            return;
        };
        let Some(amount_cents) = parse_amount_cents(self.amount_in.read(cx).value().as_ref())
        else {
            self.set_error("Enter a positive amount, e.g. 12.50.");
            cx.notify();
            return;
        };
        let name = self.name_in.read(cx).value().trim().to_string();

        match self.editing {
            Some(id) => {
                if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                    e.category_id = category_id;
                    e.name = name;
                    e.amount_cents = amount_cents;
                }
            }
            None => {
                self.entries.push(Entry {
                    id: self.next_entry_id,
                    category_id,
                    name,
                    amount_cents,
                });
                self.next_entry_id += 1;
            }
        }
        self.error = None;
        self.clear_inputs(window, cx);
        self.commit();
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
        self.name_in.update(cx, |s, cx| s.set_value(e.name.as_str(), window, cx));
        let amount = format!("{:.2}", e.amount_cents as f64 / 100.0);
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
        self.category = self.cats.flatten(kind).first().map(|(_, c)| c.id);
        cx.notify();
    }

    // ---- categories ----

    fn submit_category(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.cat_name_in.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.set_error("Category name cannot be empty.");
            cx.notify();
            return;
        }
        let editing = self.editing_cat;
        if self.cats.items.iter().any(|c| {
            c.kind == self.cat_kind && Some(c.id) != editing && c.name.eq_ignore_ascii_case(&name)
        }) {
            self.set_error("A category with that name already exists.");
            cx.notify();
            return;
        }
        match editing {
            Some(id) => {
                if !self.cats.update(id, name, self.cat_parent, self.cat_color) {
                    self.set_error("That parent is not allowed for this category.");
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
            let loaded = st.load_month(month);
            self.error = loaded.warning.map(Into::into);
            self.entries = loaded.value.entries;
        } else {
            self.entries = Vec::new();
        }
        self.next_entry_id = MonthData { entries: self.entries.clone() }.next_entry_id();
        self.clear_inputs(window, cx);
        self.refresh();
        cx.notify();
    }

    fn category_used(&self, id: CategoryId) -> bool {
        self.entries.iter().any(|e| e.category_id == id)
            || self.storage.as_ref().is_some_and(|st| {
                st.list_months()
                    .into_iter()
                    .filter(|k| *k != self.month)
                    .any(|k| st.load_month(k).value.entries.iter().any(|e| e.category_id == id))
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
                self.set_error("Cannot delete the only category that still has entries.");
                cx.notify();
                return;
            }
            None => id, // unused: nothing to move
        };
        if target != id {
            self.reassign_in_other_months(id, target);
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

mod view;
