# CashFlow

A small desktop app for tracking monthly income and expenses. Your money flow
is visualized as a Sankey chart: income sources flow into total income, which
flows out to your expense categories and savings.

Built with Rust and [GPUI Kit](https://gpui-kit.com).

![CashFlow showing a month of demo data](docs/screenshot.png)

## Features

- Add, edit and delete income and expense entries per month
- Recurring entries (rent, salary, subscriptions): tick "Repeat every month" when adding an entry and it
  is added to each later month when you open it and it is still empty. Manage them on the Recurring page
- Your own categories with colors and optional sub-categories (the chart gains a level per nesting)
- Savings or deficit shown automatically
- Yearly view: all months of a year in one Sankey chart, a per-month breakdown, and previous years one click away
- Automatic saving, plain JSON files
- Backup and export in Settings: save all your data (categories, recurring entries, all months) in one file and
  restore it later or on another computer, or export every entry as CSV for a spreadsheet. Restoring
  replaces the current data, after saving a copy of it in the `backups` folder of the data folder
- Light and dark theme
- Optional MCP server (stdio): lets an AI assistant such as Claude Code read and manage your budget, see [MCP server](#mcp-server)
- Optional update notice: off by default; when turned on in Settings (under Advanced), the app asks GitHub for the
  latest release at startup and shows a link if a newer version exists. Nothing is installed automatically
- Six languages: English, Finnish, Swedish, German, French and Spanish. The language follows your
  system by default and can be changed in Settings. Numbers, currency (EUR, USD, GBP, SEK, NOK, DKK,
  CHF) and month names are formatted the way each language expects, e.g. `€1,234.50` or `1 234,50 €`. The starter categories are translated too, as long as you have not edited them

## Download

Get the latest build from the **Releases** page:

| File | Description |
| --- | --- |
| `cashflow-<version>-windows-x64-setup.exe` | Windows installer (per-user, no admin rights needed) |
| `cashflow-<version>-windows-x64-portable.zip` | Windows portable build: unzip anywhere and run `cashflow.exe` |
| `cashflow-<version>-macos-arm64.dmg` | macOS, Apple Silicon (not notarized, see below) |
| `cashflow-<version>-linux-x64.AppImage` / `-arm64.AppImage` | Linux AppImage: make it executable and run |
| `cashflow-<version>-linux-x64.deb` / `-arm64.deb` | Debian, Ubuntu and derivatives |

The macOS app is not signed with an Apple developer certificate, so the first launch needs approval:
try to open it once, then go to System Settings → Privacy & Security and click **Open Anyway**
(or run `xattr -d com.apple.quarantine /Applications/CashFlow.app`).

Verify downloads with `SHA256SUMS.txt`.

## Where data is stored

| Build | Location |
| --- | --- |
| Installed | `%APPDATA%\budget\data` (the folder keeps its original name so existing data stays in place) |
| Portable | `data\` next to `cashflow.exe` (enabled by the `portable` marker file in the zip) |
| Any | The directory in the `BUDGET_DATA_DIR` environment variable, if set |

All your data (categories, recurring entries and every month) is in one file, `data.json`. The
previous save is kept as `data.json.bak`. `settings.json` holds the preferences of this computer: language,
currency, theme and chart labels. Uninstalling does not delete your data.

The data file has a schema version. When a new release changes the layout, the app upgrades an older file
automatically and keeps the original as `data.v<N>.json.bak`; a file from a newer release is never overwritten.
Data folders from before 0.1.4 (`categories.json` plus one `YYYY-MM.json` per month) are imported once and
the old files are moved to `legacy/`.

## MCP server

CashFlow can be used by an AI assistant such as Claude Code through the [Model Context Protocol](https://modelcontextprotocol.io):
it can read months and categories, add and edit entries, manage categories and recurring entries, change settings and
export data. If the window is open, it shows the assistant's changes within a second.

It works over **stdio**. The assistant starts `cashflow --mcp` itself, as a background process without a window, and
talks to it through pipes. Nothing listens on the network, and there is no port or token to manage. It works on the same
data files as the app, also while the window is open.

It is **off by default**. To use it:

1. Open Settings, and under **Advanced** turn on MCP server.
2. Optionally turn on **Allow changes**. Without it the assistant can only read.
3. Press **Copy Claude Code command** and run it once in a terminal. It looks like this:

   ```bash
   claude mcp add cashflow -- "C:\Program Files\CashFlow\cashflow.exe" --mcp
   ```

Any other MCP client that can start a stdio server works the same way: the command is the app itself with the `--mcp`
argument. The app does not have to be running.

You stay in control. The process reads the settings on every request, so turning the server off or changes off in the app
takes effect at once, also for an assistant that is already connected:

- Nothing is offered until the server is turned on.
- Tools that change data are not even offered, and are refused, unless **Allow changes** is on. The assistant cannot
  change these settings or the update check.
- `restore_backup` needs an explicit confirmation and saves a copy of the current data first.

Two things can write the data at once (the window and the assistant's process). Writes are serialized with a lock file
and always start from the latest saved data, so a change by one is not lost by the other.

**Tools.** Reading: `get_state`, `list_categories`, `list_months`, `list_entries`, `get_month_summary`,
`get_year_summary`, `list_recurring`, `get_settings`, `export_csv`, `export_backup` and `export_sankeymatic`.
Changing: `add_entry`, `update_entry`, `delete_entry`, `add_category`, `update_category`, `delete_category`,
`update_recurring`, `stop_recurring`, `delete_recurring`, `set_settings`, `import_sankeymatic` and
`restore_backup`. Amounts are positive numbers in the app's currency; whether an entry is income or expense follows from
its category. Months not given default to the current month. The tools are described to the assistant in full when it
connects.

## Try it with demo data

`docs/demo-data/` has sample data for September 2025 to November 2026 (categories and
entries, in one `data.json`), the same data as the screenshot. Point the app at a copy of it:

```bash
cp -r docs/demo-data /tmp/cashflow-demo
BUDGET_DATA_DIR=/tmp/cashflow-demo cargo run
```

On Windows (PowerShell):

```powershell
Copy-Item docs\demo-data $env:TEMP\cashflow-demo -Recurse
$env:BUDGET_DATA_DIR = "$env:TEMP\cashflow-demo"; cargo run
```

Use the month arrows to go to November 2026 if the app opens on another month.
The app writes to the data directory when you change something, so use a copy.

## Build from source

```bash
cargo run            # debug build
cargo build --release
cargo test
```

`cargo test` also runs the UI tests in `src/app/tests.rs`. They render the real window headlessly and drive
it with clicks and key presses (no display needed), each with its own temporary data folder.

## Website and screenshots

The product website lives in `site/` (Astro, deployed on Cloudflare Pages), see `site/README.md`. The
screenshots on the website and in this README are generated by `site/tools/screenshots.ps1`; a git
hook refreshes them on commits that change the app (`git config core.hooksPath .githooks` to enable).
The app reads two developer overrides for this: `CASHFLOW_MONTH=YYYY-MM` and `CASHFLOW_THEME=light|dark`.

## Translations

All texts live in one table in `src/i18n.rs`; every message needs a text for every language, so a
missing translation does not compile. To add a language, add a `Lang` variant, its number format and
month names, and one column in the table. The translations other than English and Finnish have not
been reviewed by native speakers, so corrections are welcome.

Known limitation: the right-click menu of text fields (Cut, Copy, Paste, Select All) comes from the
UI library, which does not ship these languages, so it stays in English.

## Releasing

1. Bump `version` in `Cargo.toml` and commit.
2. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.
3. The `Release` workflow builds the Windows installer and zip, the macOS disk image and the Linux
   AppImages and `.deb` packages (x64 and arm64), and publishes them as a GitHub release.

Packaging: `installer/cashflow.iss` (Inno Setup, Windows), `installer/package-macos.sh` and
`installer/package-linux.sh`. The app icon is generated by `tools/make_icon.py`.

## License

[MIT](LICENSE)
