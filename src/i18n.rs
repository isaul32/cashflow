//! Translations and locale-aware formatting (numbers, currency, month names).
//!
//! Every message is defined once in the `messages!` table below with a text for
//! each supported language, so a missing translation is a compile error. To add
//! a language: add a `Lang` variant, its formatting rules in `Lang::separators`,
//! month names, and one column in the table.

use serde::{Deserialize, Serialize};

use crate::model::MonthKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Fi,
    Sv,
    De,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 6] = [Lang::En, Lang::Fi, Lang::Sv, Lang::De, Lang::Fr, Lang::Es];

    /// The language's own name, shown in the language picker.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Fi => "Suomi",
            Lang::Sv => "Svenska",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Es => "Español",
        }
    }

    /// Parses a locale tag such as "fi", "fi-FI", "fi_FI.UTF-8" or "sv_SE@euro".
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let primary = tag
            .split(['-', '_', '.', '@'])
            .next()?
            .to_ascii_lowercase();
        match primary.as_str() {
            "en" => Some(Lang::En),
            "fi" => Some(Lang::Fi),
            "sv" => Some(Lang::Sv),
            "de" => Some(Lang::De),
            "fr" => Some(Lang::Fr),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }

    /// (group separator, decimal separator, minimum digits before the first
    /// group separator is used, e.g. Spanish does not group 4-digit numbers).
    fn separators(self) -> (&'static str, &'static str, usize) {
        match self {
            Lang::En => (",", ".", 4),
            Lang::Fi | Lang::Sv => ("\u{a0}", ",", 4),
            Lang::De => (".", ",", 4),
            Lang::Fr => ("\u{202f}", ",", 4),
            Lang::Es => (".", ",", 5),
        }
    }

    fn month_names(self) -> [&'static str; 12] {
        match self {
            Lang::En => [
                "January", "February", "March", "April", "May", "June", "July", "August",
                "September", "October", "November", "December",
            ],
            Lang::Fi => [
                "tammikuu", "helmikuu", "maaliskuu", "huhtikuu", "toukokuu", "kesäkuu",
                "heinäkuu", "elokuu", "syyskuu", "lokakuu", "marraskuu", "joulukuu",
            ],
            Lang::Sv => [
                "januari", "februari", "mars", "april", "maj", "juni", "juli", "augusti",
                "september", "oktober", "november", "december",
            ],
            Lang::De => [
                "Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August",
                "September", "Oktober", "November", "Dezember",
            ],
            Lang::Fr => [
                "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août",
                "septembre", "octobre", "novembre", "décembre",
            ],
            Lang::Es => [
                "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto",
                "septiembre", "octubre", "noviembre", "diciembre",
            ],
        }
    }

    fn month_abbreviations(self) -> [&'static str; 12] {
        match self {
            Lang::En => ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
            Lang::Fi => [
                "tammi", "helmi", "maalis", "huhti", "touko", "kesä", "heinä", "elo", "syys",
                "loka", "marras", "joulu",
            ],
            Lang::Sv => ["jan", "feb", "mar", "apr", "maj", "jun", "jul", "aug", "sep", "okt", "nov", "dec"],
            Lang::De => ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"],
            Lang::Fr => [
                "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.",
                "nov.", "déc.",
            ],
            Lang::Es => ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Currency {
    Eur,
    Usd,
    Gbp,
    Sek,
    Nok,
    Dkk,
    Chf,
}

impl Currency {
    pub const ALL: [Currency; 7] = [
        Currency::Eur,
        Currency::Usd,
        Currency::Gbp,
        Currency::Sek,
        Currency::Nok,
        Currency::Dkk,
        Currency::Chf,
    ];

    /// ISO 4217 code.
    pub fn code(self) -> &'static str {
        match self {
            Currency::Eur => "EUR",
            Currency::Usd => "USD",
            Currency::Gbp => "GBP",
            Currency::Sek => "SEK",
            Currency::Nok => "NOK",
            Currency::Dkk => "DKK",
            Currency::Chf => "CHF",
        }
    }

    /// The symbol a reader of `lang` expects. Currencies without a widely
    /// understood symbol use their code, except the krona in Swedish.
    pub fn symbol(self, lang: Lang) -> &'static str {
        match self {
            Currency::Eur => "€",
            Currency::Usd => "$",
            Currency::Gbp => "£",
            Currency::Sek if lang == Lang::Sv => "kr",
            other => other.code(),
        }
    }

    fn for_region(region: &str) -> Option<Currency> {
        Some(match region {
            "AT" | "BE" | "CY" | "DE" | "EE" | "ES" | "FI" | "FR" | "GR" | "HR" | "IE" | "IT"
            | "LT" | "LU" | "LV" | "MT" | "NL" | "PT" | "SI" | "SK" => Currency::Eur,
            "US" => Currency::Usd,
            "GB" => Currency::Gbp,
            "SE" => Currency::Sek,
            "NO" => Currency::Nok,
            "DK" => Currency::Dkk,
            "CH" | "LI" => Currency::Chf,
            _ => return None,
        })
    }
}

/// Language and region reported by the operating system.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemLocale {
    pub lang: Option<Lang>,
    /// Two-letter region such as "FI", if known.
    pub region: Option<String>,
}

impl SystemLocale {
    /// Builds one from a locale tag like "fi-FI" or "en_US.UTF-8".
    pub fn from_tag(tag: &str) -> SystemLocale {
        let tag = tag.split(['.', '@']).next().unwrap_or("");
        let region = tag
            .split(['-', '_'])
            .nth(1)
            .filter(|r| r.len() == 2 && r.chars().all(|c| c.is_ascii_alphabetic()))
            .map(|r| r.to_ascii_uppercase());
        SystemLocale { lang: Lang::from_tag(tag), region }
    }

    /// Picks the first supported language from the user's preferred languages
    /// (most preferred first), so `[ja-JP, sv-FI, en-US]` gives Swedish.
    ///
    /// `region` is the region of the system's regional format when the platform
    /// reports one separately from the display language; otherwise the region of
    /// the matching language tag is used (e.g. "GB" from "en-GB").
    pub fn from_preferences(tags: &[String], region: Option<String>) -> SystemLocale {
        let matched = tags.iter().find_map(|tag| Lang::from_tag(tag).map(|lang| (lang, tag)));
        SystemLocale {
            lang: matched.map(|(lang, _)| lang),
            region: region.or_else(|| matched.and_then(|(_, tag)| SystemLocale::from_tag(tag).region)),
        }
    }

    pub fn detect() -> SystemLocale {
        let tags: Vec<String> = sys_locale::get_locales().collect();
        SystemLocale::from_preferences(&tags, regional_format_region())
    }
}

/// The region of the Windows "regional format" setting (e.g. "FI"). It is kept
/// apart from the display language: an English Windows in Finland should still
/// default to euros. Other platforms report the region inside the language tag.
#[cfg(windows)]
fn regional_format_region() -> Option<String> {
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
    }
    // SAFETY: plain Win32 call; the buffer is large enough (LOCALE_NAME_MAX_LENGTH = 85).
    let tag = unsafe {
        let mut buf = [0u16; 85];
        let n = GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32);
        if n > 1 { String::from_utf16_lossy(&buf[..(n as usize - 1)]) } else { String::new() }
    };
    SystemLocale::from_tag(&tag).region
}

#[cfg(not(windows))]
fn regional_format_region() -> Option<String> {
    None
}

/// The resolved language and currency used for all text and number formatting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Locale {
    pub lang: Lang,
    pub currency: Currency,
}

impl Locale {
    /// `language` / `currency` are the user's choices; `None` means automatic.
    pub fn resolve(language: Option<Lang>, currency: Option<Currency>, system: &SystemLocale) -> Locale {
        let lang = language.or(system.lang).unwrap_or(Lang::En);
        let currency = currency
            .or_else(|| system.region.as_deref().and_then(Currency::for_region))
            .unwrap_or(if lang == Lang::Sv { Currency::Sek } else { Currency::Eur });
        Locale { lang, currency }
    }

