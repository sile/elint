use crate::parse::Context;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Item {
    pub ctx: Context,
    pub kind: ItemKind,
    pub span: Span,
}

impl Item {
    pub fn text(self, full_text: &str) -> &str {
        self.span.text(full_text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Variable,
    Atom,
    Integer,
    Comment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn text(self, full_text: &str) -> &str {
        &full_text[self.start..self.end]
    }
}
