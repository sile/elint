use crate::item::{Item, ItemKind, Span};
use crate::token::{Token, TokenKind};

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub reason: String,
}

impl ParseError {
    pub fn new<T>(span: Span, reason: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            span,
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T = ()> = Result<T, ParseError>;

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

    pub fn last_span(&self) -> Span {
        if let Some(t) = self.token_i.checked_sub(1).and_then(|i| self.tokens.get(i)) {
            t.span
        } else {
            Span { start: 0, end: 0 }
        }
    }

    pub fn next_span(&self) -> Span {
        if let Some(t) = self.tokens.get(self.token_i) {
            t.span
        } else {
            let n = self.text.len();
            Span { start: n, end: n }
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.token_i).copied()?;
        self.token_i += 1;
        Some(t)
    }

    pub fn peek_token(&self) -> Option<Token> {
        self.tokens.get(self.token_i).copied()
    }

    pub fn token(&self) -> ParseResult<Token> {
        if let Some(token) = self.tokens.get(self.token_i).copied() {
            Ok(token)
        } else {
            let n = self.text.len();
            let span = Span { start: n, end: n };
            Err(ParseError::new(span, "unexpected EOF"))
        }
    }

    pub fn is_eof(&self) -> bool {
        self.token_i == self.tokens.len()
    }

    pub fn context(&self) -> ParseResult<Context> {
        if let Some(c) = self.contexts.last().copied() {
            Ok(c)
        } else {
            let span = self.token()?.span;
            Err(ParseError::new(span, "missing context"))
        }
    }

    pub fn parse_item<F>(&mut self, f: F) -> ParseResult
    where
        F: Fn(&mut Self) -> ParseResult<ItemKind>,
    {
        let i = self.items.len();
        let ctx = self.context()?;
        let start = self.next_span().start;
        let kind = f(self)?;
        let end = self.last_span().end;

        // TODO: negative span check

        let span = Span { start, end };
        let item = Item { ctx, kind, span };
        self.items.insert(i, item);

        Ok(())
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

    pub fn parse_comments(&mut self) {
        while let Some(t) = self.peek_token()
            && t.kind == TokenKind::Comment
        {
            self.parse_item(|p| {
                let _ = p.next_token();
                Ok(ItemKind::Comment)
            })
            .expect("bug");
        }
    }

    pub fn parse_expr(&mut self) -> ParseResult<()> {
        self.with_context(Context::Expr, |p| {
            p.parse_comments();
            p.parse_item(|p| p.parse_expr_item())?;
            p.parse_comments();
            Ok(())
        })
    }

    fn parse_expr_item(&mut self) -> ParseResult<ItemKind> {
        let t = self.token()?;
        match t.kind {
            _ => todo!(),
        }
    }
}
