#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
    pub size: std::num::NonZeroUsize,
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
