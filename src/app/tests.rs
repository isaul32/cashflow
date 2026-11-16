//! UI tests: the real window is rendered headlessly and driven with clicks and key
//! presses, like a user would. Every test gets its own data folder and fixed
//! settings (English, euro, light theme), so results do not depend on the machine.
//!
//! Elements are found by their ids; the ids are the ones in `view.rs`.

use super::*;
// `use super::*` also brings in the `gpui_kit::test` macro, which would shadow the built-in
// `#[test]` that `#[gpui_kit::test]` expands to. Naming the built-in one wins over the glob.
use core::prelude::v1::test;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyWindowHandle, Bounds, Point, TestAppContext, WindowBounds};

use crate::data::DataFile;
use crate::settings::ThemeChoice;

/// An open window with the app inside it, and the data folder the app writes to.
pub(crate) struct Env {
    pub(crate) handle: AnyWindowHandle,
    pub(crate) app: Entity<CashFlowApp>,
    pub(crate) st: Storage,
}

pub(crate) fn open(cx: &mut TestAppContext, name: &str, seed: impl FnOnce(&Storage, &Categories)) -> Env {
    open_sized(cx, name, 1360.0, 860.0, seed)
}

pub(crate) fn open_sized(
    cx: &mut TestAppContext,
    name: &str,
    width: f32,
    height: f32,
    seed: impl FnOnce(&Storage, &Categories),
) -> Env {
    cx.update(|cx| gpui_kit::init(cx));
    let dir = std::env::temp_dir().join(format!("budget-test-{}-ui-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let st = Storage::new(&dir);
    st.save_settings(&Settings {
        language: Some(Lang::En),
        currency: Some(Currency::Eur),
        theme: Some(ThemeChoice::Light),
        // On by default, but tests must never contact GitHub.
        check_updates: false,
        ..Settings::default()
    })
    .unwrap();
    let cats = Categories::defaults_for(Lang::En);
    st.save_categories(&cats).unwrap();
    seed(&st, &cats);

    let storage = st.clone();
    let (handle, app) = cx.update(|cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: size(px(width), px(height)),
            })),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| CashFlowApp::with_storage(Some(storage), window, cx))
        })
        .expect("open the window")
    });
    Env { handle, app, st }
}

impl Env {
    /// Runs `f` in the window, after rendering the latest state.
    pub(crate) fn ui<R>(&self, cx: &mut TestAppContext, f: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        cx.update_window(self.handle, |_, window, cx| {
            window.render_frame(cx);
            f(window, cx)
        })
        .expect("the window is open")
    }

    /// Reads from the app state.
    pub(crate) fn read<R>(&self, cx: &mut TestAppContext, f: impl FnOnce(&CashFlowApp) -> R) -> R {
        cx.read(|cx| f(self.app.read(cx)))
    }

    /// Like [`Self::read`], with the app context too (needed to read the text of an input).
    pub(crate) fn read_inputs<R>(&self, cx: &mut TestAppContext, f: impl FnOnce(&CashFlowApp, &App) -> R) -> R {
        cx.read(|cx| f(self.app.read(cx), cx))
    }

    /// Changes the app state directly, for setting up what a test starts from.
    pub(crate) fn modify(&self, cx: &mut TestAppContext, f: impl FnOnce(&mut CashFlowApp)) {
        cx.update(|cx| self.app.update(cx, |app, cx| {
            f(app);
            cx.notify();
        }));
    }

    pub(crate) fn click(&self, cx: &mut TestAppContext, id: impl Into<ElementId>) {
        let id = id.into();
        self.ui(cx, |window, cx| window.click(id, cx));
    }

    /// Replaces the text of the focused-by-click input with `text`.
    pub(crate) fn type_into(&self, cx: &mut TestAppContext, id: &'static str, text: &str) {
        self.ui(cx, |window, cx| {
            window.click(id, cx);
            window.press("secondary-a", cx);
            window.input(text, cx);
        });
    }

    pub(crate) fn exists(&self, cx: &mut TestAppContext, id: impl Into<ElementId>) -> bool {
        let id = id.into();
        self.ui(cx, |window, _| window.try_find(id).is_some())
    }

    pub(crate) fn cat(&self, cx: &mut TestAppContext, name: &str) -> CategoryId {
        self.read(cx, |app| app.cats.items.iter().find(|c| c.name == name).expect("category").id)
    }
}

pub(crate) fn this_month() -> MonthKey {
    MonthKey::current()
}

pub(crate) fn entry(id: u64, category_id: CategoryId, name: &str, cents: i64) -> Entry {
    Entry { id, category_id, name: name.to_string(), amount_cents: cents }
}

pub(crate) fn category_id(cats: &Categories, name: &str) -> CategoryId {
    cats.items.iter().find(|c| c.name == name).expect("category").id
}

#[gpui_kit::test]
fn starts_on_an_empty_current_month(cx: &mut TestAppContext) {
    let env = open(cx, "start", |_, _| {});
    env.read(cx, |app| {
        assert!(app.page == Page::Overview);
        assert_eq!(app.month, this_month());
        assert!(app.entries.is_empty());
        assert!(app.error.is_none());
    });
    // The page shows the entry form but no shortcut back to today.
    assert!(env.exists(cx, "submit"));
    assert!(env.exists(cx, "month-prev"));
    assert!(!env.exists(cx, "month-today"));
}

