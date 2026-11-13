//! Domain model: user-defined category tree and monthly entries.

use serde::{Deserialize, Serialize};

pub type CategoryId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Income,
    Expense,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub kind: Kind,
    /// Parent category of the same kind; `None` for a top-level category.
    pub parent: Option<CategoryId>,
    /// Color as 0xRRGGBB.
    pub color: u32,
}

/// A single income or expense line. Amounts are stored in cents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: u64,
    pub category_id: CategoryId,
    pub name: String,
    pub amount_cents: i64,
}

/// Categories are shared between all months.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Categories {
    pub items: Vec<Category>,
    next_id: CategoryId,
}

impl Default for Categories {
    fn default() -> Self {
        let mut c = Categories {
            items: Vec::new(),
            next_id: 1,
        };
        for (name, color) in [("Salary", 0x4CAF50), ("Other income", 0x8BC34A)] {
            c.add(name, Kind::Income, None, color);
        }
        for (name, color) in [
            ("Housing", 0x3F51B5),
            ("Food", 0xFF9800),
            ("Transport", 0x03A9F4),
            ("Leisure", 0x9C27B0),
        ] {
            c.add(name, Kind::Expense, None, color);
        }
        c
    }
}

impl Categories {
    /// The starter categories with names in `lang` (used on a first run).
    pub fn defaults_for(lang: crate::i18n::Lang) -> Categories {
        use crate::i18n::Msg;
        let mut c = Categories { items: Vec::new(), next_id: 1 };
        for (msg, color) in [(Msg::DefSalary, 0x4CAF50), (Msg::DefOtherIncome, 0x8BC34A)] {
            c.add(msg.text(lang), Kind::Income, None, color);
        }
        for (msg, color) in [
            (Msg::DefHousing, 0x3F51B5),
            (Msg::DefFood, 0xFF9800),
            (Msg::DefTransport, 0x03A9F4),
            (Msg::DefLeisure, 0x9C27B0),
        ] {
            c.add(msg.text(lang), Kind::Expense, None, color);
        }
        c
    }

    pub fn add(
        &mut self,
        name: impl Into<String>,
        kind: Kind,
        parent: Option<CategoryId>,
        color: u32,
    ) -> CategoryId {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Category {
            id,
            name: name.into(),
            kind,
            parent,
            color,
        });
        id
    }

    pub fn get(&self, id: CategoryId) -> Option<&Category> {
        self.items.iter().find(|c| c.id == id)
    }

    /// Categories of `kind` in tree order (parents before their children)
    /// with their depth. Cycle-safe: each category appears at most once.
    pub fn flatten(&self, kind: Kind) -> Vec<(usize, &Category)> {
        fn go<'a>(
            cats: &'a Categories,
            c: &'a Category,
            depth: usize,
            seen: &mut Vec<CategoryId>,
            out: &mut Vec<(usize, &'a Category)>,
        ) {
            if seen.contains(&c.id) {
                return;
            }
            seen.push(c.id);
            out.push((depth, c));
            for child in cats.children(c.id) {
                go(cats, child, depth + 1, seen, out);
            }
        }
        let mut out = Vec::new();
        let mut seen = Vec::new();
        for c in self.items.iter().filter(|c| c.kind == kind && c.parent.is_none()) {
            go(self, c, 0, &mut seen, &mut out);
        }
        out
    }

    /// True if `id` is `ancestor` itself or lies below it in the tree.
    pub fn is_within(&self, id: CategoryId, ancestor: CategoryId) -> bool {
        let mut cur = Some(id);
        let mut steps = 0;
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            steps += 1;
            if steps > self.items.len() {
                return false; // cycle guard
            }
            cur = self.get(c).and_then(|x| x.parent);
        }
        false
    }

    /// Changes name, parent and color. The kind never changes. Returns false
    /// (and changes nothing) if the category is unknown or the new parent is
    /// itself, one of its descendants, or of a different kind.
    pub fn update(
        &mut self,
        id: CategoryId,
        name: impl Into<String>,
        parent: Option<CategoryId>,
        color: u32,
    ) -> bool {
        let Some(kind) = self.get(id).map(|c| c.kind) else {
            return false;
        };
        if let Some(p) = parent {
            let ok_kind = self.get(p).is_some_and(|pc| pc.kind == kind);
            if !ok_kind || self.is_within(p, id) {
                return false;
            }
        }
        let c = self.items.iter_mut().find(|c| c.id == id).unwrap();
        c.name = name.into();
        c.parent = parent;
        c.color = color;
        true
    }

    pub fn children(&self, id: CategoryId) -> impl Iterator<Item = &Category> {
        self.items.iter().filter(move |c| c.parent == Some(id))
    }

    /// Removes a category. Its children move to the removed category's parent
    /// and its entries are returned via `entries` re-pointed to `move_entries_to`.
    pub fn remove(
        &mut self,
        id: CategoryId,
        move_entries_to: CategoryId,
        entries: &mut [Entry],
    ) {
        let parent = self.get(id).and_then(|c| c.parent);
        for c in self.items.iter_mut().filter(|c| c.parent == Some(id)) {
            c.parent = parent;
        }
        for e in entries.iter_mut().filter(|e| e.category_id == id) {
            e.category_id = move_entries_to;
        }
        self.items.retain(|c| c.id != id);
    }
}

