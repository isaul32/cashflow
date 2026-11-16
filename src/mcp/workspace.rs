//! The MCP tools. They work on the saved data only, so they run in a process of their own
//! (`cashflow --mcp`) without any window. Every call starts by reading the latest files, and
//! every change goes through [`Storage`], which takes care that the app's window (which may
//! be open at the same time) does not lose it. The window notices the change and reloads.

use serde_json::{Value, json};

use crate::app::PALETTE;
use crate::backup;
use crate::i18n::{Currency, Lang, Locale, SystemLocale};
use crate::mcp::tools;
use crate::model::{Categories, Category, CategoryId, Entry, Kind, MonthKey};
use crate::recurring::{self, RecurringStore};
use crate::settings::{Settings, ThemeChoice};
use crate::storage::{MonthData, Storage};
use crate::{import, sankey, year};

/// Longest text accepted for a name or note.
const MAX_TEXT: usize = 200;

/// Runs one tool and returns its result, or a message that explains the failure.
///
/// The user's settings are read on every call, so turning the server off or changes on or off in
/// the app takes effect at once, also for a session that is already running.
pub fn call(st: &Storage, tool: &str, args: &Value) -> Result<Value, String> {
    let def = tools::find(tool).ok_or_else(|| format!("unknown tool: {tool}"))?;
    st.reload_if_changed();
    let mut ws = Workspace::load(st);
    if !ws.settings.mcp.enabled {
        return Err(tools::OFF.to_string());
    }
    if def.write && !ws.settings.mcp.allow_changes {
        return Err(tools::CHANGES_OFF.to_string());
    }
    if def.write && !st.has_categories() {
        // First use: the starter categories (in the user's language) become real data.
        st.save_categories(&ws.cats).map_err(|e| e.to_string())?;
    }
    ws.run(tool, args)
}

/// Everything a tool needs, read once at the start of a call.
struct Workspace {
    st: Storage,
    settings: Settings,
    lc: Locale,
    cats: Categories,
    recurring: RecurringStore,
    /// The month used when a tool is not given one: the current calendar month.
    month: MonthKey,
}

impl Workspace {
    fn load(st: &Storage) -> Workspace {
        let settings = st.load_settings().value;
        let lc = Locale::resolve(settings.language, settings.currency, &SystemLocale::detect());
        let cats = if st.has_categories() { st.load_categories() } else { Categories::defaults_for(lc.lang) };
        Workspace {
            st: st.clone(),
            settings,
            lc,
            cats,
            recurring: st.load_recurring(),
            month: MonthKey::current(),
        }
    }

    fn run(&mut self, tool: &str, args: &Value) -> Result<Value, String> {
        match tool {
            "get_state" => self.tool_get_state(),
            "list_categories" => Ok(self.tool_list_categories()),
            "list_months" => Ok(self.tool_list_months()),
            "list_entries" => self.tool_list_entries(args),
            "get_month_summary" => self.tool_month_summary(args),
            "get_year_summary" => self.tool_year_summary(args),
            "list_recurring" => Ok(self.tool_list_recurring()),
            "get_settings" => Ok(self.tool_get_settings()),
            "export_csv" => Ok(Value::String(self.st.snapshot().to_csv().trim_start_matches('\u{feff}').to_string())),
            "export_backup" => self.st.snapshot().to_json().map(Value::String),
            "export_sankeymatic" => self.tool_export_sankeymatic(args),
            "add_entry" => self.tool_add_entry(args),
            "update_entry" => self.tool_update_entry(args),
            "delete_entry" => self.tool_delete_entry(args),
            "add_category" => self.tool_add_category(args),
            "update_category" => self.tool_update_category(args),
            "delete_category" => self.tool_delete_category(args),
            "update_recurring" => self.tool_update_recurring(args),
            "stop_recurring" => self.tool_stop_recurring(args),
            "delete_recurring" => self.tool_delete_recurring(args),
            "set_settings" => self.tool_set_settings(args),
            "import_sankeymatic" => self.tool_import(args),
            "restore_backup" => self.tool_restore(args),
            other => Err(format!("tool not implemented: {other}")),
        }
    }

    fn labels(&self) -> crate::settings::Labels {
        self.settings.labels.resolved(self.lc.lang)
    }

    fn month_arg(&self, args: &Value) -> Result<MonthKey, String> {
        Ok(opt_month(args)?.unwrap_or(self.month))
    }

