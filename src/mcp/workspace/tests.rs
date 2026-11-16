//! Tests of the tools on the saved data, without any window.

use std::fs;

use serde_json::{Value, json};

use super::*;
use crate::mcp::tests::Fixture;
use crate::model::Categories;

fn ok(result: Result<Value, String>) -> Value {
    result.unwrap_or_else(|e| panic!("the tool failed: {e}"))
}

fn err(result: Result<Value, String>) -> String {
    match result {
        Ok(v) => panic!("the tool should have failed, it returned {v}"),
        Err(e) => e,
    }
}

fn month() -> MonthKey {
    MonthKey::current()
}

fn entry(id: u64, category_id: CategoryId, name: &str, cents: i64) -> Entry {
    Entry { id, category_id, name: name.to_string(), amount_cents: cents }
}

fn cat(st: &Storage, name: &str) -> CategoryId {
    st.load_categories().items.iter().find(|c| c.name == name).expect("category").id
}

/// Food 9.90 and Salary 2000 this month, Food 5 last month.
fn seed(fx: &Fixture) {
    let (food, salary) = (cat(&fx.st, "Food"), cat(&fx.st, "Salary"));
    fx.st.save_month(month(), &MonthData { entries: vec![entry(1, food, "Lunch", 990), entry(2, salary, "Pay", 200000)] }).unwrap();
    fx.st.save_month(month().prev(), &MonthData { entries: vec![entry(1, food, "Old", 500)] }).unwrap();
}

#[test]
fn reading_tools_describe_the_data() {
    let fx = Fixture::new("w-read", true, false);
    seed(&fx);

    let state = ok(fx.tool("get_state", json!({})));
    assert_eq!((state["current_month"].as_str(), state["language"].as_str(), state["currency"].as_str()), (Some(month().to_string().as_str()), Some("en"), Some("EUR")));
    assert_eq!((state["changes_allowed"].as_bool(), state["months_with_entries"].as_i64()), (Some(false), Some(2)));

    let categories = ok(fx.tool("list_categories", json!({})));
    let list = categories["categories"].as_array().unwrap();
    let food = list.iter().find(|c| c["name"] == "Food").unwrap();
    assert_eq!((food["kind"].as_str(), food["color"].as_str()), (Some("expense"), Some("#ff9800")));

    let entries = ok(fx.tool("list_entries", json!({})));
    assert_eq!(entries["entries"].as_array().unwrap().len(), 2);
    assert_eq!((entries["income"].as_f64(), entries["expenses"].as_f64(), entries["balance"].as_f64()), (Some(2000.0), Some(9.9), Some(1990.1)));
    let lunch = &entries["entries"][0];
    assert_eq!((lunch["name"].as_str(), lunch["category"].as_str(), lunch["amount_cents"].as_i64()), (Some("Lunch"), Some("Food"), Some(990)));

    let before = ok(fx.tool("list_entries", json!({"month": month().prev().to_string()})));
    assert_eq!((before["entries"].as_array().unwrap().len(), before["expenses"].as_f64()), (1, Some(5.0)));

    let summary = ok(fx.tool("get_month_summary", json!({})));
    assert_eq!((summary["income"].as_f64(), summary["expenses"].as_f64()), (Some(2000.0), Some(9.9)));
    assert_eq!(summary["by_category"][0]["category"], "Salary", "income comes first");

    let year = ok(fx.tool("get_year_summary", json!({"year": month().year})));
    assert_eq!(year["months"].as_array().unwrap().len(), 12);
    assert!(year["expenses"].as_f64().unwrap() >= 9.9);
    assert_eq!(ok(fx.tool("list_months", json!({})))["months"].as_array().unwrap().len(), 2);

    let settings = ok(fx.tool("get_settings", json!({})));
    assert_eq!((settings["language"].as_str(), settings["currency"].as_str(), settings["theme"].as_str()), (Some("en"), Some("EUR"), Some("light")));
}

#[test]
fn every_tool_is_refused_while_the_server_is_off() {
    let fx = Fixture::new("w-off", false, true);
    for tool in tools::TOOLS {
        assert!(err(fx.tool(tool.name, json!({}))).contains("MCP server is turned off"), "{}", tool.name);
    }
    assert!(err(fx.tool("no_such_tool", json!({}))).contains("unknown tool"));
}

