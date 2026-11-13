//! Yearly aggregation: sums all months of a year into one Sankey graph and
//! per-month totals.

use crate::model::{Categories, Entry, Kind};
use crate::sankey::{self, SankeyData};
use crate::settings::Labels;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MonthTotals {
    pub income_cents: i64,
    pub expense_cents: i64,
    /// Number of entries that count towards the totals.
    pub entries: usize,
}

impl MonthTotals {
    pub fn balance_cents(&self) -> i64 {
        self.income_cents - self.expense_cents
    }
}

#[derive(Clone, Debug, Default)]
pub struct YearSummary {
    pub year: i32,
    /// All entries of the year in one graph.
    pub data: SankeyData,
    /// January is index 0.
    pub months: [MonthTotals; 12],
}

/// `months[i]` holds the entries of month `i + 1`; missing months count as empty.
pub fn summarize(
    year: i32,
    cats: &Categories,
    months: &[Vec<Entry>],
    labels: &Labels,
) -> YearSummary {
    let mut totals = [MonthTotals::default(); 12];
    for (i, entries) in months.iter().take(12).enumerate() {
        for e in entries.iter().filter(|e| e.amount_cents > 0) {
            let Some(cat) = cats.get(e.category_id) else { continue };
            match cat.kind {
                Kind::Income => totals[i].income_cents += e.amount_cents,
                Kind::Expense => totals[i].expense_cents += e.amount_cents,
            }
            totals[i].entries += 1;
        }
    }
    let all: Vec<Entry> = months.iter().take(12).flatten().cloned().collect();
    YearSummary {
        year,
        data: sankey::build_with(cats, &all, labels),
        months: totals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, category_id: u64, euros: i64) -> Entry {
        Entry { id, category_id, name: String::new(), amount_cents: euros * 100 }
    }

    fn id_of(c: &Categories, name: &str) -> u64 {
        c.items.iter().find(|c| c.name == name).unwrap().id
    }

    #[test]
    fn sums_months_into_one_graph() {
        let c = Categories::default();
        let (salary, food, rent) = (id_of(&c, "Salary"), id_of(&c, "Food"), id_of(&c, "Housing"));
        let mut months = vec![Vec::new(); 12];
        months[0] = vec![entry(1, salary, 3000), entry(2, food, 400)];
        months[1] = vec![entry(1, salary, 3000), entry(2, food, 350), entry(3, rent, 1000)];

        let y = summarize(2026, &c, &months, &Labels::default());
        assert_eq!(y.year, 2026);
        assert_eq!(y.months[0], MonthTotals { income_cents: 300_000, expense_cents: 40_000, entries: 2 });
        assert_eq!(y.months[1].expense_cents, 135_000);
        assert_eq!(y.months[1].balance_cents(), 165_000);
        assert_eq!(y.months[2], MonthTotals::default());

        assert_eq!(y.data.income_cents, 600_000);
        assert_eq!(y.data.expense_cents, 175_000);
        // Food from both months is one node with the combined amount.
        let node = |n: &str| y.data.nodes.iter().position(|x| x.label == n).unwrap();
        let link = y
            .data
            .links
            .iter()
            .find(|l| l.source == node("Total income") && l.target == node("Food"))
            .unwrap();
        assert_eq!(link.value, 750.0);
    }

    #[test]
    fn empty_year_has_no_graph() {
        let y = summarize(2020, &Categories::default(), &[], &Labels::default());
        assert!(y.data.is_empty());
        assert!(y.months.iter().all(|m| m.entries == 0));
    }

    #[test]
    fn ignores_unknown_categories_and_non_positive_amounts() {
        let c = Categories::default();
        let months = vec![vec![
            entry(1, id_of(&c, "Salary"), 100),
            entry(2, 9999, 50),
            entry(3, id_of(&c, "Food"), -5),
        ]];
        let y = summarize(2026, &c, &months, &Labels::default());
        assert_eq!(y.months[0], MonthTotals { income_cents: 10_000, expense_cents: 0, entries: 1 });
    }
}
