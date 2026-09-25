//! Output formatting for CLI commands

use clap::ValueEnum;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use serde::Serialize;
use serde_json::Value;
use std::io::{self, Write};

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Json,
    #[value(name = "jsonl")]
    JsonLines,
    Table,
    Pretty,
}

pub fn print_output<T: Serialize>(value: &T, format: OutputFormat) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string(value)?;
            writeln!(handle, "{}", json)?;
        }
        OutputFormat::JsonLines => {
            let json_value = serde_json::to_value(value)?;
            if let Some(arr) = json_value.as_array() {
                for item in arr {
                    let json = serde_json::to_string(item)?;
                    writeln!(handle, "{}", json)?;
                }
            } else {
                let json = serde_json::to_string(value)?;
                writeln!(handle, "{}", json)?;
            }
        }
        OutputFormat::Pretty => {
            let json = serde_json::to_string_pretty(value)?;
            writeln!(handle, "{}", json)?;
        }
        OutputFormat::Table => {
            print_table(value, &mut handle)?;
        }
    }
    Ok(())
}

fn print_table<T: Serialize>(value: &T, handle: &mut dyn Write) -> io::Result<()> {
    let json_value = serde_json::to_value(value)?;
    match json_value {
        Value::Array(arr) => print_array_as_table(&arr, handle),
        Value::Object(obj) => print_object_as_table(&obj, handle),
        scalar => {
            let json = serde_json::to_string_pretty(&scalar)?;
            writeln!(handle, "{}", json)
        }
    }
}

/// Stringify a table cell: scalars render plainly; nested objects and
/// arrays render as compact JSON strings.
fn stringify_cell(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// Arrays render as ONE table: one row per element, columns the union of
/// all element keys in first-seen order. An empty array renders an empty
/// table with a "(no results)" header. Arrays of scalars (first element
/// not an object) keep the Index/Value layout.
fn print_array_as_table(arr: &[Value], handle: &mut dyn Write) -> io::Result<()> {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    if arr.is_empty() {
        table.set_header(vec!["(no results)"]);
        return writeln!(handle, "{}", table);
    }

    if matches!(arr.first(), Some(Value::Object(_))) {
        // Union of keys across all elements. Non-object elements (mixed
        // arrays) land in a `value` column instead of being dropped.
        let mut columns: Vec<String> = Vec::new();
        for item in arr {
            match item {
                Value::Object(obj) => {
                    for key in obj.keys() {
                        if !columns.iter().any(|k| k == key) {
                            columns.push(key.clone());
                        }
                    }
                }
                _ => {
                    if !columns.iter().any(|k| k.as_str() == "value") {
                        columns.push("value".to_string());
                    }
                }
            }
        }

        table.set_header(columns.clone());
        for item in arr {
            let row: Vec<String> = columns
                .iter()
                .map(|col| match item {
                    Value::Object(obj) => obj.get(col).map(stringify_cell).unwrap_or_default(),
                    other if col.as_str() == "value" => stringify_cell(other),
                    _ => String::new(),
                })
                .collect();
            table.add_row(row);
        }
    } else {
        table.set_header(vec!["Index", "Value"]);
        for (idx, item) in arr.iter().enumerate() {
            table.add_row(vec![idx.to_string(), stringify_cell(item)]);
        }
    }

    writeln!(handle, "{}", table)
}

/// A single object renders as a vertical Field/Value table; nested
/// values are summarized, not expanded.
fn print_object_as_table(
    obj: &serde_json::Map<String, Value>,
    handle: &mut dyn Write,
) -> io::Result<()> {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Field", "Value"]);

    for (key, val) in obj {
        let cell_val = match val {
            Value::Array(arr) => format!("[{} items]", arr.len()),
            Value::Object(obj) => format!("{{{} fields}}", obj.len()),
            other => stringify_cell(other),
        };
        table.add_row(vec![
            Cell::new(key)
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan),
            Cell::new(cell_val),
        ]);
    }

    writeln!(handle, "{}", table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn render(value: serde_json::Value) -> String {
        let mut buf: Vec<u8> = Vec::new();
        print_table(&value, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn array_of_objects_renders_one_row_per_element() {
        let out = render(json!([
            {"id": "m1", "snippet": "hello", "label_ids": ["INBOX", "UNREAD"]},
            {"id": "m2", "snippet": "world", "thread": {"id": "t1"}},
        ]));

        // One header from the union of keys, not stacked Field/Value blocks.
        for col in ["id", "snippet", "label_ids", "thread"] {
            assert!(out.contains(col), "missing column `{col}`:\n{out}");
        }
        assert!(!out.contains("Field"), "stacked layout leaked:\n{out}");

        // One row per element.
        assert_eq!(out.lines().filter(|l| l.contains("m1")).count(), 1);
        assert_eq!(out.lines().filter(|l| l.contains("m2")).count(), 1);

        // Nested values render as compact JSON.
        assert!(out.contains(r#"["INBOX","UNREAD"]"#), "in:\n{out}");
        assert!(out.contains(r#"{"id":"t1"}"#), "in:\n{out}");
    }

    #[test]
    fn array_columns_span_elements_with_missing_keys() {
        let out = render(json!([
            {"id": "m1", "from": "a@example.com"},
            {"id": "m2", "size_estimate": 2048},
        ]));

        // Union of keys: every element's key appears even when siblings
        // lack it.
        for col in ["id", "from", "size_estimate"] {
            assert!(out.contains(col), "missing column `{col}`:\n{out}");
        }
        // Values from both elements present.
        assert!(out.contains("a@example.com"));
        assert!(out.contains("2048"));
    }

    #[test]
    fn mixed_array_keeps_non_object_elements() {
        let out = render(json!([{"id": "m1"}, "stray"]));

        assert!(out.contains("id"));
        assert!(out.contains("value"));
        assert!(out.contains("m1"));
        assert!(out.contains("stray"));
    }

    #[test]
    fn empty_array_renders_no_results_header() {
        let out = render(json!([]));

        assert!(out.contains("(no results)"));
        assert!(!out.contains("Field"));
    }

    #[test]
    fn scalar_array_keeps_index_table() {
        let out = render(json!(["alpha", "beta"]));

        assert!(out.contains("Index"));
        assert!(out.contains("Value"));
        assert!(out.contains("alpha"));
        assert!(out.contains("beta"));
    }

    #[test]
    fn single_object_keeps_field_value_table() {
        let out = render(json!({"id": "m1", "labels": ["INBOX"]}));

        assert!(out.contains("Field"));
        assert!(out.contains("id"));
        assert!(out.contains("[1 items]"), "in:\n{out}");
    }
}
