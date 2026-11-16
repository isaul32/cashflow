//! An optional MCP (Model Context Protocol) server, so that an AI assistant such as
//! Claude Code can read and manage the budget.
//!
//! It speaks MCP over **stdio**: the assistant starts `cashflow --mcp` as a child process and
//! talks to it through the pipes, one JSON-RPC message per line. There is no window and nothing
//! listens on the network. Access is controlled by the user in the app's settings, which this
//! process reads on every request:
//!
//! - nothing is offered until the user turns the MCP server on;
//! - tools that change data are not offered, and are refused, unless the user allowed changes.
//!
//! The process works on the same data files as the app. [`crate::storage`] makes sure that the
//! two never overwrite each other, and the window reloads when this process changes something.

pub mod tools;
mod workspace;

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::storage::Storage;

/// Protocol versions understood, newest first. The client's version is used when it is one of these.
const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// Runs the server on the process's stdin and stdout until the client closes the pipe.
/// Returns the exit code. Problems go to stderr: stdout is the protocol.
pub fn run_stdio() -> i32 {
    let Some(storage) = Storage::default_location() else {
        eprintln!("cashflow --mcp: no data folder is available");
        return 1;
    };
    for warning in storage.take_warnings() {
        eprintln!("cashflow --mcp: {warning}");
    }
    let (stdin, stdout) = (std::io::stdin(), std::io::stdout());
    match serve(stdin.lock(), stdout.lock(), &storage) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("cashflow --mcp: {e}");
            1
        }
    }
}

/// Reads messages (one per line) from `input` and writes the answers to `output`.
pub fn serve(input: impl BufRead, mut output: impl Write, st: &Storage) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(st, line) {
            // A reply is a single line: compact JSON has no line breaks.
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn handle_line(st: &Storage, line: &str) -> Option<Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(error_reply(&Value::Null, -32700, "parse error"));
    };
    match message {
        Value::Array(batch) => {
            let replies: Vec<Value> = batch.iter().filter_map(|m| handle_message(st, m)).collect();
            (!replies.is_empty()).then_some(Value::Array(replies))
        }
        other => handle_message(st, &other),
    }
}

fn error_reply(id: &Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Answers one JSON-RPC message; `None` for notifications and responses, which get no answer.
fn handle_message(st: &Storage, message: &Value) -> Option<Value> {
    let method = message.get("method").and_then(Value::as_str)?;
    let id = message.get("id")?.clone();
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(initialize(st, &params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": list_tools(st)})),
        "tools/call" => call_tool(st, &params),
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => error_reply(&id, code, &message),
    })
}

/// The tools on offer right now, according to the user's current settings.
fn list_tools(st: &Storage) -> Vec<Value> {
    st.reload_if_changed();
    let mcp = st.load_settings().value.mcp;
    if mcp.enabled { tools::list(mcp.allow_changes) } else { Vec::new() }
}

fn initialize(st: &Storage, params: &Value) -> Value {
    let wanted = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
    let version = PROTOCOLS.iter().find(|v| **v == wanted).unwrap_or(&PROTOCOLS[0]);
    let mut instructions = tools::INSTRUCTIONS.to_string();
    if !st.load_settings().value.mcp.enabled {
        instructions.push(' ');
        instructions.push_str(tools::OFF);
    }
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {"name": "cashflow", "version": env!("CARGO_PKG_VERSION")},
        "instructions": instructions,
    })
}

fn call_tool(st: &Storage, params: &Value) -> Result<Value, (i32, String)> {
    let name = params.get("name").and_then(Value::as_str).ok_or((-32602, "the tool name is missing".to_string()))?;
    tools::find(name).ok_or((-32602, format!("unknown tool: {name}")))?;
    let args = match params.get("arguments") {
        None | Some(Value::Null) => json!({}),
        Some(v @ Value::Object(_)) => v.clone(),
        Some(_) => return Err((-32602, "arguments must be an object".to_string())),
    };
    Ok(tool_result(workspace::call(st, name, &args)))
}

/// The MCP form of a tool's outcome. Failures are results with `isError`, so the model sees them.
fn tool_result(outcome: Result<Value, String>) -> Value {
    match outcome {
        Ok(value) => {
            let text = match &value {
                Value::String(s) => s.clone(),
                other => serde_json::to_string_pretty(other).unwrap_or_default(),
            };
            let mut result = json!({"content": [{"type": "text", "text": text}], "isError": false});
            if value.is_object() {
                result["structuredContent"] = value;
            }
            result
        }
        Err(message) => json!({"content": [{"type": "text", "text": message}], "isError": true}),
    }
}

#[cfg(test)]
mod tests;