#[gpui_kit::test]
fn adds_an_expense_by_typing_and_pressing_enter(cx: &mut TestAppContext) {
    let env = open(cx, "add", |_, _| {});
    let food = env.cat(cx, "Food");

    env.click(cx, format!("cat-{food}"));
    env.type_into(cx, "entry-name", "Lunch");
    env.type_into(cx, "entry-amount", "12.50");
    env.ui(cx, |window, cx| window.press("enter", cx));

    env.read_inputs(cx, |app, cx| {
        assert_eq!(app.entries.len(), 1);
        let e = &app.entries[0];
        assert_eq!((e.category_id, e.name.as_str(), e.amount_cents), (food, "Lunch", 1250));
        assert!(app.error.is_none());
        // The form is emptied for the next entry.
        assert_eq!(app.amount_in.read(cx).value().as_ref(), "");
    });
    // It was saved, and the list shows it.
    let saved = env.st.load_month(this_month());
    assert_eq!(saved.entries.len(), 1);
    assert!(env.exists(cx, format!("edit-{}", saved.entries[0].id)));
}

#[gpui_kit::test]
fn adds_an_entry_with_the_submit_button(cx: &mut TestAppContext) {
    let env = open(cx, "submit", |_, _| {});
    env.type_into(cx, "entry-amount", "7");
    env.click(cx, "submit");
    env.read(cx, |app| assert_eq!(app.entries.iter().map(|e| e.amount_cents).collect::<Vec<_>>(), vec![700]));
}

#[gpui_kit::test]
fn an_entry_without_an_amount_shows_an_error_that_can_be_dismissed(cx: &mut TestAppContext) {
    let env = open(cx, "error", |_, _| {});
    env.click(cx, "submit");
    env.read(cx, |app| {
        assert!(app.entries.is_empty());
        assert!(app.error.is_some());
    });
    env.click(cx, "dismiss-error");
    env.read(cx, |app| assert!(app.error.is_none()));
    assert!(!env.exists(cx, "dismiss-error"));
}

#[gpui_kit::test]
fn the_kind_switch_changes_the_categories_offered(cx: &mut TestAppContext) {
    let env = open(cx, "kind", |_, _| {});
    let income = format!("seg-false-{}", Msg::KindIncome.text(Lang::En));
    let expense = format!("seg-false-{}", Msg::KindExpense.text(Lang::En));
    let salary = env.cat(cx, "Salary");
    let other_income = env.cat(cx, "Other income");

    env.click(cx, income);
    env.read(cx, |app| {
        assert!(app.kind == Kind::Income);
        // The first category of the new kind is selected (they are listed alphabetically).
        assert_eq!(app.category, Some(other_income));
    });
    assert!(env.exists(cx, format!("cat-{salary}")));

    env.click(cx, expense);
    env.read(cx, |app| assert!(app.kind == Kind::Expense));
    assert!(!env.exists(cx, format!("cat-{salary}")));
}

#[gpui_kit::test]
fn edits_an_entry(cx: &mut TestAppContext) {
    let mut food = 0;
    let env = open(cx, "edit", |st, cats| {
        food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Lunch", 990)] }).unwrap();
    });

    env.click(cx, "edit-1");
    env.read_inputs(cx, |app, cx| {
        assert_eq!(app.editing, Some(1));
        assert_eq!(app.name_in.read(cx).value().as_ref(), "Lunch");
        assert_eq!(app.amount_in.read(cx).value().as_ref(), "9.90");
    });

    env.type_into(cx, "entry-amount", "15");
    env.click(cx, "submit");
    env.read(cx, |app| {
        assert_eq!(app.editing, None);
        assert_eq!(app.entries.len(), 1);
        assert_eq!((app.entries[0].amount_cents, app.entries[0].category_id), (1500, food));
    });
    assert_eq!(env.st.load_month(this_month()).entries[0].amount_cents, 1500);
}

#[gpui_kit::test]
fn cancelling_an_edit_keeps_the_entry(cx: &mut TestAppContext) {
    let env = open(cx, "cancel-edit", |st, cats| {
        let food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Lunch", 990)] }).unwrap();
    });
    env.click(cx, "edit-1");
    env.click(cx, "cancel");
    env.read(cx, |app| {
        assert_eq!(app.editing, None);
        assert_eq!(app.entries[0].amount_cents, 990);
    });
}

#[gpui_kit::test]
fn deletes_an_entry(cx: &mut TestAppContext) {
    let env = open(cx, "delete", |st, cats| {
        let food = category_id(cats, "Food");
        let entries = vec![entry(1, food, "Lunch", 990), entry(2, food, "Dinner", 2000)];
        st.save_month(this_month(), &MonthData { entries }).unwrap();
    });
    env.click(cx, "del-1");
    env.read(cx, |app| assert_eq!(app.entries.iter().map(|e| e.id).collect::<Vec<_>>(), vec![2]));
    assert_eq!(env.st.load_month(this_month()).entries.len(), 1);
    assert!(!env.exists(cx, "edit-1"));
    assert!(env.exists(cx, "edit-2"));
}

#[gpui_kit::test]
fn month_buttons_move_between_months_and_today_returns(cx: &mut TestAppContext) {
    let env = open(cx, "months", |_, _| {});
    let today = this_month();

    env.click(cx, "month-next");
    env.read(cx, |app| assert_eq!(app.month, today.next()));
    assert!(env.exists(cx, "month-today"));
    // The shortcut sits to the left of the selector, so the selector does not move.
    env.ui(cx, |window, _| {
        assert!(window.find("month-today").bounds().right() <= window.find("month-prev").bounds().left());
    });

    env.click(cx, "month-prev");
    env.click(cx, "month-prev");
    env.read(cx, |app| assert_eq!(app.month, today.prev()));
    assert!(env.exists(cx, "month-today"));

    env.click(cx, "month-today");
    env.read(cx, |app| assert_eq!(app.month, today));
    assert!(!env.exists(cx, "month-today"));
}

