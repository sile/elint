use crate::item::{Item, Span};
use crate::token::Token;

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult = Result<(), ParseError>;

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

    pub fn with_context<F, T>(&mut self, context: Context, f: F) -> T
    where
        F: Fn(&mut Self) -> T,
    {
        self.contexts.push(context);
        let result = f(self);
        self.contexts.pop();
        result
    }
}
