//! Erlang code linter CLI.

use std::path::Path;

use elint::Span;
use elint::diagnostic::{Color, Endpoints, Source};

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    let explain_name: Option<String> = noargs::opt("explain")
        .ty("NAME")
        .doc("Print an explanation for a lint rule or shared topic and exit")
        .take(&mut args)
        .present_and_then(|o| o.value().parse())?;
    let list_requested = noargs::flag("list")
        .doc("List available explanations and exit")
        .take(&mut args)
        .is_present();

    let mut target_lint_names: Vec<String> = Vec::new();
    while let Some(a) = noargs::opt("lint")
        .short('l')
        .ty("LINT_RULE_NAME")
        .doc("Only run the named lint rule; may be repeated")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?
    {
        target_lint_names.push(a);
    }

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    while let Some(path) = noargs::arg("[PATH]..")
        .doc("Erlang source files or directories to lint; defaults to `src/` and `tests/`")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?
    {
        paths.push(path);
    }

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    if explain_name.is_some() || list_requested {
        if explain_name.is_some() && list_requested {
            eprintln!("error: --explain and --list cannot be used together");
            std::process::exit(1);
        }
        if !target_lint_names.is_empty() || !paths.is_empty() {
            eprintln!("error: --explain / --list cannot be combined with paths or --lint");
            std::process::exit(1);
        }
        if let Some(name) = explain_name {
            print_explanation(&name);
        } else {
            print!("{}", explanation_list());
        }
        return Ok(());
    }

    let default_paths = paths.is_empty();
    if paths.is_empty() {
        paths.push("src/".into());
        paths.push("tests/".into());
    }

    let mut error_count = 0;
    let mut known_errors = std::collections::HashSet::new();
    for path in paths {
        if !default_paths && !path.exists() {
            eprintln!("error: no such file or directory: {}", path.display());
            error_count += 1;
            continue;
        }
        for path in elint::fs::collect_erlang_files(path)? {
            error_count += lint_file(&path, &target_lint_names, &mut known_errors);
        }
    }

    if error_count > 0 {
        eprintln!("Found {error_count} error(s)");
        std::process::exit(1);
    }

    Ok(())
}

fn lint_file(
    path: &Path,
    target_lint_names: &[String],
    known_errors: &mut std::collections::HashSet<&'static str>,
) -> usize {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", path.display());
            return 1;
        }
    };
    let color = Color::detect();
    let source = Source::new(path, &text);

    let ctx = match elint::Context::analyze(path, text.clone()) {
        Ok(ctx) => ctx,
        Err(e) => {
            let span = Span::new(e.position.offset(), e.position.offset());
            report(
                &color,
                &source,
                None,
                &e.to_string(),
                span,
                None,
                None,
                None,
            );
            return 1;
        }
    };

    let mut error_count = 0;

    let mut expect = match elint::expect::ExpectRules::new(&ctx, target_lint_names) {
        Ok(expect) => expect,
        Err(e) => {
            report(
                &color,
                &source,
                None,
                &e.to_string(),
                e.span,
                None,
                None,
                Some("run `elint --explain elint_expect_attr` for details"),
            );
            return error_count + 1;
        }
    };

    for branch in &ctx.branches {
        for diagnostic in &branch.preprocess_diagnostics {
            report(
                &color,
                &source,
                None,
                &diagnostic.message,
                diagnostic.span,
                None,
                None,
                None,
            );
            error_count += 1;
        }

        if !branch.tree.diagnostics().is_empty() {
            for diagnostic in branch.tree.diagnostics() {
                let span = branch
                    .span_of_range(diagnostic.token_range())
                    .unwrap_or(Span::ZERO);
                let endpoints = range_endpoints(branch, diagnostic.token_range());
                report(
                    &color,
                    &source,
                    None,
                    &parse_diagnostic_message(*diagnostic),
                    span,
                    endpoints,
                    None,
                    None,
                );
                error_count += 1;
            }
            continue;
        }

        for (rule, finding) in check(target_lint_names, &ctx, branch) {
            if expect.handle_error(rule.name, finding.span) {
                continue;
            }

            let note = (!known_errors.contains(rule.name))
                .then(|| format!("run `elint --explain {}` for details", rule.name));
            report(
                &color,
                &source,
                Some(rule.name),
                rule.summary(),
                finding.span,
                finding_endpoints(branch, finding.node),
                branch.enclosing_function_name(finding.node).as_deref(),
                note.as_deref(),
            );

            error_count += 1;
            known_errors.insert(rule.name);
        }
    }

    for rule in expect.unmatched_expectations() {
        let message = format!(
            "Lint Expectation Not Met: {} ({}): {}",
            rule.name,
            rule.target.describe(),
            rule.reason
        );
        report(
            &color,
            &source,
            None,
            &message,
            rule.span,
            None,
            None,
            Some("run `elint --explain elint_expect_attr` for details"),
        );
        error_count += 1;
    }

    error_count
}

/// Shared explanations embedded in the binary, keyed by the `--explain` name.
/// The stem of each file in `docs/explain/` matches its key.
const EXPLAINS: &[(&str, &str)] = &[(
    "elint_expect_attr",
    include_str!("../docs/explain/elint_expect_attr.md"),
)];

/// Returns the markdown text for `name`, looking first for a lint rule and
/// then for a shared explanation.
fn find_explanation(name: &str) -> Option<&'static str> {
    elint::rules::RULES
        .iter()
        .find(|rule| rule.name == name)
        .map(|rule| rule.text)
        .or_else(|| {
            EXPLAINS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, text)| *text)
        })
}

