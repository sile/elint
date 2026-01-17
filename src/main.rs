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
    elint::rule_dont_use_nested_cases::check(&ast)?;

    Ok(())
}
