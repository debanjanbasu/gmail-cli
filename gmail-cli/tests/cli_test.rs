//! CLI integration tests

use assert_cmd::Command;
use predicates::prelude::*;

#[tokio::test]
async fn test_help_shows_all_commands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("auth"))
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

#[tokio::test]
async fn help_lists_transport_command() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("transport"));
}

#[tokio::test]
async fn test_message_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("message").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("thread"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("attachment"));
}

#[tokio::test]
async fn test_label_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("label").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[tokio::test]
async fn test_draft_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("draft").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("send"));
}

#[tokio::test]
async fn test_send_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("send").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("send"))
        .stdout(predicate::str::contains("send-attach"));
}

#[tokio::test]
async fn test_thread_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("thread").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("label"))
        .stdout(predicate::str::contains("trash"))
        .stdout(predicate::str::contains("untrash"))
        .stdout(predicate::str::contains("delete"));
}

#[tokio::test]
async fn test_msg_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("msg").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("label"))
        .stdout(predicate::str::contains("trash"))
        .stdout(predicate::str::contains("untrash"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("batch-label"))
        .stdout(predicate::str::contains("batch-delete"));
}

#[tokio::test]
async fn test_send_as_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("send-as").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[tokio::test]
async fn test_watch_subcommands() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("watch").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("stop"));
}

#[tokio::test]
async fn test_format_option() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("--format").arg("table").arg("--help");
    // Should show help (format accepted) without crashing
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("auth"));
}

#[tokio::test]
async fn test_invalid_format() {
    let mut cmd = Command::cargo_bin("gmail").unwrap();
    cmd.arg("--format").arg("invalid").arg("message").arg("search").arg("test");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}