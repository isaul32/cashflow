//! Import of SankeyMATIC text (the format written by `SankeyData::to_sankeymatic`).
//!
//! SankeyMATIC text carries no income/expense information, so it is inferred
//! from the graph shape: one middle node (the "hub", normally "Total income")
//! has income flowing in from the left and expenses flowing out to the right.
//! Nodes upstream of the hub become income categories, nodes downstream become
//! expense categories, and intermediate nodes become parent categories.
//!
//! "Savings" (a sink of the hub) and "Deficit" (a source of the hub) are
//! derived values in this app, so they are not imported.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use crate::i18n::{Lang, Msg};
use crate::model::{Categories, CategoryId, Entry, Kind, parse_amount_cents};
use crate::settings::Labels;

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedCategory {
    pub name: String,
    pub kind: Kind,
    /// Name of the parent category (same kind), if any.
    pub parent: Option<String>,
    pub color: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlannedEntry {
    pub category: String,
    pub kind: Kind,
    pub amount_cents: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportPlan {
    /// Parents come before their children.
    pub categories: Vec<PlannedCategory>,
    pub entries: Vec<PlannedEntry>,
    pub warnings: Vec<String>,
    pub income_cents: i64,
    pub expense_cents: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ApplyStats {
    pub categories_added: usize,
    pub entries_added: usize,
}

fn parse_hex(s: &str) -> Option<u32> {
    let s = s.trim().trim_start_matches('#');
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match s.len() {
        6 => u32::from_str_radix(s, 16).ok(),
        3 => {
            let v = u32::from_str_radix(s, 16).ok()?;
            let (r, g, b) = ((v >> 8) & 0xF, (v >> 4) & 0xF, v & 0xF);
            Some(((r * 17) << 16) | ((g * 17) << 8) | (b * 17))
        }
        _ => None,
    }
}

/// Splits a trailing `#hex` color off `text`.
fn split_color(text: &str) -> (&str, Option<u32>) {
    if let Some(i) = text.rfind('#')
        && let Some(c) = parse_hex(&text[i + 1..])
    {
        return (text[..i].trim(), Some(c));
    }
    (text.trim(), None)
}

struct Graph {
    names: Vec<String>,
    /// (target, cents) per node
    out: Vec<Vec<(usize, i64)>>,
    /// (source, cents) per node
    inn: Vec<Vec<(usize, i64)>>,
}

/// True if `name` is the user's custom label or the built-in default.
fn is_name(name: &str, custom: &str, default: Msg) -> bool {
    name.eq_ignore_ascii_case(custom)
        || Lang::ALL.iter().any(|&l| name.eq_ignore_ascii_case(default.text(l)))
}

/// Parses with the default labels.
#[cfg(test)]
pub fn parse(text: &str) -> Result<ImportPlan, String> {
    parse_with(text, &Labels::default(), Lang::En)
}

/// `labels` are the names the user gave the automatic nodes; the built-in
/// names are always recognized as well.
pub fn parse_with(text: &str, labels: &Labels, lang: Lang) -> Result<ImportPlan, String> {
    let text = text.trim_start_matches('\u{feff}');
    let mut warnings: Vec<String> = Vec::new();
    let mut colors: HashMap<String, u32> = HashMap::new();
    let mut idx: HashMap<String, usize> = HashMap::new();
    let mut g = Graph { names: Vec::new(), out: Vec::new(), inn: Vec::new() };
    let mut flow_at: HashMap<(usize, usize), i64> = HashMap::new();
    let mut flow_order: Vec<(usize, usize)> = Vec::new();

    let mut node = |g: &mut Graph, name: &str| -> usize {
        *idx.entry(name.to_string()).or_insert_with(|| {
            g.names.push(name.to_string());
            g.out.push(Vec::new());
            g.inn.push(Vec::new());
            g.names.len() - 1
        })
    };

    for (n, raw) in text.lines().enumerate() {
        let line = raw.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(':') {
            let (name, color) = split_color(rest);
            if let (false, Some(c)) = (name.is_empty(), color) {
                colors.insert(name.to_string(), c);
            }
            continue;
        }
        let (Some(lb), Some(rb)) = (line.find('['), line.find(']')) else {
            continue; // a SankeyMATIC setting line, not a flow
        };
        if lb > rb {
            continue;
        }
        let source = line[..lb].trim();
        let (target, _) = split_color(&line[rb + 1..]);
        if source.is_empty() || target.is_empty() || source == target {
            warnings.push(Msg::ImpLineSkippedNodes.format(lang, &[("line", &(n + 1).to_string())]));
            continue;
        }
        let Some(cents) = parse_amount_cents(&line[lb + 1..rb]) else {
            warnings.push(Msg::ImpLineSkippedAmount.format(lang, &[("line", &(n + 1).to_string())]));
            continue;
        };
        let (s, t) = (node(&mut g, source), node(&mut g, target));
        match flow_at.get_mut(&(s, t)) {
            Some(total) => *total += cents,
            None => {
                flow_at.insert((s, t), cents);
                flow_order.push((s, t));
            }
        }
    }
    for (s, t) in flow_order {
        let cents = flow_at[&(s, t)];
        g.out[s].push((t, cents));
        g.inn[t].push((s, cents));
    }
    if g.names.is_empty() {
        return Err(Msg::ImpNoFlows.text(lang).to_string());
    }

    let n = g.names.len();
    let in_sum: Vec<i64> = g.inn.iter().map(|v| v.iter().map(|x| x.1).sum()).collect();
    let out_sum: Vec<i64> = g.out.iter().map(|v| v.iter().map(|x| x.1).sum()).collect();

    let hub = g
        .names
        .iter()
        .position(|s| is_name(s, &labels.total, Msg::DefLabelTotal))
        .or_else(|| {
            (0..n)
                .filter(|&i| in_sum[i] > 0 && out_sum[i] > 0)
                .max_by_key(|&i| (in_sum[i].min(out_sum[i]), Reverse(i)))
        })
        .ok_or_else(|| Msg::ImpNoHub.text(lang).to_string())?;

    let reach = |start: usize, next: &dyn Fn(usize) -> Vec<usize>| -> HashSet<usize> {
        let mut seen = HashSet::new();
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            for j in next(i) {
                if j != start && seen.insert(j) {
                    stack.push(j);
                }
            }
        }
        seen
    };
    let mut income: HashSet<usize> = reach(hub, &|i| g.inn[i].iter().map(|x| x.0).collect());
    let mut expense: HashSet<usize> = reach(hub, &|i| g.out[i].iter().map(|x| x.0).collect());

    // Nodes on both sides of the hub form a loop and cannot be classified.
    let both: Vec<usize> = income.intersection(&expense).copied().collect();
    for i in both {
        income.remove(&i);
        expense.remove(&i);
        warnings.push(Msg::ImpBothSides.format(lang, &[("name", &g.names[i])]));
    }
    // Derived values in this app: skipped on purpose, so not worth a warning.
    let mut derived = 0;
    for i in expense.clone() {
        if is_name(&g.names[i], &labels.savings, Msg::DefLabelSavings) && g.out[i].is_empty() {
            expense.remove(&i);
            derived += 1;
        }
    }
    for i in income.clone() {
        if is_name(&g.names[i], &labels.deficit, Msg::DefLabelDeficit) && g.inn[i].is_empty() {
            income.remove(&i);
            derived += 1;
        }
    }
    let connected = income.len() + expense.len() + 1 + derived;
    let unconnected = n.saturating_sub(connected);
    if unconnected > 0 {
        warnings.push(Msg::ImpUnconnected.format(
            lang,
            &[("n", &unconnected.to_string()), ("hub", &g.names[hub])],
        ));
    }

    // Parent of each node and the flow amount linking it to that parent.
    let mut chosen: HashMap<usize, (usize, i64)> = HashMap::new();
    let mut sorted: Vec<usize> = income.iter().chain(expense.iter()).copied().collect();
    sorted.sort_unstable();
    for &i in &sorted {
        let is_income = income.contains(&i);
        let ok = |j: usize| j == hub || (if is_income { &income } else { &expense }).contains(&j);
        let cands: Vec<(usize, i64)> = if is_income { &g.out[i] } else { &g.inn[i] }
            .iter()
            .copied()
            .filter(|&(j, _)| ok(j))
            .collect();
        let Some(best) = cands.iter().copied().reduce(|a, b| if b.1 > a.1 { b } else { a }) else {
            continue;
        };
        if cands.len() > 1 {
            warnings.push(Msg::ImpSeveral.format(lang, &[("name", &g.names[i])]));
        }
        chosen.insert(i, best);
    }

    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut child_sum: HashMap<usize, i64> = HashMap::new();
    for &i in &sorted {
        if let Some(&(p, cents)) = chosen.get(&i) {
            children.entry(p).or_default().push(i);
            *child_sum.entry(p).or_default() += cents;
        }
    }

    let kind_of = |i: usize| if income.contains(&i) { Kind::Income } else { Kind::Expense };
    let mut plan = ImportPlan { warnings, ..Default::default() };
    let mut emitted: HashSet<usize> = HashSet::new();

    fn emit(
        i: usize,
        parent: Option<usize>,
        g: &Graph,
        colors: &HashMap<String, u32>,
        children: &HashMap<usize, Vec<usize>>,
        kind_of: &dyn Fn(usize) -> Kind,
        emitted: &mut HashSet<usize>,
        plan: &mut ImportPlan,
    ) {
        if !emitted.insert(i) {
            return;
        }
        plan.categories.push(PlannedCategory {
            name: g.names[i].clone(),
            kind: kind_of(i),
            parent: parent.map(|p| g.names[p].clone()),
            color: colors.get(&g.names[i]).copied(),
        });
        for &c in children.get(&i).into_iter().flatten() {
            emit(c, Some(i), g, colors, children, kind_of, emitted, plan);
        }
    }
    for kind in [Kind::Income, Kind::Expense] {
        for &i in sorted.iter().filter(|&&i| kind_of(i) == kind) {
            if matches!(chosen.get(&i), Some(&(p, _)) if p == hub) {
                emit(i, None, &g, &colors, &children, &kind_of, &mut emitted, &mut plan);
            }
        }
    }
    for &i in &sorted {
        if !emitted.contains(&i) {
            plan.warnings.push(Msg::ImpUnplaced.format(lang, &[("name", &g.names[i])]));
        }
    }

    // Entries: what a node carries beyond the sum of its children.
    for &i in sorted.iter().filter(|i| emitted.contains(i)) {
        let Some(&(_, total)) = chosen.get(&i) else { continue };
        let direct = total - child_sum.get(&i).copied().unwrap_or(0);
        if direct < 0 {
            plan.warnings.push(Msg::ImpChildrenExceed.format(lang, &[("name", &g.names[i])]));
        } else if direct > 0 {
            let kind = kind_of(i);
            plan.entries.push(PlannedEntry { category: g.names[i].clone(), kind, amount_cents: direct });
            match kind {
                Kind::Income => plan.income_cents += direct,
                Kind::Expense => plan.expense_cents += direct,
            }
        }
    }

    if plan.entries.is_empty() {
        return Err(Msg::ImpNothing.text(lang).to_string());
    }
    Ok(plan)
}

/// Applies a plan: reuses categories with the same kind and name (ignoring
/// case), creates the rest, and adds the entries. With `replace`, the existing
/// entries are removed first.
pub fn apply(
    plan: &ImportPlan,
    cats: &mut Categories,
    entries: &mut Vec<Entry>,
    next_entry_id: &mut u64,
    replace: bool,
    palette: &[u32],
) -> ApplyStats {
    let key = |kind: Kind, name: &str| (kind == Kind::Income, name.to_lowercase());
    let mut ids: HashMap<(bool, String), CategoryId> =
        cats.items.iter().map(|c| (key(c.kind, &c.name), c.id)).collect();
    let mut stats = ApplyStats::default();

    for pc in &plan.categories {
        let k = key(pc.kind, &pc.name);
        if ids.contains_key(&k) {
            continue;
        }
        let parent = pc.parent.as_ref().and_then(|p| ids.get(&key(pc.kind, p)).copied());
        let color = pc
            .color
            .unwrap_or_else(|| palette[cats.items.len() % palette.len().max(1)]);
        let id = cats.add(pc.name.clone(), pc.kind, parent, color);
        ids.insert(k, id);
        stats.categories_added += 1;
    }

    if replace {
        entries.clear();
    }
    for pe in &plan.entries {
        if let Some(&category_id) = ids.get(&key(pe.kind, &pe.category)) {
            entries.push(Entry {
                id: *next_entry_id,
                category_id,
                name: String::new(),
                amount_cents: pe.amount_cents,
            });
            *next_entry_id += 1;
            stats.entries_added += 1;
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sankey;

    const PALETTE: [u32; 2] = [0x111111, 0x222222];

    fn links(d: &sankey::SankeyData) -> Vec<(String, String, i64)> {
        let mut v: Vec<_> = d
            .links
            .iter()
            .map(|l| {
                (
                    d.nodes[l.source].label.clone(),
                    d.nodes[l.target].label.clone(),
                    (l.value * 100.0).round() as i64,
                )
            })
            .collect();
        v.sort();
        v
    }

    fn entry(id: u64, category_id: CategoryId, euros: i64) -> Entry {
        Entry { id, category_id, name: String::new(), amount_cents: euros * 100 }
    }

    #[test]
    fn export_then_import_round_trips() {
        let mut cats = Categories::default();
        let find = |c: &Categories, n: &str| c.items.iter().find(|x| x.name == n).unwrap().id;
        let food = find(&cats, "Food");
        let groceries = cats.add("Groceries", Kind::Expense, Some(food), 0xFFB74D);
        let salary = find(&cats, "Salary");
        let bonus = cats.add("Bonus", Kind::Income, Some(salary), 0x8BC34A);
        let entries = vec![
            entry(1, salary, 3000),
            entry(2, bonus, 500),
            entry(3, find(&cats, "Housing"), 1200),
            entry(4, food, 50),
            entry(5, groceries, 400),
        ];
        let original = sankey::build(&cats, &entries);
        let text = original.to_sankeymatic("test");

        let plan = parse(&text).unwrap();
        assert_eq!(plan.income_cents, original.income_cents);
        assert_eq!(plan.expense_cents, original.expense_cents);
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);

        let mut fresh = Categories::default();
        let mut imported = Vec::new();
        let mut next = 1;
        let stats = apply(&plan, &mut fresh, &mut imported, &mut next, true, &PALETTE);
        assert_eq!(stats.categories_added, 2); // Groceries, Bonus
        assert_eq!(links(&sankey::build(&fresh, &imported)), links(&original));
    }

    #[test]
    fn imports_plain_sankeymatic_budget() {
        let text = "\
// A typical SankeyMATIC budget
size w 800
Wages [2500] Budget
Freelance [500.50] Budget #1a1
Budget [900] Rent
Budget [600] Food
Budget [300] Food #ff0000
Budget [1200.5] Savings

:Wages #4CAF50
:Rent #3F51B5
";
        let plan = parse(text).unwrap();
        // "Budget" is picked as the hub; Savings is derived and skipped.
        assert_eq!(plan.income_cents, 300_050);
        assert_eq!(plan.expense_cents, 180_000);
        let rent = plan.categories.iter().find(|c| c.name == "Rent").unwrap();
        assert_eq!((rent.kind, rent.parent.clone(), rent.color), (Kind::Expense, None, Some(0x3F51B5)));
        // The two Budget -> Food lines are summed.
        let food = plan.entries.iter().find(|e| e.category == "Food").unwrap();
        assert_eq!(food.amount_cents, 90_000);
        assert!(plan.categories.iter().all(|c| c.name != "Savings"));
    }

    #[test]
    fn custom_labels_round_trip() {
        let cats = Categories::default();
        let find = |n: &str| cats.items.iter().find(|x| x.name == n).unwrap().id;
        let entries = vec![entry(1, find("Salary"), 100), entry(2, find("Food"), 30)];
        let labels = Labels {
            total: "Budget".into(),
            savings: "Left over".into(),
            deficit: "Shortfall".into(),
        };
        let original = sankey::build_with(&cats, &entries, &labels);
        let text = original.to_sankeymatic("t");
        assert!(text.contains("Budget [30] Food"), "{text}");

        let plan = parse_with(&text, &labels, Lang::En).unwrap();
        assert_eq!((plan.income_cents, plan.expense_cents), (10_000, 3_000));
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
        assert!(plan.categories.iter().all(|c| c.name != "Left over"));
    }

    #[test]
    fn needs_a_middle_node() {
        assert!(parse("A [10] B").is_err());
        assert!(parse("just some text").is_err());
        assert!(parse("").is_err());
    }

    #[test]
    fn warns_about_unreadable_lines() {
        let plan = parse("Pay [1000] Total income\nTotal income [abc] Rent\nTotal income [400] Rent\n").unwrap();
        assert_eq!(plan.expense_cents, 40_000);
        assert_eq!(plan.warnings.len(), 1);
        assert!(plan.warnings[0].contains("Line 2"));
    }

    #[test]
    fn apply_reuses_existing_categories_and_can_append() {
        let plan = parse("Salary [100] Total income\nTotal income [40] housing\n").unwrap();
        let mut cats = Categories::default();
        let before = cats.items.len();
        let mut entries = vec![entry(1, 1, 5)];
        let mut next = 2;
        let stats = apply(&plan, &mut cats, &mut entries, &mut next, false, &PALETTE);
        // "Salary" and "housing" match existing categories case-insensitively.
        assert_eq!(stats.categories_added, 0);
        assert_eq!(cats.items.len(), before);
        assert_eq!(entries.len(), 3); // appended to the existing one
        let mut replaced = vec![entry(1, 1, 5)];
        apply(&plan, &mut cats, &mut replaced, &mut 2, true, &PALETTE);
        assert_eq!(replaced.len(), 2);
    }

    #[test]
    fn short_hex_colors_expand() {
        assert_eq!(parse_hex("#1a1"), Some(0x11AA11));
        assert_eq!(parse_hex("4CAF50"), Some(0x4CAF50));
        assert_eq!(parse_hex("xyz"), None);
        assert_eq!(parse_hex("12345"), None);
    }
}
