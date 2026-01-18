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

    for path in paths {
        for path in elint::fs::collect_erlang_files(path)? {
            eprintln!("# {}", path.display());
            let text = std::fs::read_to_string(&path)?;
            let tokens = elint::token::tokenize(&text)?;
            let mut parser = elint::parse::Parser::new(&text, tokens);
            parser.parse_module().inspect_err(|e| {
                let (line, column, context_lines) = get_error_context(e.span.start, &text);
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
            })?;

            let ast = elint::Ast {
                text: text.clone(),
                items: parser.items,
            };
            if let Err((e, lint_name)) = check(&ast) {
                let (line, column, context_lines) = get_error_context(e.span.start, &text);
                eprintln!("Lint Error: RULE={lint_name}");
                eprintln!("  --> {}:{}:{}", path.display(), line, column);
                eprintln!("{context_lines}");
                eprintln!("\n{}\n", e.message);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn check(ast: &elint::Ast) -> Result<(), (elint::Error, &'static str)> {
    elint::rule_dont_use_nested_cases::check(&ast)?;
    Ok(())
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

    for i in start_idx..end_idx {
        let line_num = i + 1;
        let is_error_line = i == current_line_idx;

        context_lines.push_str(&format!(" {} | {}\n", line_num, lines[i]));

        // Show error indicator only on the error line
        if is_error_line {
            context_lines.push_str(&format!("   | {}^\n", " ".repeat(column.saturating_sub(1))));
        }
    }

    (line, column, context_lines)
}
