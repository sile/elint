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

    let path: std::path::PathBuf = noargs::arg("PATH")
        .take(&mut args)
        .then(|a| a.value().parse())?;

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let text = std::fs::read_to_string(&path)?;
    let tokens = elint::token::tokenize(&text)?;
    let mut parser = elint::parse::Parser::new(&text, tokens);
    parser.parse_module()?;

    let ast = elint::Ast {
        text: text.clone(),
        items: parser.items,
    };
    if let Err((e, lint_name)) = check(&ast) {
        let (line, column, context_lines) = get_error_context(e.span.start, &text);
        eprintln!("Lint Error: {lint_name}");
        eprintln!("  --> {}:{}:{}", path.display(), line, column);
        eprintln!("{context_lines}");
        eprintln!("\n{}", e.message);
        std::process::exit(1);
    }

    Ok(())
}

fn check(ast: &elint::Ast) -> Result<(), (elint::Error, &'static str)> {
    elint::rule_dont_use_nested_cases::check(&ast)?;
    Ok(())
}

fn get_error_context(byte_offset: usize, text: &str) -> (usize, usize, String) {
    let mut line = 1;
    let mut column = 1usize;
    let mut line_start = 0;

    // Find line and column from byte offset
    for (i, ch) in text.chars().enumerate() {
        if i >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
            line_start = i + 1;
        } else {
            column += 1;
        }
    }

    // Extract context lines (current line + surrounding lines)
    let mut context_lines = String::new();

    // Find the start of the current line
    for ch in text[..line_start].chars().rev() {
        if ch == '\n' {
            break;
        }
    }

    // Build context with line numbers
    let lines: Vec<&str> = text.lines().collect();
    let current_line_idx = line - 1;

    // Show previous line (if exists)
    if current_line_idx > 0 {
        context_lines.push_str(&format!(
            " {} | {}\n",
            current_line_idx,
            lines[current_line_idx - 1]
        ));
    }

    // Show current line with error indicator
    context_lines.push_str(&format!(" {} | {}\n", line, lines[current_line_idx]));
    context_lines.push_str(&format!("   | {}^\n", " ".repeat(column.saturating_sub(1))));

    // Show next line (if exists)
    if current_line_idx + 1 < lines.len() {
        context_lines.push_str(&format!(
            " {} | {}\n",
            line + 1,
            lines[current_line_idx + 1]
        ));
    }

    (line, column, context_lines)
}
