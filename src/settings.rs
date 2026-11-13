//! User settings, stored in `settings.json`.

use serde::{Deserialize, Serialize};

use crate::i18n::{Currency, Lang, Msg};

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
    /// Labels as shown for `lang`: a label that still has any language's default
    /// text follows the current language; a customized label is kept as typed.
    pub fn resolved(&self, lang: Lang) -> Labels {
        let pick = |value: &str, msg: Msg| -> String {
            if Lang::ALL.iter().any(|&l| msg.text(l).eq_ignore_ascii_case(value.trim())) {
                msg.text(lang).to_string()
            } else {
                value.to_string()
            }
        };
        Labels {
            total: pick(&self.total, Msg::DefLabelTotal),
            savings: pick(&self.savings, Msg::DefLabelSavings),
            deficit: pick(&self.deficit, Msg::DefLabelDeficit),
        }
    }

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
    /// `None` follows the system language.
    pub language: Option<Lang>,
    /// `None` follows the system region.
    pub currency: Option<Currency>,
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
    fn default_labels_follow_the_language_but_custom_ones_do_not() {
        let defaults = Labels::default();
        assert_eq!(defaults.resolved(Lang::Fi).total, "Kokonaistulot");
        assert_eq!(defaults.resolved(Lang::De).deficit, "Defizit");
        // A label saved in one language is translated when the language changes.
        let saved_in_finnish = Labels { total: "Kokonaistulot".into(), ..Labels::default() };
        assert_eq!(saved_in_finnish.resolved(Lang::Sv).total, "Totala inkomster");
        // Customized text stays as typed.
        let custom = Labels { total: "Budget".into(), ..Labels::default() };
        assert_eq!(custom.resolved(Lang::Fi).total, "Budget");
    }

    #[test]
    fn language_and_currency_round_trip_through_json() {
        let s = Settings { language: Some(Lang::Fi), currency: Some(Currency::Gbp), ..Settings::default() };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"fi\"") && json.contains("\"gbp\""), "{json}");
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), s);
        // Old files without the new fields mean "automatic".
        let old: Settings = serde_json::from_str(r#"{"labels":{"total":"X"}}"#).unwrap();
        assert_eq!((old.language, old.currency), (None, None));
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
