use assert_cmd::Command;
use predicates::prelude::*;

#[allow(deprecated)]
fn cmd() -> Command {
    Command::cargo_bin("imgdedup").unwrap()
}

#[test]
fn help_output() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deduplicate images and videos"));
}

#[test]
fn nonexistent_directory_exits_2() {
    cmd()
        .arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn no_args_exits_2() {
    cmd().assert().failure().code(2);
}
