use crate::item::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Atom,
    Variable,
    Integer,
    Float,
    Char,
    String,
    SigilString,
    Keyword,
    Symbol,
    Comment,
}
