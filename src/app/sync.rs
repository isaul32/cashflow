//! Following changes made by another process.
//!
//! An AI assistant works through `cashflow --mcp`, a separate process that writes the same
//! files as the window. The storage reports when that happens (see `Storage::watch`), and the
//! window reloads so it shows what the assistant did.

use std::time::Duration;

use super::*;
use crate::storage::Change;

/// How often the files are looked at.
const WATCH_INTERVAL: Duration = Duration::from_millis(500);

impl CashFlowApp {
    /// Starts following the data folder. Call once, with the window.
    pub(super) fn watch_storage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(storage) = self.storage.clone() else { return };
        let changes = storage.watch(WATCH_INTERVAL);
        // The loop ends with the window; the storage's watcher thread notices and stops too.
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(change) = changes.recv().await {
                if this.update_in(cx, |app, window, cx| app.apply_external_change(change, window, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn apply_external_change(&mut self, change: Change, window: &mut Window, cx: &mut Context<Self>) {
        let Some(st) = self.storage.clone() else { return };
        match change {
            Change::Data => {
                if st.reload_if_changed() {
                    self.reload_from_disk(window, cx);
                }
            }
            Change::Settings => {
                let loaded = st.load_settings();
                self.settings = loaded.value;
                self.settings.labels = self.settings.labels.sanitized();
                match self.settings.theme {
                    Some(choice) => Theme::change(theme_mode(choice), Some(window), cx),
                    None => Theme::sync_system_appearance(Some(window), cx),
                }
                self.apply_locale(window, cx);
            }
        }
        cx.notify();
    }
}
