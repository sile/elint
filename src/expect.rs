#[derive(Debug)]
pub struct ExpectRules {
    pub rules: Vec<ExpectRule>,
}

impl ExpectRules {
    pub fn new(parser: &crate::parse::Parser) -> Result<Self, crate::Error> {
        for comment in &parser.comments {
            let text = comment.text(&parser.text);
            let Some(text) = text.strip_prefix("%%").and_then(|t| t.lines().next()) else {
                continue;
            };
            let text = text.trim();

            let Some(text) = text.strip_prefix("ELINT_EXPECT:") else {
                continue;
            };
            let text = text.trim();

            for name in text.split(',') {
                let name = name.trim();
                if !crate::RULE_NAMES.contains(&name) {
                    return Err(crate::Error::new(
                        comment.span,
                        format!("unknown rule: {name}"),
                    ));
                }
            }
            //
        }
        todo!()
    }
}

#[derive(Debug)]
pub struct ExpectRule {
    pub name: &'static str,
    pub comment_span: crate::Span,
    pub target_span: crate::Span,
}
