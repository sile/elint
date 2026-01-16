pub fn try_run(args: &mut noargs::RawArgs) -> noargs::Result<bool> {
    if !noargs::cmd("parse").take(args).is_present() {
        return Ok(false);
    }

    let path: std::path::PathBuf = noargs::arg("ERL_FILE_PATH")
        .example("/path/to/input.erl")
        .take(args)
        .then(|o| o.value().parse())?;

    if args.metadata().help_mode {
        return Ok(true);
    }

    let text = std::fs::read_to_string(&path)?;
    let tokens = crate::token::tokenize(&text)?;
    let mut parser = crate::parse::Parser::new(&text, tokens);
    parser.parse_module()?;

    let view = crate::item::ItemView::new(&parser.items, 0);

    Ok(true)
}
