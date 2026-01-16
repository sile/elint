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
    let json_view = to_json(view, &text);

    // Pretty print with formatting
    let output = nojson::json(|f| {
        f.set_indent_size(2);
        f.set_spacing(true);
        f.value(&json_view.0)
    });

    println!("{}", output);

    Ok(true)
}

// TODO: rename
pub fn to_json<'a>(
    view: crate::item::ItemView<'a>,
    text: &'a str,
) -> nojson::Json<ItemViewJson<'a>> {
    let item_json = ItemViewJson {
        kind: format!("{:?}", view.kind()),
        span: (view.span().start, view.span().end),
        text: view.text(text),
        children: view
            .children()
            .map(|child| to_json(child, text).0)
            .collect(),
    };
    nojson::Json(item_json)
}

#[derive(Debug)]
pub struct ItemViewJson<'a> {
    pub kind: String,
    pub span: (usize, usize),
    pub text: &'a str,
    pub children: Vec<ItemViewJson<'a>>,
}

impl nojson::DisplayJson for ItemViewJson<'_> {
    fn fmt(&self, f: &mut nojson::JsonFormatter<'_, '_>) -> std::fmt::Result {
        f.object(|f| {
            f.member("kind", &self.kind)?;
            f.member(
                "span",
                nojson::array(|f| {
                    f.element(self.span.0)?;
                    f.element(self.span.1)
                }),
            )?;
            f.member("text", self.text)?;
            f.member(
                "children",
                nojson::array(|f| {
                    for child in &self.children {
                        f.element(child)?;
                    }
                    Ok(())
                }),
            )
        })
    }
}
