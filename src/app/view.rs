//! Views: sidebar navigation, page header, cards and lists.

use super::*;

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

fn field(label: impl Into<SharedString>, p: Pal) -> Div {
    let label: SharedString = label.into();
    v_flex().gap_1().child(
        div()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted_fg)
            .child(label),
    )
}

fn section_label(text: impl Into<SharedString>, trailing: String, p: Pal) -> Div {
    h_flex()
        .justify_between()
        .mt_2()
        .px_2()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.muted_fg)
        .child(text.into())
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

    fn sidebar(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let nav = |page: Page, icon: IconName, label: &'static str, cx: &mut Context<Self>| {
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
                .child(Icon::new(icon).size_4())
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.page = page;
                    this.error = None;
                    if page == Page::Year {
                        this.year = this.month.year;
                        this.refresh_year();
                    }
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
                    .flex_1()
                    .min_w_0()
                    .child(heading("CashFlow"))
                    .child(div().text_xs().text_color(p.muted_fg).child(self.t(Msg::Tagline))),
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
            .child(nav(Page::Overview, IconName::LayoutDashboard, self.t(Msg::NavOverview), cx))
            .child(nav(Page::Year, IconName::CalendarDays, self.t(Msg::NavYear), cx))
            .child(nav(Page::Categories, IconName::Tags, self.t(Msg::NavCategories), cx))
            .child(nav(Page::Settings, IconName::Settings, self.t(Msg::NavSettings), cx))
            .child(nav(Page::About, IconName::Info, self.t(Msg::NavAbout), cx))
            .child(div().flex_1())
    }

    fn header(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let (title, sub) = match self.page {
            Page::Overview => (self.t(Msg::NavOverview), self.t(Msg::SubOverview)),
            Page::Year => (self.t(Msg::NavYear), self.t(Msg::SubYear)),
            Page::Categories => (self.t(Msg::NavCategories), self.t(Msg::SubCategories)),
            Page::Settings => (self.t(Msg::NavSettings), self.t(Msg::SubSettings)),
            Page::About => (self.t(Msg::NavAbout), self.t(Msg::SubAbout)),
        };
        let right = match self.page {
            Page::Overview => Some(self.month_nav(p, cx).into_any_element()),
            Page::Year => Some(self.year_nav(p, cx).into_any_element()),
            _ => None,
        };
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
                    .child(Button::new("month-prev").ghost().small().icon(IconName::ChevronLeft).on_click(
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
                            .child(self.lc.month_title(self.month)),
                    )
                    .child(Button::new("month-next").ghost().small().icon(IconName::ChevronRight).on_click(
                        cx.listener(|this, _, window, cx| {
                            this.switch_month(this.month.next(), window, cx)
                        }),
                    )),
            )
            .when(self.month != MonthKey::current(), |row| {
                row.child(Button::new("month-today").outline().small().label(self.t(Msg::Today)).on_click(
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
                .child(Button::new(id).ghost().small().icon(IconName::X).on_click(
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
            .child(seg(Kind::Income, self.t(Msg::KindIncome), cx))
            .child(seg(Kind::Expense, self.t(Msg::KindExpense), cx))
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
                    .child(self.summary(&self.data, p))
                    .child(self.chart_card(
                        &self.data,
                        self.tf(Msg::NoEntriesFor, &[("period", &self.lc.month_inline(self.month))]),
                        self.t(Msg::NoEntriesHint),
                        true,
                        p,
                        cx,
                    )),
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
            .child(heading(if editing { self.t(Msg::EditEntry) } else { self.t(Msg::AddEntry) }))
            .child(self.segmented(self.kind, false, p, cx))
            .child(field(self.t(Msg::Category), p).child(if no_categories {
                div()
                    .text_sm()
                    .text_color(p.muted_fg)
                    .child(self.t(Msg::NoCategoriesHint))
                    .into_any_element()
            } else {
                h_flex().flex_wrap().gap_2().children(pills).into_any_element()
            }))
            .child(
                h_flex()
                    .gap_2()
                    .items_end()
                    .child(field(self.t(Msg::Description), p).flex_1().child(Input::new(&self.name_in)))
                    .child(
                        field(self.tf(Msg::AmountLabel, &[("symbol", self.lc.currency_symbol())]), p)
                            .w(px(110.0))
                            .child(Input::new(&self.amount_in)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("submit")
                            .primary()
                            .label(if editing { self.t(Msg::SaveChanges) } else { self.t(Msg::AddEntry) })
                            .on_click(cx.listener(|this, _, window, cx| this.submit_entry(window, cx))),
                    )
                    .when(editing, |row| {
                        row.child(Button::new("cancel").outline().label(self.t(Msg::Cancel)).on_click(
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
                Kind::Income => (self.t(Msg::Income).to_uppercase(), "+"),
                Kind::Expense => (self.t(Msg::Expenses).to_uppercase(), self.lc.minus()),
            };
            body = body.child(section_label(label, format!("{sign}{}", self.lc.money(total)), p));
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
                    .child(self.t(Msg::NothingRecorded)),
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
                    .child(heading(self.t(Msg::Entries)))
                    .child(div().text_xs().text_color(p.muted_fg).child(self.tf(Msg::CountThisMonth, &[("count", &count.to_string())]))),
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
            Kind::Expense => (self.lc.minus(), p.fg),
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
                    .child(format!("{sign}{}", self.lc.money(e.amount_cents))),
            )
            .child(Button::new(id_of("edit", id)).ghost().small().label(self.t(Msg::Edit)).on_click(
                cx.listener(move |this, _, window, cx| this.edit_entry(id, window, cx)),
            ))
            .child(Button::new(id_of("del", id)).ghost().small().icon(IconName::X).on_click(
                cx.listener(move |this, _, window, cx| this.delete_entry(id, window, cx)),
            ))
    }

    /// Confirmation card for a parsed SankeyMATIC import.
    fn import_panel(&self, p: Pal, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let plan = self.pending_import.as_ref()?;
        let existing = self.entries.len();
        let month = self.lc.month_inline(self.month);
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
                    .child(format!(
                        "• {}",
                        self.tf(
                            Msg::MoreNotes,
                            &[("notes", &self.lc.count(Plural::CountMoreNotes, plan.warnings.len() - 4))]
                        )
                    )),
            );
        }
        Some(
            card(p)
                .gap_2()
                .border_color(p.primary)
                .child(heading(self.t(Msg::ImportTitle)))
                .child(div().text_sm().child(self.tf(
                    Msg::ImportFound,
                    &[
                        ("cats", &self.lc.count(Plural::CountCategories, plan.categories.len())),
                        ("entries", &self.lc.count(Plural::CountEntries, plan.entries.len())),
                        ("income", &self.lc.money(plan.income_cents)),
                        ("expenses", &self.lc.money(plan.expense_cents)),
                    ],
                )))
                .child(div().text_xs().text_color(p.muted_fg).child(self.t(Msg::ImportReuse)))
                .children(warnings)
                .child(
                    h_flex()
                        .gap_2()
                        .pt_1()
                        .child(
                            Button::new("import-replace")
                                .primary()
                                .label(if existing > 0 {
                                    self.tf(
                                        Msg::ImportReplace,
                                        &[("entries", &self.lc.count(Plural::CountEntries, existing)), ("period", &month)],
                                    )
                                } else {
                                    self.tf(Msg::ImportInto, &[("period", &month)])
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.confirm_import(true, window, cx)
                                })),
                        )
                        .when(existing > 0, |row| {
                            row.child(
                                Button::new("import-add")
                                    .outline()
                                    .label(self.t(Msg::ImportAdd))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.confirm_import(false, window, cx)
                                    })),
                            )
                        })
                        .child(
                            Button::new("import-cancel")
                                .ghost()
                                .label(self.t(Msg::Cancel))
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_import(cx))),
                        ),
                ),
        )
    }

    fn summary(&self, d: &SankeyData, p: Pal) -> impl IntoElement {
        let balance = d.balance_cents();
        let (balance_label, balance_color, balance_note) = if d.income_cents == 0 && d.expense_cents == 0 {
            (self.t(Msg::Balance), p.muted_fg, self.t(Msg::NoDataYet).to_string())
        } else if balance >= 0 {
            let rate = if d.income_cents > 0 { balance * 100 / d.income_cents } else { 0 };
            (self.t(Msg::Savings), p.success, self.tf(Msg::SavedRate, &[("pct", &rate.to_string())]))
        } else {
            (self.t(Msg::Deficit), p.danger, self.t(Msg::SpendingExceeds).to_string())
        };
        h_flex()
            .gap_4()
            .child(stat_card(
                self.t(Msg::Income),
                self.lc.money(d.income_cents),
                p.success,
                self.t(Msg::TotalEarned).to_string(),
                p,
            ))
            .child(stat_card(
                self.t(Msg::Expenses),
                self.lc.money(d.expense_cents),
                p.danger,
                self.t(Msg::TotalSpent).to_string(),
                p,
            ))
            .child(stat_card(balance_label, self.lc.money(balance.abs()), balance_color, balance_note, p))
    }

    fn chart_card(
        &self,
        d: &SankeyData,
        empty_title: String,
        empty_hint: &'static str,
        allow_import: bool,
        p: Pal,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let lc = self.lc;
        let body = if d.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_1()
                .child(div().text_color(p.fg).font_weight(FontWeight::MEDIUM).child(empty_title))
                .child(
                    div()
                        .text_sm()
                        .text_color(p.muted_fg)
                        .child(empty_hint),
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
                        // No relaxation: nodes keep our order, so Savings is always last.
                        .iterations(sankey::SANKEY_ITERATIONS)
                        // Room for the two-line label of small neighbouring nodes.
                        .node_padding(34.0)
                        .node_label(|n| n.label.clone().into())
                        .node_color(|n| gpui_kit::rgb(n.color))
                        .value_label(move |_, v| lc.money_whole(v).into()),
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
                    .child(heading(self.t(Msg::MoneyFlow)))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .when(!d.is_empty(), |row| {
                                row.child(div().text_xs().text_color(p.muted_fg).child(self.t(Msg::Export)))
                                    .child(
                                        Button::new("export-copy")
                                            .outline()
                                            .small()
                                            .label(if self.copied { self.t(Msg::Copied) } else { self.t(Msg::Copy) })
                                            .tooltip(self.t(Msg::TipCopy))
                                            .on_click(cx.listener(|this, _, _, cx| this.copy_sankeymatic(cx))),
                                    )
                                    .child(
                                        Button::new("export-save")
                                            .outline()
                                            .small()
                                            .label(self.t(Msg::SaveTxt))
                                            .tooltip(self.t(Msg::TipSave))
                                            .on_click(cx.listener(|this, _, _, cx| this.save_sankeymatic_file(cx))),
                                    )
                                    .when(allow_import, |row| row.child(div().w_px().h_5().mx_1().bg(p.border)))
                            })
                            .when(allow_import, |row| {
                                row.child(div().text_xs().text_color(p.muted_fg).child(self.t(Msg::Import)))
                                    .child(
                                        Button::new("import-paste")
                                            .outline()
                                            .small()
                                            .label(self.t(Msg::Paste))
                                            .tooltip(self.t(Msg::TipPaste))
                                            .on_click(cx.listener(|this, _, _, cx| this.paste_import(cx))),
                                    )
                                    .child(
                                        Button::new("import-open")
                                            .outline()
                                            .small()
                                            .label(self.t(Msg::OpenTxt))
                                            .tooltip(self.t(Msg::TipOpen))
                                            .on_click(cx.listener(|this, _, _, cx| this.open_import_file(cx))),
                                    )
                            }),
                    ),
            )
            .child(body)
    }

    // ---- year ----

    fn year_nav(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(Button::new("year-prev").ghost().small().icon(IconName::ChevronLeft).on_click(
                        cx.listener(|this, _, _, cx| this.switch_year(this.year - 1, cx)),
                    ))
                    .child(
                        div()
                            .w(px(100.0))
                            .text_center()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.year.to_string()),
                    )
                    .child(Button::new("year-next").ghost().small().icon(IconName::ChevronRight).on_click(
                        cx.listener(|this, _, _, cx| this.switch_year(this.year + 1, cx)),
                    )),
            )
            .when(self.year != MonthKey::current().year, |row| {
                row.child(Button::new("year-today").outline().small().label(self.t(Msg::ThisYear)).on_click(
                    cx.listener(|this, _, _, cx| this.switch_year(MonthKey::current().year, cx)),
                ))
            })
    }

    fn year_page(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(self.year_months_card(p, cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .gap_4()
                    .child(self.summary(&self.year_summary.data, p))
                    .child(self.chart_card(
                        &self.year_summary.data,
                        self.tf(Msg::NoEntriesFor, &[("period", &self.year_summary.year.to_string())]),
                        self.t(Msg::NoEntriesYearHint),
                        false,
                        p,
                        cx,
                    )),
            )
    }

    /// One row per month with income and expense bars; click to open the month.
    fn year_months_card(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let months = &self.year_summary.months;
        let max = months
            .iter()
            .map(|m| m.income_cents.max(m.expense_cents))
            .max()
            .unwrap_or(0)
            .max(1);
        let bar = |cents: i64, color: Hsla| {
            let frac = (cents as f32 / max as f32).clamp(0.0, 1.0);
            div()
                .h(px(6.0))
                .w_full()
                .rounded_full()
                .bg(p.muted)
                .child(div().h_full().rounded_full().bg(color).w(relative(frac)))
        };
        let rows: Vec<_> = (1..=12u8)
            .map(|m| {
                let t = months[(m - 1) as usize];
                let key = MonthKey { year: self.year, month: m };
                let empty = t.entries == 0;
                let balance = t.balance_cents();
                let (text, color) = if empty {
                    ("—".to_string(), p.muted_fg)
                } else if balance >= 0 {
                    (format!("+{}", self.lc.money(balance)), p.success)
                } else {
                    (self.lc.money(balance), p.danger)
                };
                h_flex()
                    .id(id_of("ymonth", m as u64))
                    .gap_3()
                    .items_center()
                    .px_2()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|d| d.bg(p.muted))
                    .child(
                        div()
                            .w(px(52.0))
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if empty { p.muted_fg } else { p.fg })
                            .child(self.lc.month_short(m)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(bar(t.income_cents, p.success))
                            .child(bar(t.expense_cents, p.danger)),
                    )
                    .child(
                        div()
                            .w(px(112.0))
                            .text_right()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(text),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| this.open_month(key, window, cx)))
            })
            .collect();

        let legend = |color: Hsla, label: &'static str| {
            h_flex()
                .gap_1()
                .items_center()
                .child(div().size_2().rounded_full().bg(color))
                .child(div().text_xs().text_color(p.muted_fg).child(label))
        };
        card(p)
            .flex_1()
            .min_h_0()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(heading(self.t(Msg::ByMonth)))
                    .child(
                        h_flex()
                            .gap_3()
                            .child(legend(p.success, self.t(Msg::Income)))
                            .child(legend(p.danger, self.t(Msg::Expenses))),
                    ),
            )
            .child(div().text_xs().text_color(p.muted_fg).child(self.t(Msg::YearHint)))
            .child(
                div()
                    .id("year-months-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(v_flex().gap_1().children(rows)),
            )
    }

    // ---- about ----

    fn about_page(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let row = |label: &'static str, value: String| {
            h_flex()
                .gap_3()
                .items_start()
                .child(
                    div()
                        .w(px(96.0))
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(p.muted_fg)
                        .child(label),
                )
                .child(div().flex_1().min_w_0().text_sm().child(value))
        };
        let repo = env!("CARGO_PKG_REPOSITORY");
        let data_dir = self
            .storage
            .as_ref()
            .map(|s| s.dir().display().to_string())
            .unwrap_or_else(|| self.t(Msg::NotAvailable).to_string());

        h_flex().flex_1().min_h_0().items_start().p_6().child(
            v_flex().w(px(560.0)).max_h_full().child(
                card(p)
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(img(self.logo.clone()).size_16().flex_shrink_0())
                            .child(
                                v_flex()
                                    .child(div().text_xl().font_weight(FontWeight::SEMIBOLD).child("CashFlow"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(p.muted_fg)
                                            .child(self.tf(Msg::Version, &[("version", env!("CARGO_PKG_VERSION"))])),
                                    ),
                            ),
                    )
                    .child(div().text_sm().child(self.t(Msg::AboutDescription)))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(row(self.t(Msg::Author), env!("CARGO_PKG_AUTHORS").replace(':', ", ")))
                            .child(row(self.t(Msg::License), env!("CARGO_PKG_LICENSE").to_string()))
                            .child(row(self.t(Msg::Source), repo.to_string()))
                            .child(row(self.t(Msg::DataFolder), data_dir)),
                    )
                    .child(
                        h_flex().child(
                            Button::new("open-repo")
                                .outline()
                                .small()
                                .label(self.t(Msg::OpenOnGitHub))
                                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(repo))),
                        ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(heading(self.t(Msg::License)))
                            .child(
                                div()
                                    .id("license-text")
                                    .max_h(px(200.0))
                                    .overflow_y_scroll()
                                    .p_3()
                                    .rounded_md()
                                    .bg(p.muted)
                                    .text_xs()
                                    .text_color(p.muted_fg)
                                    .child(include_str!("../../LICENSE").replace("\r\n", "\n")),
                            ),
                    )
                    .child(div().text_xs().text_color(p.muted_fg).child(self.t(Msg::ThirdParty))),
            ),
        )
    }

    // ---- settings ----

    fn settings_page(&self, p: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let languages: Vec<_> = std::iter::once((None, self.t(Msg::Automatic)))
            .chain(Lang::ALL.iter().map(|&l| (Some(l), l.native_name())))
            .map(|(lang, label)| {
                pill(
                    SharedString::from(format!("lang-{lang:?}")),
                    label.to_string(),
                    None,
                    self.settings.language == lang,
                    p,
                )
                .on_click(cx.listener(move |this, _, window, cx| this.set_language(lang, window, cx)))
            })
            .collect();
        let currencies: Vec<_> = std::iter::once((None, self.t(Msg::Automatic).to_string()))
            .chain(
                Currency::ALL
                    .iter()
                    .map(|&c| {
                        let (code, symbol) = (c.code(), c.symbol(self.lc.lang));
                        (Some(c), if code == symbol { code.to_string() } else { format!("{code} {symbol}") })
                    }),
            )
            .map(|(currency, label)| {
                pill(
                    SharedString::from(format!("cur-{currency:?}")),
                    label,
                    None,
                    self.settings.currency == currency,
                    p,
                )
                .on_click(cx.listener(move |this, _, window, cx| this.set_currency(currency, window, cx)))
            })
            .collect();
        let themes: Vec<_> = [
            (None, self.t(Msg::Automatic)),
            (Some(ThemeChoice::Light), self.t(Msg::ThemeLight)),
            (Some(ThemeChoice::Dark), self.t(Msg::ThemeDark)),
        ]
        .into_iter()
        .map(|(theme, label)| {
            pill(
                SharedString::from(format!("theme-{theme:?}")),
                label.to_string(),
                None,
                self.settings.theme == theme,
                p,
            )
            .on_click(cx.listener(move |this, _, window, cx| this.set_theme(theme, window, cx)))
        })
        .collect();
        let hint = |text: &'static str| div().text_sm().text_color(p.muted_fg).child(text);

        h_flex().flex_1().min_h_0().items_start().p_6().child(
            v_flex()
                .w(px(560.0))
                .gap_4()
                .child(
                    card(p)
                        .gap_3()
                        .child(heading(self.t(Msg::LanguageTitle)))
                        .child(hint(self.t(Msg::LanguageHelp)))
                        .child(h_flex().flex_wrap().gap_2().children(languages))
                        .child(heading(self.t(Msg::CurrencyTitle)))
                        .child(hint(self.t(Msg::CurrencyHelp)))
                        .child(h_flex().flex_wrap().gap_2().children(currencies)),
                )
                .child(
                    card(p)
                        .gap_3()
                        .child(heading(self.t(Msg::ThemeTitle)))
                        .child(hint(self.t(Msg::ThemeHelp)))
                        .child(h_flex().flex_wrap().gap_2().children(themes)),
                )
                .child(
                    card(p)
                        .gap_3()
                        .child(heading(self.t(Msg::ChartLabels)))
                        .child(hint(self.t(Msg::ChartLabelsHelp)))
                        .child(field(self.t(Msg::LabelTotal), p).child(Input::new(&self.total_in)))
                        .child(field(self.t(Msg::LabelSavings), p).child(Input::new(&self.savings_in)))
                        .child(field(self.t(Msg::LabelDeficit), p).child(Input::new(&self.deficit_in)))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(Button::new("labels-save").primary().label(self.t(Msg::Save)).on_click(
                                    cx.listener(|this, _, window, cx| this.save_labels(window, cx)),
                                ))
                                .child(
                                    Button::new("labels-reset").outline().label(self.t(Msg::ResetDefaults)).on_click(
                                        cx.listener(|this, _, window, cx| this.reset_labels(window, cx)),
                                    ),
                                ),
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

        let parents: Vec<_> = std::iter::once((None, self.t(Msg::NoParent).to_string(), None))
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

        let swatch_row = |colors: &[u32], cx: &mut Context<Self>| {
            h_flex().flex_wrap().gap_2().children(colors.iter().map(|&color| {
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
            }))
        };
        let swatches = v_flex()
            .gap_2()
            .child(swatch_row(&PALETTE, cx))
            .child(swatch_row(&PALETTE_DARK, cx))
            .child(swatch_row(&PALETTE_MUTED, cx));

        card(p)
            .gap_3()
            .child(heading(if editing { self.t(Msg::EditCategory) } else { self.t(Msg::NewCategory) }))
            .child(self.segmented(self.cat_kind, true, p, cx))
            .child(field(self.t(Msg::Name), p).child(Input::new(&self.cat_name_in)))
            .child(field(self.t(Msg::Color), p).child(swatches))
            .child(field(self.t(Msg::Parent), p).child(h_flex().flex_wrap().gap_2().children(parents)))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("add-cat")
                            .primary()
                            .label(if editing { self.t(Msg::SaveChanges) } else { self.t(Msg::AddCategory) })
                            .on_click(cx.listener(|this, _, window, cx| this.submit_category(window, cx))),
                    )
                    .when(editing, |row| {
                        row.child(Button::new("cancel-cat").outline().label(self.t(Msg::Cancel)).on_click(
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
            let label = if kind == Kind::Income { self.t(Msg::Income) } else { self.t(Msg::Expenses) }.to_uppercase();
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
                        .child(Button::new(id_of("catedit", id)).ghost().small().label(self.t(Msg::Edit)).on_click(
                            cx.listener(move |this, _, window, cx| this.edit_category(id, window, cx)),
                        ))
                        .child(Button::new(id_of("catdel", id)).ghost().small().icon(IconName::X).on_click(
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
            .child(heading(self.t(Msg::AllCategories)))
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
        let page = match self.page {
            Page::Overview => self.overview(p, cx).into_any_element(),
            Page::Year => self.year_page(p, cx).into_any_element(),
            Page::Categories => self.categories_page(p, cx).into_any_element(),
            Page::Settings => self.settings_page(p, cx).into_any_element(),
            Page::About => self.about_page(p, cx).into_any_element(),
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
            .child(self.sidebar(p, cx))
            .child(main)
    }
}
