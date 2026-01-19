fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    let extended_mode = noargs::flag("ext").short('x').take(&mut args).is_present();
    if extended_mode {
        let _ = elint::command_parse::try_run(&mut args)?;
        if let Some(help) = args.finish()? {
            print!("{help}");
        }
        return Ok(());
    }

    let only_parse = noargs::flag("only-parse").take(&mut args).is_present();

    let mut target_lint_names: Vec<String> = Vec::new();
    while let Some(a) = noargs::opt("lint")
        .short('l')
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
            // eprintln!("# {}", path.display());
            let text = std::fs::read_to_string(&path)?;
            let tokens = elint::token::tokenize(&text)?;
            let mut parser = elint::parse::Parser::new(&text, tokens);
            parser.parse_module().inspect_err(|e| {
                let (line, column, context_lines) = get_error_context(e.span.start, &text);
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
            })?;
            if only_parse {
                continue;
            }

            let mut expect = elint::expect::ExpectRules::new(&parser).inspect_err(|e| {
                let (line, column, context_lines) = get_error_context(e.span.start, &text);
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
            })?;

            let ast = elint::Ast {
                text: text.clone(),
                items: parser.items,
            };
            for (rule, span) in check(&target_lint_names, &ast) {
                if expect.handle_error(rule.name, span) {
                    continue;
                }

                let (line, column, context_lines) = get_error_context(span.start, &text);
                eprintln!("Lint Error: RULE={}", rule.name);
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
                if !known_errors.contains(rule.name) {
                    eprintln!(
                        "To suppress this error, add a preceding comment `%% ELINT_EXPECT: {}`",
                        rule.name
                    );
                    eprintln!("\nLint Rule Details\n=======\n\n{}\n", rule.text.trim());
                    eprintln!("------\n");
                }

                error_count += 1;
                known_errors.insert(rule.name);
            }

            for (lint_name, span) in expect.unmatched_expectations() {
                if !target_lint_names.is_empty()
                    && !target_lint_names.iter().any(|n| n == lint_name)
                {
                    continue;
                }

                let (line, column, context_lines) = get_error_context(span.start, &text);
                eprintln!("Lint Expectation Not Met: RULE={lint_name}");
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
                error_count += 1;
            }
        }
    }

    if error_count > 0 {
        eprintln!("Found {error_count} lint error(s)");
        std::process::exit(1);
    }

    Ok(())
}

fn check(
    target_lint_names: &[String],
    ast: &elint::Ast,
) -> Vec<(&'static elint::Rule, elint::Span)> {
    let mut errors = Vec::new();
    for rule in elint::RULES {
        if !target_lint_names.is_empty() && !target_lint_names.iter().any(|n| n == rule.name) {
            continue;
        }

        for e in (rule.check)(ast) {
            errors.push((rule, e));
        }
    }
    errors
}

fn get_error_context(byte_offset: usize, text: &str) -> (usize, usize, String) {
    let mut line = 1usize;
    let mut column = 1usize;

    // Find line and column from byte offset
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

    // Extract context lines (current line + surrounding lines)
    let mut context_lines = String::new();

    // Build context with line numbers
    let lines: Vec<&str> = text.lines().collect();
    let current_line_idx = line - 1;

    // Calculate range: show 2 previous lines if exist (ditto for next lines)
    let start_idx = current_line_idx.saturating_sub(2);
    let end_idx = (current_line_idx + 3).min(lines.len());

    // Calculate the width needed for line numbers
    let max_line_num = end_idx;
    let line_num_width = max_line_num.to_string().len();

    for i in start_idx..end_idx {
        let line_num = i + 1;
        let is_error_line = i == current_line_idx;

        context_lines.push_str(&format!(
            " {:width$} | {}\n",
            line_num,
            lines[i],
            width = line_num_width
        ));

        // Show error indicator only on the error line
        if is_error_line {
            let padding = " ".repeat(line_num_width + 3 + column);
            context_lines.push_str(&format!("{padding}^\n"));
        }
    }

    (line, column, context_lines)
}