    /// A category from an id (a number) or a name.
    fn category_arg(&self, value: &Value) -> Result<CategoryId, String> {
        if let Some(id) = value.as_u64() {
            return self.cats.get(id).map(|c| c.id).ok_or_else(|| format!("there is no category with id {id}"));
        }
        let name = value.as_str().ok_or("a category is given as an id or a name")?.trim();
        let matches: Vec<&Category> = self.cats.items.iter().filter(|c| c.name.eq_ignore_ascii_case(name)).collect();
        match matches.as_slice() {
            [one] => Ok(one.id),
            [] => match name.parse::<u64>().ok().filter(|id| self.cats.get(*id).is_some()) {
                Some(id) => Ok(id),
                None => Err(format!(
                    "there is no category named \"{name}\"; the categories are: {}",
                    self.cats.items.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ")
                )),
            },
            many => Err(format!(
                "several categories are named \"{name}\", use an id: {}",
                many.iter()
                    .map(|c| format!("{} ({})", c.id, kind_name(c.kind)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    fn entry_json(&self, e: &Entry) -> Value {
        let cat = self.cats.get(e.category_id);
        json!({
            "id": e.id,
            "category_id": e.category_id,
            "category": backup::category_path(&self.cats, e.category_id),
            "kind": cat.map(|c| kind_name(c.kind)),
            "name": e.name,
            "amount": money(e.amount_cents),
            "amount_cents": e.amount_cents,
        })
    }

    /// (income, expenses) of entries whose category is known.
    fn totals(&self, entries: &[Entry]) -> (i64, i64) {
        let (mut income, mut expense) = (0, 0);
        for e in entries.iter().filter(|e| e.amount_cents > 0) {
            match self.cats.get(e.category_id).map(|c| c.kind) {
                Some(Kind::Income) => income += e.amount_cents,
                Some(Kind::Expense) => expense += e.amount_cents,
                None => {}
            }
        }
        (income, expense)
    }

    /// The total of each category that has entries, income first, largest first.
    fn by_category(&self, entries: &[Entry]) -> Vec<Value> {
        let mut sums: Vec<(CategoryId, i64)> = Vec::new();
        for e in entries.iter().filter(|e| e.amount_cents > 0 && self.cats.get(e.category_id).is_some()) {
            match sums.iter_mut().find(|(id, _)| *id == e.category_id) {
                Some((_, sum)) => *sum += e.amount_cents,
                None => sums.push((e.category_id, e.amount_cents)),
            }
        }
        sums.sort_by_key(|(id, sum)| {
            let income = self.cats.get(*id).is_some_and(|c| c.kind == Kind::Income);
            (!income, -*sum)
        });
        sums.into_iter()
            .map(|(id, sum)| {
                json!({
                    "category_id": id,
                    "category": backup::category_path(&self.cats, id),
                    "kind": self.cats.get(id).map(|c| kind_name(c.kind)),
                    "total": money(sum),
                })
            })
            .collect()
    }

    // ---- reading ----

    fn tool_get_state(&self) -> Result<Value, String> {
        Ok(json!({
            "app_version": env!("CARGO_PKG_VERSION"),
            "data_version": crate::data::VERSION,
            "current_month": self.month.to_string(),
            "language": lang_code(self.lc.lang),
            "currency": self.lc.currency.code(),
            "theme": theme_name(self.settings.theme),
            "categories": self.cats.items.len(),
            "months_with_entries": self.st.list_months().len(),
            "recurring_entries": self.recurring.items.len(),
            "changes_allowed": self.settings.mcp.allow_changes,
        }))
    }

    fn tool_list_categories(&self) -> Value {
        let mut out = Vec::new();
        for kind in [Kind::Income, Kind::Expense] {
            for (depth, c) in self.cats.flatten_sorted(kind) {
                out.push(json!({
                    "id": c.id,
                    "name": c.name,
                    "kind": kind_name(c.kind),
                    "parent_id": c.parent,
                    "depth": depth,
                    "path": backup::category_path(&self.cats, c.id),
                    "color": format!("#{:06x}", c.color),
                }));
            }
        }
        json!({"categories": out})
    }

    fn tool_list_months(&self) -> Value {
        let months: Vec<Value> = self
            .st
            .list_months()
            .into_iter()
            .map(|key| {
                let entries = self.st.load_month(key).entries;
                let (income, expense) = self.totals(&entries);
                json!({"month": key.to_string(), "entries": entries.len(), "income": money(income), "expenses": money(expense)})
            })
            .collect();
        json!({"currency": self.lc.currency.code(), "months": months})
    }

    fn tool_list_entries(&self, args: &Value) -> Result<Value, String> {
        let month = self.month_arg(args)?;
        let entries = self.st.load_month(month).entries;
        let (income, expense) = self.totals(&entries);
        Ok(json!({
            "month": month.to_string(),
            "currency": self.lc.currency.code(),
            "entries": entries.iter().map(|e| self.entry_json(e)).collect::<Vec<_>>(),
            "income": money(income),
            "expenses": money(expense),
            "balance": money(income - expense),
        }))
    }

    fn tool_month_summary(&self, args: &Value) -> Result<Value, String> {
        let month = self.month_arg(args)?;
        let entries = self.st.load_month(month).entries;
        let (income, expense) = self.totals(&entries);
        Ok(json!({
            "month": month.to_string(),
            "currency": self.lc.currency.code(),
            "income": money(income),
            "expenses": money(expense),
            "balance": money(income - expense),
            "entries": entries.len(),
            "by_category": self.by_category(&entries),
        }))
    }

    fn year_entries(&self, year: i32) -> Vec<Vec<Entry>> {
        (1..=12u8).map(|m| self.st.load_month(MonthKey { year, month: m }).entries).collect()
    }

    fn tool_year_summary(&self, args: &Value) -> Result<Value, String> {
        let year = match args.get("year") {
            None | Some(Value::Null) => self.month.year,
            Some(v) => year_value(v)?,
        };
        let months = self.year_entries(year);
        let summary = year::summarize(year, &self.cats, &months, &self.labels());
        let all: Vec<Entry> = months.iter().flatten().cloned().collect();
        let per_month: Vec<Value> = summary
            .months
            .iter()
            .enumerate()
            .map(|(i, t)| {
                json!({
                    "month": MonthKey { year, month: i as u8 + 1 }.to_string(),
                    "income": money(t.income_cents),
                    "expenses": money(t.expense_cents),
                    "balance": money(t.balance_cents()),
                    "entries": t.entries,
                })
            })
            .collect();
        Ok(json!({
            "year": year,
            "currency": self.lc.currency.code(),
            "income": money(summary.data.income_cents),
            "expenses": money(summary.data.expense_cents),
            "balance": money(summary.data.balance_cents()),
            "months": per_month,
            "by_category": self.by_category(&all),
        }))
    }

    fn tool_list_recurring(&self) -> Value {
        let items: Vec<Value> = self
            .recurring
            .items
            .iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "category_id": r.category_id,
                    "category": backup::category_path(&self.cats, r.category_id),
                    "kind": self.cats.get(r.category_id).map(|c| kind_name(c.kind)),
                    "name": r.name,
                    "amount": money(r.amount_cents),
                    "from": r.start.to_string(),
                    "until": r.end.map(|e| e.to_string()),
                    "stopped": r.end.is_some(),
                })
            })
            .collect();
        json!({"currency": self.lc.currency.code(), "recurring": items})
    }

    fn tool_get_settings(&self) -> Value {
        let labels = self.labels();
        json!({
            "language": self.settings.language.map_or_else(|| "auto".to_string(), lang_code),
            "currency": self.settings.currency.map_or("auto", |c| c.code()),
            "theme": theme_name(self.settings.theme),
            "labels": {"total": labels.total, "savings": labels.savings, "deficit": labels.deficit},
            "effective_language": lang_code(self.lc.lang),
            "effective_currency": self.lc.currency.code(),
        })
    }

    fn tool_export_sankeymatic(&self, args: &Value) -> Result<Value, String> {
        let text = match args.get("year") {
            Some(v) if !v.is_null() => {
                let year = year_value(v)?;
                let summary = year::summarize(year, &self.cats, &self.year_entries(year), &self.labels());
                summary.data.to_sankeymatic(&year.to_string())
            }
            _ => {
                let month = self.month_arg(args)?;
                let data = sankey::build_with(&self.cats, &self.st.load_month(month).entries, &self.labels());
                data.to_sankeymatic(&month.to_string())
            }
        };
        Ok(Value::String(text))
    }

    // ---- entries ----

    fn tool_add_entry(&mut self, args: &Value) -> Result<Value, String> {
        let category = self.category_arg(args.get("category").ok_or("category is required")?)?;
        let cents = amount_cents(args.get("amount").ok_or("amount is required")?)?;
        let name = text_arg(args, "name")?.unwrap_or_default();
        let month = self.month_arg(args)?;
        let repeat = args.get("repeat_monthly").and_then(Value::as_bool).unwrap_or(false);
        let st = self.st.clone();

        // Like opening the month in the app: an empty month first gets its recurring entries.
        recurring::apply_month(&st, &mut self.recurring, month).map_err(|e| e.to_string())?;
        let mut data = st.load_month(month);
        let entry = Entry { id: data.next_entry_id(), category_id: category, name: name.clone(), amount_cents: cents };
        data.entries.push(entry.clone());
        st.save_month(month, &data).map_err(|e| e.to_string())?;

        let mut repeating = false;
        if repeat {
            if !self.recurring.repeats(category, &name, cents, month) {
                self.recurring.add(category, name, cents, month);
            }
            st.save_recurring(&self.recurring).map_err(|e| e.to_string())?;
            repeating = true;
        }
        Ok(json!({"month": month.to_string(), "added": self.entry_json(&entry), "repeats_monthly": repeating}))
    }

    fn tool_update_entry(&mut self, args: &Value) -> Result<Value, String> {
        let id = id_arg(args, "id")?;
        let month = self.month_arg(args)?;
        let mut data = self.st.load_month(month);
        let category = match args.get("category") {
            Some(v) if !v.is_null() => Some(self.category_arg(v)?),
            _ => None,
        };
        let name = text_arg(args, "name")?;
        let cents = match args.get("amount") {
            Some(v) if !v.is_null() => Some(amount_cents(v)?),
            _ => None,
        };
        if category.is_none() && name.is_none() && cents.is_none() {
            return Err("nothing to change: give category, name or amount".to_string());
        }
        let entry = data
            .entries
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or_else(|| format!("there is no entry {id} in {month}"))?;
        if let Some(c) = category {
            entry.category_id = c;
        }
        if let Some(n) = name {
            entry.name = n;
        }
        if let Some(c) = cents {
            entry.amount_cents = c;
        }
        let updated = entry.clone();
        self.st.save_month(month, &data).map_err(|e| e.to_string())?;
        Ok(json!({"month": month.to_string(), "updated": self.entry_json(&updated)}))
    }

    fn tool_delete_entry(&mut self, args: &Value) -> Result<Value, String> {
        let id = id_arg(args, "id")?;
        let month = self.month_arg(args)?;
        let mut data = self.st.load_month(month);
        let position = data
            .entries
            .iter()
            .position(|e| e.id == id)
            .ok_or_else(|| format!("there is no entry {id} in {month}"))?;
        let removed = data.entries.remove(position);
        self.st.save_month(month, &data).map_err(|e| e.to_string())?;
        Ok(json!({"month": month.to_string(), "deleted": self.entry_json(&removed)}))
    }

    // ---- categories ----

    /// A category name that is free for `kind` (ignoring `except`, the category being renamed).
    fn check_category_name(&self, name: &str, kind: Kind, except: Option<CategoryId>) -> Result<(), String> {
        if name.is_empty() {
            return Err("the category name is empty".to_string());
        }
        if self.cats.items.iter().any(|c| c.kind == kind && Some(c.id) != except && c.name.eq_ignore_ascii_case(name)) {
            return Err(format!("there is already a {} category named \"{name}\"", kind_name(kind)));
        }
        Ok(())
    }

    fn tool_add_category(&mut self, args: &Value) -> Result<Value, String> {
        let name = text_arg(args, "name")?.ok_or("name is required")?;
        let kind = match args.get("kind").and_then(Value::as_str) {
            Some("income") => Kind::Income,
            Some("expense") => Kind::Expense,
            _ => return Err("kind must be \"income\" or \"expense\"".to_string()),
        };
        self.check_category_name(&name, kind, None)?;
        let parent = match args.get("parent") {
            Some(v) if !v.is_null() => {
                let id = self.category_arg(v)?;
                if self.cats.get(id).map(|c| c.kind) != Some(kind) {
                    return Err("a sub-category must have the same kind as its parent".to_string());
                }
                Some(id)
            }
            _ => None,
        };
        let color = match args.get("color").and_then(Value::as_str) {
            Some(c) => parse_color(c)?,
            None => PALETTE[self.cats.items.len() % PALETTE.len()],
        };
        let mut cats = self.cats.clone();
        let id = cats.add(name, kind, parent, color);
        self.st.save_categories(&cats).map_err(|e| e.to_string())?;
        let c = cats.get(id).expect("just added");
        Ok(json!({"added": {"id": c.id, "name": c.name, "kind": kind_name(c.kind), "parent_id": c.parent, "color": format!("#{:06x}", c.color)}}))
    }

    fn tool_update_category(&mut self, args: &Value) -> Result<Value, String> {
        let id = self.category_arg(args.get("category").ok_or("category is required")?)?;
        let current = self.cats.get(id).cloned().ok_or("the category does not exist")?;
        let name = text_arg(args, "name")?.unwrap_or_else(|| current.name.clone());
        self.check_category_name(&name, current.kind, Some(id))?;
        let parent = match args.get("parent") {
            None => current.parent,
            Some(Value::Null) => None,
            Some(v) => Some(self.category_arg(v)?),
        };
        let color = match args.get("color").and_then(Value::as_str) {
            Some(c) => parse_color(c)?,
            None => current.color,
        };
        let mut cats = self.cats.clone();
        if !cats.update(id, name, parent, color) {
            return Err("that parent is not possible: it must have the same kind and not be inside this category".to_string());
        }
        self.st.save_categories(&cats).map_err(|e| e.to_string())?;
        let c = cats.get(id).expect("exists");
        Ok(json!({"updated": {"id": c.id, "name": c.name, "kind": kind_name(c.kind), "parent_id": c.parent, "color": format!("#{:06x}", c.color)}}))
    }

    fn tool_delete_category(&mut self, args: &Value) -> Result<Value, String> {
        let id = self.category_arg(args.get("category").ok_or("category is required")?)?;
        let cat = self.cats.get(id).cloned().ok_or("the category does not exist")?;
        let st = self.st.clone();
        // Entries move to the parent, or to another top-level category of the same kind.
        let target = cat
            .parent
            .or_else(|| self.cats.flatten(cat.kind).iter().map(|(_, c)| c.id).find(|&other| other != id));
        let used = self.recurring.uses_category(id)
            || st.list_months().into_iter().any(|k| st.load_month(k).entries.iter().any(|e| e.category_id == id));
        let target = match target {
            Some(t) => t,
            None if used => {
                return Err(format!(
                    "\"{}\" is the only {} category and it has entries; add another one first",
                    cat.name,
                    kind_name(cat.kind)
                ));
            }
            None => id,
        };
        let mut moved = 0;
        if target != id {
            for key in st.list_months() {
                let mut data = st.load_month(key);
                let mut changed = false;
                for e in data.entries.iter_mut().filter(|e| e.category_id == id) {
                    e.category_id = target;
                    moved += 1;
                    changed = true;
                }
                if changed {
                    st.save_month(key, &data).map_err(|e| e.to_string())?;
                }
            }
            if self.recurring.reassign_category(id, target) {
                st.save_recurring(&self.recurring).map_err(|e| e.to_string())?;
            }
        }
        let mut cats = self.cats.clone();
        cats.remove(id, target, &mut []);
        st.save_categories(&cats).map_err(|e| e.to_string())?;
        Ok(json!({
            "deleted": cat.name,
            "entries_moved": moved,
            "moved_to": (target != id).then(|| backup::category_path(&cats, target)),
        }))
    }

    // ---- recurring entries ----

    fn recurring_json(&self, id: u64) -> Value {
        self.tool_list_recurring()["recurring"]
            .as_array()
            .and_then(|items| items.iter().find(|r| r["id"] == id).cloned())
            .unwrap_or(Value::Null)
    }

    fn tool_update_recurring(&mut self, args: &Value) -> Result<Value, String> {
        let id = id_arg(args, "id")?;
        let category = match args.get("category") {
            Some(v) if !v.is_null() => Some(self.category_arg(v)?),
            _ => None,
        };
        let name = text_arg(args, "name")?;
        let cents = match args.get("amount") {
            Some(v) if !v.is_null() => Some(amount_cents(v)?),
            _ => None,
        };
        if category.is_none() && name.is_none() && cents.is_none() {
            return Err("nothing to change: give category, name or amount".to_string());
        }
        let item = self
            .recurring
            .items
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("there is no recurring entry {id}"))?;
        if let Some(c) = category {
            item.category_id = c;
        }
        if let Some(n) = name {
            item.name = n;
        }
        if let Some(c) = cents {
            item.amount_cents = c;
        }
        self.st.save_recurring(&self.recurring).map_err(|e| e.to_string())?;
        Ok(json!({"updated": self.recurring_json(id)}))
    }

    fn tool_stop_recurring(&mut self, args: &Value) -> Result<Value, String> {
        let id = id_arg(args, "id")?;
        if !self.recurring.items.iter().any(|r| r.id == id) {
            return Err(format!("there is no recurring entry {id}"));
        }
        self.recurring.stop(id, MonthKey::current());
        self.st.save_recurring(&self.recurring).map_err(|e| e.to_string())?;
        Ok(json!({"stopped": self.recurring_json(id)}))
    }

    fn tool_delete_recurring(&mut self, args: &Value) -> Result<Value, String> {
        let id = id_arg(args, "id")?;
        let before = self.recurring_json(id);
        if before.is_null() {
            return Err(format!("there is no recurring entry {id}"));
        }
        self.recurring.remove(id);
        self.st.save_recurring(&self.recurring).map_err(|e| e.to_string())?;
        Ok(json!({"deleted": before}))
    }

    // ---- settings, import and restore ----

    fn tool_set_settings(&mut self, args: &Value) -> Result<Value, String> {
        // Only these can be changed. In particular the assistant cannot touch the MCP settings
        // (its own permissions) or the update check.
        if let Some(unknown) = args
            .as_object()
            .and_then(|m| m.keys().find(|k| !["language", "currency", "theme", "labels"].contains(&k.as_str())))
        {
            return Err(format!(
                "\"{unknown}\" cannot be changed here; the settings are language, currency, theme and labels"
            ));
        }
        // Check everything before changing anything.
        let language = match args.get("language").and_then(Value::as_str) {
            None => None,
            Some("auto") => Some(None),
            Some(code) => Some(Some(Lang::from_tag(code).ok_or_else(|| format!("unknown language \"{code}\""))?)),
        };
        let currency = match args.get("currency").and_then(Value::as_str) {
            None => None,
            Some("auto") => Some(None),
            Some(code) => Some(Some(
                Currency::ALL
                    .iter()
                    .copied()
                    .find(|c| c.code().eq_ignore_ascii_case(code))
                    .ok_or_else(|| format!("unknown currency \"{code}\""))?,
            )),
        };
        let theme = match args.get("theme").and_then(Value::as_str) {
            None => None,
            Some("auto") => Some(None),
            Some("light") => Some(Some(ThemeChoice::Light)),
            Some("dark") => Some(Some(ThemeChoice::Dark)),
            Some(other) => return Err(format!("unknown theme \"{other}\"")),
        };
        let labels = match args.get("labels") {
            None | Some(Value::Null) => None,
            Some(Value::Object(map)) => {
                let mut labels = self.labels();
                for (key, value) in map {
                    let text = value.as_str().ok_or("labels must be text")?;
                    match key.as_str() {
                        "total" => labels.total = text.to_string(),
                        "savings" => labels.savings = text.to_string(),
                        "deficit" => labels.deficit = text.to_string(),
                        other => return Err(format!("unknown label \"{other}\"")),
                    }
                }
                Some(labels.sanitized())
            }
            Some(_) => return Err("labels must be an object".to_string()),
        };
        if language.is_none() && currency.is_none() && theme.is_none() && labels.is_none() {
            return Err("nothing to change: give language, currency, theme or labels".to_string());
        }

        let mut settings = self.settings.clone();
        if let Some(labels) = labels {
            settings.labels = labels;
        }
        if let Some(language) = language {
            settings.language = language;
        }
        if let Some(currency) = currency {
            settings.currency = currency;
        }
        if let Some(theme) = theme {
            settings.theme = theme;
        }
        self.st.save_settings(&settings).map_err(|e| e.to_string())?;
        self.lc = Locale::resolve(settings.language, settings.currency, &SystemLocale::detect());
        self.settings = settings;
        // Unedited starter categories follow the language, as they do in the app.
        let mut cats = self.cats.clone();
        if cats.translate_defaults(self.lc.lang) {
            self.st.save_categories(&cats).map_err(|e| e.to_string())?;
            self.cats = cats;
        }
        Ok(self.tool_get_settings())
    }

    fn tool_import(&mut self, args: &Value) -> Result<Value, String> {
        let text = args.get("text").and_then(Value::as_str).ok_or("text is required")?;
        let month = self.month_arg(args)?;
        let replace = args.get("replace").and_then(Value::as_bool).unwrap_or(false);
        let plan = import::parse_with(text, &self.labels(), self.lc.lang)?;
        let mut cats = self.cats.clone();
        let mut entries = self.st.load_month(month).entries;
        let mut next_id = MonthData { entries: entries.clone() }.next_entry_id();
        let stats = import::apply(&plan, &mut cats, &mut entries, &mut next_id, replace, &PALETTE);
        self.st.save_categories_and_month(&cats, month, &MonthData { entries }).map_err(|e| e.to_string())?;
        Ok(json!({
            "month": month.to_string(),
            "entries_added": stats.entries_added,
            "categories_added": stats.categories_added,
            "warnings": plan.warnings,
        }))
    }

    fn tool_restore(&mut self, args: &Value) -> Result<Value, String> {
        if args.get("confirm").and_then(Value::as_bool) != Some(true) {
            return Err("this replaces all data; call again with confirm: true to go on".to_string());
        }
        let text = args.get("backup").and_then(Value::as_str).ok_or("backup is required")?;
        let data = backup::parse(text)?;
        let copy = backup::save_safety_copy(&self.st).map_err(|e| e.to_string())?;
        self.st.replace_all(&data).map_err(|e| e.to_string())?;
        Ok(json!({
            "restored": {"categories": data.categories.items.len(), "months": data.months.len(), "recurring": data.recurring.items.len()},
            "copy_of_previous_data": copy.display().to_string(),
        }))
    }
}

// ---- small helpers ----

fn money(cents: i64) -> f64 {
    cents as f64 / 100.0
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Income => "income",
        Kind::Expense => "expense",
    }
}