#[gpui_kit::test]
fn entries_belong_to_their_month(cx: &mut TestAppContext) {
    let env = open(cx, "month-entries", |st, cats| {
        let food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Now", 100)] }).unwrap();
        st.save_month(this_month().prev(), &MonthData { entries: vec![entry(1, food, "Before", 200)] }).unwrap();
    });
    env.click(cx, "month-prev");
    env.read(cx, |app| assert_eq!(app.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Before"]));
    env.click(cx, "month-today");
    env.read(cx, |app| assert_eq!(app.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Now"]));
}

#[gpui_kit::test]
fn the_year_page_has_its_own_navigation(cx: &mut TestAppContext) {
    let env = open(cx, "year", |_, _| {});
    let year = this_month().year;

    env.click(cx, Msg::NavYear.text(Lang::En));
    env.read(cx, |app| {
        assert!(app.page == Page::Year);
        assert_eq!(app.year, year);
    });
    assert!(!env.exists(cx, "year-today"));

    env.click(cx, "year-prev");
    env.read(cx, |app| assert_eq!(app.year, year - 1));
    env.ui(cx, |window, _| {
        assert!(window.find("year-today").bounds().right() <= window.find("year-prev").bounds().left());
    });

    env.click(cx, "year-next");
    env.click(cx, "year-next");
    env.read(cx, |app| assert_eq!(app.year, year + 1));
    env.click(cx, "year-today");
    env.read(cx, |app| assert_eq!(app.year, year));
    assert!(!env.exists(cx, "year-today"));
}

#[gpui_kit::test]
fn the_sidebar_reaches_every_page(cx: &mut TestAppContext) {
    let env = open(cx, "pages", |_, _| {});
    let pages = [
        (Msg::NavSettings, Page::Settings),
        (Msg::NavAbout, Page::About),
        (Msg::NavCategories, Page::Categories),
        (Msg::NavYear, Page::Year),
        (Msg::NavOverview, Page::Overview),
    ];
    for (label, page) in pages {
        env.click(cx, label.text(Lang::En));
        env.read(cx, |app| assert!(app.page == page, "{}", label.text(Lang::En)));
    }
}

/// The entry list sits below the entry form in a scrolling column. Depending on the platform's
/// font metrics the list can start below the fold, so scroll to the end before clicking in it.
/// The wheel is aimed at the submit button: over the middle of the column it would land on the
/// category picker, which is a scroll area of its own and would take the scrolling.
fn scroll_list_into_view(env: &Env, cx: &mut TestAppContext) {
    env.ui(cx, |window, cx| {
        window.scroll("submit", ScrollDelta::Pixels(point(px(0.), px(-4000.))), cx);
    });
}

#[gpui_kit::test]
fn an_empty_month_can_start_from_the_one_before(cx: &mut TestAppContext) {
    let env = open(cx, "copy", |st, cats| {
        let (food, housing) = (category_id(cats, "Food"), category_id(cats, "Housing"));
        let last_month = MonthData { entries: vec![entry(5, housing, "Rent", 80000), entry(9, food, "Lunch", 990)] };
        st.save_month(this_month().prev(), &last_month).unwrap();
    });
    // This month is empty and the one before it is not: the empty list offers a way to start.
    env.read(cx, |app| assert!(app.entries.is_empty()));
    assert!(env.exists(cx, "copy-previous"));

    scroll_list_into_view(&env, cx);
    env.click(cx, "copy-previous");
    env.read(cx, |app| {
        // The same entries, numbered afresh for this month.
        let copied: Vec<_> = app.entries.iter().map(|e| (e.id, e.name.as_str(), e.amount_cents)).collect();
        assert_eq!(copied, vec![(1, "Rent", 80000), (2, "Lunch", 990)]);
        assert!(app.notice.is_some());
    });
    assert_eq!(env.st.load_month(this_month()).entries.len(), 2, "it was saved");
    assert_eq!(env.st.load_month(this_month().prev()).entries.len(), 2, "the month it came from is untouched");
    assert!(!env.exists(cx, "copy-previous"), "the month is no longer empty");

    // The copy is a month of its own: changing it leaves the other alone.
    scroll_list_into_view(&env, cx);
    env.click(cx, "del-1");
    assert_eq!(env.st.load_month(this_month()).entries.len(), 1);
    assert_eq!(env.st.load_month(this_month().prev()).entries.len(), 2);
}

#[gpui_kit::test]
fn the_copy_button_needs_an_empty_month_after_one_with_entries(cx: &mut TestAppContext) {
    // Nothing to copy from.
    let env = open(cx, "copy-nothing", |_, _| {});
    assert!(!env.exists(cx, "copy-previous"));

    // This month and the one before both have entries: nothing is empty, so nothing is offered.
    let env = open(cx, "copy-both", |st, cats| {
        let food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Now", 100)] }).unwrap();
        st.save_month(this_month().prev(), &MonthData { entries: vec![entry(1, food, "Before", 200)] }).unwrap();
    });
    assert!(!env.exists(cx, "copy-previous"));
    // The next month is empty and this one has entries: now it is offered.
    env.click(cx, "month-next");
    assert!(env.exists(cx, "copy-previous"));
    env.click(cx, "copy-previous");
    env.read(cx, |app| assert_eq!(app.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Now"]));
}

#[gpui_kit::test]
fn a_backup_can_be_restored_after_confirming(cx: &mut TestAppContext) {
    let env = open(cx, "restore", |st, cats| {
        let food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Current", 500)] }).unwrap();
    });
    // A backup that has other data: the same month with a different entry.
    let backup: DataFile = {
        let mut data = env.st.snapshot();
        let food = category_id(&data.categories, "Food");
        let month = MonthData { entries: vec![entry(1, food, "FromBackup", 4200)] };
        data.months.insert(this_month().to_string(), month);
        data
    };

    env.click(cx, Msg::NavSettings.text(Lang::En));
    env.modify(cx, |app| app.pending_restore = Some(backup.clone()));
    assert!(env.exists(cx, "restore-confirm"));

    // Cancelling changes nothing.
    env.click(cx, "restore-cancel");
    env.read(cx, |app| {
        assert!(app.pending_restore.is_none());
        assert_eq!(app.entries[0].name, "Current");
    });
    assert!(!env.exists(cx, "restore-confirm"));

    env.modify(cx, |app| app.pending_restore = Some(backup.clone()));
    env.click(cx, "restore-confirm");
    env.read(cx, |app| {
        assert!(app.pending_restore.is_none());
        assert!(app.notice.is_some());
        assert_eq!(app.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["FromBackup"]);
    });
    assert_eq!(env.st.load_month(this_month()).entries[0].amount_cents, 4200);
    // A copy of what was replaced was saved first.
    let copies = std::fs::read_dir(env.st.dir().join("backups")).unwrap().count();
    assert_eq!(copies, 1);
}

#[gpui_kit::test]
fn changing_the_language_relabels_the_window_and_is_saved(cx: &mut TestAppContext) {
    let env = open(cx, "language", |_, _| {});
    env.click(cx, Msg::NavSettings.text(Lang::En));

    env.click(cx, "lang-Some(Fi)");
    env.read(cx, |app| {
        assert_eq!(app.settings.language, Some(Lang::Fi));
        assert_eq!(app.lc.lang, Lang::Fi);
    });
    assert_eq!(env.st.load_settings().value.language, Some(Lang::Fi));
    // The sidebar now uses the Finnish labels (the element ids are the labels).
    assert!(env.exists(cx, Msg::NavSettings.text(Lang::Fi)));
    assert!(!env.exists(cx, Msg::NavSettings.text(Lang::En)));

    env.click(cx, "lang-Some(En)");
    assert!(env.exists(cx, Msg::NavSettings.text(Lang::En)));
}

#[gpui_kit::test]
fn the_theme_choice_is_applied_and_saved(cx: &mut TestAppContext) {
    let env = open(cx, "theme", |_, _| {});
    env.click(cx, Msg::NavSettings.text(Lang::En));
    // Under the chart's names and the language, so further down than the window shows.
    env.ui(cx, |window, cx| {
        window.scroll("settings-scroll", ScrollDelta::Pixels(point(px(0.), px(-4000.))), cx);
    });
    assert!(!env.ui(cx, |_, cx| cx.theme().is_dark()));

    env.click(cx, "theme-Some(Dark)");
    assert!(env.ui(cx, |_, cx| cx.theme().is_dark()));
    assert_eq!(env.st.load_settings().value.theme, Some(ThemeChoice::Dark));

    env.click(cx, "theme-Some(Light)");
    assert!(!env.ui(cx, |_, cx| cx.theme().is_dark()));
}

#[gpui_kit::test]
fn the_update_check_can_be_turned_off_and_is_remembered(cx: &mut TestAppContext) {
    let env = open(cx, "updates", |_, _| {});
    // The app is on by default; the test fixture turns it off so that no test contacts GitHub.
    // Pretend it is on, as it is for a user who never touched it, without starting a check.
    env.modify(cx, |app| app.settings.check_updates = true);
    env.click(cx, Msg::NavSettings.text(Lang::En));
    assert!(env.exists(cx, "updates-false") && env.exists(cx, "updates-true"));

    env.click(cx, "updates-false");
    env.read(cx, |app| {
        assert!(!app.settings.check_updates);
        assert!(app.update.is_none());
    });
    assert!(!env.st.load_settings().value.check_updates, "the choice is saved");
}

#[gpui_kit::test]
fn adds_and_deletes_a_category(cx: &mut TestAppContext) {
    let env = open(cx, "categories", |_, _| {});
    env.click(cx, Msg::NavCategories.text(Lang::En));

    env.type_into(cx, "cat-name", "Pets");
    env.click(cx, "add-cat");
    let id = env.cat(cx, "Pets");
    env.read(cx, |app| {
        assert!(app.cats.get(id).is_some_and(|c| c.kind == Kind::Expense));
        assert!(app.error.is_none());
    });
    assert!(env.st.load_categories().items.iter().any(|c| c.name == "Pets"));

    // A second category with the same name is refused.
    env.type_into(cx, "cat-name", "pets");
    env.click(cx, "add-cat");
    env.read(cx, |app| assert!(app.error.is_some()));

    env.click(cx, format!("catdel-{id}"));
    env.read(cx, |app| assert!(app.cats.get(id).is_none()));
    assert!(!env.st.load_categories().items.iter().any(|c| c.name == "Pets"));
}

#[gpui_kit::test]
fn deleting_a_category_moves_its_entries(cx: &mut TestAppContext) {
    let mut ids = (0, 0);
    let env = open(cx, "category-move", |st, cats| {
        ids = (category_id(cats, "Food"), category_id(cats, "Housing"));
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, ids.0, "Lunch", 900)] }).unwrap();
    });
    env.click(cx, Msg::NavCategories.text(Lang::En));
    env.click(cx, format!("catdel-{}", ids.0));
    env.read(cx, |app| {
        assert!(app.cats.get(ids.0).is_none());
        // The entry did not vanish: it moved to another expense category.
        assert_eq!(app.entries.len(), 1);
        assert!(app.cats.get(app.entries[0].category_id).is_some());
    });
}

#[gpui_kit::test]
fn adapts_to_small_and_large_windows(cx: &mut TestAppContext) {
    let env = open_sized(cx, "sizes", 1360.0, 860.0, |_, _| {});
    let handle = env.handle;
    for (w, h) in [(900.0, 620.0), (820.0, 640.0), (1700.0, 1000.0), (1360.0, 860.0)] {
        cx.simulate_window_resize(handle, size(px(w), px(h)));
        for page in [Msg::NavOverview, Msg::NavYear, Msg::NavSettings] {
            env.click(cx, page.text(Lang::En));
            // Everything stays reachable: the navigation and the page header are on screen.
            env.ui(cx, |window, _| {
                let nav = window.find(page.text(Lang::En)).bounds();
                assert!(nav.size.width > px(0.) && nav.bottom() <= px(h), "{w}x{h} {}", page.text(Lang::En));
            });
        }
    }
    env.click(cx, Msg::NavOverview.text(Lang::En));
    // The entry form still works in the smallest window, where the page is one scrolling column.
    cx.simulate_window_resize(handle, size(px(820.0), px(640.0)));
    env.ui(cx, |window, cx| {
        window.scroll("overview-compact-scroll", ScrollDelta::Pixels(point(px(0.), px(-3000.))), cx);
    });
    env.type_into(cx, "entry-amount", "3");
    env.click(cx, "submit");
    env.read(cx, |app| assert_eq!(app.entries.len(), 1));
}

#[gpui_kit::test]
fn the_mcp_server_is_off_until_the_user_turns_it_on(cx: &mut TestAppContext) {
    let env = open(cx, "mcp-card", |_, _| {});
    env.read(cx, |app| assert!(!app.settings.mcp.enabled && !app.settings.mcp.allow_changes));
    env.click(cx, Msg::NavSettings.text(Lang::En));
    assert!(env.exists(cx, "mcp-true") && env.exists(cx, "mcp-false"));
    assert!(!env.exists(cx, "mcp-changes-true"), "the details are hidden while it is off");
    assert!(!env.exists(cx, "mcp-copy-command"));

    env.click(cx, "mcp-true");
    env.read(cx, |app| assert!(app.settings.mcp.enabled && !app.settings.mcp.allow_changes, "read-only at first"));
    assert!(env.st.load_settings().value.mcp.enabled, "the choice is saved, where the MCP process reads it");
    assert!(env.exists(cx, "mcp-changes-true") && env.exists(cx, "mcp-copy-command"));

    env.click(cx, "mcp-changes-true");
    assert!(env.st.load_settings().value.mcp.allow_changes);
    env.click(cx, "mcp-changes-false");
    assert!(!env.st.load_settings().value.mcp.allow_changes);

    env.click(cx, "mcp-false");
    assert!(!env.st.load_settings().value.mcp.enabled);
    assert!(!env.exists(cx, "mcp-copy-command"));
}

#[gpui_kit::test]
fn the_claude_code_command_starts_this_app_in_mcp_mode(cx: &mut TestAppContext) {
    let env = open(cx, "mcp-command", |_, _| {});
    env.modify(cx, |app| app.settings.mcp.enabled = true);
    env.click(cx, Msg::NavSettings.text(Lang::En));

    let command = env.read(cx, |app| app.mcp_command());
    assert!(command.starts_with("claude mcp add cashflow -- \""), "{command}");
    assert!(command.ends_with("\" --mcp"), "{command}");

    // The button is further down the page than the window shows.
    env.ui(cx, |window, cx| {
        window.scroll("settings-scroll", ScrollDelta::Pixels(point(px(0.), px(-4000.))), cx);
    });
    env.click(cx, "mcp-copy-command");
    env.read(cx, |app| assert!(app.notice.is_some()));
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied, Some(command));
}

#[gpui_kit::test]
fn the_window_follows_changes_made_by_another_process(cx: &mut TestAppContext) {
    // The watcher runs on its own thread, which the deterministic test scheduler only accepts when told.
    cx.executor().allow_parking();
    let env = open(cx, "follow", |st, cats| {
        let food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Lunch", 990)] }).unwrap();
    });
    env.read(cx, |app| assert_eq!(app.entries.len(), 1));
    let food = env.cat(cx, "Food");

    // `cashflow --mcp` is another process with its own storage on the same folder.
    let assistant = Storage::new(env.st.dir());
    let mut month_data = assistant.load_month(this_month());
    month_data.entries.push(entry(month_data.next_entry_id(), food, "From the assistant", 1250));
    assistant.save_month(this_month(), &month_data).unwrap();

    let mut waited = 0;
    while env.read(cx, |app| app.entries.len()) < 2 && waited < 400 {
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
        waited += 1;
    }
    env.read(cx, |app| {
        assert_eq!(app.entries.len(), 2, "the window reloaded");
        assert!(app.entries.iter().any(|e| e.name == "From the assistant" && e.amount_cents == 1250));
    });
    assert!(env.exists(cx, "edit-2"), "and shows the new entry");

    // A change the window makes itself afterwards keeps the assistant's entry.
    env.type_into(cx, "entry-amount", "3");
    env.click(cx, "submit");
    assert_eq!(Storage::new(env.st.dir()).load_month(this_month()).entries.len(), 3);

    // Settings changed elsewhere (e.g. the language) are applied too.
    let mut settings = assistant.load_settings().value;
    settings.language = Some(Lang::Fi);
    assistant.save_settings(&settings).unwrap();
    let mut waited = 0;
    while env.read(cx, |app| app.lc.lang) != Lang::Fi && waited < 400 {
        cx.run_until_parked();
        std::thread::sleep(std::time::Duration::from_millis(20));
        waited += 1;
    }
    env.read(cx, |app| assert_eq!(app.settings.language, Some(Lang::Fi)));
    assert!(env.exists(cx, Msg::NavSettings.text(Lang::Fi)), "the window is in Finnish now");
}

#[gpui_kit::test]
fn the_settings_page_fits_the_window_at_every_width(cx: &mut TestAppContext) {
    let env = open_sized(cx, "settings-widths", 1360.0, 860.0, |_, _| {});
    env.click(cx, Msg::NavSettings.text(Lang::En));
    for width in [800.0, 900.0, 1000.0, 1100.0, 1200.0, 1250.0, 1300.0, 1350.0, 1400.0, 1500.0, 1700.0, 2000.0] {
        cx.simulate_window_resize(env.handle, size(px(width), px(860.0)));
        env.ui(cx, |window, _| {
            // The backup buttons are in the right-hand column when there are two columns,
            // and the language pills in the left one: both must be wholly inside the window.
            for id in ["backup-save", "lang-None", "label-total"] {
                let bounds = window.find(id).bounds();
                assert!(bounds.right() <= px(width), "{id} sticks out at width {width}: right edge {:?}", bounds.right());
            }
        });
    }
}

#[gpui_kit::test]
fn the_update_check_and_the_assistant_are_on_the_page_without_unfolding_anything(cx: &mut TestAppContext) {
    let env = open(cx, "advanced", |_, _| {});
    env.click(cx, Msg::NavSettings.text(Lang::En));
    // Nothing is folded away: the update check and the MCP server are there at once.
    for id in ["lang-None", "theme-None", "label-total", "backup-save", "updates-false", "mcp-true"] {
        assert!(env.exists(cx, id), "{id}");
    }
    assert!(!env.exists(cx, "mcp-changes-true"), "the MCP details still follow its own switch");
}

#[gpui_kit::test]
fn the_everyday_settings_come_before_the_rest(cx: &mut TestAppContext) {
    // One column in a narrow window: the chart's names first, then language, appearance, backup, then the advanced ones.
    let env = open_sized(cx, "settings-order", 1100.0, 860.0, |_, _| {});
    env.click(cx, Msg::NavSettings.text(Lang::En));
    env.ui(cx, |window, _| {
        let tops: Vec<_> = ["label-total", "lang-None", "theme-None", "backup-save", "updates-false", "mcp-true"]
            .iter()
            .map(|id| window.find(*id).bounds().top())
            .collect();
        assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    });
}

#[gpui_kit::test]
fn changing_the_language_renames_the_starter_categories_that_were_not_edited(cx: &mut TestAppContext) {
    let mut ids = (0, 0);
    let env = open(cx, "starter-names", |st, cats| {
        // One starter category has been renamed by the user; the rest are as they came.
        let mut cats = cats.clone();
        let food = cats.items.iter().position(|c| c.name == "Food").unwrap();
        cats.items[food].name = "Groceries".to_string();
        ids = (cats.items[food].id, category_id(&cats, "Housing"));
        st.save_categories(&cats).unwrap();
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, ids.0, "Lunch", 990), entry(2, ids.1, "Rent", 80000)] }).unwrap();
    });
    env.click(cx, Msg::NavSettings.text(Lang::En));

    // No restart needed: the names change as soon as the language does.
    env.click(cx, "lang-Some(Fi)");
    let fi = Categories::defaults_for(Lang::Fi);
    env.read(cx, |app| {
        assert_eq!(app.cats.get(ids.0).unwrap().name, "Groceries", "an edited category keeps its name");
        assert_eq!(app.cats.get(ids.1).unwrap().name, fi.get(ids.1).unwrap().name);
        assert_eq!(app.cats.get(1).unwrap().name, fi.get(1).unwrap().name);
        // The entries still point at the same categories.
        assert_eq!(app.entries.iter().map(|e| e.category_id).collect::<Vec<_>>(), vec![ids.0, ids.1]);
    });
    // It was saved, not only shown.
    assert_eq!(env.st.load_categories().get(ids.1).unwrap().name, fi.get(ids.1).unwrap().name);

    // And back again.
    env.click(cx, "lang-Some(En)");
    env.read(cx, |app| assert_eq!(app.cats.get(ids.1).unwrap().name, "Housing"));
}

