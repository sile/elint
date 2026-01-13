use crate::item::Item;

#[derive(Debug)]
pub enum ParseError {
    Tokenize(erl_tokenize::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Tokenize(e) => write!(f, "Tokenization error: {}", e),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Tokenize(e) => Some(e),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseContext {
    Expr,
}

#[derive(Debug)]
pub struct Parser {
    pub ctx: ParseContext,
    pub items: Vec<Item>,
    pub tokens: Vec<erl_tokenize::Token>,
    pub token_i: usize,
}

impl Parser {
    pub fn new(ctx: ParseContext, tokens: Vec<erl_tokenize::Token>) -> Self {
        Self {
            ctx,
            items: Vec::new(),
            tokens,
            token_i: 0,
        }
    }
}