#[test]
fn changing_tools_need_the_users_permission() {
    let fx = Fixture::new("w-readonly", true, false);
    seed(&fx);
    for (tool, args) in [
        ("add_entry", json!({"category": "Food", "amount": 5})),
        ("update_entry", json!({"id": 1, "amount": 1})),
        ("delete_entry", json!({"id": 1})),
        ("add_category", json!({"name": "X", "kind": "expense"})),
        ("update_category", json!({"category": "Food", "name": "Y"})),
        ("delete_category", json!({"category": "Food"})),
        ("copy_month", json!({})),
        ("set_settings", json!({"theme": "dark"})),
        ("restore_backup", json!({"backup": "{}", "confirm": true})),
        ("import_sankeymatic", json!({"text": "A [1] B"})),
    ] {
        assert!(err(fx.tool(tool, args)).contains("Changes are turned off"), "{tool}");
    }
    assert_eq!(fx.st.load_month(month()).entries.len(), 2);
    // Every tool the definitions call a write tool is covered by the list above.
    let covered = 10;
    assert_eq!(tools::TOOLS.iter().filter(|t| t.write).count(), covered);
}

#[test]
fn adds_entries_by_category_name_or_id() {
    let fx = Fixture::new("w-add", true, true);
    let food = cat(&fx.st, "Food");

    let added = ok(fx.tool("add_entry", json!({"category": "food", "amount": 12.34, "name": "  Dinner \n out "})));
    assert_eq!(added["added"]["amount_cents"], 1234);
    assert_eq!(added["added"]["name"], "Dinner out");
    assert_eq!(added["added"]["category_id"], food);
    ok(fx.tool("add_entry", json!({"category": food, "amount": 5, "month": month().prev().to_string()})));

    let saved = fx.st.load_month(month()).entries;
    assert_eq!((saved.len(), saved[0].amount_cents), (1, 1234));
    assert_eq!(fx.st.load_month(month().prev()).entries.len(), 1);
}

#[test]
fn bad_input_is_explained_and_changes_nothing() {
    let fx = Fixture::new("w-bad", true, true);
    assert!(err(fx.tool("add_entry", json!({"amount": 5}))).contains("category"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food"}))).contains("amount"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food", "amount": 0}))).contains("positive"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food", "amount": -3}))).contains("positive"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food", "amount": "lots"}))).contains("number"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food", "amount": 1e30}))).contains("range"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food", "amount": 5, "month": "2026-13"}))).contains("month"));
    assert!(err(fx.tool("add_entry", json!({"category": "Food", "amount": 5, "name": "x".repeat(300)}))).contains("longer"));
    let message = err(fx.tool("add_entry", json!({"category": "Pets", "amount": 5})));
    assert!(message.contains("Pets") && message.contains("Housing"), "{message}");
    assert!(err(fx.tool("add_entry", json!({"category": 9999, "amount": 5}))).contains("9999"));
    assert!(err(fx.tool("update_entry", json!({"id": 99}))).contains("nothing to change"));
    assert!(err(fx.tool("update_entry", json!({"id": 99, "amount": 5}))).contains("no entry 99"));
    assert!(err(fx.tool("delete_entry", json!({"id": 99}))).contains("no entry 99"));
    assert!(fx.st.list_months().is_empty());
}

