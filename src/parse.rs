use crate::item::Item;
use crate::token::Token;

#[derive(Debug)]
pub enum ParseError {
    Tokenize(erl_tokenize::Error), // TODO: remove
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
pub enum Context {
    Expr,
}

#[derive(Debug)]
pub struct Parser<'text> {
    pub text: &'text str,
    pub contexts: Vec<Context>,
    pub items: Vec<Item>,
    pub tokens: Vec<Token>,
    pub token_i: usize,
}

impl<'text> Parser<'text> {
    pub fn new(text: &'text str, tokens: Vec<Token>) -> Self {
        Self {
            contexts: Vec::new(),
            items: Vec::new(),
            text,
            tokens,
            token_i: 0,
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.token_i).copied()?;
        self.token_i += 01;
        Some(t)
    }
}
