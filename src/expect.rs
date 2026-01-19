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
                let Some((rule_name, _)) = crate::RULES.iter().find(|v| v.0 == name) else {
                    return Err(crate::Error::new(
                        comment.span,
                        format!("unknown rule: {name}"),
                    ));
                };

                rules.push(ExpectRule {
                    name: rule_name,
                    comment_span: comment.span,
                    target_span,
                    matched: false,
                });
            }
        }

        Ok(Self { rules })
    }

    pub fn handle_error(&mut self, lint_name: &'static str, span: crate::Span) -> bool {
        let mut expected = false;
        for rule in &mut self.rules {
            if rule.name == lint_name && rule.target_span.contains(span) {
                rule.matched = true;
                expected = true;
            }
        }
        expected
    }

    pub fn unmatched_expectations(&self) -> impl Iterator<Item = (&'static str, crate::Span)> {
        self.rules
            .iter()
            .filter(|r| !r.matched)
            .map(|r| (r.name, r.comment_span))
    }
}

#[derive(Debug)]
pub struct ExpectRule {
    pub name: &'static str,
    pub comment_span: crate::Span,
    pub target_span: crate::Span,
    pub matched: bool,
}