#[gpui_kit::test]
fn automatic_language_translates_the_starter_categories_when_the_app_starts(cx: &mut TestAppContext) {
    // The data was made in English but the settings now say Finnish (e.g. the system language changed).
    let env = open(cx, "starter-start", |st, _| {
        let mut settings = st.load_settings().value;
        settings.language = Some(Lang::Fi);
        st.save_settings(&settings).unwrap();
    });
    let fi = Categories::defaults_for(Lang::Fi);
    env.read(cx, |app| {
        assert_eq!(app.lc.lang, Lang::Fi);
        assert_eq!(app.cats.items.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), fi.items.iter().map(|c| c.name.clone()).collect::<Vec<_>>());
    });
    assert_eq!(env.st.load_categories().get(4).unwrap().name, fi.get(4).unwrap().name);
}

#[gpui_kit::test]
fn a_newer_release_offers_a_direct_download_and_the_release_page(cx: &mut TestAppContext) {
    use crate::update::Release;
    let env = open(cx, "update-banner", |_, _| {});
    assert!(!env.exists(cx, "update-view"), "no banner without a newer release");

    let page = "https://github.com/isaul32/cashflow/releases/tag/v9.9.9";
    let file = "https://github.com/isaul32/cashflow/releases/download/v9.9.9/cashflow-9.9.9-windows-x64-setup.exe";
    env.modify(cx, |app| {
        app.update = Some(Release { version: "9.9.9".into(), url: page.into(), download: Some(file.into()) });
    });
    assert!(env.exists(cx, "update-download") && env.exists(cx, "update-view"));

    env.click(cx, "update-download");
    assert_eq!(cx.opened_url().as_deref(), Some(file), "the file itself, not the page");
    env.click(cx, "update-view");
    assert_eq!(cx.opened_url().as_deref(), Some(page));
    env.read(cx, |app| assert!(app.update.is_some(), "opening a link does not dismiss the notice"));

    // A release that has no file for this computer shows only the page.
    env.modify(cx, |app| app.update = Some(Release { version: "9.9.9".into(), url: page.into(), download: None }));
    assert!(!env.exists(cx, "update-download") && env.exists(cx, "update-view"));

    env.click(cx, "update-dismiss");
    env.read(cx, |app| assert!(app.update.is_none()));
}