fn lang_code(lang: Lang) -> String {
    serde_json::to_value(lang).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn theme_name(theme: Option<ThemeChoice>) -> &'static str {
    match theme {
        None => "auto",
        Some(ThemeChoice::Light) => "light",
        Some(ThemeChoice::Dark) => "dark",
    }
}

/// `month` as `YYYY-MM`, if given.
fn opt_month(args: &Value) -> Result<Option<MonthKey>, String> {
    match args.get("month") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => s.parse().map(Some).map_err(|_| format!("invalid month \"{s}\", use YYYY-MM")),
        Some(_) => Err("month must be text like 2026-11".to_string()),
    }
}

fn year_value(value: &Value) -> Result<i32, String> {
    value
        .as_i64()
        .filter(|y| (1900..=2200).contains(y))
        .map(|y| y as i32)
        .ok_or_else(|| "year must be a number between 1900 and 2200".to_string())
}

/// A positive amount in currency units, as cents.
fn amount_cents(value: &Value) -> Result<i64, String> {
    let amount = value.as_f64().ok_or("amount must be a number")?;
    if !amount.is_finite() || amount <= 0.0 {
        return Err("amount must be a positive number".to_string());
    }
    let cents = (amount * 100.0).round();
    if !(1.0..=1e12).contains(&cents) {
        return Err("amount is out of range".to_string());
    }
    Ok(cents as i64)
}

fn id_arg(args: &Value, key: &str) -> Result<u64, String> {
    args.get(key).and_then(Value::as_u64).ok_or_else(|| format!("{key} is required and must be a whole number"))
}

/// An optional text argument, trimmed on one line and limited in length.
fn text_arg(args: &Value, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
            if one_line.chars().count() > MAX_TEXT {
                return Err(format!("{key} is longer than {MAX_TEXT} characters"));
            }
            Ok(Some(one_line))
        }
        Some(_) => Err(format!("{key} must be text")),
    }
}

/// `#RRGGBB` (the `#` is optional).
fn parse_color(text: &str) -> Result<u32, String> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err("a colour is written as #RRGGBB".to_string());
    }
    u32::from_str_radix(hex, 16).map_err(|_| "a colour is written as #RRGGBB".to_string())
}

#[cfg(test)]
mod tests;