/// A calendar month, used as the storage key (`YYYY-MM`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MonthKey {
    pub year: i32,
    pub month: u8,
}

impl MonthKey {
    pub fn new(year: i32, month: u8) -> Option<Self> {
        (1..=12).contains(&month).then_some(Self { year, month })
    }

    pub fn prev(self) -> Self {
        if self.month == 1 {
            Self { year: self.year - 1, month: 12 }
        } else {
            Self { month: self.month - 1, ..self }
        }
    }

    pub fn next(self) -> Self {
        if self.month == 12 {
            Self { year: self.year + 1, month: 1 }
        } else {
            Self { month: self.month + 1, ..self }
        }
    }

    pub fn current() -> Self {
        use chrono::Datelike;
        let today = chrono::Local::now().date_naive();
        Self {
            year: today.year(),
            month: today.month() as u8,
        }
    }
}

impl std::fmt::Display for MonthKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl std::str::FromStr for MonthKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        let (y, m) = s.split_once('-').ok_or(())?;
        if y.len() != 4 || m.len() != 2 {
            return Err(());
        }
        MonthKey::new(y.parse().map_err(|_| ())?, m.parse().map_err(|_| ())?).ok_or(())
    }
}

/// Parses a user-typed amount ("12", "12.5", "12,50") into cents.
/// Returns `None` for empty, non-numeric, non-positive or absurdly large input.
pub fn parse_amount_cents(input: &str) -> Option<i64> {
    // Spaces (also the no-break ones used as thousands separators) are ignored.
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let v: f64 = cleaned.replace(',', ".").parse().ok()?;
    if !v.is_finite() || v <= 0.0 || v > 1.0e9 {
        return None;
    }
    Some((v * 100.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_edits_and_rejects_cycles() {
        let mut c = Categories::default();
        let find = |c: &Categories, n: &str| c.items.iter().find(|x| x.name == n).unwrap().id;
        let food = find(&c, "Food");
        let housing = find(&c, "Housing");
        let salary = find(&c, "Salary");
        let sub = c.add("Groceries", Kind::Expense, Some(food), 1);

        assert!(c.update(sub, "Supermarket", Some(housing), 0xABCDEF));
        let got = c.get(sub).unwrap();
        assert_eq!((got.name.as_str(), got.parent, got.color), ("Supermarket", Some(housing), 0xABCDEF));

        assert!(!c.update(food, "Food", Some(food), 0)); // itself
        assert!(c.update(sub, "Supermarket", Some(food), 0));
        assert!(!c.update(food, "Food", Some(sub), 0)); // own descendant
        assert!(!c.update(food, "Food", Some(salary), 0)); // other kind
        assert!(!c.update(9999, "x", None, 0)); // unknown
        assert_eq!(c.get(food).unwrap().parent, None);
    }

    #[test]
    fn month_key_round_trips() {
        let k = MonthKey::new(2026, 3).unwrap();
        assert_eq!(k.to_string(), "2026-03");
        assert_eq!("2026-03".parse::<MonthKey>(), Ok(k));
        assert!("2026-13".parse::<MonthKey>().is_err());
        assert!("2026-3".parse::<MonthKey>().is_err());
        assert!("categories".parse::<MonthKey>().is_err());
    }

    #[test]
    fn month_navigation_wraps_years() {
        let jan = MonthKey::new(2026, 1).unwrap();
        let dec = MonthKey::new(2025, 12).unwrap();
        assert_eq!(jan.prev(), dec);
        assert_eq!(dec.next(), jan);
        assert_eq!(jan.next(), MonthKey::new(2026, 2).unwrap());
    }

    #[test]
    fn parses_amounts() {
        assert_eq!(parse_amount_cents("12"), Some(1200));
        assert_eq!(parse_amount_cents(" 12,50 "), Some(1250));
        assert_eq!(parse_amount_cents("1 234,50"), Some(123_450));
        assert_eq!(parse_amount_cents("1\u{a0}234,50"), Some(123_450));
        assert_eq!(parse_amount_cents("0.07"), Some(7));
        assert_eq!(parse_amount_cents("19.99"), Some(1999));
        assert_eq!(parse_amount_cents(""), None);
        assert_eq!(parse_amount_cents("abc"), None);
        assert_eq!(parse_amount_cents("-5"), None);
        assert_eq!(parse_amount_cents("0"), None);
        assert_eq!(parse_amount_cents("NaN"), None);
    }

    #[test]
    fn flatten_orders_parents_before_children() {
        let mut c = Categories::default();
        let food = c.items.iter().find(|x| x.name == "Food").unwrap().id;
        c.add("Groceries", Kind::Expense, Some(food), 0);
        let names: Vec<_> = c
            .flatten(Kind::Expense)
            .iter()
            .map(|(d, c)| (*d, c.name.clone()))
            .collect();
        let food_pos = names.iter().position(|(_, n)| n == "Food").unwrap();
        assert_eq!(names[food_pos + 1], (1, "Groceries".to_string()));
    }
}