#[gpui_kit::test]
fn the_colour_picker_is_one_row_and_a_new_category_takes_its_parents_colour(cx: &mut TestAppContext) {
    let env = open(cx, "cat-colour", |_, _| {});
    env.click(cx, Msg::NavCategories.text(Lang::En));
    let swatch = |color: u32| format!("swatch-{color}");

    // One row of colours: the vivid ones are there, the darker and softer rows of before are not.
    assert!(env.exists(cx, swatch(0x3F51B5)));
    assert!(!env.exists(cx, swatch(0x1A237E)) && !env.exists(cx, swatch(0x8E99C4)));

    // Choosing a parent gives the new category the parent's colour...
    let (food, rent) = (env.cat(cx, "Food"), env.cat(cx, "Rent"));
    let (food_color, rent_color) = env.read(cx, |app| (app.cats.get(food).unwrap().color, app.cats.get(rent).unwrap().color));
    env.click(cx, format!("parent-{food}"));
    env.read(cx, |app| assert_eq!(app.cat_color, food_color));
    env.type_into(cx, "cat-name", "Snacks");
    env.click(cx, "add-cat");
    let snacks = env.cat(cx, "Snacks");
    env.read(cx, |app| {
        let c = app.cats.get(snacks).unwrap();
        assert_eq!((c.parent, c.color), (Some(food), food_color));
    });

    // ...also one that is not in the row: it is shown there, selected.
    env.click(cx, "parent-0");
    env.click(cx, format!("parent-{rent}"));
    env.read(cx, |app| assert_eq!(app.cat_color, rent_color));
    assert!(env.exists(cx, swatch(rent_color)), "the parent's colour is in the picker");

    // Without a parent it goes back to the palette's next colour.
    env.click(cx, "parent-0");
    env.read(cx, |app| assert_eq!(app.cat_color, PALETTE[app.cats.items.len() % PALETTE.len()]));

    // A colour the user picked is theirs: choosing a parent afterwards does not change it.
    env.click(cx, swatch(0x4CAF50));
    env.click(cx, format!("parent-{food}"));
    env.read(cx, |app| assert_eq!(app.cat_color, 0x4CAF50));
    env.type_into(cx, "cat-name", "Treats");
    env.click(cx, "add-cat");
    let treats = env.cat(cx, "Treats");
    env.read(cx, |app| assert_eq!(app.cats.get(treats).unwrap().color, 0x4CAF50));
    // After adding, the form starts over: no parent and no chosen colour.
    env.read(cx, |app| assert!(app.cat_parent.is_none() && !app.cat_color_chosen));
}

