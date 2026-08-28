//! End-to-end CLI tests that run the built `elint` binary.

use std::path::PathBuf;
use std::process::{Command, Output};

fn elint(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_elint"))
        .args(args)
        .output()
        .expect("failed to run elint")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout is not UTF-8")
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).expect("stderr is not UTF-8")
}

/// Writes `src` to a per-test temp file and returns its path.
fn temp_erl(case: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("elint-cli-{}", std::process::id()))
        .join(case);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("t.erl");
    std::fs::write(&path, src).expect("write temp file");
    path
}

#[test]
fn list_shows_rules_and_shared_explanations() {
    let out = elint(&["--list"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(text.contains("Lint rules:\n"), "{text}");
    for rule in elint::rules::RULES {
        assert!(text.contains(&format!("  {}\n", rule.name)), "{text}");
    }
    assert!(text.contains("\nAdditional explanations:\n"), "{text}");
    assert!(text.contains("  elint_expect_attr\n"), "{text}");
    assert!(
        text.ends_with("\nRun `elint --explain <name>` to print one.\n"),
        "{text}"
    );
}

#[test]
fn explain_rule_prints_rule_description() {
    let out = elint(&["--explain", "element_bif"]);
    assert!(out.status.success());
    let rule = elint::rules::RULES
        .iter()
        .find(|rule| rule.name == "element_bif")
        .expect("element_bif rule");
    assert_eq!(stdout(&out), rule.text);
}

#[test]
fn explain_shared_prints_shared_explanation() {
    let out = elint(&["--explain", "elint_expect_attr"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("-elint_expect"));
}

#[test]
fn explain_readme_is_not_an_explanation() {
    let out = elint(&["--explain", "README"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("unknown explanation"), "{err}");
    assert!(err.contains("--list"), "{err}");
}

#[test]
fn unknown_explain_name_fails_and_points_to_list() {
    let out = elint(&["--explain", "no_such_name"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("no_such_name"), "{err}");
    assert!(err.contains("--list"), "{err}");
}

#[test]
fn explain_and_list_together_are_rejected() {
    let out = elint(&["--explain", "element_bif", "--list"]);
    assert!(!out.status.success());
}

#[test]
fn explanation_options_reject_paths_and_lint_filters() {
    for args in [
        vec!["--explain", "element_bif", "src"],
        vec!["--explain", "element_bif", "--lint", "element_bif"],
        vec!["--list", "src"],
        vec!["--list", "--lint", "element_bif"],
    ] {
        let out = elint(&args);
        assert!(!out.status.success(), "args: {args:?}");
    }
}

#[test]
fn help_shows_no_subcommands() {
    let out = elint(&["--help"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert!(!text.contains("Commands:"), "{text}");
    assert!(!text.contains("<COMMAND>"), "{text}");
    assert!(!text.contains("elint doc"), "{text}");
    assert!(!text.contains("elint explain"), "{text}");
    assert!(text.contains("--explain <NAME>"), "{text}");
    assert!(text.contains("--list"), "{text}");
}

#[test]
fn lint_without_paths_still_succeeds() {
    let out = elint(&[]);
    assert!(out.status.success());
}

#[test]
fn lint_path_still_reports_and_notes_only_the_rule() {
    let out = elint(&["rules/element_bif/ng.erl"]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("error[element_bif]"), "{err}");
    assert!(
        err.contains("note: run `elint --explain element_bif` for details"),
        "{err}"
    );
    assert!(!err.contains("elint doc"), "{err}");
    assert!(!err.contains("elint explain"), "{err}");
    assert!(!err.contains("elint_expect_attr"), "{err}");
}

#[test]
fn expect_diagnostics_point_to_shared_explanation() {
    let cases = [
        (
            "invalid_payload",
            "-module(t).\n-elint_expect(element_bif).\nfoo() -> ok.\n",
        ),
        (
            "missing_reason",
            "-module(t).\n-elint_expect(element_bif, {function, foo, 0}, 1).\nfoo() -> ok.\n",
        ),
        (
            "unknown_rule",
            "-module(t).\n-elint_expect(no_such_rule, {function, foo, 0}, \"reason\").\nfoo() -> ok.\n",
        ),
        (
            "unknown_function",
            "-module(t).\n-elint_expect(element_bif, {function, no_such, 0}, \"reason\").\nfoo() -> ok.\n",
        ),
        (
            "unmatched",
            "-module(t).\n-elint_expect(element_bif, {function, foo, 0}, \"reason\").\nfoo() -> ok.\n",
        ),
    ];
    for (case, src) in cases {
        let path = temp_erl(case, src);
        let out = elint(&[path.to_str().expect("path is UTF-8")]);
        assert!(!out.status.success(), "{case}");
        let err = stderr(&out);
        assert!(
            err.contains("note: run `elint --explain elint_expect_attr` for details"),
            "{case}: {err}"
        );
    }
}
