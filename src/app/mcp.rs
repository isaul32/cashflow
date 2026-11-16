//! The MCP settings in the window.
//!
//! The server itself is `cashflow --mcp`, a separate process that an AI assistant starts (see
//! `crate::mcp`). The window only holds the user's permissions, which that process reads from
//! the settings file, and helps to connect an assistant.

use std::path::PathBuf;

use super::*;

impl CashFlowApp {
    pub(super) fn set_mcp_enabled(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.mcp.enabled = on;
        self.persist_settings();
        cx.notify();
    }

    pub(super) fn set_mcp_changes(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.mcp.allow_changes = on;
        self.persist_settings();
        cx.notify();
    }

    /// The program an assistant starts: this app. An AppImage is mounted somewhere temporary
    /// while it runs, so for that the file the user started is the one to use.
    fn mcp_program() -> String {
        std::env::var_os("APPIMAGE")
            .map(PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .map_or_else(|| "cashflow".to_string(), |p| p.display().to_string())
    }

    /// The command that adds CashFlow to Claude Code.
    pub(super) fn mcp_command(&self) -> String {
        format!("claude mcp add cashflow -- \"{}\" --mcp", Self::mcp_program().replace('"', "\\\""))
    }

    pub(super) fn copy_to_clipboard(&mut self, text: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.notice = Some(self.t(Msg::NoticeCopied).into());
        cx.notify();
    }
}