#[gpui_kit::test]
fn changing_the_parent_of_an_existing_category_keeps_its_colour(cx: &mut TestAppContext) {
    let env = open(cx, "cat-colour-edit", |_, _| {});
    env.click(cx, Msg::NavCategories.text(Lang::En));
    let (leisure, health) = (env.cat(cx, "Leisure"), env.cat(cx, "Health"));
    let color = env.read(cx, |app| app.cats.get(health).unwrap().color);
    env.click(cx, format!("catedit-{health}"));
    env.click(cx, format!("parent-{leisure}"));
    env.read(cx, |app| assert_eq!(app.cat_color, color, "only a new category follows its parent"));
    env.click(cx, "add-cat");
    env.read(cx, |app| {
        let c = app.cats.get(health).unwrap();
        assert_eq!((c.parent, c.color), (Some(leisure), color));
    });
}

/// Scrolls the settings page to the end, where the reset buttons are.
fn scroll_settings_to_the_end(env: &Env, cx: &mut TestAppContext) {
    env.ui(cx, |window, cx| {
        window.scroll("settings-scroll", ScrollDelta::Pixels(point(px(0.), px(-6000.))), cx);
    });
}

#[gpui_kit::test]
fn all_settings_can_be_reset_after_confirming(cx: &mut TestAppContext) {
    let env = open(cx, "reset-settings", |st, cats| {
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, category_id(cats, "Food"), "Lunch", 990)] }).unwrap();
    });
    // The system speaks English, so "automatic" means English here whatever the machine says.
    env.modify(cx, |app| {
        app.system = SystemLocale { lang: Some(Lang::En), region: Some("FI".into()) };
        app.settings.language = Some(Lang::Fi);
        app.settings.theme = Some(ThemeChoice::Dark);
        app.settings.check_updates = false;
        app.settings.mcp.enabled = true;
        app.settings.mcp.allow_changes = true;
        app.settings.labels.total = "Mine".into();
    });
    env.click(cx, Msg::NavSettings.text(Lang::En));
    scroll_settings_to_the_end(&env, cx);

    // Nothing happens until it is confirmed, and it can be called off.
    env.click(cx, "reset-settings");
    scroll_settings_to_the_end(&env, cx);
    assert!(env.exists(cx, "reset-confirm"));
    env.click(cx, "reset-cancel");
    assert!(!env.exists(cx, "reset-confirm"));
    env.read(cx, |app| assert_eq!(app.settings.language, Some(Lang::Fi)));

    env.click(cx, "reset-settings");
    scroll_settings_to_the_end(&env, cx);
    env.click(cx, "reset-confirm");
    env.read(cx, |app| {
        assert_eq!(app.settings, Settings::default(), "every setting is back to its default");
        assert!(app.settings.check_updates && !app.settings.mcp.enabled && !app.settings.mcp.allow_changes);
        assert_eq!(app.lc.lang, Lang::En, "automatic follows the system again");
        assert!(app.notice.is_some());
        // The data is not touched.
        assert_eq!(app.entries.len(), 1);
    });
    assert_eq!(env.st.load_settings().value, Settings::default(), "it was saved");
    assert_eq!(env.st.load_month(this_month()).entries.len(), 1);
    assert!(!env.exists(cx, "reset-confirm"));
    // The chart's names are the standard ones again.
    env.read_inputs(cx, |app, cx| assert_eq!(app.total_in.read(cx).value().as_ref(), "Total income"));
}

