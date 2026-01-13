fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    noargs::HELP_FLAG.take_help(&mut args);

    let name: String = noargs::arg("NAME")
        .take(&mut args)
        .then(|a| a.value().parse())?;

    args.finish()?;

    println!("Hello, {}", name);

    Ok(())
}
