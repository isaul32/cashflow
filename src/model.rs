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

/// The entries of `source` as the start of another month: the same categories, notes and
/// amounts, numbered from 1. Entries whose category no longer exists are left out.
pub fn copied_entries(source: &[Entry], cats: &Categories) -> Vec<Entry> {
    source
        .iter()
        .filter(|e| cats.get(e.category_id).is_some())
        .enumerate()
        .map(|(i, e)| Entry { id: i as u64 + 1, ..e.clone() })
        .collect()
}

/// Case-insensitive sort key for names.
pub fn name_key(name: &str) -> String {
    name.to_lowercase()
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

/// Names that starter categories used to have: (id of the starter category, old name). A category
/// that still has such a name counts as unedited, so it moves to the current name with the others.
const FORMER_DEFAULT_NAMES: &[(CategoryId, &str)] =
    &[(5, "Liikenne"), (4, "Alimentation"), (4, "Alimentación")];


impl Categories {
    /// The starter categories with names in `lang` (used on a first run): two kinds of income, and
    /// housing, food, transport, leisure and health, with sub-categories under housing and food.
    pub fn defaults_for(lang: crate::i18n::Lang) -> Categories {
        use crate::i18n::Msg;
        let mut c = Categories { items: Vec::new(), next_id: 1 };
        // The ids are fixed (1 to 10): an unedited starter category is recognised by its id.
        let name = |msg: Msg| msg.text(lang);
        c.add(name(Msg::DefSalary), Kind::Income, None, 0x4CAF50);
        c.add(name(Msg::DefOtherIncome), Kind::Income, None, 0x8BC34A);
        let housing = c.add(name(Msg::DefHousing), Kind::Expense, None, 0x3F51B5);
        let food = c.add(name(Msg::DefFood), Kind::Expense, None, 0xFF9800);
        c.add(name(Msg::DefTransport), Kind::Expense, None, 0x03A9F4);
        c.add(name(Msg::DefLeisure), Kind::Expense, None, 0x9C27B0);
        c.add(name(Msg::DefRent), Kind::Expense, Some(housing), 0x5C6BC0);
        c.add(name(Msg::DefGroceries), Kind::Expense, Some(food), 0xFFB74D);
        c.add(name(Msg::DefEatingOut), Kind::Expense, Some(food), 0xFF8A65);
        c.add(name(Msg::DefHealth), Kind::Expense, None, 0xE91E63);
        c
    }

    /// Gives the starter categories the names of `lang`, as long as they have not been edited.
    /// A starter category counts as unedited when it still has its original kind, colour and
    /// place at the top level and its name is that category's name in some language. Edited ones,
    /// and ones the user made, are left alone, and so is a rename that would collide with another
    /// category of the same kind. Returns true if anything changed.
    pub fn translate_defaults(&mut self, lang: crate::i18n::Lang) -> bool {
        let target = Categories::defaults_for(lang);
        let all: Vec<Categories> = crate::i18n::Lang::ALL.iter().map(|&l| Categories::defaults_for(l)).collect();
        let mut changed = false;
        for new in &target.items {
            let Some(pos) = self.items.iter().position(|c| c.id == new.id) else { continue };
            let current = &self.items[pos];
            let unedited = current.kind == new.kind
                && current.color == new.color
                && current.parent == new.parent
                && (all.iter().any(|names| names.get(new.id).is_some_and(|d| d.name == current.name))
                    || FORMER_DEFAULT_NAMES.iter().any(|(id, old)| *id == new.id && *old == current.name));
            if !unedited || current.name == new.name {
                continue;
            }
            let taken = self
                .items
                .iter()
                .any(|o| o.id != new.id && o.kind == new.kind && o.name.eq_ignore_ascii_case(&new.name));
            if !taken {
                self.items[pos].name = new.name.clone();
                changed = true;
            }
        }
        changed
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
        self.flatten_impl(kind, false)
    }

    /// Like [`Categories::flatten`], but siblings are sorted alphabetically.
    pub fn flatten_sorted(&self, kind: Kind) -> Vec<(usize, &Category)> {
        self.flatten_impl(kind, true)
    }

    fn flatten_impl(&self, kind: Kind, sorted: bool) -> Vec<(usize, &Category)> {
        fn go<'a>(
            cats: &'a Categories,
            c: &'a Category,
            depth: usize,
            sorted: bool,
            seen: &mut Vec<CategoryId>,
            out: &mut Vec<(usize, &'a Category)>,
        ) {
            if seen.contains(&c.id) {
                return;
            }
            seen.push(c.id);
            out.push((depth, c));
            let mut kids: Vec<&Category> = cats.children(c.id).collect();
            if sorted {
                kids.sort_by_key(|k| name_key(&k.name));
            }
            for child in kids {
                go(cats, child, depth + 1, sorted, seen, out);
            }
        }
        let mut roots: Vec<&Category> =
            self.items.iter().filter(|c| c.kind == kind && c.parent.is_none()).collect();
        if sorted {
            roots.sort_by_key(|c| name_key(&c.name));
        }
        let mut out = Vec::new();
        let mut seen = Vec::new();
        for c in roots {
            go(self, c, 0, sorted, &mut seen, &mut out);
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

/// True if `text` is digits grouped in threes by `mark`: "1,234" or "12,345,678".
fn is_grouped(text: &str, mark: char) -> bool {
    let parts: Vec<&str> = text.split(mark).collect();
    parts.len() >= 2
        && (1..=3).contains(&parts[0].len())
        && parts[1..].iter().all(|p| p.len() == 3)
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

/// Parses a user-typed amount into cents. `decimal` is the decimal mark of the
/// reader's number format ('.' or ','); the other of the two is the grouping mark.
///
/// Reads what people actually type:
/// - spaces (also no-break ones) and apostrophes (Swiss) are ignored,
/// - with both marks the last one is the decimal mark: "1,234.50" and "1.234,50",
/// - a lone `decimal` mark is a decimal: "12.50" in English, "12,50" in Finnish,
/// - a lone other mark that groups digits in threes is a grouping: "1,234" in
///   English means 1234, not 1.23; otherwise it is read as a decimal ("12,5").
///
/// More than two decimals, signs, letters, zero and absurdly large amounts give
/// `None`, so a typo is reported instead of being rounded silently.
pub fn parse_amount_cents(input: &str, decimal: char) -> Option<i64> {
    let s: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\'' | '’'))
        .collect();
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit() || c == ',' || c == '.') {
        return None;
    }
    let other = if decimal == '.' { ',' } else { '.' };
    let (decimals, others) = (s.matches(decimal).count(), s.matches(other).count());

    // (whole part, fraction digits, whether a decimal mark was used)
    let (whole, fraction, has_decimal_mark): (&str, &str, bool) = if decimals > 0 && others > 0 {
        // Both marks: the last one is the decimal mark and must appear once.
        let last = s.rfind(['.', ','])?;
        let mark = s[last..].chars().next()?;
        let (whole, rest) = s.split_at(last);
        let grouping = if mark == '.' { ',' } else { '.' };
        if whole.contains(mark) || !is_grouped(whole, grouping) {
            return None;
        }
        (whole, &rest[1..], true)
    } else if decimals > 0 {
        if decimals > 1 {
            return None;
        }
        let (whole, fraction) = s.split_once(decimal)?;
        (whole, fraction, true)
    } else if others > 0 {
        if is_grouped(&s, other) {
            (s.as_str(), "", false)
        } else if others == 1 {
            let (whole, fraction) = s.split_once(other)?;
            (whole, fraction, true)
        } else {
            return None;
        }
    } else {
        (s.as_str(), "", false)
    };

    // A decimal mark must be followed by one or two digits ("12," and "12.345" are typos).
    if has_decimal_mark && !(1..=2).contains(&fraction.len()) {
        return None;
    }
    let whole_digits: String = whole.chars().filter(char::is_ascii_digit).collect();
    if whole_digits.len() > 10 {
        return None;
    }
    let whole_value: i64 = if whole_digits.is_empty() { 0 } else { whole_digits.parse().ok()? };
    let fraction_value: i64 = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()? * 10,
        _ => fraction.parse().ok()?,
    };
    let cents = whole_value * 100 + fraction_value;
    (cents > 0 && whole_value <= 1_000_000_000).then_some(cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_sorted_orders_siblings_alphabetically() {
        let mut c = Categories { items: Vec::new(), next_id: 1 };
        let zeta = c.add("Zeta", Kind::Expense, None, 1);
        c.add("Äiti", Kind::Expense, None, 1);
        let alpha = c.add("alpha", Kind::Expense, None, 1);
        c.add("beta", Kind::Expense, Some(zeta), 1);
        c.add("Alku", Kind::Expense, Some(zeta), 1);
        c.add("child", Kind::Expense, Some(alpha), 1);
        let names: Vec<_> =
            c.flatten_sorted(Kind::Expense).iter().map(|(d, x)| format!("{d}{}", x.name)).collect();
        assert_eq!(names, ["0alpha", "1child", "0Zeta", "1Alku", "1beta", "0Äiti"]);
        // The plain order stays the creation order.
        assert_eq!(c.flatten(Kind::Expense)[0].1.name, "Zeta");
    }

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
        // English: '.' is the decimal mark.
        let en = |s: &str| parse_amount_cents(s, '.');
        assert_eq!(en("12"), Some(1200));
        assert_eq!(en("12.5"), Some(1250));
        assert_eq!(en("12.50"), Some(1250));
        assert_eq!(en("0.07"), Some(7));
        assert_eq!(en("19.99"), Some(1999));
        assert_eq!(en(" 1 234.50 "), Some(123_450));
        assert_eq!(en("1\u{a0}234.50"), Some(123_450));
        // "1,234" is a thousand two hundred thirty-four, never 1.23.
        assert_eq!(en("1,234"), Some(123_400));
        assert_eq!(en("1,234.50"), Some(123_450));
        assert_eq!(en("1,234,567.89"), Some(123_456_789));
        assert_eq!(en("12,5"), Some(1250)); // a lone comma that does not group is a decimal
        assert_eq!(en("1.234.50"), None);
        assert_eq!(en("1.234"), None); // three decimals: report instead of rounding
        assert_eq!(en("12."), None);
        assert_eq!(en("1.2.3"), None);
        // Finnish, Swedish, German, French, Spanish: ',' is the decimal mark.
        let fi = |s: &str| parse_amount_cents(s, ',');
        assert_eq!(fi("12,50"), Some(1250));
        assert_eq!(fi("12,5"), Some(1250));
        assert_eq!(fi(" 12,50 "), Some(1250));
        assert_eq!(fi("1 234,50"), Some(123_450));
        assert_eq!(fi("1\u{a0}234,50"), Some(123_450));
        assert_eq!(fi("1\u{202f}234,50"), Some(123_450));
        assert_eq!(fi("1.234,50"), Some(123_450));
        assert_eq!(fi("1.234"), Some(123_400)); // '.' groups here
        assert_eq!(fi("1.234.567"), Some(123_456_700));
        assert_eq!(fi("12.50"), Some(1250)); // an English-style decimal still works
        assert_eq!(fi("1,234.50"), Some(123_450));
        assert_eq!(fi("1,234"), None); // three decimals
        assert_eq!(fi("12,"), None);
        // Swiss apostrophes.
        assert_eq!(parse_amount_cents("1'234.50", '.'), Some(123_450));
        assert_eq!(parse_amount_cents("1’234,50", ','), Some(123_450));
        // Never valid.
        for bad in ["", "abc", "-5", "+5x", "0", "0,00", "NaN", "1e3", ",", ".", "99999999999"] {
            assert_eq!(en(bad), None, "{bad:?}");
            assert_eq!(fi(bad), None, "{bad:?}");
        }
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

    fn names(cats: &Categories) -> Vec<&str> {
        cats.items.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn unedited_starter_categories_follow_the_language() {
        use crate::i18n::Lang;
        let mut cats = Categories::defaults_for(Lang::En);
        assert!(cats.translate_defaults(Lang::Fi));
        assert_eq!(names(&cats), names(&Categories::defaults_for(Lang::Fi)));
        // The ids stay, so entries keep pointing at the same categories.
        assert_eq!(cats.items.iter().map(|c| c.id).collect::<Vec<_>>(), (1..=10).collect::<Vec<_>>());
        // Nothing left to do the second time, and any language can be reached from any other.
        assert!(!cats.translate_defaults(Lang::Fi));
        for from in Lang::ALL {
            for to in Lang::ALL {
                let mut c = Categories::defaults_for(from);
                c.translate_defaults(to);
                assert_eq!(c, Categories::defaults_for(to), "{from:?} to {to:?}");
            }
        }
        // The categories of an old data file that started out as the English defaults count too.
        let mut old = Categories::default();
        assert!(old.translate_defaults(Lang::Sv));
        assert_eq!(old.items[3].name, "Mat");
    }

    #[test]
    fn edited_starter_categories_keep_their_names() {
        use crate::i18n::Lang;
        let mut cats = Categories::defaults_for(Lang::En);
        let food = cats.items.iter().position(|c| c.name == "Food").unwrap();
        cats.items[food].name = "Groceries".to_string(); // renamed
        let housing = cats.items.iter().position(|c| c.name == "Housing").unwrap();
        cats.items[housing].color = 0x123456; // recoloured
        let transport = cats.items.iter().position(|c| c.name == "Transport").unwrap();
        cats.items[transport].parent = Some(housing as u64 + 1); // moved under another
        assert!(cats.translate_defaults(Lang::Fi));
        let fi = Categories::defaults_for(Lang::Fi);
        assert_eq!(cats.items[food].name, "Groceries");
        assert_eq!(cats.items[housing].name, "Housing");
        assert_eq!(cats.items[transport].name, "Transport");
        // The untouched ones were translated.
        assert_eq!(cats.items[0].name, fi.items[0].name);
        assert_eq!(cats.items[5].name, fi.items[5].name);
    }

    #[test]
    fn categories_made_by_the_user_and_name_clashes_are_left_alone() {
        use crate::i18n::Lang;
        let mut cats = Categories::defaults_for(Lang::En);
        let custom = cats.add("Pets", Kind::Expense, None, 0x654321);
        // The user already has an expense category called like the Finnish "Ruoka".
        cats.add("ruoka", Kind::Expense, None, 0x111111);
        assert!(cats.translate_defaults(Lang::Fi));
        assert_eq!(cats.get(custom).unwrap().name, "Pets");
        assert!(names(&cats).contains(&"Food"), "the clash keeps the old name: {:?}", names(&cats));
        assert!(names(&cats).contains(&"Palkka"), "others still move: {:?}", names(&cats));
        // A deleted starter category is not brought back.
        let mut cats = Categories::defaults_for(Lang::En);
        cats.items.retain(|c| c.name != "Leisure");
        cats.translate_defaults(Lang::De);
        assert_eq!(cats.items.len(), 9);
    }

    #[test]
    fn a_copied_month_has_the_same_entries_with_fresh_numbers() {
        let cats = Categories::default();
        let source = vec![
            Entry { id: 7, category_id: 4, name: "Lunch".into(), amount_cents: 990 },
            Entry { id: 9, category_id: 999, name: "Orphan".into(), amount_cents: 1 },
            Entry { id: 12, category_id: 3, name: String::new(), amount_cents: 80000 },
        ];
        let copy = copied_entries(&source, &cats);
        assert_eq!(copy.len(), 2, "an entry of a category that is gone is not copied");
        assert_eq!(copy[0], Entry { id: 1, category_id: 4, name: "Lunch".into(), amount_cents: 990 });
        assert_eq!(copy[1], Entry { id: 2, category_id: 3, name: String::new(), amount_cents: 80000 });
        assert!(copied_entries(&[], &cats).is_empty());
    }

    #[test]
    fn a_starter_category_with_a_former_default_name_counts_as_unedited() {
        use crate::i18n::Lang;
        // Finnish used to call Transport "Liikenne". Data made then is still a starter category.
        let mut cats = Categories::defaults_for(Lang::Fi);
        let transport = cats.items.iter().position(|c| c.id == 5).unwrap();
        assert_eq!(cats.items[transport].name, "Liikkuminen");
        cats.items[transport].name = "Liikenne".to_string();
        assert!(cats.translate_defaults(Lang::Fi), "it moves to the current name");
        assert_eq!(cats.items[transport].name, "Liikkuminen");

        // And it follows the language like the others.
        cats.items[transport].name = "Liikenne".to_string();
        assert!(cats.translate_defaults(Lang::En));
        assert_eq!(cats.items[transport].name, "Transport");

        // A name that was never a default is an edit.
        cats.translate_defaults(Lang::Fi); // everything is Finnish again
        cats.items[transport].name = "Liikennekulut".to_string();
        assert!(!cats.translate_defaults(Lang::Fi));
        assert_eq!(cats.items[transport].name, "Liikennekulut");
    }

    #[test]
    fn the_starter_categories_are_the_ones_of_the_screenshots() {
        use crate::i18n::Lang;
        let cats = Categories::defaults_for(Lang::En);
        let line = |c: &Category| {
            let parent = c.parent.and_then(|p| cats.get(p)).map_or(String::new(), |p| format!("{} / ", p.name));
            format!("{}{} ({:?})", parent, c.name, c.kind)
        };
        assert_eq!(
            cats.items.iter().map(line).collect::<Vec<_>>(),
            vec![
                "Salary (Income)",
                "Other income (Income)",
                "Housing (Expense)",
                "Food (Expense)",
                "Transport (Expense)",
                "Leisure (Expense)",
                "Housing / Rent (Expense)",
                "Food / Groceries (Expense)",
                "Food / Eating out (Expense)",
                "Health (Expense)",
            ]
        );
        // Every language has all of them, with different names for each, and the next one made gets a fresh number.
        for lang in Lang::ALL {
            let c = Categories::defaults_for(lang);
            assert_eq!(c.items.len(), 10, "{lang:?}");
            let mut names: Vec<&str> = c.items.iter().map(|c| c.name.as_str()).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), 10, "{lang:?} has two categories with the same name");
        }
        let mut c = Categories::defaults_for(Lang::Fi);
        assert_eq!(c.add("Lemmikit", Kind::Expense, None, 0x1), 11);
    }

    #[test]
    fn sub_categories_of_the_starter_set_move_with_their_parent_only_while_unedited() {
        use crate::i18n::Lang;
        let mut cats = Categories::defaults_for(Lang::En);
        let rent = cats.items.iter().position(|c| c.name == "Rent").unwrap();
        cats.items[rent].parent = None; // the user moved it to the top level
        let groceries = cats.items.iter().position(|c| c.name == "Groceries").unwrap();
        assert!(cats.translate_defaults(Lang::Fi));
        assert_eq!(cats.items[rent].name, "Rent", "a moved one counts as edited");
        assert_eq!(cats.items[groceries].name, "Ruokakauppa");
    }
}
