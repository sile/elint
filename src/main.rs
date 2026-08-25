//! Erlang code linter CLI.

use std::path::Path;

use elint::diagnostic::{Color, Source};
use elint::Span;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    if noargs::cmd("explain")
        .doc("Print the markdown description of a lint rule")
        .take(&mut args)
        .is_present()
    {
        let name: String = noargs::arg("<RULE_NAME>")
            .take(&mut args)
            .then(|a| Ok::<_, &str>(a.value().to_string()))?;
        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }
        explain_rule(&name)?;
        return Ok(());
    }

    if noargs::cmd("doc")
        .doc("Print an embedded document, or list the available documents")
        .take(&mut args)
        .is_present()
    {
        let name: Option<String> = noargs::arg("<DOC_NAME>")
            .take(&mut args)
            .present_and_then(|a| Ok::<_, &str>(a.value().to_string()))?;
        if let Some(help) = args.finish()? {
            print!("{help}");
            return Ok(());
        }
        match name {
            Some(name) => print_doc(&name),
            None => print_doc_list(),
        }
        return Ok(());
    }

    let mut target_lint_names: Vec<String> = Vec::new();
    while let Some(a) = noargs::opt("lint")
        .short('l')
        .ty("LINT_RULE_NAME")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?
    {
        target_lint_names.push(a);
    }

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    while let Some(path) = noargs::arg("[PATH]..")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?
    {
        paths.push(path);
    }

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    if paths.is_empty() {
        paths.push("src/".into());
        paths.push("tests/".into());
    }

    let mut error_count = 0;
    let mut known_errors = std::collections::HashSet::new();
    for path in paths {
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
            report(&color, &source, None, &e.to_string(), span, None);
            return 1;
        }
    };

    let mut error_count = 0;

    let mut expect = match elint::expect::ExpectRules::new(&ctx, target_lint_names) {
        Ok(expect) => expect,
        Err(e) => {
            report(&color, &source, None, &e.to_string(), e.span, None);
            return error_count + 1;
        }
    };

    for branch in &ctx.branches {
        for diagnostic in &branch.preprocess_diagnostics {
            report(&color, &source, None, &diagnostic.message, diagnostic.span, None);
            error_count += 1;
        }

        if !branch.tree.diagnostics().is_empty() {
            for diagnostic in branch.tree.diagnostics() {
                let span = branch
                    .span_of_range(diagnostic.range())
                    .unwrap_or(Span::ZERO);
                report(
                    &color,
                    &source,
                    None,
                    &parse_diagnostic_message(*diagnostic),
                    span,
                    None,
                );
                error_count += 1;
            }
            continue;
        }

        for (rule, span) in check(target_lint_names, &ctx, branch) {
            if expect.handle_error(rule.name, span) {
                continue;
            }

            let note = (!known_errors.contains(rule.name)).then(|| {
                format!(
                    "run `elint explain {}` for details; suppress with `-elint_expect({}, ...)`",
                    rule.name, rule.name
                )
            });
            report(
                &color,
                &source,
                Some(rule.name),
                rule.summary(),
                span,
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
        report(&color, &source, None, &message, rule.span, None);
        error_count += 1;
    }

    error_count
}

fn explain_rule(name: &str) -> noargs::Result<()> {
    let Some(rule) = elint::rules::RULES.iter().find(|rule| rule.name == name) else {
        eprintln!("unknown lint rule: {name}");
        std::process::exit(1);
    };
    print!("{}", rule.text);
    Ok(())
}

/// Documents embedded in the binary, keyed by the `elint doc` name.
const DOCS: &[(&str, &str, &str)] = &[
    (
        "expectations",
        "The `-elint_expect` notation and suppression",
        include_str!("../docs/expectations.md"),
    ),
    (
        "diagnostics",
        "How problems are reported and what is ignored",
        include_str!("../docs/diagnostics.md"),
    ),
    (
        "preprocessing",
        "The tokenize / preprocess / parse pipeline",
        include_str!("../docs/preprocessing.md"),
    ),
];

fn print_doc(name: &str) {
    let Some((_, _, text)) = DOCS.iter().find(|(n, _, _)| *n == name) else {
        eprintln!("unknown document: {name}");
        std::process::exit(1);
    };
    print!("{text}");
}

fn print_doc_list() {
    for (name, description, _) in DOCS {
        println!("{name} - {description}");
    }
}

fn check(
    target_lint_names: &[String],
    ctx: &elint::Context,
    branch: &elint::BranchContext,
) -> Vec<(&'static elint::rules::Rule, elint::Span)> {
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

fn report(
    color: &Color,
    source: &Source<'_>,
    code: Option<&str>,
    message: &str,
    span: Span,
    note: Option<&str>,
) {
    let _ = elint::diagnostic::render(
        &mut std::io::stderr(),
        *color,
        source,
        code,
        message,
        span,
        note,
    );
}
