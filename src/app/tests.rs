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
struct Env {
    handle: AnyWindowHandle,
    app: Entity<CashFlowApp>,
    st: Storage,
}

fn open(cx: &mut TestAppContext, name: &str, seed: impl FnOnce(&Storage, &Categories)) -> Env {
    open_sized(cx, name, 1360.0, 860.0, seed)
}

fn open_sized(
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
    fn ui<R>(&self, cx: &mut TestAppContext, f: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        cx.update_window(self.handle, |_, window, cx| {
            window.render_frame(cx);
            f(window, cx)
        })
        .expect("the window is open")
    }

    /// Reads from the app state.
    fn read<R>(&self, cx: &mut TestAppContext, f: impl FnOnce(&CashFlowApp) -> R) -> R {
        cx.read(|cx| f(self.app.read(cx)))
    }

    /// Like [`Self::read`], with the app context too (needed to read the text of an input).
    fn read_inputs<R>(&self, cx: &mut TestAppContext, f: impl FnOnce(&CashFlowApp, &App) -> R) -> R {
        cx.read(|cx| f(self.app.read(cx), cx))
    }

    /// Changes the app state directly, for setting up what a test starts from.
    fn modify(&self, cx: &mut TestAppContext, f: impl FnOnce(&mut CashFlowApp)) {
        cx.update(|cx| self.app.update(cx, |app, cx| {
            f(app);
            cx.notify();
        }));
    }

    fn click(&self, cx: &mut TestAppContext, id: impl Into<ElementId>) {
        let id = id.into();
        self.ui(cx, |window, cx| window.click(id, cx));
    }

    /// Replaces the text of the focused-by-click input with `text`.
    fn type_into(&self, cx: &mut TestAppContext, id: &'static str, text: &str) {
        self.ui(cx, |window, cx| {
            window.click(id, cx);
            window.press("secondary-a", cx);
            window.input(text, cx);
        });
    }

    fn exists(&self, cx: &mut TestAppContext, id: impl Into<ElementId>) -> bool {
        let id = id.into();
        self.ui(cx, |window, _| window.try_find(id).is_some())
    }

    fn cat(&self, cx: &mut TestAppContext, name: &str) -> CategoryId {
        self.read(cx, |app| app.cats.items.iter().find(|c| c.name == name).expect("category").id)
    }
}

fn this_month() -> MonthKey {
    MonthKey::current()
}

fn entry(id: u64, category_id: CategoryId, name: &str, cents: i64) -> Entry {
    Entry { id, category_id, name: name.to_string(), amount_cents: cents }
}

fn category_id(cats: &Categories, name: &str) -> CategoryId {
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
        (Msg::NavRecurring, Page::Recurring),
        (Msg::NavYear, Page::Year),
        (Msg::NavOverview, Page::Overview),
    ];
    for (label, page) in pages {
        env.click(cx, label.text(Lang::En));
        env.read(cx, |app| assert!(app.page == page, "{}", label.text(Lang::En)));
    }
}

#[gpui_kit::test]
fn a_repeating_entry_is_added_to_the_next_month_when_it_is_opened(cx: &mut TestAppContext) {
    let env = open(cx, "repeat", |_, _| {});
    let housing = env.cat(cx, "Housing");
    let today = this_month();

    env.click(cx, format!("cat-{housing}"));
    env.type_into(cx, "entry-name", "Rent");
    env.type_into(cx, "entry-amount", "800");
    env.click(cx, "repeat-toggle");
    env.click(cx, "submit");
    env.read(cx, |app| {
        assert_eq!(app.recurring.items.len(), 1);
        assert!(!app.repeat, "the toggle resets for the next entry");
        assert_eq!(app.entries.len(), 1);
    });

    // The month before the start and the months in between are not touched...
    env.click(cx, "month-prev");
    env.read(cx, |app| assert!(app.entries.is_empty()));
    env.click(cx, "month-today");
    // ...the next month gets the entry the moment it is opened.
    env.click(cx, "month-next");
    env.read(cx, |app| {
        assert_eq!(app.month, today.next());
        assert_eq!(app.entries.len(), 1);
        let e = &app.entries[0];
        assert_eq!((e.name.as_str(), e.category_id, e.amount_cents), ("Rent", housing, 80000));
    });
    // Months nobody opened stay empty.
    assert!(env.st.load_month(today.next().next()).entries.is_empty());
    assert!(env.st.load_month(today.prev()).entries.is_empty());
}

