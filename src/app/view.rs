//! Views: sidebar navigation, page header, cards and lists.

use super::*;
use gpui_kit::component::{Theme, ThemeMode};

/// Theme colors copied out of the context so view code can freely borrow `cx`.
#[derive(Clone, Copy)]
struct Pal {
    bg: Hsla,
    fg: Hsla,
    muted: Hsla,
    muted_fg: Hsla,
    border: Hsla,
    primary: Hsla,
    success: Hsla,
    danger: Hsla,
    sidebar: Hsla,
    sidebar_border: Hsla,
    accent: Hsla,
    accent_fg: Hsla,
    page: Hsla,
}

fn pal(cx: &App) -> Pal {
    let t = cx.theme();
    Pal {
        bg: t.background,
        fg: t.foreground,
        muted: t.muted,
        muted_fg: t.muted_foreground,
        border: t.border,
        primary: t.primary,
        success: t.success,
        danger: t.danger,
        sidebar: t.sidebar,
        sidebar_border: t.sidebar_border,
        accent: t.sidebar_accent,
        accent_fg: t.sidebar_accent_foreground,
        page: t.muted.opacity(0.5),
    }
}

fn hsla(color: u32) -> Hsla {
    gpui_kit::rgb(color).into()
}

fn card(p: Pal) -> Div {
    v_flex()
        .bg(p.bg)
        .border_1()
        .border_color(p.border)
        .rounded_lg()
        .p_4()
}

fn heading(text: impl Into<SharedString>) -> Div {
    div().font_weight(FontWeight::SEMIBOLD).child(text.into())
}

fn field(label: &'static str, p: Pal) -> Div {
    v_flex().gap_1().child(
        div()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted_fg)
            .child(label),
    )
}

fn section_label(text: &'static str, trailing: String, p: Pal) -> Div {
    h_flex()
        .justify_between()
        .mt_2()
        .px_2()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.muted_fg)
        .child(text)
        .child(trailing)
}

/// A selectable rounded chip, optionally with a color dot.
fn pill(
    id: SharedString,
    label: String,
    color: Option<u32>,
    selected: bool,
    p: Pal,
) -> Stateful<Div> {
    let tint = color.map(hsla).unwrap_or(p.primary);
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1()
        .rounded_full()
        .border_1()
        .text_sm()
        .cursor_pointer()
        .border_color(if selected { tint } else { p.border })
        .when(selected, |d| {
            d.bg(tint.opacity(0.15)).font_weight(FontWeight::MEDIUM)
        })
        .hover(|d| d.bg(p.muted))
        .children(color.map(|c| div().size_2().rounded_full().bg(hsla(c))))
        .child(label)
}

impl CashFlowApp {
    // ---- shell ----

