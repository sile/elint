#[derive(Debug)]
pub struct ExpectRules {
    pub rules: Vec<ExpectRule>,
}

impl ExpectRules {
    pub fn new(parser: &crate::parse::Parser) -> Result<Self, crate::Error> {
        let mut rules = Vec::new();

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

            let Some(target_span) = parser
                .items
                .binary_search_by_key(&comment.span, |t| t.span)
                .err()
                .and_then(|i| parser.items.get(i).map(|t| t.span))
            else {
                continue;
            };

            for name in text.split(',') {
                let name = name.trim();
                let Some(rule_name) = crate::RULE_NAMES.iter().find(|v| **v==name) else {
                    return Err(crate::Error::new(
                        comment.span,
                        format!("unknown rule: {name}"),
                    ));
                };

                rules.push(ExpectRule {
                    name: rule_name,
                    comment_span: comment.span,
                    target_span,
                });
            }
        }

        Ok(Self { rules })
    }
}

#[derive(Debug)]
pub struct ExpectRule {
    pub name: &'static str,
    pub comment_span: crate::Span,
    pub target_span: crate::Span,
}
