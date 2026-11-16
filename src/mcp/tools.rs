//! The tools the MCP server offers: name, description and input schema. What they do is in
//! `workspace.rs`.

use serde_json::{Value, json};

/// Shown when a tool that changes data is used while changes are not allowed.
pub const CHANGES_OFF: &str =
    "Changes are turned off in CashFlow. The user can allow them in Settings, under MCP server.";

/// Shown while the user has not turned the MCP server on.
pub const OFF: &str =
    "The MCP server is turned off in CashFlow. The user can turn it on in Settings, under MCP server.";

/// Tells the model what the app is and how to use the tools.
pub const INSTRUCTIONS: &str = "CashFlow is a desktop app that tracks monthly income and expenses and draws \
them as a Sankey chart. Data is organised in months (YYYY-MM). Every entry belongs to a category, and the \
category's kind (income or expense) decides which it is, so amounts are always positive numbers in the app's \
currency. Categories can be nested one inside another. Start with get_state and list_categories. Tools that change \
data are only available when the user has allowed changes in the app's settings. If the app's window is open it \
shows changes within a second.";

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    /// Changes data, so it needs the user's permission.
    pub write: bool,
    /// Hard to undo (deletes or replaces data).
    destructive: bool,
    schema: fn() -> Value,
}

/// All tools. Read-only ones first.
pub static TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "get_state",
        description: "Overall state: app and data version, the current month, language, currency and theme, and whether changes are allowed.",
        write: false,
        destructive: false,
        schema: no_args,
    },
    ToolDef {
        name: "list_categories",
        description: "All categories with id, name, kind (income or expense), parent and colour.",
        write: false,
        destructive: false,
        schema: no_args,
    },
    ToolDef {
        name: "list_months",
        description: "The months that have entries, with entry counts and income and expense totals.",
        write: false,
        destructive: false,
        schema: no_args,
    },
    ToolDef {
        name: "list_entries",
        description: "The entries of a month, with their ids, categories and amounts, and the month's totals.",
        write: false,
        destructive: false,
        schema: || object(json!({"month": month()}), &[]),
    },
    ToolDef {
        name: "get_month_summary",
        description: "Income, expenses and balance of a month, and the total per category.",
        write: false,
        destructive: false,
        schema: || object(json!({"month": month()}), &[]),
    },
    ToolDef {
        name: "get_year_summary",
        description: "Income, expenses and balance of a year, per month and per category.",
        write: false,
        destructive: false,
        schema: || object(json!({"year": {"type": "integer", "description": "Year, e.g. 2026. Defaults to the current year."}}), &[]),
    },
    ToolDef {
        name: "get_settings",
        description: "The user's settings: language, currency, theme and the names of the chart's automatic nodes.",
        write: false,
        destructive: false,
        schema: no_args,
    },
    ToolDef {
        name: "export_csv",
        description: "Every entry of every month as CSV text (month, type, category, note, amount).",
        write: false,
        destructive: false,
        schema: no_args,
    },
    ToolDef {
        name: "export_backup",
        description: "All data (categories and all months) as one JSON document, the same as the app's backup file.",
        write: false,
        destructive: false,
        schema: no_args,
    },
    ToolDef {
        name: "export_sankeymatic",
        description: "The money flow chart of a month or a year as SankeyMATIC input text.",
        write: false,
        destructive: false,
        schema: || {
            object(
                json!({
                    "month": month(),
                    "year": {"type": "integer", "description": "Give a year instead of a month to get the whole year."},
                }),
                &[],
            )
        },
    },
    ToolDef {
        name: "add_entry",
        description: "Adds an entry to a month. The category decides whether it is income or expense.",
        write: true,
        destructive: false,
        schema: || {
            object(
                json!({
                    "category": category_ref("Category id or name."),
                    "amount": amount(),
                    "name": {"type": "string", "description": "Optional note, e.g. \"Rent\"."},
                    "month": month(),
                }),
                &["category", "amount"],
            )
        },
    },
    ToolDef {
        name: "update_entry",
        description: "Changes an entry's category, note or amount.",
        write: true,
        destructive: false,
        schema: || {
            object(
                json!({
                    "id": {"type": "integer", "description": "Entry id, from list_entries."},
                    "month": month(),
                    "category": category_ref("New category id or name."),
                    "name": {"type": "string"},
                    "amount": amount(),
                }),
                &["id"],
            )
        },
    },
    ToolDef {
        name: "delete_entry",
        description: "Deletes an entry from a month.",
        write: true,
        destructive: true,
        schema: || object(json!({"id": {"type": "integer"}, "month": month()}), &["id"]),
    },
    ToolDef {
        name: "copy_month",
        description: "Starts an empty month with the entries of another month (by default the month before it), like the Copy button in the app. Fails if the target month already has entries.",
        write: true,
        destructive: false,
        schema: || {
            object(
                json!({
                    "from": {
                        "type": "string",
                        "pattern": "^[0-9]{4}-(0[1-9]|1[0-2])$",
                        "description": "Month to copy from, as YYYY-MM. Defaults to the month before the target month.",
                    },
                    "to": {
                        "type": "string",
                        "pattern": "^[0-9]{4}-(0[1-9]|1[0-2])$",
                        "description": "Empty month to fill, as YYYY-MM. Defaults to the current month.",
                    },
                }),
                &[],
            )
        },
    },
    ToolDef {
        name: "add_category",
        description: "Creates a category. A category has a kind, and a sub-category must have the same kind as its parent.",
        write: true,
        destructive: false,
        schema: || {
            object(
                json!({
                    "name": {"type": "string"},
                    "kind": {"type": "string", "enum": ["income", "expense"]},
                    "parent": category_ref("Parent category id or name, to make this a sub-category."),
                    "color": {"type": "string", "description": "Colour as #RRGGBB. Left out, a sub-category gets its parent's colour and another category gets a colour that is picked."},
                }),
                &["name", "kind"],
            )
        },
    },
    ToolDef {
        name: "update_category",
        description: "Renames, recolours or moves a category. Give parent null to make it a top-level category.",
        write: true,
        destructive: false,
        schema: || {
            object(
                json!({
                    "category": category_ref("Category id or name."),
                    "name": {"type": "string"},
                    "parent": {"description": "New parent category id or name, or null for none."},
                    "color": {"type": "string", "description": "#RRGGBB"},
                }),
                &["category"],
            )
        },
    },
    ToolDef {
        name: "delete_category",
        description: "Deletes a category. Its entries (in every month) move to its parent, or to another category of the same kind.",
        write: true,
        destructive: true,
        schema: || object(json!({"category": category_ref("Category id or name.")}), &["category"]),
    },
    ToolDef {
        name: "set_settings",
        description: "Changes language, currency, theme or the names of the chart's automatic nodes. Use \"auto\" to follow the system.",
        write: true,
        destructive: false,
        schema: || {
            object(
                json!({
                    "language": {"type": "string", "enum": ["auto", "en", "fi", "sv", "de", "fr", "es"]},
                    "currency": {"type": "string", "enum": ["auto", "EUR", "USD", "GBP", "SEK", "NOK", "DKK", "CHF"]},
                    "theme": {"type": "string", "enum": ["auto", "light", "dark"]},
                    "labels": {
                        "type": "object",
                        "properties": {"total": {"type": "string"}, "savings": {"type": "string"}, "deficit": {"type": "string"}},
                        "additionalProperties": false,
                    },
                }),
                &[],
            )
        },
    },
    ToolDef {
        name: "import_sankeymatic",
        description: "Imports SankeyMATIC input text into a month, creating missing categories. With replace, the month's existing entries are removed first.",
        write: true,
        destructive: true,
        schema: || {
            object(
                json!({
                    "text": {"type": "string", "description": "SankeyMATIC input, as produced by export_sankeymatic."},
                    "month": month(),
                    "replace": {"type": "boolean", "description": "Remove the month's existing entries first. Default false."},
                }),
                &["text"],
            )
        },
    },
    ToolDef {
        name: "restore_backup",
        description: "Replaces ALL data with a backup made by export_backup. A copy of the current data is saved first. Needs confirm: true.",
        write: true,
        destructive: true,
        schema: || {
            object(
                json!({
                    "backup": {"type": "string", "description": "The backup JSON, as returned by export_backup."},
                    "confirm": {"type": "boolean", "description": "Must be true: this replaces everything."},
                }),
                &["backup", "confirm"],
            )
        },
    },
];

pub fn find(name: &str) -> Option<&'static ToolDef> {
    TOOLS.iter().find(|t| t.name == name)
}

/// The tools as MCP describes them. Tools that change data are left out unless allowed.
pub fn list(allow_changes: bool) -> Vec<Value> {
    TOOLS
        .iter()
        .filter(|t| allow_changes || !t.write)
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": (t.schema)(),
                "annotations": {"readOnlyHint": !t.write, "destructiveHint": t.destructive, "openWorldHint": false},
            })
        })
        .collect()
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type": "object", "properties": properties, "required": required, "additionalProperties": false})
}

fn no_args() -> Value {
    object(json!({}), &[])
}

fn month() -> Value {
    json!({
        "type": "string",
        "pattern": "^[0-9]{4}-(0[1-9]|1[0-2])$",
        "description": "Month as YYYY-MM. Defaults to the current month.",
    })
}

fn category_ref(description: &str) -> Value {
    json!({"type": ["string", "integer"], "description": description})
}

fn amount() -> Value {
    json!({
        "type": "number",
        "exclusiveMinimum": 0,
        "description": "A positive amount in the app's currency, e.g. 12.50. Income or expense follows from the category.",
    })
}