    fn sidebar(&self, p: Pal, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let nav = |page: Page, glyph: &'static str, label: &'static str, cx: &mut Context<Self>| {
            let active = self.page == page;
            div()
                .id(label)
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .rounded_md()
                .text_sm()
                .cursor_pointer()
                .hover(|d| d.bg(p.accent))
                .when(active, |d| {
                    d.bg(p.accent)
                        .text_color(p.accent_fg)
                        .font_weight(FontWeight::SEMIBOLD)
                })
                .child(div().w_4().text_center().child(glyph))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.page = page;
                    this.error = None;
                    cx.notify();
                }))
        };

        let brand = h_flex()
            .gap_3()
            .items_center()
            .px_2()
            .pb_4()
            .child(img(self.logo.clone()).size_9().flex_shrink_0())
            .child(
                v_flex()
                    .child(heading("CashFlow"))
                    .child(div().text_xs().text_color(p.muted_fg).child("Monthly cash flow")),
            );

        v_flex()
            .w(px(220.0))
            .h_full()
            .flex_shrink_0()
            .p_4()
            .gap_1()
            .bg(p.sidebar)
            .border_r_1()
            .border_color(p.sidebar_border)
            .child(brand)
            .child(nav(Page::Overview, "◎", "Overview", cx))
            .child(nav(Page::Categories, "◆", "Categories", cx))
            .child(nav(Page::Settings, "⚙", "Settings", cx))
            .child(div().flex_1())
            .child(
                Button::new("theme-toggle")
                    .ghost()
                    .small()
                    .label(if dark { "☀  Light mode" } else { "☾  Dark mode" })
                    .on_click(cx.listener(move |_, _, window, cx| {
                        let mode = if dark { ThemeMode::Light } else { ThemeMode::Dark };
                        Theme::change(mode, Some(window), cx);
                    })),
            )
            .child(
                div()
                    .px_2()
                    .pt_2()
                    .text_xs()
                    .text_color(p.muted_fg)
                    .child("Changes are saved automatically"),
            )
    }

    fn header(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, sub) = match self.page {
            Page::Overview => ("Overview", "Where your money comes from and where it goes"),
            Page::Categories => ("Categories", "Organize income and spending your way"),
            Page::Settings => ("Settings", "Customize how the chart is labeled"),
        };
        let right = (self.page == Page::Overview).then(|| self.month_nav(p, cx));
        h_flex()
            .px_6()
            .py_4()
            .items_center()
            .justify_between()
            .bg(p.bg)
            .border_b_1()
            .border_color(p.border)
            .child(
                v_flex()
                    .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().text_sm().text_color(p.muted_fg).child(sub)),
            )
            .children(right)
    }

    fn month_nav(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .p_1()
                    .rounded_lg()
                    .border_1()
                    .border_color(p.border)
                    .child(Button::new("month-prev").ghost().small().label("◀").on_click(
                        cx.listener(|this, _, window, cx| {
                            this.switch_month(this.month.prev(), window, cx)
                        }),
                    ))
                    .child(
                        div()
                            .w(px(140.0))
                            .text_center()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.month.title()),
                    )
                    .child(Button::new("month-next").ghost().small().label("▶").on_click(
                        cx.listener(|this, _, window, cx| {
                            this.switch_month(this.month.next(), window, cx)
                        }),
                    )),
            )
            .when(self.month != MonthKey::current(), |row| {
                row.child(Button::new("month-today").outline().small().label("Today").on_click(
                    cx.listener(|this, _, window, cx| {
                        this.switch_month(MonthKey::current(), window, cx)
                    }),
                ))
            })
    }

    fn error_banner(&self, p: Pal, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (msg, color, id) = match (&self.error, &self.notice) {
            (Some(e), _) => (e.clone(), p.danger, "dismiss-error"),
            (None, Some(n)) => (n.clone(), p.success, "dismiss-notice"),
            (None, None) => return None,
        };
        Some(
            h_flex()
                .mx_6()
                .mt_4()
                .px_4()
                .py_2()
                .gap_3()
                .items_center()
                .rounded_md()
                .border_1()
                .border_color(color)
                .bg(color.opacity(0.08))
                .text_sm()
                .text_color(color)
                .child(div().flex_1().child(msg))
                .child(Button::new(id).ghost().small().label("✕").on_click(
                    cx.listener(|this, _, _, cx| {
                        this.error = None;
                        this.notice = None;
                        cx.notify();
                    }),
                )),
        )
    }

    /// Income / Expense switch. `for_category` selects which form it drives.
    fn segmented(&self, current: Kind, for_category: bool, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let locked = for_category && self.editing_cat.is_some();
        let seg = |kind: Kind, label: &'static str, cx: &mut Context<Self>| {
            let selected = current == kind;
            let accent = if kind == Kind::Income { p.success } else { p.danger };
            div()
                .id(SharedString::from(format!("seg-{for_category}-{label}")))
                .flex_1()
                .py_1()
                .rounded_md()
                .text_center()
                .text_sm()
                .when(!locked, |d| d.cursor_pointer())
                .when(selected, |d| {
                    d.bg(p.bg).shadow_sm().text_color(accent).font_weight(FontWeight::SEMIBOLD)
                })
                .when(!selected, |d| d.text_color(p.muted_fg))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if for_category {
                        this.set_cat_kind(kind, cx);
                    } else {
                        this.set_kind(kind, cx);
                    }
                }))
        };
        h_flex()
            .gap_1()
            .p_1()
            .rounded_lg()
            .bg(p.muted)
            .when(locked, |d| d.opacity(0.7))
            .child(seg(Kind::Income, "Income", cx))
            .child(seg(Kind::Expense, "Expense", cx))
    }

    // ---- overview ----

    fn overview(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_1()
            .min_h_0()
            .gap_4()
            .p_6()
            .child(
                v_flex()
                    .w(px(380.0))
                    .h_full()
                    .flex_shrink_0()
                    .gap_4()
                    .child(self.entry_form(p, cx))
                    .child(self.entry_list(p, cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .gap_4()
                    .children(self.import_panel(p, cx))
                    .child(self.summary(p))
                    .child(self.chart_card(p, cx)),
            )
    }

    fn entry_form(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let editing = self.editing.is_some();
        let pills: Vec<_> = self
            .cats
            .flatten(self.kind)
            .into_iter()
            .map(|(depth, c)| {
                let id = c.id;
                let label = format!("{}{}", "› ".repeat(depth), c.name);
                pill(id_of("cat", id), label, Some(c.color), self.category == Some(id), p)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.category = Some(id);
                        cx.notify();
                    }))
            })
            .collect();
        let no_categories = pills.is_empty();

        card(p)
            .gap_3()
            .child(heading(if editing { "Edit entry" } else { "Add entry" }))
            .child(self.segmented(self.kind, false, p, cx))
            .child(field("Category", p).child(if no_categories {
                div()
                    .text_sm()
                    .text_color(p.muted_fg)
                    .child("No categories yet. Create one on the Categories page.")
                    .into_any_element()
            } else {
                h_flex().flex_wrap().gap_2().children(pills).into_any_element()
            }))
            .child(
                h_flex()
                    .gap_2()
                    .items_end()
                    .child(field("Description", p).flex_1().child(Input::new(&self.name_in)))
                    .child(field("Amount (€)", p).w(px(110.0)).child(Input::new(&self.amount_in))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("submit")
                            .primary()
                            .label(if editing { "Save changes" } else { "Add entry" })
                            .on_click(cx.listener(|this, _, window, cx| this.submit_entry(window, cx))),
                    )
                    .when(editing, |row| {
                        row.child(Button::new("cancel").outline().label("Cancel").on_click(
                            cx.listener(|this, _, window, cx| {
                                this.clear_inputs(window, cx);
                                cx.notify();
                            }),
                        ))
                    }),
            )
    }

    fn entry_list(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex().gap_1();
        let mut count = 0;
        for kind in [Kind::Income, Kind::Expense] {
            let items: Vec<&Entry> = self
                .entries
                .iter()
                .filter(|e| self.cats.get(e.category_id).is_some_and(|c| c.kind == kind))
                .collect();
            if items.is_empty() {
                continue;
            }
            count += items.len();
            let total: i64 = items.iter().map(|e| e.amount_cents).sum();
            let (label, sign) = match kind {
                Kind::Income => ("INCOME", "+"),
                Kind::Expense => ("EXPENSES", "−"),
            };
            body = body.child(section_label(label, format!("{sign}{}", money(total)), p));
            for e in items {
                body = body.child(self.entry_row(e, kind, p, cx));
            }
        }
        if count == 0 {
            body = body.child(
                div()
                    .py_6()
                    .text_sm()
                    .text_center()
                    .text_color(p.muted_fg)
                    .child("Nothing recorded for this month yet."),
            );
        }
        card(p)
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(heading("Entries"))
                    .child(div().text_xs().text_color(p.muted_fg).child(format!("{count} this month"))),
            )
            .child(div().id("entries-scroll").flex_1().min_h_0().overflow_y_scroll().child(body))
    }

    fn entry_row(&self, e: &Entry, kind: Kind, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let id = e.id;
        let cat = self.cats.get(e.category_id);
        let color = cat.map_or(0x888888, |c| c.color);
        let cat_name = cat.map(|c| c.name.clone()).unwrap_or_default();
        let (title, subtitle) = if e.name.is_empty() {
            (cat_name, String::new())
        } else {
            (e.name.clone(), cat_name)
        };
        let (sign, amount_color) = match kind {
            Kind::Income => ("+", p.success),
            Kind::Expense => ("−", p.fg),
        };
        h_flex()
            .id(id_of("row", id))
            .gap_3()
            .items_center()
            .px_2()
            .py_1()
            .rounded_md()
            .hover(|d| d.bg(p.muted))
            .child(div().size_3().flex_shrink_0().rounded_full().bg(hsla(color)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .when(!subtitle.is_empty(), |c| {
                        c.child(div().text_xs().text_color(p.muted_fg).child(subtitle))
                    }),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(amount_color)
                    .child(format!("{sign}{}", money(e.amount_cents))),
            )
            .child(Button::new(id_of("edit", id)).ghost().small().label("Edit").on_click(
                cx.listener(move |this, _, window, cx| this.edit_entry(id, window, cx)),
            ))
            .child(Button::new(id_of("del", id)).ghost().small().label("✕").on_click(
                cx.listener(move |this, _, window, cx| this.delete_entry(id, window, cx)),
            ))
    }

    /// Confirmation card for a parsed SankeyMATIC import.
    fn import_panel(&self, p: Pal, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let plan = self.pending_import.as_ref()?;
        let existing = self.entries.len();
        let month = self.month.title();
        let mut warnings: Vec<_> = plan
            .warnings
            .iter()
            .take(4)
            .map(|w| div().text_xs().text_color(p.muted_fg).child(format!("• {w}")))
            .collect();
        if plan.warnings.len() > 4 {
            warnings.push(
                div()
                    .text_xs()
                    .text_color(p.muted_fg)
                    .child(format!("• …and {} more notes", plan.warnings.len() - 4)),
            );
        }
        Some(
            card(p)
                .gap_2()
                .border_color(p.primary)
                .child(heading("Import from SankeyMATIC"))
                .child(div().text_sm().child(format!(
                    "Found {} categories and {} entries: income {}, expenses {}.",
                    plan.categories.len(),
                    plan.entries.len(),
                    money(plan.income_cents),
                    money(plan.expense_cents),
                )))
                .child(div().text_xs().text_color(p.muted_fg).child(
                    "Categories with the same name are reused; the others are created.",
                ))
                .children(warnings)
                .child(
                    h_flex()
                        .gap_2()
                        .pt_1()
                        .child(
                            Button::new("import-replace")
                                .primary()
                                .label(if existing > 0 {
                                    format!("Replace {existing} entries in {month}")
                                } else {
                                    format!("Import into {month}")
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.confirm_import(true, window, cx)
                                })),
                        )
                        .when(existing > 0, |row| {
                            row.child(
                                Button::new("import-add")
                                    .outline()
                                    .label("Add to existing")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_import(false, window, cx)
                                    })),
                            )
                        })
                        .child(
                            Button::new("import-cancel")
                                .ghost()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_import(cx))),
                        ),
                ),
        )
    }

    fn summary(&self, p: Pal) -> impl IntoElement {
        let d = &self.data;
        let balance = d.balance_cents();
        let (balance_label, balance_color, balance_note) = if d.income_cents == 0 && d.expense_cents == 0 {
            ("Balance", p.muted_fg, "No data yet".to_string())
        } else if balance >= 0 {
            let rate = if d.income_cents > 0 { balance * 100 / d.income_cents } else { 0 };
            ("Savings", p.success, format!("{rate}% of income saved"))
        } else {
            ("Deficit", p.danger, "Spending exceeds income".to_string())
        };
        h_flex()
            .gap_4()
            .child(stat_card("Income", money(d.income_cents), p.success, "Total earned".to_string(), p))
            .child(stat_card("Expenses", money(d.expense_cents), p.danger, "Total spent".to_string(), p))
            .child(stat_card(balance_label, money(balance.abs()), balance_color, balance_note, p))
    }

    fn chart_card(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let d = &self.data;
        let body = if d.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_1()
                .child(div().text_color(p.fg).font_weight(FontWeight::MEDIUM).child(format!("No entries for {}", self.month.title())))
                .child(
                    div()
                        .text_sm()
                        .text_color(p.muted_fg)
                        .child("Add income and expenses on the left to see how your money flows."),
                )
                .into_any_element()
        } else {
            let nodes: Vec<FlowNode> = d.nodes.clone();
            let links = d.links.iter().map(|l| SankeyLink::new(l.source, l.target, l.value));
            div()
                .flex_1()
                .min_h_0()
                .child(
                    SankeyChart::new(nodes, links)
                        // Left alignment keeps all direct children of the middle node in one
                        // column, so flows never run underneath other nodes.
                        .node_align(SankeyAlign::Left)
                        // Room for the two-line label of small neighbouring nodes.
                        .node_padding(34.0)
                        .node_label(|n| n.label.clone().into())
                        .node_color(|n| gpui_kit::rgb(n.color))
                        .value_label(|_, v| format!("€{}", group(v.round() as i64)).into()),
                )
                .into_any_element()
        };
        card(p)
            .flex_1()
            .min_h_0()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(heading("Money flow"))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .when(!d.is_empty(), |row| {
                                row.child(div().text_xs().text_color(p.muted_fg).child("Export"))
                                    .child(
                                        Button::new("export-copy")
                                            .outline()
                                            .small()
                                            .label(if self.copied { "Copied ✓" } else { "Copy" })
                                            .tooltip("Copy as SankeyMATIC text (paste at sankeymatic.com)")
                                            .on_click(cx.listener(|this, _, _, cx| this.copy_sankeymatic(cx))),
                                    )
                                    .child(
                                        Button::new("export-save")
                                            .outline()
                                            .small()
                                            .label("Save .txt")
                                            .tooltip("Save the SankeyMATIC text to a file")
                                            .on_click(cx.listener(|this, _, _, cx| this.save_sankeymatic_file(cx))),
                                    )
                                    .child(div().w_px().h_5().mx_1().bg(p.border))
                            })
                            .child(div().text_xs().text_color(p.muted_fg).child("Import"))
                            .child(
                                Button::new("import-paste")
                                    .outline()
                                    .small()
                                    .label("Paste")
                                    .tooltip("Import SankeyMATIC text from the clipboard")
                                    .on_click(cx.listener(|this, _, _, cx| this.paste_import(cx))),
                            )
                            .child(
                                Button::new("import-open")
                                    .outline()
                                    .small()
                                    .label("Open .txt")
                                    .tooltip("Import SankeyMATIC text from a file")
                                    .on_click(cx.listener(|this, _, _, cx| this.open_import_file(cx))),
                            ),
                    ),
            )
            .child(body)
    }

    // ---- settings ----

    fn settings_page(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex().flex_1().min_h_0().items_start().p_6().child(
            v_flex().w(px(520.0)).child(
                card(p)
                    .gap_3()
                    .child(heading("Chart labels"))
                    .child(div().text_sm().text_color(p.muted_fg).child(
                        "Names of the automatic nodes in the money flow chart. They are also \
                         used in the SankeyMATIC export and recognized on import.",
                    ))
                    .child(field("Middle node (all income flows in here)", p).child(Input::new(&self.total_in)))
                    .child(field("When income exceeds expenses", p).child(Input::new(&self.savings_in)))
                    .child(field("When expenses exceed income", p).child(Input::new(&self.deficit_in)))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("labels-save").primary().label("Save").on_click(
                                cx.listener(|this, _, window, cx| this.save_labels(window, cx)),
                            ))
                            .child(Button::new("labels-reset").outline().label("Reset to defaults").on_click(
                                cx.listener(|this, _, window, cx| this.reset_labels(window, cx)),
                            )),
                    ),
            ),
        )
    }

    // ---- categories ----

    fn categories_page(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_1()
            .min_h_0()
            .gap_4()
            .p_6()
            .child(
                v_flex()
                    .w(px(380.0))
                    .h_full()
                    .flex_shrink_0()
                    .child(self.category_form(p, cx)),
            )
            .child(self.category_list(p, cx))
    }

    fn category_form(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let editing = self.editing_cat.is_some();

        let parents: Vec<_> = std::iter::once((None, "No parent".to_string(), None))
            .chain(
                self.cats
                    .flatten(self.cat_kind)
                    .into_iter()
                    .filter(|(_, c)| self.editing_cat.is_none_or(|e| !self.cats.is_within(c.id, e)))
                    .map(|(d, c)| (Some(c.id), format!("{}{}", "› ".repeat(d), c.name), Some(c.color))),
            )
            .map(|(pid, label, color)| {
                pill(id_of("parent", pid.unwrap_or(0)), label, color, self.cat_parent == pid, p)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.cat_parent = pid;
                        cx.notify();
                    }))
            })
            .collect();

        let swatches: Vec<_> = PALETTE
            .iter()
            .map(|&color| {
                let selected = self.cat_color == color;
                div()
                    .id(id_of("swatch", color as u64))
                    .size_6()
                    .rounded_full()
                    .bg(hsla(color))
                    .border_2()
                    .border_color(if selected { p.fg } else { p.bg })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.cat_color = color;
                        cx.notify();
                    }))
            })
            .collect();

        card(p)
            .gap_3()
            .child(heading(if editing { "Edit category" } else { "New category" }))
            .child(self.segmented(self.cat_kind, true, p, cx))
            .child(field("Name", p).child(Input::new(&self.cat_name_in)))
            .child(field("Color", p).child(h_flex().flex_wrap().gap_2().children(swatches)))
            .child(field("Parent", p).child(h_flex().flex_wrap().gap_2().children(parents)))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("add-cat")
                            .primary()
                            .label(if editing { "Save changes" } else { "Add category" })
                            .on_click(cx.listener(|this, _, window, cx| this.submit_category(window, cx))),
                    )
                    .when(editing, |row| {
                        row.child(Button::new("cancel-cat").outline().label("Cancel").on_click(
                            cx.listener(|this, _, window, cx| {
                                this.clear_category_form(window, cx);
                                cx.notify();
                            }),
                        ))
                    }),
            )
    }

    fn category_list(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex().gap_1();
        for kind in [Kind::Income, Kind::Expense] {
            let flat = self.cats.flatten(kind);
            let label = if kind == Kind::Income { "INCOME" } else { "EXPENSES" };
            body = body.child(section_label(label, format!("{}", flat.len()), p));
            for (depth, c) in flat {
                let id = c.id;
                body = body.child(
                    h_flex()
                        .id(id_of("catrow", id))
                        .gap_3()
                        .items_center()
                        .px_2()
                        .py_1()
                        .pl(px(8.0 + 20.0 * depth as f32))
                        .rounded_md()
                        .hover(|d| d.bg(p.muted))
                        .child(div().size_3().flex_shrink_0().rounded_full().bg(hsla(c.color)))
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .font_weight(if depth == 0 { FontWeight::MEDIUM } else { FontWeight::NORMAL })
                                .child(c.name.clone()),
                        )
                        .child(Button::new(id_of("catedit", id)).ghost().small().label("Edit").on_click(
                            cx.listener(move |this, _, window, cx| this.edit_category(id, window, cx)),
                        ))
                        .child(Button::new(id_of("catdel", id)).ghost().small().label("✕").on_click(
                            cx.listener(move |this, _, window, cx| this.delete_category(id, window, cx)),
                        )),
                );
            }
        }
        card(p)
            .flex_1()
            .min_w_0()
            .h_full()
            .gap_2()
            .child(heading("All categories"))
            .child(div().id("cats-scroll").flex_1().min_h_0().overflow_y_scroll().child(body))
    }
}

fn stat_card(label: &'static str, value: String, accent: Hsla, note: String, p: Pal) -> impl IntoElement {
    card(p)
        .flex_1()
        .gap_1()
        .child(div().h_1().w_8().mb_1().rounded_full().bg(accent))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted_fg)
                .child(label.to_uppercase()),
        )
        .child(div().text_2xl().font_weight(FontWeight::SEMIBOLD).text_color(accent).child(value))
        .child(div().text_xs().text_color(p.muted_fg).child(note))
}

impl Render for CashFlowApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = pal(cx);
        let dark = cx.theme().is_dark();
        let page = match self.page {
            Page::Overview => self.overview(p, cx).into_any_element(),
            Page::Categories => self.categories_page(p, cx).into_any_element(),
            Page::Settings => self.settings_page(p, cx).into_any_element(),
        };
        let main = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(p.page)
            .child(self.header(p, cx))
            .children(self.error_banner(p, cx))
            .child(page);

        h_flex()
            .size_full()
            .bg(p.bg)
            .text_color(p.fg)
            .child(self.sidebar(p, dark, cx))
            .child(main)
    }
}
