fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    let extended_mode = noargs::flag("ext").short('h').take(&mut args).is_present();
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

    let code = std::fs::read_to_string(&path)?;
    elint::parse_code(&code)?;

    Ok(())
}