#[test]
fn entries_can_be_changed_and_deleted() {
    let fx = Fixture::new("w-edit", true, true);
    seed(&fx);
    let housing = cat(&fx.st, "Housing");

    let updated = ok(fx.tool("update_entry", json!({"id": 1, "amount": 11.5, "category": "Housing", "name": "Rent"})));
    assert_eq!((updated["updated"]["amount_cents"].as_i64(), updated["updated"]["category_id"].as_u64()), (Some(1150), Some(housing)));
    // Only what is given changes.
    ok(fx.tool("update_entry", json!({"id": 1, "name": "Flat"})));
    let saved = fx.st.load_month(month()).entries;
    let flat = saved.iter().find(|e| e.id == 1).unwrap();
    assert_eq!((flat.name.as_str(), flat.amount_cents, flat.category_id), ("Flat", 1150, housing));

    ok(fx.tool("delete_entry", json!({"id": 1})));
    assert_eq!(fx.st.load_month(month()).entries.iter().map(|e| e.id).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn categories_can_be_added_changed_and_deleted() {
    let fx = Fixture::new("w-cats", true, true);
    seed(&fx);
    let food = cat(&fx.st, "Food");

    let pets = ok(fx.tool("add_category", json!({"name": "Pets", "kind": "expense", "color": "#112233"})));
    assert_eq!((pets["added"]["kind"].as_str(), pets["added"]["color"].as_str()), (Some("expense"), Some("#112233")));
    let vet = ok(fx.tool("add_category", json!({"name": "Vet", "kind": "expense", "parent": "Pets"})));
    let (pets_id, vet_id) = (pets["added"]["id"].as_u64().unwrap(), vet["added"]["id"].as_u64().unwrap());
    assert_eq!(fx.st.load_categories().get(vet_id).unwrap().parent, Some(pets_id));

    // The same rules as in the window.
    assert!(err(fx.tool("add_category", json!({"name": "pets", "kind": "expense"}))).contains("already"));
    assert!(err(fx.tool("add_category", json!({"name": "", "kind": "expense"}))).contains("empty"));
    assert!(err(fx.tool("add_category", json!({"name": "Bonus", "kind": "income", "parent": "Pets"}))).contains("same kind"));
    assert!(err(fx.tool("add_category", json!({"name": "Q", "kind": "other"}))).contains("kind"));
    assert!(err(fx.tool("add_category", json!({"name": "Q", "kind": "income", "color": "red"}))).contains("#RRGGBB"));
    ok(fx.tool("add_category", json!({"name": "Pets", "kind": "income"})));
    assert!(err(fx.tool("add_entry", json!({"category": "Pets", "amount": 1}))).contains("several"));

    let renamed = ok(fx.tool("update_category", json!({"category": vet_id, "name": "Vet bills", "parent": null})));
    assert_eq!((renamed["updated"]["name"].as_str(), renamed["updated"]["parent_id"].is_null()), (Some("Vet bills"), true));
    ok(fx.tool("update_category", json!({"category": vet_id, "parent": pets_id})));
    assert!(err(fx.tool("update_category", json!({"category": pets_id, "parent": vet_id}))).contains("not possible"));

    // Deleting a category moves its entries, in every month.
    let removed = ok(fx.tool("delete_category", json!({"category": "Food"})));
    assert_eq!(removed["entries_moved"], 2);
    for m in [month(), month().prev()] {
        assert!(fx.st.load_month(m).entries.iter().all(|e| e.category_id != food), "{m}");
    }
    assert!(fx.st.load_categories().get(food).is_none());
}

#[test]
fn the_only_category_of_a_kind_with_entries_stays() {
    let fx = Fixture::new("w-only", true, true);
    let mut cats = Categories::default();
    cats.items.clear();
    let only = cats.add("Only", Kind::Expense, None, 0x123456);
    fx.st.save_categories(&cats).unwrap();
    fx.st.save_month(month(), &MonthData { entries: vec![entry(1, only, "x", 100)] }).unwrap();
    assert!(err(fx.tool("delete_category", json!({"category": "Only"}))).contains("only expense category"));
    assert_eq!(fx.st.load_categories().items.len(), 1);
}

#[test]
fn a_month_can_be_copied_into_an_empty_one() {
    let fx = Fixture::new("w-copy", true, true);
    seed(&fx); // this month: Lunch and Pay; the month before: Old
    let target = month().next();

    let copied = ok(fx.tool("copy_month", json!({"from": month().to_string(), "to": target.to_string()})));
    assert_eq!((copied["entries_copied"].as_u64(), copied["to"].as_str()), (Some(2), Some(target.to_string().as_str())));
    let entries = fx.st.load_month(target).entries;
    assert_eq!(entries.iter().map(|e| (e.id, e.name.as_str(), e.amount_cents)).collect::<Vec<_>>(), vec![(1, "Lunch", 990), (2, "Pay", 200000)]);
    assert_eq!(fx.st.load_month(month()).entries.len(), 2, "the source is untouched");

    // Without `from` it is the month before the target: here the month before next month is this one.
    let later = target.next();
    ok(fx.tool("copy_month", json!({"to": later.to_string()})).clone());
    assert_eq!(fx.st.load_month(later).entries.len(), 2);

    // A month that has entries is not filled, and the other mistakes are explained.
    assert!(err(fx.tool("copy_month", json!({}))).contains("already has entries"), "this month has entries");
    let far = later.next().next();
    assert!(err(fx.tool("copy_month", json!({"from": far.to_string(), "to": far.next().to_string()}))).contains("no entries to copy"));
    assert!(err(fx.tool("copy_month", json!({"from": target.to_string(), "to": target.to_string()}))).contains("same"));
    assert!(err(fx.tool("copy_month", json!({"to": "nonsense"}))).contains("invalid"));
    assert_eq!(fx.st.load_month(target).entries.len(), 2);
}

#[test]
fn settings_can_be_changed_but_only_the_ordinary_ones() {
    let fx = Fixture::new("w-settings", true, true);

    let changed = ok(fx.tool("set_settings", json!({"language": "fi", "theme": "dark", "labels": {"total": "Tulot"}})));
    assert_eq!((changed["language"].as_str(), changed["theme"].as_str()), (Some("fi"), Some("dark")));
    let saved = fx.st.load_settings().value;
    assert_eq!((saved.language, saved.theme, saved.labels.total.as_str()), (Some(Lang::Fi), Some(ThemeChoice::Dark), "Tulot"));
    // The user's permissions were not touched by saving the other settings.
    assert!(saved.mcp.enabled && saved.mcp.allow_changes);

    ok(fx.tool("set_settings", json!({"language": "auto", "currency": "usd", "theme": "auto"})));
    let saved = fx.st.load_settings().value;
    assert_eq!((saved.language, saved.currency, saved.theme), (None, Some(Currency::Usd), None));

    // Everything is checked first: a bad value changes nothing.
    assert!(err(fx.tool("set_settings", json!({"language": "sv", "currency": "ZZZ"}))).contains("currency"));
    assert_eq!(fx.st.load_settings().value.language, None);
    assert!(err(fx.tool("set_settings", json!({}))).contains("nothing"));
    assert!(err(fx.tool("set_settings", json!({"theme": "purple"}))).contains("theme"));
    assert!(err(fx.tool("set_settings", json!({"labels": {"bogus": "x"}}))).contains("bogus"));

    // The assistant cannot change its own permissions or turn on network use.
    for key in ["mcp", "allow_changes", "enabled", "check_updates"] {
        let message = err(fx.tool("set_settings", json!({key: true})));
        assert!(message.contains("cannot be changed"), "{key}: {message}");
    }
    let saved = fx.st.load_settings().value;
    assert!(saved.mcp.allow_changes && !saved.check_updates);
}

#[test]
fn sankeymatic_text_can_be_imported_into_a_month() {
    let fx = Fixture::new("w-import", true, true);
    seed(&fx);
    let text = ok(fx.tool("export_sankeymatic", json!({})));
    let target = month().next();

    let result = ok(fx.tool("import_sankeymatic", json!({"text": text, "month": target.to_string()})));
    assert_eq!(result["entries_added"], 2);
    let total: i64 = fx.st.load_month(target).entries.iter().map(|e| e.amount_cents).sum();
    assert_eq!(total, 990 + 200000);

    let again = ok(fx.tool("import_sankeymatic", json!({"text": text, "month": target.to_string(), "replace": true})));
    assert_eq!(again["entries_added"], 2);
    assert_eq!(fx.st.load_month(target).entries.len(), 2, "replacing swaps the entries instead of adding");
    assert!(err(fx.tool("import_sankeymatic", json!({"text": ""}))).len() > 3);
    assert!(err(fx.tool("import_sankeymatic", json!({}))).contains("text"));
}

#[test]
fn exports_can_be_read_and_the_backup_restored() {
    let fx = Fixture::new("w-export", true, true);
    seed(&fx);

    let csv = ok(fx.tool("export_csv", json!({})));
    let csv = csv.as_str().unwrap();
    assert!(csv.starts_with("month,type,category,note,amount"), "{csv}");
    assert!(csv.contains(",\"Food\",\"Lunch\",9.90"));
    assert!(ok(fx.tool("export_sankeymatic", json!({"year": month().year}))).as_str().unwrap().contains("Food"));

    let backup = ok(fx.tool("export_backup", json!({})));
    ok(fx.tool("delete_entry", json!({"id": 1})));
    assert_eq!(fx.st.load_month(month()).entries.len(), 1);

    assert!(err(fx.tool("restore_backup", json!({"backup": backup}))).contains("confirm"));
    assert_eq!(fx.st.load_month(month()).entries.len(), 1, "nothing changes without confirm");
    assert!(err(fx.tool("restore_backup", json!({"backup": "not a backup", "confirm": true}))).len() > 3);

    let restored = ok(fx.tool("restore_backup", json!({"backup": backup, "confirm": true})));
    assert!(restored["copy_of_previous_data"].as_str().unwrap().contains("backups"));
    assert_eq!(fx.st.load_month(month()).entries.len(), 2);
}

#[test]
fn the_first_use_saves_the_starter_categories_in_the_users_language() {
    // Nothing has been saved yet and the user's language is Finnish.
    let dir = std::env::temp_dir().join(format!("budget-test-{}-mcp-w-first", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let st = Storage::new(&dir);
    st.save_settings(&Settings {
        language: Some(Lang::Fi),
        currency: Some(Currency::Eur),
        mcp: crate::settings::McpSettings { enabled: true, allow_changes: true },
        ..Settings::default()
    })
    .unwrap();
    assert!(!st.has_categories());

    // Reading already shows them...
    let names: Vec<String> = ok(call(&st, "list_categories", &json!({})))["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"Ruoka".to_string()), "{names:?}");
    assert!(!st.has_categories(), "reading does not create the data file");

    // ...and the first change makes them real.
    ok(call(&st, "add_entry", &json!({"category": "Ruoka", "amount": 5})));
    assert!(st.has_categories());
    assert!(st.load_categories().items.iter().any(|c| c.name == "Ruoka"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_call_sees_what_another_process_saved_meanwhile() {
    let fx = Fixture::new("w-other", true, true);
    ok(fx.tool("add_entry", json!({"category": "Food", "amount": 1, "name": "mine"})));

    // The app's window (another process) adds an entry to the same month.
    let window = Storage::new(fx.st.dir());
    let mut month_data = window.load_month(month());
    month_data.entries.push(entry(month_data.next_entry_id(), 1, "from the window", 200));
    window.save_month(month(), &month_data).unwrap();

    let entries = ok(fx.tool("list_entries", json!({})));
    assert_eq!(entries["entries"].as_array().unwrap().len(), 2);
    // And a change made now keeps the window's entry.
    ok(fx.tool("add_entry", json!({"category": "Food", "amount": 3, "name": "later"})));
    assert_eq!(Storage::new(fx.st.dir()).load_month(month()).entries.len(), 3);
}

#[test]
fn changing_the_language_renames_the_unedited_starter_categories() {
    let fx = Fixture::new("w-starter", true, true);
    ok(fx.tool("update_category", json!({"category": "Food", "name": "Meals"})));

    ok(fx.tool("set_settings", json!({"language": "fi"})));
    let fi = Categories::defaults_for(Lang::Fi);
    let cats = fx.st.load_categories();
    assert_eq!(cats.get(3).unwrap().name, fi.get(3).unwrap().name, "an unedited one follows the language");
    assert_eq!(cats.get(4).unwrap().name, "Meals", "an edited one keeps its name");

    // The tools find categories by their new names.
    ok(fx.tool("add_entry", json!({"category": fi.get(3).unwrap().name, "amount": 5})));
    // Switching to a language where the names are the same does nothing more.
    ok(fx.tool("set_settings", json!({"currency": "usd"})));
    assert_eq!(fx.st.load_categories().get(3).unwrap().name, fi.get(3).unwrap().name);
}

#[test]
fn a_sub_category_starts_in_its_parents_colour() {
    let fx = Fixture::new("w-colour", true, true);
    let food_color = fx.st.load_categories().get(cat(&fx.st, "Food")).unwrap().color;
    let hex = |color: u32| format!("#{color:06x}");

    let snacks = ok(fx.tool("add_category", json!({"name": "Snacks", "kind": "expense", "parent": "Food"})));
    assert_eq!(snacks["added"]["color"], hex(food_color));
    // A colour that is given wins.
    let treats = ok(fx.tool("add_category", json!({"name": "Treats", "kind": "expense", "parent": "Food", "color": "#112233"})));
    assert_eq!(treats["added"]["color"], "#112233");
    // Without a parent it is the next colour of the palette, not any category's.
    let next = PALETTE[fx.st.load_categories().items.len() % PALETTE.len()];
    let top = ok(fx.tool("add_category", json!({"name": "Pets", "kind": "expense"})));
    assert_eq!(top["added"]["color"], hex(next));
}