#[gpui_kit::test]
fn recurring_entries_can_be_stopped_and_deleted(cx: &mut TestAppContext) {
    let env = open(cx, "recurring-page", |st, cats| {
        let housing = category_id(cats, "Housing");
        let mut store = crate::recurring::RecurringStore::default();
        store.add(housing, "Rent".into(), 80000, this_month());
        st.save_recurring(&store).unwrap();
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, housing, "Rent", 80000)] }).unwrap();
    });
    env.click(cx, Msg::NavRecurring.text(Lang::En));
    assert!(env.exists(cx, "rec-1"));
    assert!(env.exists(cx, "rec-stop-1"));

    env.click(cx, "rec-stop-1");
    env.read(cx, |app| assert_eq!(app.recurring.items[0].end, Some(this_month())));
    assert!(!env.exists(cx, "rec-stop-1"), "an ended entry cannot be stopped again");
    assert_eq!(env.st.load_recurring().items[0].end, Some(this_month()));

    env.click(cx, "rec-del-1");
    env.read(cx, |app| {
        assert!(app.recurring.items.is_empty());
        // Entries it already added stay.
        assert_eq!(app.entries.len(), 1);
    });
    assert!(env.st.load_recurring().items.is_empty());
    assert!(!env.exists(cx, "rec-1"));
}

#[gpui_kit::test]
fn an_edited_entry_can_be_made_recurring_once(cx: &mut TestAppContext) {
    let env = open(cx, "edit-repeat", |st, cats| {
        let food = category_id(cats, "Food");
        st.save_month(this_month(), &MonthData { entries: vec![entry(1, food, "Gym", 3000)] }).unwrap();
    });
    for _ in 0..2 {
        env.click(cx, "edit-1");
        env.click(cx, "repeat-toggle");
        env.click(cx, "submit");
    }
    // The second time the same values already repeat, so there is no second template.
    env.read(cx, |app| {
        assert_eq!(app.recurring.items.len(), 1);
        assert_eq!(app.recurring.items[0].name, "Gym");
        assert_eq!(app.entries.len(), 1);
    });
    assert_eq!(env.st.load_recurring().items.len(), 1);
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
    assert!(!env.ui(cx, |_, cx| cx.theme().is_dark()));

    env.click(cx, "theme-Some(Dark)");
    assert!(env.ui(cx, |_, cx| cx.theme().is_dark()));
    assert_eq!(env.st.load_settings().value.theme, Some(ThemeChoice::Dark));

    env.click(cx, "theme-Some(Light)");
    assert!(!env.ui(cx, |_, cx| cx.theme().is_dark()));
}

#[gpui_kit::test]
fn the_update_check_is_off_by_default(cx: &mut TestAppContext) {
    let env = open(cx, "updates", |_, _| {});
    env.read(cx, |app| {
        assert!(!app.settings.check_updates);
        assert!(app.update.is_none());
    });
    env.click(cx, Msg::NavSettings.text(Lang::En));
    // Both choices are there; "Off" is the selected one. (Turning it on would contact GitHub.)
    assert!(env.exists(cx, "updates-false"));
    assert!(env.exists(cx, "updates-true"));
    env.click(cx, "updates-false");
    assert!(!env.st.load_settings().value.check_updates);
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
        for page in [Msg::NavOverview, Msg::NavYear, Msg::NavSettings, Msg::NavRecurring] {
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