    pub fn t(&self, msg: Msg) -> &'static str {
        msg.text(self.lang)
    }

    /// Message with `{name}` placeholders replaced.
    pub fn tf(&self, msg: Msg, args: &[(&str, &str)]) -> String {
        msg.format(self.lang, args)
    }

    /// The decimal mark of the language's number format ('.' or ',').
    pub fn decimal_mark(&self) -> char {
        self.lang.separators().1.chars().next().unwrap_or('.')
    }

    /// The minus sign: a real minus (U+2212) in Finnish and Swedish, a hyphen otherwise.
    pub fn minus(&self) -> &'static str {
        match self.lang {
            Lang::Fi | Lang::Sv => "−",
            _ => "-",
        }
    }

    pub fn currency_symbol(&self) -> &'static str {
        self.currency.symbol(self.lang)
    }

    fn group(&self, digits: &str) -> String {
        let (sep, _, min_digits) = self.lang.separators();
        if digits.len() < min_digits {
            return digits.to_string();
        }
        let mut out = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push_str(sep);
            }
            out.push(ch);
        }
        out
    }

    /// Cents as a number with grouping and two decimals ("1,234.50"), no currency.
    fn number_cents(&self, cents_abs: i64) -> String {
        let (_, dec, _) = self.lang.separators();
        format!("{}{}{:02}", self.group(&(cents_abs / 100).to_string()), dec, cents_abs % 100)
    }

    fn with_currency(&self, number: String, negative: bool) -> String {
        let symbol = self.currency_symbol();
        let sign = if negative { self.minus() } else { "" };
        if self.lang == Lang::En {
            // English puts a single-glyph symbol directly in front, a code after a space.
            let gap = if symbol.chars().count() == 1 { "" } else { "\u{a0}" };
            format!("{sign}{symbol}{gap}{number}")
        } else {
            format!("{sign}{number}\u{a0}{symbol}")
        }
    }

    /// An amount with currency and two decimals: "€1,234.50" or "1 234,50 €".
    pub fn money(&self, cents: i64) -> String {
        self.with_currency(self.number_cents(cents.abs()), cents < 0)
    }

    /// A whole-unit amount for chart labels (rounded, no decimals).
    pub fn money_whole(&self, value: f64) -> String {
        let whole = value.round() as i64;
        self.with_currency(self.group(&whole.abs().to_string()), whole < 0)
    }

    /// An amount for an input field: decimal separator only, no grouping or currency.
    pub fn plain_amount(&self, cents: i64) -> String {
        let (_, dec, _) = self.lang.separators();
        format!("{}{}{:02}", cents.abs() / 100, dec, cents.abs() % 100)
    }

    /// The month as it is written inside a sentence: "November 2026" (English and
    /// German capitalize months), "marraskuu 2026", "novembre 2026", "noviembre de 2026".
    pub fn month_inline(&self, key: MonthKey) -> String {
        let name = self.lang.month_names()[(key.month - 1) as usize];
        let joiner = if self.lang == Lang::Es { " de " } else { " " };
        format!("{name}{joiner}{}", key.year)
    }

    /// The month as a heading or label: like `month_inline`, first letter capitalized.
    pub fn month_title(&self, key: MonthKey) -> String {
        capitalize(&self.month_inline(key))
    }

    /// A counted phrase such as "1 entry" / "3 entries".
    pub fn count(&self, plural: Plural, n: usize) -> String {
        plural.text(self.lang, n as i64)
    }

    /// Short month name for lists: "Nov", "marras".
    pub fn month_short(&self, month: u8) -> &'static str {
        self.lang.month_abbreviations()[(month.clamp(1, 12) - 1) as usize]
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

macro_rules! messages {
    ($($key:ident { en: $en:expr, fi: $fi:expr, sv: $sv:expr, de: $de:expr, fr: $fr:expr, es: $es:expr $(,)? })*) => {
        /// Every translatable message of the app.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Msg { $($key),* }

        impl Msg {
            #[cfg(test)]
            pub const ALL: &'static [Msg] = &[$(Msg::$key),*];

            /// The message with `{name}` placeholders replaced.
            pub fn format(self, lang: Lang, args: &[(&str, &str)]) -> String {
                let mut out = self.text(lang).to_string();
                for (key, value) in args {
                    out = out.replace(&format!("{{{key}}}"), value);
                }
                out
            }

            pub fn text(self, lang: Lang) -> &'static str {
                match self {
                    $(Msg::$key => match lang {
                        Lang::En => $en,
                        Lang::Fi => $fi,
                        Lang::Sv => $sv,
                        Lang::De => $de,
                        Lang::Fr => $fr,
                        Lang::Es => $es,
                    }),*
                }
            }
        }
    };
}

