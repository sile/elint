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
            let (content, remaining) = RuleContent::parse(text)?;
            contents.push(content);
            text = remaining;
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

impl RuleContent {
    pub fn parse(text: &str) -> ParseResult<(Self, &str)> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct RulePattern {
    pub contexts: Vec<Context>,
    pub items: Vec<Item>,
    pub comments: Vec<Item>,
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