#[gpui_kit::test]
fn all_data_can_be_deleted_after_confirming_and_a_copy_is_kept(cx: &mut TestAppContext) {
    let env = open(cx, "reset-data", |st, cats| {
        let mut cats = cats.clone();
        cats.add("Pets", Kind::Expense, None, 0x112233);
        st.save_categories(&cats).unwrap();
        let food = category_id(&cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Lunch", 990)] }).unwrap();
        st.save_month(this_month().prev(), &MonthData { entries: vec![entry(1, food, "Old", 500)] }).unwrap();
    });
    env.click(cx, Msg::NavSettings.text(Lang::En));
    scroll_settings_to_the_end(&env, cx);

    // Called off: everything stays.
    env.click(cx, "reset-data");
    scroll_settings_to_the_end(&env, cx);
    env.click(cx, "reset-cancel");
    env.read(cx, |app| assert_eq!(app.entries.len(), 1));
    assert_eq!(env.st.list_months().len(), 2);

    env.click(cx, "reset-data");
    scroll_settings_to_the_end(&env, cx);
    env.click(cx, "reset-confirm");
    let starter = Categories::defaults_for(Lang::En);
    env.read(cx, |app| {
        assert!(app.entries.is_empty());
        assert_eq!(app.cats, starter, "the starter categories are back, in the language of the app");
        assert!(app.notice.is_some());
    });
    assert!(env.st.list_months().is_empty());
    assert_eq!(env.st.load_categories(), starter);
    // Settings are not data: they stay.
    assert_eq!(env.st.load_settings().value.language, Some(Lang::En));

    // A copy of what was deleted is where a restore's copy goes, and it is a backup that can be restored.
    let copies: Vec<_> = std::fs::read_dir(env.st.dir().join("backups")).unwrap().flatten().collect();
    assert_eq!(copies.len(), 1);
    let saved = crate::backup::parse(&std::fs::read_to_string(copies[0].path()).unwrap()).unwrap();
    assert_eq!(saved.months.len(), 2);
    assert!(saved.categories.items.iter().any(|c| c.name == "Pets"));
}

#[gpui_kit::test]
fn only_one_reset_question_is_asked_at_a_time(cx: &mut TestAppContext) {
    let env = open(cx, "reset-one", |_, _| {});
    env.click(cx, Msg::NavSettings.text(Lang::En));
    scroll_settings_to_the_end(&env, cx);
    env.click(cx, "reset-data");
    scroll_settings_to_the_end(&env, cx);
    env.click(cx, "reset-settings");
    scroll_settings_to_the_end(&env, cx);
    env.read(cx, |app| assert!(app.pending_reset == Some(Reset::Settings)), );
    env.click(cx, "reset-confirm");
    // Confirming answered the second question only: the data is as it was (empty, but the file was not replaced).
    assert!(!env.st.dir().join("backups").exists(), "deleting data was not what was confirmed");
}

/// The screenshots show the demo data in `site/tools/demo`. It must use the starter categories
/// exactly, so the images never show categories that a new user does not get.
#[test]
fn the_demo_data_uses_the_starter_categories() {
    for (code, lang) in [("en", Lang::En), ("fi", Lang::Fi), ("sv", Lang::Sv), ("de", Lang::De), ("fr", Lang::Fr), ("es", Lang::Es)] {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("site/tools/demo").join(code);
        let st = Storage::new(&dir);
        let cats = st.load_categories();
        assert_eq!(cats, Categories::defaults_for(lang), "{}: demo categories differ from the starter ones", code);
        let months = st.list_months();
        assert!(!months.is_empty(), "{}: no demo entries", code);
        for month in months {
            for e in st.load_month(month).entries {
                assert!(cats.get(e.category_id).is_some(), "{}: entry {:?} has an unknown category", code, e.name);
            }
        }
    }
}
