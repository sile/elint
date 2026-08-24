//! Erlang code linter CLI.

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
        eprintln!("Found {error_count} lint error(s)");
        std::process::exit(1);
    }

    Ok(())
}

fn lint_file(
    path: &std::path::Path,
    target_lint_names: &[String],
    known_errors: &mut std::collections::HashSet<&'static str>,
) -> usize {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            return 1;
        }
    };

    let ctx = match elint::Context::analyze(path, text.clone()) {
        Ok(ctx) => ctx,
        Err(e) => {
            let (line, column, context_lines) = get_error_context(e.position.offset(), &text);
            eprintln!("Tokenize error: {e}");
            eprintln!("  --> {}:{}:{}", path.display(), line, column);
            eprintln!("{context_lines}");
            return 1;
        }
    };

    let mut error_count = 0;

    let mut expect = match elint::expect::ExpectRules::new(&ctx) {
        Ok(expect) => expect,
        Err(e) => {
            let (line, column, context_lines) = get_error_context(e.span.start, &ctx.text);
            eprintln!("{e:?}");
            eprintln!("  --> {}:{}:{}", path.display(), line, column);
            eprintln!("{context_lines}");
            return error_count + 1;
        }
    };

    for branch in &ctx.branches {
        for diagnostic in &branch.preprocess_diagnostics {
            let (line, column, context_lines) = get_error_context(diagnostic.span.start, &ctx.text);
            eprintln!("Preprocess: {}", diagnostic.message);
            eprintln!("  --> {}:{}:{}", path.display(), line, column);
            eprintln!("{context_lines}");
            error_count += 1;
        }

        if !branch.tree.diagnostics().is_empty() {
            for diagnostic in branch.tree.diagnostics() {
                let span = branch
                    .span_of_range(diagnostic.range())
                    .unwrap_or(elint::Span::ZERO);
                let (line, column, context_lines) = get_error_context(span.start, &ctx.text);
                eprintln!("Parse: {diagnostic:?}");
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
                error_count += 1;
            }
            continue;
        }

        for (rule, span) in check(target_lint_names, &ctx, branch) {
            if expect.handle_error(rule.name, span) {
                continue;
            }

            let (line, column, context_lines) = get_error_context(span.start, &ctx.text);
            eprintln!("Lint Error: RULE={}", rule.name);
            eprintln!("  --> {}:{}:{}", path.display(), line, column);
            eprintln!("{context_lines}");
            if !known_errors.contains(rule.name) {
                eprintln!(
                    "To suppress this error, add `-elint_expect({}, {{function, Name, Arity}}, \"reason\").`",
                    rule.name
                );
                eprintln!("For details, run `elint explain {}`", rule.name);
            }

            error_count += 1;
            known_errors.insert(rule.name);
        }
    }

    for rule in expect.unmatched_expectations() {
        if !target_lint_names.is_empty() && !target_lint_names.iter().any(|n| n == rule.name) {
            continue;
        }

        let (line, column, context_lines) = get_error_context(rule.span.start, &ctx.text);
        eprintln!(
            "Lint Expectation Not Met: {} ({}/{}): {}",
            rule.name, rule.target.0, rule.target.1, rule.reason
        );
        eprintln!("  --> {}:{}:{}", path.display(), line, column);
        eprintln!("{context_lines}");
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

fn get_error_context(byte_offset: usize, text: &str) -> (usize, usize, String) {
    let mut line = 1usize;
    let mut column = 1usize;

    for (i, ch) in text.char_indices() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    let mut context_lines = String::new();
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return (line, column, context_lines);
    }
    let current_line_idx = (line - 1).min(lines.len().saturating_sub(1));

    let start_idx = current_line_idx.saturating_sub(2);
    let end_idx = (current_line_idx + 3).min(lines.len());

    let max_line_num = end_idx;
    let line_num_width = max_line_num.to_string().len();

    for (i, line_text) in lines.iter().enumerate().take(end_idx).skip(start_idx) {
        let line_num = i + 1;
        let is_error_line = i == current_line_idx;

        context_lines.push_str(&format!(
            " {:width$} | {line_text}\n",
            line_num,
            width = line_num_width
        ));

        if is_error_line {
            let padding = " ".repeat(line_num_width + 3 + column);
            context_lines.push_str(&format!("{padding}^\n"));
        }
    }

    (line, column, context_lines)
}
