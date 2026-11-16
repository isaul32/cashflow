//! Protocol tests: the server is driven through `serve` with in-memory pipes, exactly as
//! an assistant drives the `cashflow --mcp` process (the real process is in `tests/mcp_stdio.rs`).

use std::fs;
use std::io::Cursor;

use serde_json::{Value, json};

use super::*;
use crate::i18n::{Currency, Lang};
use crate::model::Categories;
use crate::settings::{McpSettings, Settings, ThemeChoice};

/// A data folder with fixed settings (English, euro) and the starter categories.
pub(crate) struct Fixture {
    pub st: Storage,
}

impl Fixture {
    /// `enabled` and `allow_changes` are what the user chose in the app's settings.
    pub fn new(name: &str, enabled: bool, allow_changes: bool) -> Fixture {
        let dir = std::env::temp_dir().join(format!("budget-test-{}-mcp-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let st = Storage::new(&dir);
        let fx = Fixture { st };
        fx.set_mcp(enabled, allow_changes);
        fx.st.save_categories(&Categories::defaults_for(Lang::En)).unwrap();
        fx
    }

    /// Changes the permissions the way the app's settings do: by writing the settings file.
    pub fn set_mcp(&self, enabled: bool, allow_changes: bool) {
        self.st
            .save_settings(&Settings {
                language: Some(Lang::En),
                currency: Some(Currency::Eur),
                theme: Some(ThemeChoice::Light),
                mcp: McpSettings { enabled, allow_changes },
                ..Settings::default()
            })
            .unwrap();
    }

    /// Runs a tool directly.
    pub fn tool(&self, name: &str, args: Value) -> Result<Value, String> {
        workspace::call(&self.st, name, &args)
    }

    /// Sends the messages (one per line) and returns the replies, one per line.
    pub fn session(&self, messages: &[Value]) -> Vec<Value> {
        let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
        let mut output = Vec::new();
        serve(Cursor::new(input), &mut output, &self.st).unwrap();
        String::from_utf8(output).unwrap().lines().map(|l| serde_json::from_str(l).expect("a reply is one JSON line")).collect()
    }

    /// One request, one reply.
    pub fn rpc(&self, method: &str, params: Value) -> Value {
        let mut replies = self.session(&[json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params})]);
        assert_eq!(replies.len(), 1);
        replies.remove(0)
    }

