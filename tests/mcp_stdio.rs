//! Runs the real program as an assistant does: `cashflow --mcp` with pipes for stdin and
//! stdout. This also checks that the Windows build, which has no console, can use pipes.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

fn data_dir(name: &str, mcp: Value) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("budget-test-{}-stdio-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let settings = json!({"language": "en", "currency": "eur", "theme": "light", "mcp": mcp});
    std::fs::write(dir.join("settings.json"), settings.to_string()).unwrap();
    dir
}

/// The program, started with `--mcp`, and its pipes.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
}

impl Session {
    fn start(dir: &Path) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cashflow"))
            .arg("--mcp")
            .env("BUDGET_DATA_DIR", dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start cashflow --mcp");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        // Read on a thread so that a program that never answers fails the test instead of hanging it.
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Session { child, stdin, lines, next_id: 0 }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn receive(&mut self) -> Value {
        let line = self.lines.recv_timeout(Duration::from_secs(30)).expect("the program answered");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"))
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        let reply = self.receive();
        assert_eq!(reply["id"], id, "{reply}");
        reply
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": tool, "arguments": arguments}))["result"].clone()
    }

    /// Closes stdin, as a client does when it is done, and waits for the program to end.
    fn finish(mut self) -> (std::process::ExitStatus, String) {
        drop(self.stdin.take());
        let started = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(started.elapsed() < Duration::from_secs(20), "the program did not end when stdin closed");
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut stderr = String::new();
        if let Some(mut pipe) = self.child.stderr.take() {
            std::io::Read::read_to_string(&mut pipe, &mut stderr).unwrap();
        }
        (status, stderr)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn an_assistant_can_use_the_program_over_stdio() {
    let dir = data_dir("use", json!({"enabled": true, "allow_changes": true}));
    let mut session = Session::start(&dir);

    let init = session.request("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}));
    assert_eq!(init["result"]["serverInfo"]["name"], "cashflow");
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    session.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));

    let tools = session.request("tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"add_entry") && names.contains(&"get_month_summary"), "{names:?}");

    let added = session.call("add_entry", json!({"category": "Food", "amount": 12.5, "name": "Lunch"}));
    assert_eq!(added["isError"], false, "{added}");
    let entries = session.call("list_entries", json!({}));
    assert_eq!(entries["structuredContent"]["entries"][0]["name"], "Lunch");
    assert_eq!(entries["structuredContent"]["expenses"], 12.5);

    let (status, stderr) = session.finish();
    assert!(status.success(), "exit status {status}, stderr: {stderr}");
    assert!(stderr.is_empty(), "nothing but protocol goes out: {stderr}");

    // The data is in the app's data file.
    let data: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("data.json")).unwrap()).unwrap();
    let month = chrono::Local::now().format("%Y-%m").to_string();
    assert_eq!(data["months"][&month]["entries"][0]["amount_cents"], 1250, "{data}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_program_offers_nothing_while_the_user_has_not_allowed_it() {
    let dir = data_dir("off", json!({"enabled": false, "allow_changes": false}));
    let mut session = Session::start(&dir);
    let init = session.request("initialize", json!({"protocolVersion": "2025-06-18"}));
    assert!(init["result"]["instructions"].as_str().unwrap().contains("turned off"));
    assert!(session.request("tools/list", json!({}))["result"]["tools"].as_array().unwrap().is_empty());
    let result = session.call("list_entries", json!({}));
    assert_eq!(result["isError"], true);
    assert!(session.finish().0.success());
    assert!(!dir.join("data.json").exists(), "a refused session must not create data");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn permissions_changed_in_the_app_apply_to_a_running_session() {
    let dir = data_dir("live", json!({"enabled": true, "allow_changes": false}));
    let mut session = Session::start(&dir);
    session.request("initialize", json!({"protocolVersion": "2025-06-18"}));
    assert_eq!(session.call("add_entry", json!({"category": "Food", "amount": 1}))["isError"], true);

    // The user allows changes in the app's settings; the assistant's session is not restarted.
    let settings = json!({"language": "en", "currency": "eur", "mcp": {"enabled": true, "allow_changes": true}});
    std::fs::write(dir.join("settings.json"), settings.to_string()).unwrap();
    assert_eq!(session.call("add_entry", json!({"category": "Food", "amount": 1}))["isError"], false);
    assert!(session.finish().0.success());
    let _ = std::fs::remove_dir_all(&dir);
}
