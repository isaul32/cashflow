//! Pure conversion from categories + entries into Sankey nodes and links.
//! Has no UI dependencies so it can be unit tested.
//!
//! Flow layout:
//! income categories (leaf -> parent) -> "Total income" -> expense categories
//! (parent -> leaf), plus a "Savings" node when income exceeds expenses or a
//! "Deficit" node on the income side when expenses exceed income.

use std::collections::{HashMap, HashSet};

use crate::model::{Categories, Category, CategoryId, Entry, Kind};
use crate::settings::Labels;

pub const SAVINGS_COLOR: u32 = 0x009688;
pub const DEFICIT_COLOR: u32 = 0xE53935;
pub const TOTAL_COLOR: u32 = 0x607D8B;

#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    pub label: String,
    pub color: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowLink {
    pub source: usize,
    pub target: usize,
    /// Value in whole currency units.
    pub value: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyData {
    pub nodes: Vec<FlowNode>,
    pub links: Vec<FlowLink>,
    pub income_cents: i64,
    pub expense_cents: i64,
}

impl SankeyData {
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    pub fn balance_cents(&self) -> i64 {
        self.income_cents - self.expense_cents
    }

    /// Text for pasting into the SankeyMATIC "Inputs" box:
    /// one `Source [amount] Target` line per flow, then `:Node #color` lines.
    /// Node names are made unique and stripped of characters SankeyMATIC treats
    /// as syntax.
    pub fn to_sankeymatic(&self, title: &str) -> String {
        let names = unique_names(&self.nodes);
        let mut out = format!("// {}\n", title.replace(['\n', '\r'], " "));
        for l in &self.links {
            out.push_str(&format!(
                "{} [{}] {}\n",
                names[l.source],
                format_amount(l.value),
                names[l.target]
            ));
        }
        out.push('\n');
        for (node, name) in self.nodes.iter().zip(&names) {
            out.push_str(&format!(":{name} #{:06X}\n", node.color & 0xFFFFFF));
        }
        out
    }
}

/// Up to two decimals, no trailing zeros: 3000.0 -> "3000", 12.5 -> "12.5".
fn format_amount(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn clean_name(raw: &str) -> String {
    let replaced: String = raw
        .chars()
        .map(|c| if matches!(c, '[' | ']' | '#' | ':' | '\n' | '\r') { ' ' } else { c })
        .collect();
    let collapsed = replaced.replace("//", "/");
    let name = collapsed.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() { "Unnamed".to_string() } else { name }
}

/// Sanitized node names, made unique (case-insensitively) with " (2)", " (3)"...
fn unique_names(nodes: &[FlowNode]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    nodes
        .iter()
        .map(|n| {
            let base = clean_name(&n.label);
            let mut name = base.clone();
            let mut i = 2;
            while !seen.insert(name.to_lowercase()) {
                name = format!("{base} ({i})");
                i += 1;
            }
            name
        })
        .collect()
}

struct Builder<'a> {
    cats: &'a Categories,
    /// Direct entry sum per category.
    direct: HashMap<CategoryId, i64>,
    /// Categories already placed in the graph (guards against cycles).
    seen: HashSet<CategoryId>,
    data: SankeyData,
}

/// Builds the graph with the default node labels.
#[cfg(test)]
pub fn build(cats: &Categories, entries: &[Entry]) -> SankeyData {
    build_with(cats, entries, &Labels::default())
}

pub fn build_with(cats: &Categories, entries: &[Entry], labels: &Labels) -> SankeyData {
    let mut direct: HashMap<CategoryId, i64> = HashMap::new();
    for e in entries.iter().filter(|e| e.amount_cents > 0) {
        if cats.get(e.category_id).is_some() {
            *direct.entry(e.category_id).or_default() += e.amount_cents;
        }
    }

    let mut b = Builder {
        cats,
        direct,
        seen: HashSet::new(),
        data: SankeyData::default(),
    };

    let roots = |kind: Kind| -> Vec<&Category> {
        cats.items
            .iter()
            .filter(|c| c.kind == kind && c.parent.is_none())
            .collect()
    };
    let income_roots = roots(Kind::Income);
    let expense_roots = roots(Kind::Expense);

    let income: i64 = income_roots.iter().map(|c| b.subtree_total(c.id)).sum();
    let expense: i64 = expense_roots.iter().map(|c| b.subtree_total(c.id)).sum();
    b.data.income_cents = income;
    b.data.expense_cents = expense;
    if income == 0 && expense == 0 {
        return b.data;
    }

    let total = b.node(labels.total.clone(), TOTAL_COLOR);

    for c in income_roots {
        b.place(c, total, Kind::Income);
    }
    if expense > income {
        let deficit = b.node(labels.deficit.clone(), DEFICIT_COLOR);
        b.link(deficit, total, expense - income);
    }
    for c in expense_roots {
        b.place(c, total, Kind::Expense);
    }
    if income > expense {
        let savings = b.node(labels.savings.clone(), SAVINGS_COLOR);
        b.link(total, savings, income - expense);
    }
    b.data
}

impl Builder<'_> {
    fn node(&mut self, label: impl Into<String>, color: u32) -> usize {
        self.data.nodes.push(FlowNode {
            label: label.into(),
            color,
        });
        self.data.nodes.len() - 1
    }

    fn link(&mut self, source: usize, target: usize, cents: i64) {
        self.data.links.push(FlowLink {
            source,
            target,
            value: cents as f64 / 100.0,
        });
    }

    /// Direct entries plus all descendants. Cycle-safe via a visited set.
    fn subtree_total(&self, id: CategoryId) -> i64 {
        fn go(b: &Builder, id: CategoryId, visited: &mut HashSet<CategoryId>) -> i64 {
            if !visited.insert(id) {
                return 0;
            }
            let own = b.direct.get(&id).copied().unwrap_or(0);
            own + b.cats.children(id).map(|c| go(b, c.id, visited)).sum::<i64>()
        }
        go(self, id, &mut HashSet::new())
    }

    /// Adds `cat` and its descendants. `anchor` is the node on the other side
    /// of the flow (the total for top-level categories, the parent otherwise).
    fn place(&mut self, cat: &Category, anchor: usize, kind: Kind) {
        if !self.seen.insert(cat.id) {
            return;
        }
        let value = self.subtree_total(cat.id);
        if value == 0 {
            return;
        }
        let node = self.node(cat.name.clone(), cat.color);
        match kind {
            Kind::Income => self.link(node, anchor, value),
            Kind::Expense => self.link(anchor, node, value),
        }
        let cats = self.cats;
        for child in cats.children(cat.id) {
            self.place(child, node, kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, category_id: CategoryId, euros: i64) -> Entry {
        Entry {
            id,
            category_id,
            name: String::new(),
            amount_cents: euros * 100,
        }
    }

    fn id_of(c: &Categories, name: &str) -> CategoryId {
        c.items.iter().find(|c| c.name == name).unwrap().id
    }

    fn node_idx(d: &SankeyData, label: &str) -> usize {
        d.nodes.iter().position(|n| n.label == label).unwrap()
    }

    fn link_value(d: &SankeyData, from: &str, to: &str) -> Option<f64> {
        let (s, t) = (node_idx(d, from), node_idx(d, to));
        d.links
            .iter()
            .find(|l| l.source == s && l.target == t)
            .map(|l| l.value)
    }

    #[test]
    fn exports_sankeymatic_text() {
        let c = Categories::default();
        let e = [
            entry(1, id_of(&c, "Salary"), 3000),
            entry(2, id_of(&c, "Housing"), 1000),
        ];
        let text = build(&c, &e).to_sankeymatic("November 2026");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "// November 2026");
        assert!(lines.contains(&"Salary [3000] Total income"));
        assert!(lines.contains(&"Total income [1000] Housing"));
        assert!(lines.contains(&"Total income [2000] Savings"));
        assert!(lines.contains(&":Salary #4CAF50"));
        assert!(lines.contains(&":Savings #009688"));
    }

    #[test]
    fn export_formats_cents_and_cleans_names() {
        let mut c = Categories::default();
        // Same name as an income category, plus syntax characters.
        let salary = c.add("salary", Kind::Expense, None, 0x112233);
        let odd = c.add("[Fun] #1: a//b", Kind::Expense, None, 0x445566);
        let e = [
            entry(1, id_of(&c, "Salary"), 100),
            Entry { id: 2, category_id: salary, name: String::new(), amount_cents: 1250 },
            Entry { id: 3, category_id: odd, name: String::new(), amount_cents: 5 },
        ];
        let text = build(&c, &e).to_sankeymatic("t");
        assert!(text.contains("Total income [12.5] salary (2)"), "{text}");
        assert!(text.contains("Total income [0.05] Fun 1 a/b"), "{text}");
        assert!(text.contains(":salary (2) #112233"), "{text}");
        assert!(text.contains(":Fun 1 a/b #445566"), "{text}");
    }

    #[test]
    fn empty_month_has_no_graph() {
        let d = build(&Categories::default(), &[]);
        assert!(d.is_empty());
        assert!(d.nodes.is_empty());
    }

    #[test]
    fn surplus_goes_to_savings() {
        let c = Categories::default();
        let e = [
            entry(1, id_of(&c, "Salary"), 3000),
            entry(2, id_of(&c, "Housing"), 1000),
            entry(3, id_of(&c, "Food"), 500),
        ];
        let d = build(&c, &e);
        assert_eq!(d.balance_cents(), 150_000);
        assert_eq!(link_value(&d, "Salary", "Total income"), Some(3000.0));
        assert_eq!(link_value(&d, "Total income", "Housing"), Some(1000.0));
        assert_eq!(link_value(&d, "Total income", "Savings"), Some(1500.0));
        // Categories without entries are omitted.
        assert!(d.nodes.iter().all(|n| n.label != "Leisure"));
    }

    #[test]
    fn custom_labels_name_the_automatic_nodes() {
        let c = Categories::default();
        let labels = Labels {
            total: "Budget".into(),
            savings: "Left over".into(),
            deficit: "Shortfall".into(),
        };
        let surplus = build_with(
            &c,
            &[entry(1, id_of(&c, "Salary"), 100), entry(2, id_of(&c, "Food"), 40)],
            &labels,
        );
        assert_eq!(link_value(&surplus, "Salary", "Budget"), Some(100.0));
        assert_eq!(link_value(&surplus, "Budget", "Left over"), Some(60.0));
        let short = build_with(
            &c,
            &[entry(1, id_of(&c, "Salary"), 10), entry(2, id_of(&c, "Food"), 40)],
            &labels,
        );
        assert_eq!(link_value(&short, "Shortfall", "Budget"), Some(30.0));
    }

    #[test]
    fn deficit_is_added_on_income_side() {
        let c = Categories::default();
        let e = [
            entry(1, id_of(&c, "Salary"), 1000),
            entry(2, id_of(&c, "Housing"), 1500),
        ];
        let d = build(&c, &e);
        assert_eq!(d.balance_cents(), -50_000);
        assert_eq!(link_value(&d, "Deficit", "Total income"), Some(500.0));
        assert!(d.nodes.iter().all(|n| n.label != "Savings"));
    }

    #[test]
    fn subcategories_add_a_level_and_roll_up() {
        let mut c = Categories::default();
        let food = id_of(&c, "Food");
        let groceries = c.add("Groceries", Kind::Expense, Some(food), 0);
        let eating_out = c.add("Eating out", Kind::Expense, Some(food), 0);
        let e = [
            entry(1, id_of(&c, "Salary"), 2000),
            entry(2, food, 50), // direct entry on the parent
            entry(3, groceries, 300),
            entry(4, eating_out, 100),
        ];
        let d = build(&c, &e);
        assert_eq!(link_value(&d, "Total income", "Food"), Some(450.0));
        assert_eq!(link_value(&d, "Food", "Groceries"), Some(300.0));
        assert_eq!(link_value(&d, "Food", "Eating out"), Some(100.0));
        assert_eq!(d.expense_cents, 45_000);
    }

    #[test]
    fn income_subcategories_flow_towards_total() {
        let mut c = Categories::default();
        let salary = id_of(&c, "Salary");
        let bonus = c.add("Bonus", Kind::Income, Some(salary), 0);
        let e = [entry(1, salary, 2000), entry(2, bonus, 500)];
        let d = build(&c, &e);
        assert_eq!(link_value(&d, "Bonus", "Salary"), Some(500.0));
        assert_eq!(link_value(&d, "Salary", "Total income"), Some(2500.0));
    }

    #[test]
    fn ignores_unknown_categories_and_non_positive_amounts() {
        let c = Categories::default();
        let e = [
            entry(1, id_of(&c, "Salary"), 1000),
            entry(2, 9999, 500),
            entry(3, id_of(&c, "Food"), -20),
            entry(4, id_of(&c, "Food"), 0),
        ];
        let d = build(&c, &e);
        assert_eq!(d.income_cents, 100_000);
        assert_eq!(d.expense_cents, 0);
    }

    #[test]
    fn category_cycle_does_not_hang() {
        let mut c = Categories::default();
        let a = c.add("A", Kind::Expense, None, 0);
        let b = c.add("B", Kind::Expense, Some(a), 0);
        c.items.iter_mut().find(|x| x.id == a).unwrap().parent = Some(b);
        let e = [entry(1, a, 10), entry(2, b, 10)];
        let _ = build(&c, &e);
    }

    #[test]
    fn removing_category_moves_entries_and_children() {
        let mut c = Categories::default();
        let food = id_of(&c, "Food");
        let housing = id_of(&c, "Housing");
        let groceries = c.add("Groceries", Kind::Expense, Some(food), 0);
        let mut e = vec![entry(1, food, 100)];
        c.remove(food, housing, &mut e);
        assert_eq!(e[0].category_id, housing);
        assert_eq!(c.get(groceries).unwrap().parent, None);
        assert!(c.get(food).is_none());
    }
}