    pub fn tool_names(&self) -> Vec<String> {
        let reply = self.rpc("tools/list", json!({}));
        reply["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect()
    }
}

#[test]
fn initialize_negotiates_the_protocol_version() {
    let fx = Fixture::new("init", true, false);
    let reply = fx.rpc("initialize", json!({"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}}));
    assert_eq!(reply["id"], 1);
    assert_eq!(reply["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(reply["result"]["serverInfo"]["name"], "cashflow");
    assert!(reply["result"]["capabilities"]["tools"].is_object());
    let instructions = reply["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("CashFlow") && !instructions.contains("turned off"));
    // A version it does not know gets the newest one it does.
    assert_eq!(fx.rpc("initialize", json!({"protocolVersion": "1999-01-01"}))["result"]["protocolVersion"], PROTOCOLS[0]);
}

#[test]
fn initialize_says_when_the_server_is_off() {
    let fx = Fixture::new("init-off", false, false);
    let reply = fx.rpc("initialize", json!({"protocolVersion": "2025-06-18"}));
    assert!(reply["result"]["instructions"].as_str().unwrap().contains("turned off"));
}

#[test]
fn notifications_get_no_answer_and_unknown_methods_an_error() {
    let fx = Fixture::new("notify", true, false);
    let replies = fx.session(&[
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 7, "method": "ping"}),
        json!({"jsonrpc": "2.0", "id": 8, "method": "resources/list"}),
    ]);
    assert_eq!(replies.len(), 2, "the notification has no reply");
    assert_eq!((replies[0]["id"].as_i64(), &replies[0]["result"]), (Some(7), &json!({})));
    assert_eq!((replies[1]["id"].as_i64(), replies[1]["error"]["code"].as_i64()), (Some(8), Some(-32601)));
}

#[test]
fn the_tools_on_offer_follow_the_users_settings_live() {
    let fx = Fixture::new("list", false, false);
    assert!(fx.tool_names().is_empty(), "nothing is offered until the user turns it on");

    fx.set_mcp(true, false);
    let read_only = fx.tool_names();
    assert!(read_only.contains(&"list_entries".to_string()));
    assert!(!read_only.contains(&"add_entry".to_string()) && !read_only.contains(&"restore_backup".to_string()));

    // The same session notices when the user allows changes in the app, and when they take it back.
    let input = |id: u64| json!({"jsonrpc": "2.0", "id": id, "method": "tools/list"});
    fx.set_mcp(true, true);
    let all = fx.session(&[input(1)]);
    assert_eq!(all[0]["result"]["tools"].as_array().unwrap().len(), tools::TOOLS.len());
    fx.set_mcp(false, true);
    assert!(fx.tool_names().is_empty());

    // Every tool has a description and an object schema, and the flags match the definitions.
    fx.set_mcp(true, true);
    let reply = fx.rpc("tools/list", json!({}));
    for tool in reply["result"]["tools"].as_array().unwrap() {
        assert!(!tool["description"].as_str().unwrap().is_empty());
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
        let write = tools::find(tool["name"].as_str().unwrap()).unwrap().write;
        assert_eq!(tool["annotations"]["readOnlyHint"].as_bool().unwrap(), !write);
    }
}

#[test]
fn a_tool_call_returns_text_and_structured_content() {
    let fx = Fixture::new("call", true, false);
    let reply = fx.rpc("tools/call", json!({"name": "list_categories", "arguments": {}}));
    assert_eq!(reply["result"]["isError"], false);
    assert!(reply["result"]["structuredContent"]["categories"].as_array().unwrap().len() >= 6);
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Food"));
    // Arguments are optional.
    let reply = fx.rpc("tools/call", json!({"name": "get_state"}));
    assert_eq!(reply["result"]["isError"], false);
    assert_eq!(reply["result"]["structuredContent"]["currency"], "EUR");
}

#[test]
fn a_failing_tool_is_a_result_the_model_can_read() {
    let fx = Fixture::new("fail", true, true);
    let reply = fx.rpc("tools/call", json!({"name": "delete_entry", "arguments": {"id": 99}}));
    assert_eq!(reply["result"]["isError"], true);
    assert!(reply["result"]["content"][0]["text"].as_str().unwrap().contains("no entry 99"));
    assert!(reply.get("error").is_none());
}

#[test]
fn calls_are_refused_unless_the_user_allowed_them() {
    let fx = Fixture::new("refuse", false, false);
    let reply = fx.rpc("tools/call", json!({"name": "list_entries", "arguments": {}}));
    assert_eq!(reply["result"]["isError"], true);
    assert!(reply["result"]["content"][0]["text"].as_str().unwrap().contains("MCP server is turned off"));

    fx.set_mcp(true, false);
    assert_eq!(fx.rpc("tools/call", json!({"name": "list_entries"}))["result"]["isError"], false);
    let reply = fx.rpc("tools/call", json!({"name": "add_entry", "arguments": {"category": "Food", "amount": 5}}));
    assert_eq!(reply["result"]["isError"], true);
    assert!(reply["result"]["content"][0]["text"].as_str().unwrap().contains("Changes are turned off"));
    assert!(fx.st.list_months().is_empty(), "nothing was written");

    fx.set_mcp(true, true);
    assert_eq!(fx.rpc("tools/call", json!({"name": "add_entry", "arguments": {"category": "Food", "amount": 5}}))["result"]["isError"], false);
    assert_eq!(fx.st.list_months().len(), 1);
}

#[test]
fn bad_calls_are_protocol_errors() {
    let fx = Fixture::new("bad", true, true);
    assert_eq!(fx.rpc("tools/call", json!({"name": "no_such_tool"}))["error"]["code"], -32602);
    assert_eq!(fx.rpc("tools/call", json!({}))["error"]["code"], -32602);
    assert_eq!(fx.rpc("tools/call", json!({"name": "get_state", "arguments": [1]}))["error"]["code"], -32602);
}

#[test]
fn broken_input_and_batches_are_handled() {
    let fx = Fixture::new("batch", true, false);
    let input = format!(
        "\u{feff}{{ not json\n\n{}\n   \n{}\n",
        json!([
            {"jsonrpc": "2.0", "id": 1, "method": "ping"},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "ping"},
        ]),
        json!([{"jsonrpc": "2.0", "method": "notifications/initialized"}]),
    );
    let mut output = Vec::new();
    serve(Cursor::new(input), &mut output, &fx.st).unwrap();
    let lines: Vec<Value> = String::from_utf8(output).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    // A parse error, then the batch's two answers in one reply; the batch of only a notification gets none.
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["error"]["code"], -32700);
    assert_eq!(lines[1].as_array().unwrap().iter().map(|r| r["id"].as_i64().unwrap()).collect::<Vec<_>>(), vec![1, 2]);
}

#[test]
fn replies_are_single_lines_even_with_multi_line_text() {
    let fx = Fixture::new("lines", true, true);
    // A note with line breaks is stored on one line, and the answer to the call is a single line too.
    fx.rpc("tools/call", json!({"name": "add_entry", "arguments": {"category": "Food", "amount": 3, "name": "a\nb"}}));
    let mut output = Vec::new();
    let call = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "export_csv", "arguments": {}}});
    serve(Cursor::new(format!("{call}\n")), &mut output, &fx.st).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let reply: Value = serde_json::from_str(&text).unwrap();
    assert!(reply["result"]["content"][0]["text"].as_str().unwrap().contains('\n'), "the CSV text itself has lines");
}

#[test]
fn the_server_ends_when_the_client_closes_the_pipe() {
    let fx = Fixture::new("eof", true, false);
    let mut output = Vec::new();
    assert!(serve(Cursor::new(String::new()), &mut output, &fx.st).is_ok());
    assert!(output.is_empty());
}
