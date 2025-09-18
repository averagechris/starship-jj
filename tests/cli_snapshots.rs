use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use regex::Regex;

fn strip_ansi(s: &str) -> String {
    // Matches common ANSI color sequences like \x1B[31m or \x1B[39;49m
    let re = Regex::new(r"\x1B\[[0-9;]*m").unwrap();
    re.replace_all(s, "").into_owned()
}

fn normalize_paths(s: &str) -> String {
    // Replace absolute-looking paths to stabilize snapshots across machines
    let drive = Regex::new(r#"[A-Za-z]:\\[^\s"]+"#).unwrap();
    let unix = Regex::new(r#"/(?:Users|home)/[^\s"]+"#).unwrap();
    let s = drive.replace_all(s, "<ABS_PATH>");
    let s = unix.replace_all(&s, "<ABS_PATH>");
    s.into_owned()
}

fn stable(s: &str) -> String {
    normalize_paths(&strip_ansi(s))
}

#[test]
fn config_default_snapshot() {
    let mut cmd = Command::cargo_bin("starship-jj").unwrap();
    let assert = cmd
        .arg("starship")
        .arg("config")
        .arg("default")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    insta::assert_snapshot!("config_default", stable(&out));
}

#[test]
fn config_path_snapshot_redacted() {
    let mut cmd = Command::cargo_bin("starship-jj").unwrap();
    let assert = cmd
        .arg("starship")
        .arg("config")
        .arg("path")
        .assert()
        .success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    // Replace trailing starship-jj.toml path with redacted token for stability
    let re =
        Regex::new(r".*starship-jj[\\/]starship-jj\.toml\s*$|.*starship-jj\.toml\s*$").unwrap();
    let out = re
        .replace(&out, "<CONFIG_PATH>/starship-jj.toml\n")
        .into_owned();

    insta::assert_snapshot!("config_path", stable(&out));
}

fn have_jj() -> bool {
    which::which("jj").is_ok()
}

#[test]
fn prompt_in_empty_repo_baseline() {
    if !have_jj() {
        eprintln!("skipping: jj not found in PATH");
        return;
    }

    let tmp = assert_fs::TempDir::new().unwrap();
    // Initialize a fresh jj repo (git backend)
    let mut init = Command::new("jj");
    init.arg("git").arg("init").current_dir(tmp.path());
    init.assert().success();

    // Use deterministic env to reduce noise
    let mut cmd = Command::cargo_bin("starship-jj").unwrap();
    cmd.arg("starship").arg("prompt");
    cmd.current_dir(tmp.path());
    cmd.env("SJJ__MODULE_SEPARATOR", "|");
    cmd.env("SJJ__RESET_COLOR", "false");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    insta::assert_snapshot!("prompt_empty_repo", stable(&out));
}

#[test]
fn prompt_with_description_and_changes() {
    if !have_jj() {
        eprintln!("skipping: jj not found in PATH");
        return;
    }

    let tmp = assert_fs::TempDir::new().unwrap();
    let mut init = Command::new("jj");
    init.arg("git").arg("init").current_dir(tmp.path());
    init.assert().success();

    // Create a file to produce a working copy diff
    let file = tmp.child("foo.txt");
    file.write_str("hello\n").unwrap();

    // Set a description on the working copy commit
    let mut desc = Command::new("jj");
    desc.arg("describe")
        .arg("-m")
        .arg("hello world")
        .current_dir(tmp.path());
    desc.assert().success();

    let mut cmd = Command::cargo_bin("starship-jj").unwrap();
    cmd.arg("starship").arg("prompt");
    cmd.current_dir(tmp.path());
    cmd.env("SJJ__MODULE_SEPARATOR", "|");
    cmd.env("SJJ__RESET_COLOR", "false");

    let assert = cmd.assert().success();
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    insta::assert_snapshot!("prompt_desc_and_changes", stable(&out));
}
