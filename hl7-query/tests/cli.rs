use assert_cmd::cmd::Command;
use predicates::prelude::*;

#[test]
fn file_doesnt_exist() {
    let mut cmd = Command::cargo_bin("hq").expect("binary exists");

    cmd.arg("test/file/doesnt/exist");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));
}

#[test]
fn parse_valid_file() {
    let mut cmd = Command::cargo_bin("hq").expect("binary exists");

    cmd.arg("--colour").arg("never").arg(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/sample_adt_a01.hl7"
    ));
    cmd.assert().success().stdout(predicate::str::contains(
        r"MSH|^~\&|AccMgr|1|||20050110045504||ADT^A01|599102|P|2.3|||AL",
    ));
}

#[test]
fn parse_valid_stdin() {
    let mut cmd = Command::cargo_bin("hq").expect("binary exists");

    cmd.arg("--colour").arg("never");
    cmd.pipe_stdin(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/sample_adt_a01.hl7"
    ))
    .expect("test file exists");
    cmd.assert().success().stdout(predicate::str::contains(
        r"MSH|^~\&|AccMgr|1|||20050110045504||ADT^A01|599102|P|2.3|||AL",
    ));
}

#[test]
fn should_not_parse_invalid_hl7() {
    let mut cmd = Command::cargo_bin("hq").expect("binary exists");

    cmd.arg("--colour").arg("never");
    cmd.write_stdin("Hello world");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to parse input as HL7 message"));
}

#[test]
fn should_output_tabular_data() {
    let mut cmd = Command::cargo_bin("hq").expect("binary exists");

    cmd.arg("--colour").arg("never").arg("-o").arg("table").arg(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../assets/sample_adt_a01.hl7"
    ));
    cmd.assert().success().stdout(predicate::str::contains(
        "MSH.10\t599102",
    ));
}

#[test]
fn should_print_help() {
    let mut cmd = Command::cargo_bin("hq").expect("binary exists");

    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r"Usage: hq [OPTIONS] [INPUT]"));
}