messages! {
    // ---- sidebar and headers ----
    Tagline {
        en: "Monthly cash flow", fi: "Kuukauden kassavirta", sv: "Månadens kassaflöde",
        de: "Monatlicher Cashflow", fr: "Trésorerie mensuelle", es: "Flujo de caja mensual",
    }
    NavOverview {
        en: "Overview", fi: "Yleiskatsaus", sv: "Översikt", de: "Übersicht", fr: "Aperçu", es: "Resumen",
    }
    NavYear { en: "Year", fi: "Vuosi", sv: "År", de: "Jahr", fr: "Année", es: "Año" }
    NavCategories {
        en: "Categories", fi: "Kategoriat", sv: "Kategorier", de: "Kategorien", fr: "Catégories", es: "Categorías",
    }
    NavSettings {
        en: "Settings", fi: "Asetukset", sv: "Inställningar", de: "Einstellungen", fr: "Paramètres", es: "Ajustes",
    }
    NavAbout { en: "About", fi: "Tietoja", sv: "Om", de: "Info", fr: "À propos", es: "Acerca de" }
    ThemeLight {
        en: "Light mode", fi: "Vaalea tila", sv: "Ljust läge", de: "Heller Modus", fr: "Mode clair", es: "Modo claro",
    }
    ThemeDark {
        en: "Dark mode", fi: "Tumma tila", sv: "Mörkt läge", de: "Dunkler Modus", fr: "Mode sombre", es: "Modo oscuro",
    }
    SubOverview {
        en: "Where your money comes from and where it goes", fi: "Mistä rahasi tulevat ja minne ne menevät",
        sv: "Var dina pengar kommer ifrån och vart de går", de: "Woher dein Geld kommt und wohin es geht",
        fr: "D'où vient votre argent et où il va", es: "De dónde viene tu dinero y a dónde va",
    }
    SubYear {
        en: "All months of a year added together", fi: "Vuoden kaikki kuukaudet yhteensä",
        sv: "Årets alla månader sammanlagda", de: "Alle Monate eines Jahres zusammengefasst",
        fr: "Tous les mois d'une année additionnés", es: "Todos los meses de un año sumados",
    }
    SubCategories {
        en: "Organize income and spending your way", fi: "Järjestä tulot ja menot omalla tavallasi",
        sv: "Organisera inkomster och utgifter på ditt sätt", de: "Ordne Einnahmen und Ausgaben auf deine Art",
        fr: "Organisez revenus et dépenses à votre façon", es: "Organiza ingresos y gastos a tu manera",
    }
    SubSettings {
        en: "Customize language, currency and chart labels", fi: "Muokkaa kieltä, valuuttaa ja kaavion nimiä",
        sv: "Anpassa språk, valuta och diagramtexter", de: "Sprache, Währung und Diagrammbeschriftungen anpassen",
        fr: "Personnalisez la langue, la devise et les libellés du graphique",
        es: "Personaliza el idioma, la moneda y las etiquetas del gráfico",
    }
    SubAbout {
        en: "The app, its author and license", fi: "Sovellus, sen tekijä ja lisenssi",
        sv: "Appen, dess upphovsperson och licens", de: "Die App, ihr Autor und die Lizenz",
        fr: "L'application, son auteur et sa licence", es: "La aplicación, su autor y su licencia",
    }

    // ---- kinds and totals ----
    KindIncome { en: "Income", fi: "Tulo", sv: "Inkomst", de: "Einnahme", fr: "Revenu", es: "Ingreso" }
    KindExpense { en: "Expense", fi: "Meno", sv: "Utgift", de: "Ausgabe", fr: "Dépense", es: "Gasto" }
    Income { en: "Income", fi: "Tulot", sv: "Inkomster", de: "Einnahmen", fr: "Revenus", es: "Ingresos" }
    Expenses { en: "Expenses", fi: "Menot", sv: "Utgifter", de: "Ausgaben", fr: "Dépenses", es: "Gastos" }
    Balance { en: "Balance", fi: "Saldo", sv: "Saldo", de: "Saldo", fr: "Solde", es: "Saldo" }
    Savings { en: "Savings", fi: "Säästöt", sv: "Sparande", de: "Ersparnisse", fr: "Épargne", es: "Ahorros" }
    Deficit { en: "Deficit", fi: "Alijäämä", sv: "Underskott", de: "Defizit", fr: "Déficit", es: "Déficit" }
    NoDataYet {
        en: "No data yet", fi: "Ei vielä tietoja", sv: "Inga data än", de: "Noch keine Daten",
        fr: "Pas encore de données", es: "Aún sin datos",
    }
    SavedRate {
        en: "{pct}% of income saved", fi: "{pct} % tuloista säästetty", sv: "{pct} % av inkomsten sparad",
        de: "{pct} % der Einnahmen gespart", fr: "{pct} % des revenus épargnés",
        es: "{pct} % de los ingresos ahorrados",
    }
    SpendingExceeds {
        en: "Spending exceeds income", fi: "Menot ylittävät tulot", sv: "Utgifterna överstiger inkomsterna",
        de: "Ausgaben übersteigen Einnahmen", fr: "Les dépenses dépassent les revenus",
        es: "Los gastos superan los ingresos",
    }
    TotalEarned {
        en: "Total earned", fi: "Ansaittu yhteensä", sv: "Totalt intjänat", de: "Insgesamt verdient",
        fr: "Total gagné", es: "Total ganado",
    }
    TotalSpent {
        en: "Total spent", fi: "Käytetty yhteensä", sv: "Totalt spenderat", de: "Insgesamt ausgegeben",
        fr: "Total dépensé", es: "Total gastado",
    }

    // ---- entry form and list ----
    AddEntry {
        en: "Add entry", fi: "Lisää merkintä", sv: "Lägg till post", de: "Eintrag hinzufügen",
        fr: "Ajouter une entrée", es: "Añadir movimiento",
    }
    EditEntry {
        en: "Edit entry", fi: "Muokkaa merkintää", sv: "Redigera post", de: "Eintrag bearbeiten",
        fr: "Modifier l'entrée", es: "Editar movimiento",
    }
    SaveChanges {
        en: "Save changes", fi: "Tallenna muutokset", sv: "Spara ändringar", de: "Änderungen speichern",
        fr: "Enregistrer les modifications", es: "Guardar cambios",
    }
    Cancel { en: "Cancel", fi: "Peruuta", sv: "Avbryt", de: "Abbrechen", fr: "Annuler", es: "Cancelar" }
    Edit { en: "Edit", fi: "Muokkaa", sv: "Redigera", de: "Bearbeiten", fr: "Modifier", es: "Editar" }
    Category { en: "Category", fi: "Kategoria", sv: "Kategori", de: "Kategorie", fr: "Catégorie", es: "Categoría" }
    NoCategoriesHint {
        en: "No categories yet. Create one on the Categories page.",
        fi: "Ei vielä kategorioita. Luo sellainen Kategoriat-sivulla.",
        sv: "Inga kategorier än. Skapa en på sidan Kategorier.",
        de: "Noch keine Kategorien. Lege eine auf der Seite „Kategorien“ an.",
        fr: "Aucune catégorie. Créez-en une dans la page Catégories.",
        es: "Aún no hay categorías. Crea una en la página Categorías.",
    }
    Description {
        en: "Description", fi: "Kuvaus", sv: "Beskrivning", de: "Beschreibung", fr: "Description",
        es: "Descripción",
    }
    AmountLabel {
        en: "Amount ({symbol})", fi: "Summa ({symbol})", sv: "Belopp ({symbol})", de: "Betrag ({symbol})",
        fr: "Montant ({symbol})", es: "Importe ({symbol})",
    }
    PlaceholderNote {
        en: "Optional note", fi: "Vapaaehtoinen huomautus", sv: "Valfri anteckning", de: "Optionale Notiz",
        fr: "Note facultative", es: "Nota opcional",
    }
    Entries { en: "Entries", fi: "Merkinnät", sv: "Poster", de: "Einträge", fr: "Entrées", es: "Movimientos" }
    CountThisMonth {
        en: "{count} this month", fi: "{count} tässä kuussa", sv: "{count} denna månad",
        de: "{count} in diesem Monat", fr: "{count} ce mois-ci", es: "{count} este mes",
    }
    NothingRecorded {
        en: "Nothing recorded for this month yet.", fi: "Tälle kuulle ei ole vielä merkintöjä.",
        sv: "Inget registrerat för den här månaden än.", de: "Für diesen Monat ist noch nichts erfasst.",
        fr: "Rien n'est encore enregistré pour ce mois.", es: "Aún no hay nada registrado este mes.",
    }
    NoEntriesFor {
        en: "No entries for {period}", fi: "Ei merkintöjä: {period}", sv: "Inga poster för {period}",
        de: "Keine Einträge für {period}", fr: "Aucune entrée pour {period}", es: "Sin movimientos en {period}",
    }
    NoEntriesHint {
        en: "Add income and expenses on the left to see how your money flows.",
        fi: "Lisää tulot ja menot vasemmalla nähdäksesi, miten rahasi virtaavat.",
        sv: "Lägg till inkomster och utgifter till vänster för att se hur pengarna flödar.",
        de: "Füge links Einnahmen und Ausgaben hinzu, um den Geldfluss zu sehen.",
        fr: "Ajoutez des revenus et des dépenses à gauche pour voir comment circule votre argent.",
        es: "Añade ingresos y gastos a la izquierda para ver cómo fluye tu dinero.",
    }
    NoEntriesYearHint {
        en: "Entries you add on the Overview page are summed up here.",
        fi: "Yleiskatsaus-sivulla lisäämäsi merkinnät summataan tähän.",
        sv: "Poster du lägger till på översiktssidan summeras här.",
        de: "Einträge von der Übersichtsseite werden hier summiert.",
        fr: "Les entrées ajoutées dans l'aperçu sont additionnées ici.",
        es: "Los movimientos que añadas en Resumen se suman aquí.",
    }

    // ---- chart, export, import ----
    MoneyFlow {
        en: "Money flow", fi: "Rahavirta", sv: "Pengaflöde", de: "Geldfluss", fr: "Flux d'argent",
        es: "Flujo de dinero",
    }
    Export { en: "Export", fi: "Vie", sv: "Exportera", de: "Export", fr: "Exporter", es: "Exportar" }
    Import { en: "Import", fi: "Tuo", sv: "Importera", de: "Import", fr: "Importer", es: "Importar" }
    Copy { en: "Copy", fi: "Kopioi", sv: "Kopiera", de: "Kopieren", fr: "Copier", es: "Copiar" }
    Copied {
        en: "Copied ✓", fi: "Kopioitu ✓", sv: "Kopierat ✓", de: "Kopiert ✓", fr: "Copié ✓", es: "Copiado ✓",
    }
    SaveTxt {
        en: "Save .txt", fi: "Tallenna .txt", sv: "Spara .txt", de: "Als .txt speichern",
        fr: "Enregistrer .txt", es: "Guardar .txt",
    }
    Paste { en: "Paste", fi: "Liitä", sv: "Klistra in", de: "Einfügen", fr: "Coller", es: "Pegar" }
    OpenTxt {
        en: "Open .txt", fi: "Avaa .txt", sv: "Öppna .txt", de: ".txt öffnen", fr: "Ouvrir .txt",
        es: "Abrir .txt",
    }
    TipCopy {
        en: "Copy as SankeyMATIC text", fi: "Kopioi SankeyMATIC-tekstinä",
        sv: "Kopiera som SankeyMATIC-text", de: "Als SankeyMATIC-Text kopieren",
        fr: "Copier au format texte SankeyMATIC", es: "Copiar como texto de SankeyMATIC",
    }
    TipSave {
        en: "Save the SankeyMATIC text to a file", fi: "Tallenna SankeyMATIC-teksti tiedostoon",
        sv: "Spara SankeyMATIC-texten i en fil", de: "SankeyMATIC-Text in eine Datei speichern",
        fr: "Enregistrer le texte SankeyMATIC dans un fichier",
        es: "Guardar el texto de SankeyMATIC en un archivo",
    }
    TipPaste {
        en: "Import SankeyMATIC text from the clipboard", fi: "Tuo SankeyMATIC-teksti leikepöydältä",
        sv: "Importera SankeyMATIC-text från urklipp", de: "SankeyMATIC-Text aus der Zwischenablage importieren",
        fr: "Importer le texte SankeyMATIC depuis le presse-papiers",
        es: "Importar texto de SankeyMATIC desde el portapapeles",
    }
    TipOpen {
        en: "Import SankeyMATIC text from a file", fi: "Tuo SankeyMATIC-teksti tiedostosta",
        sv: "Importera SankeyMATIC-text från en fil", de: "SankeyMATIC-Text aus einer Datei importieren",
        fr: "Importer le texte SankeyMATIC depuis un fichier",
        es: "Importar texto de SankeyMATIC desde un archivo",
    }
    ImportTitle {
        en: "Import from SankeyMATIC", fi: "Tuo SankeyMATICista", sv: "Importera från SankeyMATIC",
        de: "Aus SankeyMATIC importieren", fr: "Importer depuis SankeyMATIC", es: "Importar desde SankeyMATIC",
    }
    ImportFound {
        en: "Found {cats} and {entries}: income {income}, expenses {expenses}.",
        fi: "Löytyi {cats} ja {entries}: tulot {income}, menot {expenses}.",
        sv: "Hittade {cats} och {entries}: inkomster {income}, utgifter {expenses}.",
        de: "{cats} und {entries} gefunden: Einnahmen {income}, Ausgaben {expenses}.",
        fr: "{cats} et {entries} trouvées : revenus {income}, dépenses {expenses}.",
        es: "Se encontraron {cats} y {entries}: ingresos {income}, gastos {expenses}.",
    }
    ImportReuse {
        en: "Categories with the same name are reused; the others are created.",
        fi: "Samannimiset kategoriat käytetään uudelleen, muut luodaan.",
        sv: "Kategorier med samma namn återanvänds, övriga skapas.",
        de: "Kategorien mit gleichem Namen werden wiederverwendet, die übrigen neu angelegt.",
        fr: "Les catégories du même nom sont réutilisées, les autres sont créées.",
        es: "Las categorías con el mismo nombre se reutilizan; las demás se crean.",
    }
    ImportReplace {
        en: "Replace {entries} in {period}", fi: "Korvaa {entries} ({period})",
        sv: "Ersätt {entries} ({period})", de: "{entries} ersetzen ({period})",
        fr: "Remplacer {entries} ({period})", es: "Reemplazar {entries} ({period})",
    }
    ImportInto {
        en: "Import into {period}", fi: "Tuo merkinnät ({period})", sv: "Importera ({period})",
        de: "Importieren ({period})", fr: "Importer ({period})", es: "Importar ({period})",
    }
    ImportAdd {
        en: "Add to existing", fi: "Lisää nykyisten joukkoon", sv: "Lägg till i befintliga",
        de: "Zu vorhandenen hinzufügen", fr: "Ajouter aux existantes", es: "Añadir a los existentes",
    }
    MoreNotes {
        en: "…and {notes}", fi: "…ja {notes}", sv: "…och {notes}", de: "…und {notes}", fr: "…et {notes}",
        es: "…y {notes}",
    }
    PromptOpenFile {
        en: "Select a SankeyMATIC text file", fi: "Valitse SankeyMATIC-tekstitiedosto",
        sv: "Välj en SankeyMATIC-textfil", de: "SankeyMATIC-Textdatei auswählen",
        fr: "Sélectionnez un fichier texte SankeyMATIC", es: "Selecciona un archivo de texto de SankeyMATIC",
    }

    // ---- year page and navigation ----
    ByMonth { en: "By month", fi: "Kuukausittain", sv: "Per månad", de: "Nach Monat", fr: "Par mois", es: "Por mes" }
    YearHint {
        en: "Click a month to open it. The number is the balance.",
        fi: "Avaa kuukausi napsauttamalla. Luku on saldo.",
        sv: "Klicka på en månad för att öppna den. Talet är saldot.",
        de: "Klicke auf einen Monat, um ihn zu öffnen. Die Zahl ist der Saldo.",
        fr: "Cliquez sur un mois pour l'ouvrir. Le chiffre est le solde.",
        es: "Haz clic en un mes para abrirlo. La cifra es el saldo.",
    }
    Today { en: "Today", fi: "Tänään", sv: "Idag", de: "Heute", fr: "Aujourd'hui", es: "Hoy" }
    ThisYear {
        en: "This year", fi: "Tämä vuosi", sv: "I år", de: "Dieses Jahr", fr: "Cette année", es: "Este año",
    }

    // ---- categories page ----
    AddCategory {
        en: "Add category", fi: "Lisää kategoria", sv: "Lägg till kategori", de: "Kategorie hinzufügen",
        fr: "Ajouter une catégorie", es: "Añadir categoría",
    }
    NewCategory {
        en: "New category", fi: "Uusi kategoria", sv: "Ny kategori", de: "Neue Kategorie",
        fr: "Nouvelle catégorie", es: "Nueva categoría",
    }
    EditCategory {
        en: "Edit category", fi: "Muokkaa kategoriaa", sv: "Redigera kategori", de: "Kategorie bearbeiten",
        fr: "Modifier la catégorie", es: "Editar categoría",
    }
    Name { en: "Name", fi: "Nimi", sv: "Namn", de: "Name", fr: "Nom", es: "Nombre" }
    Color { en: "Color", fi: "Väri", sv: "Färg", de: "Farbe", fr: "Couleur", es: "Color" }
    Parent {
        en: "Parent", fi: "Yläkategoria", sv: "Överordnad kategori", de: "Übergeordnete Kategorie",
        fr: "Catégorie parente", es: "Categoría superior",
    }
    NoParent {
        en: "No parent", fi: "Ei yläkategoriaa", sv: "Ingen", de: "Keine", fr: "Aucune", es: "Ninguna",
    }
    AllCategories {
        en: "All categories", fi: "Kaikki kategoriat", sv: "Alla kategorier", de: "Alle Kategorien",
        fr: "Toutes les catégories", es: "Todas las categorías",
    }
    CatNamePlaceholder {
        en: "New category name", fi: "Uuden kategorian nimi", sv: "Nytt kategorinamn",
        de: "Name der neuen Kategorie", fr: "Nom de la nouvelle catégorie", es: "Nombre de la nueva categoría",
    }

    // ---- settings page ----
    ChartLabels {
        en: "Chart labels", fi: "Kaavion nimet", sv: "Diagramtexter", de: "Diagrammbeschriftungen",
        fr: "Libellés du graphique", es: "Etiquetas del gráfico",
    }
    ChartLabelsHelp {
        en: "Names of the automatic nodes in the money flow chart. They are also used in the SankeyMATIC export and recognized on import.",
        fi: "Rahavirtakaavion automaattisten solmujen nimet. Niitä käytetään myös SankeyMATIC-viennissä, ja ne tunnistetaan tuonnissa.",
        sv: "Namn på de automatiska noderna i pengaflödesdiagrammet. De används även i SankeyMATIC-exporten och känns igen vid import.",
        de: "Namen der automatischen Knoten im Geldflussdiagramm. Sie werden auch beim SankeyMATIC-Export verwendet und beim Import erkannt.",
        fr: "Noms des nœuds automatiques du graphique. Ils sont aussi utilisés dans l'export SankeyMATIC et reconnus à l'import.",
        es: "Nombres de los nodos automáticos del gráfico. También se usan en la exportación a SankeyMATIC y se reconocen al importar.",
    }
    LabelTotal {
        en: "Middle node (all income flows in here)", fi: "Keskisolmu (kaikki tulot virtaavat tähän)",
        sv: "Mittnod (alla inkomster flödar hit)", de: "Mittelknoten (alle Einnahmen fließen hierher)",
        fr: "Nœud central (tous les revenus y arrivent)", es: "Nodo central (todos los ingresos fluyen aquí)",
    }
    LabelSavings {
        en: "When income exceeds expenses", fi: "Kun tulot ylittävät menot",
        sv: "När inkomsterna överstiger utgifterna", de: "Wenn die Einnahmen die Ausgaben übersteigen",
        fr: "Quand les revenus dépassent les dépenses", es: "Cuando los ingresos superan los gastos",
    }
    LabelDeficit {
        en: "When expenses exceed income", fi: "Kun menot ylittävät tulot",
        sv: "När utgifterna överstiger inkomsterna", de: "Wenn die Ausgaben die Einnahmen übersteigen",
        fr: "Quand les dépenses dépassent les revenus", es: "Cuando los gastos superan los ingresos",
    }
    Save { en: "Save", fi: "Tallenna", sv: "Spara", de: "Speichern", fr: "Enregistrer", es: "Guardar" }
    ResetDefaults {
        en: "Reset to defaults", fi: "Palauta oletukset", sv: "Återställ standardvärden",
        de: "Auf Standard zurücksetzen", fr: "Rétablir les valeurs par défaut",
        es: "Restablecer valores predeterminados",
    }
    LanguageTitle { en: "Language", fi: "Kieli", sv: "Språk", de: "Sprache", fr: "Langue", es: "Idioma" }
    LanguageHelp {
        en: "Dates and numbers follow the language. \"Automatic\" uses your system language.",
        fi: "Päivämäärät ja luvut noudattavat kieltä. \"Automaattinen\" käyttää järjestelmän kieltä.",
        sv: "Datum och tal följer språket. \"Automatiskt\" använder systemspråket.",
        de: "Datum und Zahlen richten sich nach der Sprache. „Automatisch“ verwendet die Systemsprache.",
        fr: "Les dates et les nombres suivent la langue. « Automatique » utilise la langue du système.",
        es: "Las fechas y los números siguen el idioma. «Automático» usa el idioma del sistema.",
    }
    Automatic {
        en: "Automatic", fi: "Automaattinen", sv: "Automatiskt", de: "Automatisch", fr: "Automatique",
        es: "Automático",
    }
    CurrencyTitle { en: "Currency", fi: "Valuutta", sv: "Valuta", de: "Währung", fr: "Devise", es: "Moneda" }
    CurrencyHelp {
        en: "Changes only how amounts are displayed; nothing is converted.",
        fi: "Muuttaa vain summien esitystapaa; mitään ei muunneta.",
        sv: "Ändrar bara hur belopp visas; inget räknas om.",
        de: "Ändert nur die Darstellung der Beträge; es wird nichts umgerechnet.",
        fr: "Ne change que l'affichage des montants ; rien n'est converti.",
        es: "Solo cambia cómo se muestran los importes; no se convierte nada.",
    }

    // ---- about page ----
    AboutDescription {
        en: "Track your monthly income and expenses and see where the money goes as a Sankey chart.",
        fi: "Seuraa kuukausittaisia tulojasi ja menojasi ja näe Sankey-kaaviosta, minne raha menee.",
        sv: "Håll koll på dina månatliga inkomster och utgifter och se vart pengarna går i ett Sankey-diagram.",
        de: "Erfasse deine monatlichen Einnahmen und Ausgaben und sieh in einem Sankey-Diagramm, wohin das Geld fließt.",
        fr: "Suivez vos revenus et dépenses mensuels et voyez où va l'argent dans un diagramme de Sankey.",
        es: "Registra tus ingresos y gastos mensuales y mira adónde va el dinero en un diagrama de Sankey.",
    }
    Version {
        en: "Version {version}", fi: "Versio {version}", sv: "Version {version}", de: "Version {version}",
        fr: "Version {version}", es: "Versión {version}",
    }
    Author { en: "Author", fi: "Tekijä", sv: "Upphovsperson", de: "Autor", fr: "Auteur", es: "Autor" }
    License { en: "License", fi: "Lisenssi", sv: "Licens", de: "Lizenz", fr: "Licence", es: "Licencia" }
    Source { en: "Source", fi: "Lähdekoodi", sv: "Källkod", de: "Quellcode", fr: "Code source", es: "Código fuente" }
    DataFolder {
        en: "Data folder", fi: "Datakansio", sv: "Datamapp", de: "Datenordner", fr: "Dossier de données",
        es: "Carpeta de datos",
    }
    NotAvailable {
        en: "not available", fi: "ei saatavilla", sv: "inte tillgänglig", de: "nicht verfügbar",
        fr: "indisponible", es: "no disponible",
    }
    OpenOnGitHub {
        en: "Open on GitHub", fi: "Avaa GitHubissa", sv: "Öppna på GitHub", de: "Auf GitHub öffnen",
        fr: "Ouvrir sur GitHub", es: "Abrir en GitHub",
    }
    ThirdParty {
        en: "Built with Rust, GPUI and GPUI Kit (Apache-2.0). Icons from Lucide (ISC license).",
        fi: "Tehty Rustilla, GPUI:lla ja GPUI Kitillä (Apache-2.0). Kuvakkeet: Lucide (ISC-lisenssi).",
        sv: "Byggd med Rust, GPUI och GPUI Kit (Apache-2.0). Ikoner från Lucide (ISC-licens).",
        de: "Entwickelt mit Rust, GPUI und GPUI Kit (Apache-2.0). Symbole von Lucide (ISC-Lizenz).",
        fr: "Réalisé avec Rust, GPUI et GPUI Kit (Apache-2.0). Icônes de Lucide (licence ISC).",
        es: "Creado con Rust, GPUI y GPUI Kit (Apache-2.0). Iconos de Lucide (licencia ISC).",
    }

    // ---- notices and errors ----
    NoticeLabelsSaved {
        en: "Chart labels saved.", fi: "Kaavion nimet tallennettu.", sv: "Diagramtexterna sparades.",
        de: "Diagrammbeschriftungen gespeichert.", fr: "Libellés du graphique enregistrés.",
        es: "Etiquetas del gráfico guardadas.",
    }
    NoticeSaved {
        en: "Saved {path}", fi: "Tallennettu: {path}", sv: "Sparade {path}", de: "Gespeichert: {path}",
        fr: "Enregistré : {path}", es: "Guardado: {path}",
    }
    NoticeImported {
        en: "Import complete: {entries} into {period} ({newcats}).",
        fi: "Tuonti valmis: {entries}, {period} ({newcats}).",
        sv: "Importen klar: {entries}, {period} ({newcats}).",
        de: "Import abgeschlossen: {entries}, {period} ({newcats}).",
        fr: "Import terminé : {entries}, {period} ({newcats}).",
        es: "Importación completada: {entries}, {period} ({newcats}).",
    }
    ErrPickCategory {
        en: "Pick a category first (create one on the Categories page).",
        fi: "Valitse ensin kategoria (luo sellainen Kategoriat-sivulla).",
        sv: "Välj en kategori först (skapa en på sidan Kategorier).",
        de: "Wähle zuerst eine Kategorie (lege eine auf der Seite „Kategorien“ an).",
        fr: "Choisissez d'abord une catégorie (créez-en une dans la page Catégories).",
        es: "Elige primero una categoría (crea una en la página Categorías).",
    }
    ErrPositiveAmount {
        en: "Enter a positive amount, e.g. {example}.", fi: "Anna positiivinen summa, esim. {example}.",
        sv: "Ange ett positivt belopp, t.ex. {example}.", de: "Gib einen positiven Betrag ein, z. B. {example}.",
        fr: "Saisissez un montant positif, p. ex. {example}.", es: "Introduce un importe positivo, p. ej. {example}.",
    }
    ErrClipboardEmpty {
        en: "The clipboard has no text. Copy SankeyMATIC text first.",
        fi: "Leikepöydällä ei ole tekstiä. Kopioi ensin SankeyMATIC-teksti.",
        sv: "Urklippet innehåller ingen text. Kopiera först SankeyMATIC-text.",
        de: "Die Zwischenablage enthält keinen Text. Kopiere zuerst SankeyMATIC-Text.",
        fr: "Le presse-papiers ne contient pas de texte. Copiez d'abord du texte SankeyMATIC.",
        es: "El portapapeles no contiene texto. Copia primero texto de SankeyMATIC.",
    }
    ErrCatNameEmpty {
        en: "Category name cannot be empty.", fi: "Kategorian nimi ei voi olla tyhjä.",
        sv: "Kategorinamnet får inte vara tomt.", de: "Der Kategoriename darf nicht leer sein.",
        fr: "Le nom de la catégorie ne peut pas être vide.", es: "El nombre de la categoría no puede estar vacío.",
    }
    ErrCatExists {
        en: "A category with that name already exists.", fi: "Samanniminen kategoria on jo olemassa.",
        sv: "En kategori med det namnet finns redan.", de: "Eine Kategorie mit diesem Namen existiert bereits.",
        fr: "Une catégorie portant ce nom existe déjà.", es: "Ya existe una categoría con ese nombre.",
    }
    ErrParent {
        en: "That parent is not allowed for this category.", fi: "Tämä yläkategoria ei kelpaa tälle kategorialle.",
        sv: "Den överordnade kategorin är inte tillåten här.", de: "Diese übergeordnete Kategorie ist nicht zulässig.",
        fr: "Cette catégorie parente n'est pas autorisée.", es: "Esa categoría superior no está permitida aquí.",
    }
    ErrCannotDelete {
        en: "Cannot delete the only category that still has entries.",
        fi: "Ainoaa kategoriaa, jolla vielä on merkintöjä, ei voi poistaa.",
        sv: "Den enda kategorin som fortfarande har poster kan inte tas bort.",
        de: "Die einzige Kategorie mit Einträgen kann nicht gelöscht werden.",
        fr: "Impossible de supprimer la seule catégorie qui contient encore des entrées.",
        es: "No se puede eliminar la única categoría que aún tiene movimientos.",
    }
    ErrNoDataDir {
        en: "No data directory available; changes will not be saved.",
        fi: "Datakansiota ei ole saatavilla; muutoksia ei tallenneta.",
        sv: "Ingen datamapp tillgänglig; ändringar sparas inte.",
        de: "Kein Datenordner verfügbar; Änderungen werden nicht gespeichert.",
        fr: "Aucun dossier de données disponible ; les modifications ne seront pas enregistrées.",
        es: "No hay carpeta de datos disponible; los cambios no se guardarán.",
    }
    ErrSaveFailed {
        en: "Saving failed: {error}", fi: "Tallennus epäonnistui: {error}", sv: "Det gick inte att spara: {error}",
        de: "Speichern fehlgeschlagen: {error}", fr: "Échec de l'enregistrement : {error}",
        es: "Error al guardar: {error}",
    }
    ErrSaveMonthFailed {
        en: "Saving {month} failed: {error}", fi: "Kuukauden {month} tallennus epäonnistui: {error}",
        sv: "Det gick inte att spara {month}: {error}", de: "Speichern von {month} fehlgeschlagen: {error}",
        fr: "Échec de l'enregistrement de {month} : {error}", es: "Error al guardar {month}: {error}",
    }
    ErrSaveSettingsFailed {
        en: "Saving settings failed: {error}", fi: "Asetusten tallennus epäonnistui: {error}",
        sv: "Det gick inte att spara inställningarna: {error}",
        de: "Speichern der Einstellungen fehlgeschlagen: {error}",
        fr: "Échec de l'enregistrement des paramètres : {error}", es: "Error al guardar los ajustes: {error}",
    }
    ErrSaveFileFailed {
        en: "Could not save the file: {error}", fi: "Tiedoston tallennus epäonnistui: {error}",
        sv: "Det gick inte att spara filen: {error}", de: "Datei konnte nicht gespeichert werden: {error}",
        fr: "Impossible d'enregistrer le fichier : {error}", es: "No se pudo guardar el archivo: {error}",
    }
    ErrReadFileFailed {
        en: "Could not read the file: {error}", fi: "Tiedoston lukeminen epäonnistui: {error}",
        sv: "Det gick inte att läsa filen: {error}", de: "Datei konnte nicht gelesen werden: {error}",
        fr: "Impossible de lire le fichier : {error}", es: "No se pudo leer el archivo: {error}",
    }
    ErrImportFailed {
        en: "Import failed: {error}", fi: "Tuonti epäonnistui: {error}", sv: "Importen misslyckades: {error}",
        de: "Import fehlgeschlagen: {error}", fr: "Échec de l'import : {error}", es: "Error al importar: {error}",
    }

    // ---- import parser messages ----
    ImpNoFlows {
        en: "No flows found. Expected lines like: Salary [3000] Total income",
        fi: "Virtoja ei löytynyt. Odotettuja rivejä: Salary [3000] Total income",
        sv: "Inga flöden hittades. Förväntade rader som: Salary [3000] Total income",
        de: "Keine Flüsse gefunden. Erwartet werden Zeilen wie: Salary [3000] Total income",
        fr: "Aucun flux trouvé. Lignes attendues, par exemple : Salary [3000] Total income",
        es: "No se encontraron flujos. Se esperan líneas como: Salary [3000] Total income",
    }
    ImpNoHub {
        en: "Could not find a middle node that has income flowing in and expenses flowing out (like \"Total income\").",
        fi: "Keskisolmua, johon tulot virtaavat sisään ja josta menot virtaavat ulos (kuten \"Total income\"), ei löytynyt.",
        sv: "Hittade ingen mittnod där inkomster flödar in och utgifter flödar ut (som \"Total income\").",
        de: "Es wurde kein Mittelknoten gefunden, in den Einnahmen fließen und aus dem Ausgaben fließen (wie \"Total income\").",
        fr: "Aucun nœud central trouvé où arrivent les revenus et d'où partent les dépenses (comme \"Total income\").",
        es: "No se encontró un nodo central al que lleguen ingresos y del que salgan gastos (como \"Total income\").",
    }
    ImpNothing {
        en: "The text has flows, but nothing that could be imported as income or expenses.",
        fi: "Tekstissä on virtoja, mutta mitään ei voi tuoda tuloina tai menoina.",
        sv: "Texten har flöden, men inget som kan importeras som inkomster eller utgifter.",
        de: "Der Text enthält Flüsse, aber nichts, das als Einnahmen oder Ausgaben importiert werden kann.",
        fr: "Le texte contient des flux, mais rien qui puisse être importé comme revenus ou dépenses.",
        es: "El texto tiene flujos, pero nada que se pueda importar como ingresos o gastos.",
    }
    ImpLineSkippedNodes {
        en: "Line {line}: skipped (missing or identical source/target).",
        fi: "Rivi {line}: ohitettu (lähde tai kohde puuttuu tai ne ovat samat).",
        sv: "Rad {line}: hoppades över (källa/mål saknas eller är identiska).",
        de: "Zeile {line}: übersprungen (Quelle/Ziel fehlt oder ist identisch).",
        fr: "Ligne {line} : ignorée (source/cible manquante ou identique).",
        es: "Línea {line}: omitida (origen/destino ausente o idéntico).",
    }
    ImpLineSkippedAmount {
        en: "Line {line}: skipped (amount is not a positive number).",
        fi: "Rivi {line}: ohitettu (summa ei ole positiivinen luku).",
        sv: "Rad {line}: hoppades över (beloppet är inte ett positivt tal).",
        de: "Zeile {line}: übersprungen (Betrag ist keine positive Zahl).",
        fr: "Ligne {line} : ignorée (le montant n'est pas un nombre positif).",
        es: "Línea {line}: omitida (el importe no es un número positivo).",
    }
    ImpBothSides {
        en: "\"{name}\" is on both sides of the middle node and was skipped.",
        fi: "\"{name}\" on keskisolmun molemmin puolin ja ohitettiin.",
        sv: "\"{name}\" finns på båda sidor om mittnoden och hoppades över.",
        de: "\"{name}\" liegt auf beiden Seiten des Mittelknotens und wurde übersprungen.",
        fr: "\"{name}\" est des deux côtés du nœud central et a été ignoré.",
        es: "\"{name}\" está a ambos lados del nodo central y se omitió.",
    }
    ImpUnconnected {
        en: "Nodes not connected to \"{hub}\" were ignored: {n}.",
        fi: "Solmuja, joita ei ole yhdistetty solmuun \"{hub}\", ohitettiin: {n}.",
        sv: "Noder som inte är kopplade till \"{hub}\" ignorerades: {n}.",
        de: "Nicht mit \"{hub}\" verbundene Knoten wurden ignoriert: {n}.",
        fr: "Nœuds non reliés à \"{hub}\" ignorés : {n}.",
        es: "Nodos no conectados a \"{hub}\" omitidos: {n}.",
    }
    ImpSeveral {
        en: "\"{name}\" is linked to several nodes; only the largest flow was imported.",
        fi: "\"{name}\" on yhdistetty useampaan solmuun; vain suurin virta tuotiin.",
        sv: "\"{name}\" är kopplad till flera noder; bara det största flödet importerades.",
        de: "\"{name}\" ist mit mehreren Knoten verbunden; nur der größte Fluss wurde importiert.",
        fr: "\"{name}\" est relié à plusieurs nœuds ; seul le flux le plus important a été importé.",
        es: "\"{name}\" está conectado a varios nodos; solo se importó el flujo mayor.",
    }
    ImpUnplaced {
        en: "\"{name}\" could not be placed in the category tree and was skipped.",
        fi: "\"{name}\" ei mahtunut kategoriapuuhun ja ohitettiin.",
        sv: "\"{name}\" kunde inte placeras i kategoriträdet och hoppades över.",
        de: "\"{name}\" konnte nicht in den Kategoriebaum eingeordnet werden und wurde übersprungen.",
        fr: "\"{name}\" n'a pas pu être placé dans l'arbre des catégories et a été ignoré.",
        es: "\"{name}\" no se pudo colocar en el árbol de categorías y se omitió.",
    }
    ImpChildrenExceed {
        en: "\"{name}\": sub-categories add up to more than the node itself.",
        fi: "\"{name}\": alakategoriat ovat yhteensä suuremmat kuin solmu itse.",
        sv: "\"{name}\": underkategorierna summerar till mer än noden själv.",
        de: "\"{name}\": Die Unterkategorien ergeben mehr als der Knoten selbst.",
        fr: "\"{name}\" : les sous-catégories totalisent plus que le nœud lui-même.",
        es: "\"{name}\": las subcategorías suman más que el propio nodo.",
    }

    // ---- defaults created on first run ----
    DefSalary { en: "Salary", fi: "Palkka", sv: "Lön", de: "Gehalt", fr: "Salaire", es: "Salario" }
    DefOtherIncome {
        en: "Other income", fi: "Muut tulot", sv: "Övriga inkomster", de: "Sonstige Einnahmen",
        fr: "Autres revenus", es: "Otros ingresos",
    }
    DefHousing { en: "Housing", fi: "Asuminen", sv: "Boende", de: "Wohnen", fr: "Logement", es: "Vivienda" }
    DefFood { en: "Food", fi: "Ruoka", sv: "Mat", de: "Essen", fr: "Alimentation", es: "Alimentación" }
    DefTransport { en: "Transport", fi: "Liikenne", sv: "Transport", de: "Verkehr", fr: "Transports", es: "Transporte" }
    DefLeisure { en: "Leisure", fi: "Vapaa-aika", sv: "Fritid", de: "Freizeit", fr: "Loisirs", es: "Ocio" }
    DefLabelTotal {
        en: "Total income", fi: "Kokonaistulot", sv: "Totala inkomster", de: "Gesamteinnahmen",
        fr: "Revenus totaux", es: "Ingresos totales",
    }
    DefLabelSavings { en: "Savings", fi: "Säästöt", sv: "Sparande", de: "Ersparnisse", fr: "Épargne", es: "Ahorros" }
    DefLabelDeficit { en: "Deficit", fi: "Alijäämä", sv: "Underskott", de: "Defizit", fr: "Déficit", es: "Déficit" }
}

