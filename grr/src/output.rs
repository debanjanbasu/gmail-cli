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
        Value::Array(arr) => {
            if arr.is_empty() {
                return Ok(());
            }

            if let Some(first) = arr.first() {
                if let Value::Object(_) = first {
                    print_object_array_as_table(&arr, handle)?;
                } else {
                    print_simple_array_as_table(&arr, handle)?;
                }
            }
        }
        Value::Object(obj) => {
            print_object_as_table(&obj, handle)?;
        }
        _ => {
            let json = serde_json::to_string_pretty(&json_value)?;
            writeln!(handle, "{}", json)?;
        }
    }
    Ok(())
}

fn print_object_array_as_table(arr: &[Value], handle: &mut dyn Write) -> io::Result<()> {
    if arr.is_empty() {
        return Ok(());
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Field", "Value"]);

    for item in arr {
        if let Value::Object(obj) = item {
            for (key, val) in obj {
                let cell_val = match val {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => serde_json::to_string(val).unwrap_or_default(),
                };
                table.add_row(vec![
                    Cell::new(key).add_attribute(Attribute::Bold),
                    Cell::new(cell_val),
                ]);
            }
            table.add_row(vec![Cell::new("---"), Cell::new("---")]);
        }
    }

    writeln!(handle, "{}", table)
}

fn print_simple_array_as_table(arr: &[Value], handle: &mut dyn Write) -> io::Result<()> {
    let mut table = Table::new();
    table.set_header(vec!["Index", "Value"]);

    for (idx, item) in arr.iter().enumerate() {
        let val = match item {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => serde_json::to_string(item).unwrap_or_default(),
        };
        table.add_row(vec![idx.to_string(), val]);
    }

    writeln!(handle, "{}", table)
}

fn print_object_as_table(
    obj: &serde_json::Map<String, Value>,
    handle: &mut dyn Write,
) -> io::Result<()> {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Field", "Value"]);

    for (key, val) in obj {
        let cell_val = match val {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => format!("[{} items]", arr.len()),
            Value::Object(obj) => format!("{{{} fields}}", obj.len()),
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
