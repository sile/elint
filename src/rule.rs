use crate::item::Item;
use crate::parse::ParseResult;

#[derive(Debug, Clone)]
pub struct Rule {
    pub title: String,
    pub full_text: String,
    pub ng_pattern: NgPattern,
}

impl Rule {
    pub fn parse(text: &str) -> ParseResult<Self> {
        let full_text = text.to_owned();
        let text = text.strip_prefix("# RULE:").expect("TODO");
        let (title, text) = text.trim().split_once('\n').expect("TODO");
        let title = title.trim().to_owned();

        let code = text
            .split_once("\n```erlang\n")
            .expect("TODO")
            .1
            .split_once("\n```\n")
            .expect("TODO")
            .0;

        Ok(Self {
            title,
            full_text,
            ng_pattern: NgPattern::parse(code)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NgPattern {
    pub text: String,
    pub items: Vec<Item>,
    pub comments: Vec<Item>,
}

impl NgPattern {
    pub fn parse(text: &str) -> ParseResult<Self> {
        let tokens = crate::token::tokenize(text).expect("TODO");
        let mut parser = crate::parse::Parser::new(text, tokens);
        parser.parse_expr()?;
        Ok(Self {
            text: text.to_owned(),

            items: parser.items,
            comments: parser.comments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rule() {
        let text = include_str!("../rules/rule-dont-use-nested-cases.md");
        Rule::parse(text).expect("failed to parse rule text");
    }
}
