//! CLI integration tests: exercise the namespaced command surface.
//!
//! All tests use --help / clap failures only, so they need no config
//! file, network, or credential.

use assert_cmd::Command;
use predicates::prelude::*;

fn grr() -> Command {
    Command::cargo_bin("grr").unwrap()
}

#[test]
fn top_level_help_lists_namespaces() {
    grr()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("gmail"))
        .stdout(predicate::str::contains("transport"))
        .stdout(predicate::str::contains("schema"));
}

#[test]
fn gmail_help_lists_all_services() {
    grr()
        .arg("gmail")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("message"))
        .stdout(predicate::str::contains("label"))
        .stdout(predicate::str::contains("draft"))
        .stdout(predicate::str::contains("send"))
        .stdout(predicate::str::contains("thread"))
        .stdout(predicate::str::contains("history"))
        .stdout(predicate::str::contains("send-as"))
        .stdout(predicate::str::contains("profile"))
        .stdout(predicate::str::contains("watch"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("msg"));
}

#[test]
fn message_subcommands() {
    grr()
        .arg("gmail")
        .arg("message")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("thread"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("batch-read"))
        .stdout(predicate::str::contains("attachment"));
}

#[test]
fn msg_bulk_ops_expose_search_and_dry_run() {
    grr()
        .arg("gmail")
        .arg("msg")
        .arg("batch-label")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--search"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--max"));

    grr()
        .arg("gmail")
        .arg("msg")
        .arg("batch-delete")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--search"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn label_subcommands() {
    grr()
        .arg("gmail")
        .arg("label")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn draft_subcommands() {
    grr()
        .arg("gmail")
        .arg("draft")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("send"));
}

#[test]
fn thread_subcommands() {
    grr()
        .arg("gmail")
        .arg("thread")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("label"))
        .stdout(predicate::str::contains("trash"))
        .stdout(predicate::str::contains("untrash"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn send_as_subcommands() {
    grr()
        .arg("gmail")
        .arg("send-as")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn watch_subcommands() {
    grr()
        .arg("gmail")
        .arg("watch")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("stop"));
}

#[test]
fn auth_help_lists_login_and_status() {
    grr()
        .arg("auth")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("--device"));
}

#[test]
fn schema_dumps_json_without_config() {
    // Must succeed with no ~/.grr config: pure contract introspection.
    grr()
        .arg("schema")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\":\"grr\""))
        .stdout(predicate::str::contains("\"subcommands\""))
        .stdout(predicate::str::contains("batch-read"))
        .stdout(predicate::str::contains("gmail"));
}

#[test]
fn get_message_help_documents_body_flags() {
    grr()
        .arg("gmail")
        .arg("message")
        .arg("get")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--body"))
        .stdout(predicate::str::contains("--max-length"));
}

#[test]
fn format_rejection_is_a_clap_error() {
    grr()
        .arg("gmail")
        .arg("message")
        .arg("search")
        .arg("--format")
        .arg("invalid")
        .arg("test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn batch_label_ids_and_search_conflict() {
    grr()
        .arg("gmail")
        .arg("msg")
        .arg("batch-label")
        .arg("--ids")
        .arg("a,b")
        .arg("--search")
        .arg("in:inbox")
        .arg("--remove")
        .arg("INBOX")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn bare_gmail_without_subcommand_shows_help() {
    grr().arg("gmail").assert().failure();
}
