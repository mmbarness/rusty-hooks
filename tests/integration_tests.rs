use assert_cmd::Command;
use rexpect::session::spawn_command;
use tempdir::TempDir;
mod common;

#[test]
fn with_valid_script_folder() -> Result<(), Box<dyn std::error::Error>> {
    let _ = TempDir::new("rusty_hooks_tests");
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("test should fail if this is inaccessible");
    let scripts_arg = format!("{}/tests/files/scripts", manifest_dir);
    let rusty_hooks_bin_path = assert_cmd::cargo::cargo_bin("rusty-hooks");
    let mut cmd = std::process::Command::new(rusty_hooks_bin_path);
    cmd
        .arg("--script-folder")
        .arg(scripts_arg);

    let mut process = spawn_command(cmd, Some(30000))?;

    process.exp_regex(common::stdout_strs::LOGGING_REGEX);

    process.send_control('c')?;

    Ok(())
}


#[test]
fn without_args() {
    Command::cargo_bin("rusty-hooks")
        .unwrap()
        .assert()
        .failure()
        .stderr(common::stdout_strs::FAILURE_STR);
}