/// Messages with a singular and a plural form. All six languages have exactly
/// these two forms (French treats 0 like 1); a language with more forms, such
/// as Russian or Polish, would need a richer rule here.
macro_rules! plurals {
    ($($key:ident {
        en: ($en1:expr, $en2:expr), fi: ($fi1:expr, $fi2:expr), sv: ($sv1:expr, $sv2:expr),
        de: ($de1:expr, $de2:expr), fr: ($fr1:expr, $fr2:expr), es: ($es1:expr, $es2:expr) $(,)?
    })*) => {
        /// A counted phrase such as "3 entries"; the text holds `{n}`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Plural { $($key),* }

        impl Plural {
            #[cfg(test)]
            pub const ALL: &'static [Plural] = &[$(Plural::$key),*];

            /// The (singular, plural) texts for `lang`.
            pub fn forms(self, lang: Lang) -> (&'static str, &'static str) {
                match self {
                    $(Plural::$key => match lang {
                        Lang::En => ($en1, $en2),
                        Lang::Fi => ($fi1, $fi2),
                        Lang::Sv => ($sv1, $sv2),
                        Lang::De => ($de1, $de2),
                        Lang::Fr => ($fr1, $fr2),
                        Lang::Es => ($es1, $es2),
                    }),*
                }
            }
        }
    };
}

