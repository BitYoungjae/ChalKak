use std::process::Command;

fn chalkak() -> Command {
    Command::new(env!("CARGO_BIN_EXE_chalkak"))
}

#[test]
fn cli_short_version_flag() {
    let output = chalkak().arg("-V").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.starts_with("ChalKak "));
    assert!(
        !stdout.contains('('),
        "short version should not contain git hash"
    );
}

#[test]
fn cli_long_version_flag() {
    let output = chalkak().arg("--version").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.starts_with("ChalKak "));
}

#[test]
fn cli_short_help_flag() {
    let output = chalkak().arg("-h").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--help"));
}

#[test]
fn cli_long_help_flag() {
    let output = chalkak().arg("--help").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--version"));
}
