//! Output formatting: JSON or human-readable.

use serde::Serialize;

/// Format a serializable value as JSON string.
pub fn as_json<T: Serialize>(val: &T) -> String {
    serde_json::to_string_pretty(val).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

/// Print a labeled key-value pair for human-readable output.
pub fn kv(key: &str, value: &str) -> String {
    format!("{:<28} {}", key, value)
}

/// Format a table of rows with headers.
pub fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "(no data)".to_string();
    }

    // Compute column widths
    let cols = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(cols) {
            widths[i] = widths[i].max(cell.len());
        }
    }

    let mut out = String::new();

    // Header row
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&format!("{:<width$}", h, width = widths[i]));
    }
    out.push('\n');

    // Separator
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*w));
    }
    out.push('\n');

    // Data rows
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(cols) {
            if i > 0 {
                out.push_str("  ");
            }
            out.push_str(&format!("{:<width$}", cell, width = widths[i]));
        }
        out.push('\n');
    }

    out
}