/// Whether `n` takes the singular form in `lang`.
fn takes_singular(lang: Lang, n: i64) -> bool {
    match lang {
        Lang::Fr => n == 0 || n == 1,
        _ => n == 1,
    }
}

impl Plural {
    /// The phrase for `n`, e.g. "1 entry" or "3 entries".
    pub fn text(self, lang: Lang, n: i64) -> String {
        let (one, other) = self.forms(lang);
        let template = if takes_singular(lang, n) { one } else { other };
        template.replace("{n}", &n.to_string())
    }
}

plurals! {
    CountCategories {
        en: ("{n} category", "{n} categories"), fi: ("{n} kategoria", "{n} kategoriaa"),
        sv: ("{n} kategori", "{n} kategorier"), de: ("{n} Kategorie", "{n} Kategorien"),
        fr: ("{n} catégorie", "{n} catégories"), es: ("{n} categoría", "{n} categorías"),
    }
    CountEntries {
        en: ("{n} entry", "{n} entries"), fi: ("{n} merkintä", "{n} merkintää"),
        sv: ("{n} post", "{n} poster"), de: ("{n} Eintrag", "{n} Einträge"),
        fr: ("{n} entrée", "{n} entrées"), es: ("{n} movimiento", "{n} movimientos"),
    }
    CountNewCategories {
        en: ("{n} new category", "{n} new categories"), fi: ("{n} uusi kategoria", "{n} uutta kategoriaa"),
        sv: ("{n} ny kategori", "{n} nya kategorier"), de: ("{n} neue Kategorie", "{n} neue Kategorien"),
        fr: ("{n} nouvelle catégorie", "{n} nouvelles catégories"),
        es: ("{n} categoría nueva", "{n} categorías nuevas"),
    }
    CountMoreNotes {
        en: ("{n} more note", "{n} more notes"), fi: ("{n} muu huomautus", "{n} muuta huomautusta"),
        sv: ("{n} anteckning till", "{n} anteckningar till"), de: ("{n} weiterer Hinweis", "{n} weitere Hinweise"),
        fr: ("{n} autre remarque", "{n} autres remarques"), es: ("{n} nota más", "{n} notas más"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholders(s: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut rest = s;
        while let Some(start) = rest.find('{') {
            let Some(end) = rest[start..].find('}') else { break };
            out.push(rest[start + 1..start + end].to_string());
            rest = &rest[start + end + 1..];
        }
        out.sort();
        out
    }

    #[test]
    fn every_translation_is_filled_and_keeps_its_placeholders() {
        for &msg in Msg::ALL {
            let english = placeholders(msg.text(Lang::En));
            for lang in Lang::ALL {
                let text = msg.text(lang);
                assert!(!text.trim().is_empty(), "{msg:?} is empty in {lang:?}");
                assert_eq!(placeholders(text), english, "{msg:?} placeholders differ in {lang:?}: {text}");
            }
        }
    }

    fn loc(lang: Lang, currency: Currency) -> Locale {
        Locale { lang, currency }
    }

    #[test]
    fn formats_money_per_language() {
        let e = Currency::Eur;
        assert_eq!(loc(Lang::En, e).money(123_456), "€1,234.56");
        assert_eq!(loc(Lang::Fi, e).money(123_456), "1\u{a0}234,56\u{a0}€");
        assert_eq!(loc(Lang::Sv, e).money(123_456), "1\u{a0}234,56\u{a0}€");
        assert_eq!(loc(Lang::De, e).money(123_456), "1.234,56\u{a0}€");
        assert_eq!(loc(Lang::Fr, e).money(123_456), "1\u{202f}234,56\u{a0}€");
        // Spanish does not group four-digit numbers, but groups from five digits.
        assert_eq!(loc(Lang::Es, e).money(123_456), "1234,56\u{a0}€");
        assert_eq!(loc(Lang::Es, e).money(1_234_567), "12.345,67\u{a0}€");
        // Small and negative amounts.
        assert_eq!(loc(Lang::En, e).money(5), "€0.05");
        assert_eq!(loc(Lang::En, e).money(-500), "-€5.00");
        assert_eq!(loc(Lang::Fi, e).money(-500), "−5,00\u{a0}€");
        assert_eq!(loc(Lang::De, e).money(-500), "-5,00\u{a0}€");
    }

    #[test]
    fn places_other_currencies_sensibly() {
        assert_eq!(loc(Lang::En, Currency::Usd).money(500), "$5.00");
        assert_eq!(loc(Lang::En, Currency::Gbp).money(500), "£5.00");
        assert_eq!(loc(Lang::En, Currency::Sek).money(500), "SEK\u{a0}5.00");
        assert_eq!(loc(Lang::Sv, Currency::Sek).money(500), "5,00\u{a0}kr");
        assert_eq!(loc(Lang::De, Currency::Chf).money(500), "5,00\u{a0}CHF");
    }

    #[test]
    fn decimal_marks_match_the_formatted_output() {
        for lang in Lang::ALL {
            let l = loc(lang, Currency::Eur);
            let mark = l.decimal_mark();
            assert!(l.plain_amount(1250).contains(mark), "{lang:?}");
            // What the app shows can always be typed back in.
            assert_eq!(crate::model::parse_amount_cents(&l.plain_amount(123_456), mark), Some(123_456), "{lang:?}");
            assert_eq!(
                crate::model::parse_amount_cents(&l.money(123_456).replace(l.currency_symbol(), ""), mark),
                Some(123_456),
                "{lang:?}"
            );
        }
    }

    #[test]
    fn formats_whole_and_plain_amounts() {
        assert_eq!(loc(Lang::En, Currency::Eur).money_whole(1234.6), "€1,235");
        assert_eq!(loc(Lang::Fi, Currency::Eur).money_whole(1234.4), "1\u{a0}234\u{a0}€");
        assert_eq!(loc(Lang::Fi, Currency::Eur).plain_amount(1250), "12,50");
        assert_eq!(loc(Lang::En, Currency::Eur).plain_amount(1250), "12.50");
    }

    #[test]
    fn month_titles_follow_the_language() {
        let nov = MonthKey::new(2026, 11).unwrap();
        assert_eq!(loc(Lang::En, Currency::Eur).month_title(nov), "November 2026");
        assert_eq!(loc(Lang::Fi, Currency::Eur).month_title(nov), "Marraskuu 2026");
        assert_eq!(loc(Lang::Es, Currency::Eur).month_title(nov), "Noviembre de 2026");
        assert_eq!(loc(Lang::Fr, Currency::Eur).month_title(MonthKey::new(2026, 8).unwrap()), "Août 2026");
        assert_eq!(loc(Lang::De, Currency::Eur).month_short(3), "Mär");
    }

    #[test]
    fn plural_phrases_have_both_forms_in_every_language() {
        for &plural in Plural::ALL {
            for lang in Lang::ALL {
                let (one, other) = plural.forms(lang);
                assert!(one.contains("{n}") && other.contains("{n}"), "{plural:?} {lang:?}");
                assert_ne!(one, other, "{plural:?} has identical forms in {lang:?}");
            }
        }
    }

    #[test]
    fn picks_the_right_plural_form() {
        let en = loc(Lang::En, Currency::Eur);
        assert_eq!(en.count(Plural::CountEntries, 1), "1 entry");
        assert_eq!(en.count(Plural::CountEntries, 0), "0 entries");
        assert_eq!(en.count(Plural::CountEntries, 5), "5 entries");
        let fi = loc(Lang::Fi, Currency::Eur);
        assert_eq!(fi.count(Plural::CountCategories, 1), "1 kategoria");
        assert_eq!(fi.count(Plural::CountCategories, 2), "2 kategoriaa");
        assert_eq!(fi.count(Plural::CountMoreNotes, 1), "1 muu huomautus");
        // French treats zero like one.
        let fr = loc(Lang::Fr, Currency::Eur);
        assert_eq!(fr.count(Plural::CountEntries, 0), "0 entrée");
        assert_eq!(fr.count(Plural::CountEntries, 2), "2 entrées");
        assert_eq!(loc(Lang::Es, Currency::Eur).count(Plural::CountNewCategories, 3), "3 categorías nuevas");
    }

    #[test]
    fn months_inside_sentences_follow_each_languages_capitalization() {
        let nov = MonthKey::new(2026, 11).unwrap();
        let aug = MonthKey::new(2026, 8).unwrap();
        assert_eq!(loc(Lang::En, Currency::Eur).month_inline(nov), "November 2026");
        assert_eq!(loc(Lang::De, Currency::Eur).month_inline(nov), "November 2026");
        assert_eq!(loc(Lang::Fi, Currency::Eur).month_inline(nov), "marraskuu 2026");
        assert_eq!(loc(Lang::Fr, Currency::Eur).month_inline(aug), "août 2026");
        assert_eq!(loc(Lang::Es, Currency::Eur).month_inline(nov), "noviembre de 2026");
        // Headings still start with a capital letter.
        assert_eq!(loc(Lang::Fr, Currency::Eur).month_title(aug), "Août 2026");
    }

    #[test]
    fn composed_sentences_read_correctly_for_one_and_many() {
        let compose = |lang: Lang, cats: usize, entries: usize| {
            let l = loc(lang, Currency::Eur);
            l.tf(
                Msg::ImportFound,
                &[
                    ("cats", &l.count(Plural::CountCategories, cats)),
                    ("entries", &l.count(Plural::CountEntries, entries)),
                    ("income", "A"),
                    ("expenses", "B"),
                ],
            )
        };
        assert_eq!(compose(Lang::En, 1, 1), "Found 1 category and 1 entry: income A, expenses B.");
        assert_eq!(compose(Lang::En, 3, 12), "Found 3 categories and 12 entries: income A, expenses B.");
        assert_eq!(compose(Lang::Fi, 1, 1), "Löytyi 1 kategoria ja 1 merkintä: tulot A, menot B.");
        assert_eq!(compose(Lang::Fi, 2, 5), "Löytyi 2 kategoriaa ja 5 merkintää: tulot A, menot B.");
        assert_eq!(compose(Lang::De, 1, 2), "1 Kategorie und 2 Einträge gefunden: Einnahmen A, Ausgaben B.");

        // No month name is inflected: the period is a plain value in a fixed frame.
        let fr = loc(Lang::Fr, Currency::Eur);
        let period = fr.month_inline(MonthKey::new(2026, 4).unwrap());
        assert_eq!(
            fr.tf(Msg::ImportReplace, &[("entries", &fr.count(Plural::CountEntries, 3)), ("period", &period)]),
            "Remplacer 3 entrées (avril 2026)"
        );
        let fi = loc(Lang::Fi, Currency::Eur);
        assert_eq!(
            fi.tf(Msg::NoticeImported, &[("entries", "1 merkintä"), ("period", "marraskuu 2026"), ("newcats", "0 uutta kategoriaa")]),
            "Tuonti valmis: 1 merkintä, marraskuu 2026 (0 uutta kategoriaa)."
        );
    }

    #[test]
    fn parses_locale_tags() {
        assert_eq!(Lang::from_tag("fi-FI"), Some(Lang::Fi));
        assert_eq!(Lang::from_tag("sv_SE.UTF-8"), Some(Lang::Sv));
        assert_eq!(Lang::from_tag("EN"), Some(Lang::En));
        assert_eq!(Lang::from_tag("ja-JP"), None);
        let s = SystemLocale::from_tag("en_GB.UTF-8");
        assert_eq!((s.lang, s.region.as_deref()), (Some(Lang::En), Some("GB")));
        assert_eq!(SystemLocale::from_tag("de").region, None);
    }

    #[test]
    fn picks_the_first_supported_preferred_language() {
        let tags = |list: &[&str]| list.iter().map(|t| t.to_string()).collect::<Vec<_>>();
        // Unsupported languages are skipped; the region comes from the matching tag.
        let s = SystemLocale::from_preferences(&tags(&["ja-JP", "sv-FI", "en-US"]), None);
        assert_eq!((s.lang, s.region.as_deref()), (Some(Lang::Sv), Some("FI")));
        // The first supported preference wins, not the last.
        let s = SystemLocale::from_preferences(&tags(&["en-GB", "fi-FI"]), None);
        assert_eq!((s.lang, s.region.as_deref()), (Some(Lang::En), Some("GB")));
        // A separately reported regional format beats the language tag's region.
        let s = SystemLocale::from_preferences(&tags(&["en-US"]), Some("FI".to_string()));
        assert_eq!((s.lang, s.region.as_deref()), (Some(Lang::En), Some("FI")));
        assert_eq!(Locale::resolve(None, None, &s).currency, Currency::Eur);
        // Nothing usable: no language, no region.
        let s = SystemLocale::from_preferences(&tags(&["ja-JP"]), None);
        assert_eq!((s.lang, s.region), (None, None));
        assert_eq!(SystemLocale::from_preferences(&[], None), SystemLocale::default());
    }

    #[test]
    fn detecting_the_system_locale_does_not_panic() {
        let _ = SystemLocale::detect();
    }

    #[test]
    fn resolves_language_and_currency() {
        let fi = SystemLocale::from_tag("fi-FI");
        assert_eq!(Locale::resolve(None, None, &fi), Locale { lang: Lang::Fi, currency: Currency::Eur });
        // The user's choice wins over the system, and currency follows the region.
        let us = SystemLocale::from_tag("en-US");
        assert_eq!(Locale::resolve(Some(Lang::De), None, &us), Locale { lang: Lang::De, currency: Currency::Usd });
        assert_eq!(
            Locale::resolve(None, Some(Currency::Gbp), &fi),
            Locale { lang: Lang::Fi, currency: Currency::Gbp }
        );
        // Unknown system language falls back to English and euros.
        let unknown = SystemLocale::from_tag("ja-JP");
        assert_eq!(Locale::resolve(None, None, &unknown), Locale { lang: Lang::En, currency: Currency::Eur });
        assert_eq!(Locale::resolve(Some(Lang::Sv), None, &SystemLocale::default()).currency, Currency::Sek);
    }

    #[test]
    fn replaces_named_placeholders() {
        let l = loc(Lang::Fi, Currency::Eur);
        assert_eq!(l.tf(Msg::CountThisMonth, &[("count", "12")]), "12 tässä kuussa");
    }
}
