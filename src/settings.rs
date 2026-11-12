//! User settings, stored in `settings.json`.

use serde::{Deserialize, Serialize};

pub const MAX_LABEL_CHARS: usize = 40;

/// Names of the automatic nodes in the money flow chart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Labels {
    /// The middle node every income flows into.
    pub total: String,
    /// Shown when income exceeds expenses.
    pub savings: String,
    /// Shown when expenses exceed income.
    pub deficit: String,
}

impl Default for Labels {
    fn default() -> Self {
        Labels {
            total: "Total income".to_string(),
            savings: "Savings".to_string(),
            deficit: "Deficit".to_string(),
        }
    }
}

impl Labels {
    /// Trims, collapses whitespace, limits the length and replaces empty
    /// labels with the defaults.
    pub fn sanitized(&self) -> Labels {
        let default = Labels::default();
        let clean = |s: &str, fallback: &str| -> String {
            let one_line = s.split_whitespace().collect::<Vec<_>>().join(" ");
            let name: String = one_line.chars().take(MAX_LABEL_CHARS).collect();
            if name.trim().is_empty() { fallback.to_string() } else { name.trim().to_string() }
        };
        Labels {
            total: clean(&self.total, &default.total),
            savings: clean(&self.savings, &default.savings),
            deficit: clean(&self.deficit, &default.deficit),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub labels: Labels,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_labels() {
        let l = Labels {
            total: "  My   budget \n".to_string(),
            savings: "   ".to_string(),
            deficit: "x".repeat(100),
        }
        .sanitized();
        assert_eq!(l.total, "My budget");
        assert_eq!(l.savings, "Savings");
        assert_eq!(l.deficit.chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"labels":{"total":"Budget"}}"#).unwrap();
        assert_eq!(s.labels.total, "Budget");
        assert_eq!(s.labels.savings, "Savings");
        let empty: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Settings::default());
    }
}
