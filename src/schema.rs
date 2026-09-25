//! `grr schema`: dump the command tree as JSON.
//!
//! The binary is its own contract (inspired by gogcli's "discover the
//! contract"): agents and docs generators read this instead of scraping
//! --help text.

use anyhow::Result;
use clap::{Arg, Command};
use serde_json::{Value, json};

use crate::output::{OutputFormat, print_output};

#[derive(clap::Args, Debug, Clone)]
pub struct SchemaArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: OutputFormat,
}

pub fn handle_schema_cmd(root: Command, args: SchemaArgs) -> Result<()> {
    print_output(&walk(&root), args.format)?;
    Ok(())
}

fn walk(cmd: &Command) -> Value {
    let args: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| a.get_id() != "help")
        .map(arg_json)
        .collect();

    let subcommands: Vec<Value> = cmd
        .get_subcommands()
        .filter(|sc| sc.get_name() != "help")
        .map(walk)
        .collect();

    json!({
        "name": cmd.get_name(),
        "about": cmd.get_about().map(|a| a.to_string()),
        "args": args,
        "subcommands": subcommands,
    })
}

fn arg_json(arg: &Arg) -> Value {
    // Set/Append actions consume values; SetTrue/SetFalse/Help/... don't.
    let takes_value = matches!(
        arg.get_action(),
        clap::ArgAction::Set | clap::ArgAction::Append
    );
    let mut v = json!({
        "id": arg.get_id().as_str(),
        "long": arg.get_long(),
        "short": arg.get_short().map(|c| c.to_string()),
        "required": arg.is_required_set(),
        "takes_value": takes_value,
    });

    if takes_value {
        let defaults: Vec<String> = arg
            .get_default_values()
            .iter()
            .map(|d| d.to_string_lossy().into_owned())
            .collect();
        if !defaults.is_empty() {
            v["default"] = json!(defaults);
        }
        let possible: Vec<String> = arg
            .get_possible_values()
            .iter()
            .map(|p| p.get_name().to_string())
            .collect();
        if !possible.is_empty() {
            v["possible_values"] = json!(possible);
        }
    }

    v
}
