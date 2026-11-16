//! Recurring entries: a monthly template (rent, salary, subscriptions) that adds a
//! normal entry to a month when that month is opened.
//!
//! The rules are checked when the user opens a month that is still empty (and at
//! startup for the month shown). Nothing is added to other months in the
//! background, so past months are never changed behind the user's back. A template
//! remembers the months it was applied to, so an entry the user deleted from a
//! month does not come back.

use std::io;

use serde::{Deserialize, Serialize};

use crate::model::{CategoryId, Entry, MonthKey};
use crate::storage::Storage;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recurring {
    pub id: u64,
    pub category_id: CategoryId,
    pub name: String,
    pub amount_cents: i64,
    /// First month that gets the entry.
    pub start: MonthKey,
    /// Last month that gets the entry; `None` while it goes on.
    pub end: Option<MonthKey>,
    /// Months that already got their entry.
    #[serde(default)]
    pub applied: Vec<MonthKey>,
}

impl Recurring {
    fn covers(&self, month: MonthKey) -> bool {
        month >= self.start && self.end.is_none_or(|end| month <= end)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecurringStore {
    pub items: Vec<Recurring>,
}

impl RecurringStore {
    /// Adds a template that starts in `start`. The entry for `start` itself is
    /// expected to exist already, so that month counts as applied.
    pub fn add(&mut self, category_id: CategoryId, name: String, amount_cents: i64, start: MonthKey) -> u64 {
        let id = self.items.iter().map(|r| r.id).max().map_or(1, |m| m + 1);
        self.items.push(Recurring { id, category_id, name, amount_cents, start, end: None, applied: vec![start] });
        id
    }

    /// Whether a template with these values already adds entries in `month`.
    pub fn repeats(&self, category_id: CategoryId, name: &str, amount_cents: i64, month: MonthKey) -> bool {
        self.items.iter().any(|r| {
            r.covers(month) && r.category_id == category_id && r.name == name && r.amount_cents == amount_cents
        })
    }

    /// Ends a template after `last` (never before its own start month).
    pub fn stop(&mut self, id: u64, last: MonthKey) {
        if let Some(r) = self.items.iter_mut().find(|r| r.id == id) {
            r.end = Some(last.max(r.start));
        }
    }

    /// Removes a template. Entries it already added stay.
    pub fn remove(&mut self, id: u64) {
        self.items.retain(|r| r.id != id);
    }

    pub fn uses_category(&self, id: CategoryId) -> bool {
        self.items.iter().any(|r| r.category_id == id)
    }

    /// Moves templates to another category; true if anything changed.
    pub fn reassign_category(&mut self, from: CategoryId, to: CategoryId) -> bool {
        let mut changed = false;
        for r in self.items.iter_mut().filter(|r| r.category_id == from) {
            r.category_id = to;
            changed = true;
        }
        changed
    }

    /// Indexes of the templates that still owe an entry to `month`.
    fn due(&self, month: MonthKey) -> Vec<usize> {
        (0..self.items.len())
            .filter(|&i| self.items[i].covers(month) && !self.items[i].applied.contains(&month))
            .collect()
    }
}

/// Called when `month` is opened: if it has no entries yet, adds the entries the
/// recurring templates owe it, and saves the store if it changed. A month that
/// already has entries is left alone.
pub fn apply_month(st: &Storage, store: &mut RecurringStore, month: MonthKey) -> io::Result<()> {
    let due = store.due(month);
    if due.is_empty() {
        return Ok(());
    }
    let mut data = st.load_month(month);
    if !data.entries.is_empty() {
        return Ok(());
    }
    let mut next_id = data.next_entry_id();
    for &i in &due {
        let r = &store.items[i];
        data.entries.push(Entry {
            id: next_id,
            category_id: r.category_id,
            name: r.name.clone(),
            amount_cents: r.amount_cents,
        });
        next_id += 1;
    }
    st.save_month(month, &data)?;
    for i in due {
        store.items[i].applied.push(month);
    }
    st.save_recurring(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MonthData;
    use std::fs;

    fn temp_storage(name: &str) -> Storage {
        let dir = std::env::temp_dir().join(format!("budget-test-{}-recurring-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Storage::new(dir)
    }

    fn month(y: i32, m: u8) -> MonthKey {
        MonthKey::new(y, m).unwrap()
    }

    #[test]
    fn opening_an_empty_month_adds_the_entries() {
        let st = temp_storage("open");
        let mut store = RecurringStore::default();
        store.add(3, "Rent".into(), 80000, month(2026, 9));
        apply_month(&st, &mut store, month(2026, 10)).unwrap();
        let entries = st.load_month(month(2026, 10)).entries;
        assert_eq!(entries.len(), 1);
        assert_eq!((entries[0].category_id, entries[0].amount_cents, entries[0].name.as_str()), (3, 80000, "Rent"));
        // The store was saved with the month marked, and other months are untouched.
        assert_eq!(st.load_recurring(), store);
        assert!(st.load_month(month(2026, 9)).entries.is_empty());
        assert!(st.load_month(month(2026, 11)).entries.is_empty());
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn nothing_is_added_to_months_that_are_not_opened() {
        let st = temp_storage("background");
        let mut store = RecurringStore::default();
        store.add(1, "Gym".into(), 3000, month(2026, 1));
        apply_month(&st, &mut store, month(2026, 5)).unwrap();
        assert_eq!(st.list_months(), vec![month(2026, 5)]);
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn a_month_with_entries_is_left_alone() {
        let st = temp_storage("nonempty");
        let manual = MonthData { entries: vec![Entry { id: 1, category_id: 1, name: "x".into(), amount_cents: 1 }] };
        st.save_month(month(2026, 3), &manual).unwrap();
        let mut store = RecurringStore::default();
        store.add(1, "Gym".into(), 3000, month(2026, 1));
        apply_month(&st, &mut store, month(2026, 3)).unwrap();
        assert_eq!(st.load_month(month(2026, 3)), manual);
        assert_eq!(store.items[0].applied, vec![month(2026, 1)], "the month is not marked as applied");
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn a_deleted_entry_does_not_come_back() {
        let st = temp_storage("deleted");
        let mut store = RecurringStore::default();
        store.add(1, "Gym".into(), 3000, month(2026, 1));
        apply_month(&st, &mut store, month(2026, 2)).unwrap();
        st.save_month(month(2026, 2), &MonthData::default()).unwrap();
        apply_month(&st, &mut store, month(2026, 2)).unwrap();
        assert!(st.load_month(month(2026, 2)).entries.is_empty());
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn months_outside_the_range_are_not_filled() {
        let st = temp_storage("range");
        let mut store = RecurringStore::default();
        let id = store.add(1, "Course".into(), 5000, month(2026, 3));
        store.stop(id, month(2026, 5));
        apply_month(&st, &mut store, month(2026, 2)).unwrap();
        apply_month(&st, &mut store, month(2026, 6)).unwrap();
        assert!(st.list_months().is_empty());
        apply_month(&st, &mut store, month(2026, 5)).unwrap();
        assert_eq!(st.load_month(month(2026, 5)).entries.len(), 1);
        // Stopping never ends before the start month.
        let id = store.add(1, "X".into(), 1, month(2026, 8));
        store.stop(id, month(2026, 1));
        assert_eq!(store.items[1].end, Some(month(2026, 8)));
        let _ = fs::remove_dir_all(st.dir());
    }

    #[test]
    fn entries_get_fresh_ids_and_templates_can_move() {
        let st = temp_storage("ids");
        let mut store = RecurringStore::default();
        store.add(1, "Rent".into(), 100, month(2026, 1));
        store.add(1, "Gym".into(), 200, month(2026, 1));
        apply_month(&st, &mut store, month(2026, 2)).unwrap();
        let ids: Vec<u64> = st.load_month(month(2026, 2)).entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![1, 2]);
        assert!(store.uses_category(1));
        // An entry that already repeats with the same values needs no second template.
        assert!(store.repeats(1, "Rent", 100, month(2026, 5)));
        assert!(!store.repeats(1, "Rent", 101, month(2026, 5)));
        assert!(!store.repeats(1, "Rent", 100, month(2025, 12)));
        assert!(store.reassign_category(1, 4) && store.uses_category(4) && !store.uses_category(1));
        store.remove(1);
        assert_eq!(store.items.len(), 1);
        let _ = fs::remove_dir_all(st.dir());
    }
}
