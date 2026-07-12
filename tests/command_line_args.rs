use assert_cmd::Command;
use predicates::prelude::*;
use rexpect::session::spawn_command;
use std::time::Duration;
use tempdir::TempDir;

#[test]
fn with_valid_script_folder() -> Result<(), Box<dyn std::error::Error>> {
    let _ = TempDir::new("rusty_hooks_tests");
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("test should fail if this is inaccessible");
    let scripts_arg = format!("{}/tests/files/scripts", manifest_dir);
    let rusty_hooks_bin_path = assert_cmd::cargo::cargo_bin("rusty-hooks");
    let mut cmd = std::process::Command::new(rusty_hooks_bin_path);
    cmd.arg("--script-folder")
        .arg(scripts_arg)
        .arg("--log-level")
        .arg("debug")
        .arg("--disable-usenet-webhook");
    let mut process: rexpect::session::PtySession = spawn_command(cmd, Some(30000))?;

    std::thread::sleep(Duration::from_millis(500));
    process.send_control('c')?;
    Ok(())
}

#[test]
fn with_invalid_script_folder() -> Result<(), Box<dyn std::error::Error>> {
    let _ = TempDir::new("rusty_hooks_tests");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let scripts_arg = format!("{}/tests/fi_es/scripts", manifest_dir);
    let rusty_hooks_bin_path = assert_cmd::cargo::cargo_bin("rusty-hooks");
    Command::new(rusty_hooks_bin_path)
        .arg("--script-folder")
        .arg(scripts_arg)
        .arg("--log-level")
        .arg("debug")
        .arg("--disable-usenet-webhook")
        .assert()
        .failure()
        .stderr(predicate::str::contains("explicit panic"));
    Ok(())
}

#[test]
fn without_args() {
    Command::cargo_bin("rusty-hooks")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "the following required arguments were not provided",
        ));
}
