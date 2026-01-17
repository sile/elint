#[derive(Debug, Clone)]
pub struct Rule {
    pub title: String,
    pub ng: NgRule,
    pub ok: Option<OkRule>,
}

#[derive(Debug, Clone)]
pub struct NgRule {
    pub contents: Vec<RuleContent>,
}

#[derive(Debug, Clone)]
pub struct OkRule {
    pub contents: Vec<RuleContent>,
}

#[derive(Debug, Clone)]
pub enum RuleContent {
    Text(String),
    Code(RulePattern),
}

#[derive(Debug, Clone)]
pub struct RulePattern {}