/// Prints the explanation for `name`, or fails with a pointer to `--list`.
fn print_explanation(name: &str) {
    let Some(text) = find_explanation(name) else {
        eprintln!("error: unknown explanation: {name}");
        eprintln!("run `elint --list` to see the available explanations");
        std::process::exit(1);
    };
    print!("{text}");
}

/// Renders the `--list` output: lint rules and shared explanations in name
/// order, with a usage hint at the end.
fn explanation_list() -> String {
    let mut out = String::from("Lint rules:\n");
    for rule in elint::rules::RULES {
        out.push_str(&format!("  {}\n", rule.name));
    }
    out.push_str("\nAdditional explanations:\n");
    for (name, _) in EXPLAINS {
        out.push_str(&format!("  {name}\n"));
    }
    out.push_str("\nRun `elint --explain <name>` to print one.\n");
    out
}

fn check(
    target_lint_names: &[String],
    ctx: &elint::Context,
    branch: &elint::BranchContext,
) -> Vec<(&'static elint::rules::Rule, elint::Finding)> {
    let mut errors = Vec::new();
    for rule in elint::rules::RULES {
        if !target_lint_names.is_empty() && !target_lint_names.iter().any(|n| n == rule.name) {
            continue;
        }

        for e in (rule.check)(ctx, branch) {
            errors.push((rule, e));
        }
    }
    errors
}

/// Human-readable message for one parse diagnostic.
fn parse_diagnostic_message(diagnostic: erl_parse::Diagnostic) -> String {
    let kind = match diagnostic.kind() {
        erl_parse::DiagnosticKind::UnexpectedToken => "unexpected token",
        erl_parse::DiagnosticKind::UnexpectedEof => "unexpected end of file",
        erl_parse::DiagnosticKind::SkippedToken => "skipped token",
        erl_parse::DiagnosticKind::MissingToken => "missing token",
        erl_parse::DiagnosticKind::NestingDepthExceeded => "nesting depth exceeded",
    };
    match diagnostic.expected() {
        erl_parse::Expected::Category(c) => format!("{kind}; expected {c}"),
        erl_parse::Expected::TokenKind(k) => format!("{kind}; expected {k:?}"),
        erl_parse::Expected::Unspecified => kind.to_string(),
    }
}

/// Resolves the exact first and last token spans of a finding's node, used
/// to draw the carets of a multi-line diagnostic.
fn finding_endpoints(branch: &elint::BranchContext, node: erl_parse::NodeId) -> Option<Endpoints> {
    let view = branch.tree.view(node)?;
    range_endpoints(branch, view.token_range())
}

/// Resolves the exact first and last token spans of a token range, used to
/// draw the carets of a multi-line diagnostic.
fn range_endpoints(
    branch: &elint::BranchContext,
    range: erl_parse::TokenRange,
) -> Option<Endpoints> {
    if range.is_empty() {
        return None;
    }
    let first = branch.span_of_range(erl_parse::TokenRange::single(range.start()))?;
    let last_index = erl_parse::TokenIndex::new(range.end().get() - 1);
    let last = branch.span_of_range(erl_parse::TokenRange::single(last_index))?;
    Some(Endpoints { first, last })
}

#[allow(clippy::too_many_arguments)]
fn report(
    color: &Color,
    source: &Source<'_>,
    code: Option<&str>,
    message: &str,
    span: Span,
    endpoints: Option<Endpoints>,
    enclosing: Option<&str>,
    note: Option<&str>,
) {
    let _ = elint::diagnostic::render(
        &mut std::io::stderr(),
        *color,
        source,
        code,
        message,
        span,
        endpoints,
        enclosing,
        note,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explanation_list_lists_every_rule_and_shared_explanation() {
        let out = explanation_list();
        for rule in elint::rules::RULES {
            assert!(out.contains(&format!("  {}\n", rule.name)), "{out:?}");
        }
        for (name, _) in EXPLAINS {
            assert!(out.contains(&format!("  {name}\n")), "{out:?}");
        }
        assert!(out.starts_with("Lint rules:\n"), "{out:?}");
        assert!(out.contains("\nAdditional explanations:\n"), "{out:?}");
        assert!(
            out.ends_with("\nRun `elint --explain <name>` to print one.\n"),
            "{out:?}"
        );
    }

    #[test]
    fn explanation_list_is_sorted_by_name() {
        let out = explanation_list();
        let rules: Vec<&str> = out
            .split("\nAdditional explanations:\n")
            .next()
            .expect("Additional explanations section")
            .lines()
            .skip(1)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let mut sorted = rules.clone();
        sorted.sort_unstable();
        assert_eq!(rules, sorted, "{out:?}");
    }

    #[test]
    fn find_explanation_returns_rule_then_shared_text() {
        let rule = elint::rules::RULES
            .iter()
            .find(|rule| rule.name == "element_bif")
            .expect("element_bif rule");
        assert_eq!(find_explanation("element_bif"), Some(rule.text));
        let (_, text) = EXPLAINS
            .iter()
            .find(|(name, _)| *name == "elint_expect_attr")
            .expect("elint_expect_attr explanation");
        assert_eq!(find_explanation("elint_expect_attr"), Some(*text));
    }

    #[test]
    fn find_explanation_rejects_unknown_names() {
        assert_eq!(find_explanation("README"), None);
        assert_eq!(find_explanation("no_such_name"), None);
    }

    #[test]
    fn explanation_names_do_not_overlap_rule_names() {
        for (name, _) in EXPLAINS {
            assert!(
                !elint::rules::RULES.iter().any(|rule| rule.name == *name),
                "shared explanation name collides with a lint rule: {name}"
            );
        }
    }

    #[test]
    fn explanation_list_does_not_mention_readme() {
        assert!(!explanation_list().contains("README"));
    }
}
