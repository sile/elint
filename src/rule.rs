use crate::item::Item;
use crate::parse::ParseResult;

#[derive(Debug, Clone)]
pub struct Rule {
    pub title: String,
    pub ng: NgRule,
    pub ok: Option<OkRule>,
}

impl Rule {
    pub fn parse(text: &str) -> ParseResult<Self> {
        let text = text.strip_prefix("# RULE:").expect("TODO");
        let (title, text) = text.trim().split_once('\n').expect("TODO");
        let title = title.trim().to_owned();

        let text = text.split_once("## NG\n").expect("TODO").1;
        if let Some((ng_text, ok_text)) = text.split_once("\n## OKn") {
            Ok(Self {
                title,
                ng: NgRule::parse(ng_text.trim())?,
                ok: Some(OkRule::parse(ok_text.trim())?),
            })
        } else {
            Ok(Self {
                title,
                ng: NgRule::parse(text.trim())?,
                ok: None,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Expr,
}

#[derive(Debug, Clone)]
pub struct NgRule {
    pub contents: Vec<RuleContent>,
}

impl NgRule {
    pub fn parse(mut text: &str) -> ParseResult<Self> {
        let mut contents = Vec::new();
        while !text.is_empty() {
            let Some((t0, t1)) = text.split_once("```erlang\n") else {
                contents.push(RuleContent::Text(text.to_owned()));
                break;
            };
            contents.push(RuleContent::Text(t0.to_owned()));

            let (code, remaining) = t1.split_once("```").expect("bug");
            contents.push(RuleContent::Code(RulePattern::parse(code)?));
            text = remaining.trim();
        }
        Ok(Self { contents })
    }
}

#[derive(Debug, Clone)]
pub struct OkRule {
    pub contents: Vec<RuleContent>,
}

impl OkRule {
    pub fn parse(text: &str) -> ParseResult<Self> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum RuleContent {
    Text(String),
    Code(RulePattern),
}

#[derive(Debug, Clone)]
pub struct RulePattern {
    pub text: String,
    pub contexts: Vec<Context>,
    pub items: Vec<Item>,
    pub comments: Vec<Item>,
}

impl RulePattern {
    pub fn parse(text: &str) -> ParseResult<Self> {
        // TODO: other contexts
        let tokens = crate::token::tokenize(text).expect("TODO");
        let mut parser = crate::parse::Parser::new(text, tokens);
        parser.parse_expr()?;
        Ok(Self {
            text: text.to_owned(),
            contexts: vec![Context::Expr],
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
