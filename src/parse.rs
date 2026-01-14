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
        self.parse_comments()?;

        let i = self.items.len();
        let ctx = self.context()?;
        let start = self.next_span().start;
        let kind = f(self)?;
        let end = self.last_span().end;

        // TODO: negative span check

        let span = Span { start, end };
        let item = Item { ctx, kind, span };
        self.items.insert(i, item);

        self.parse_comments()?;
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

    pub fn parse_comments(&mut self) -> ParseResult<()> {
        let ctx = self.context()?;
        while let Some(t) = self.peek_token()
            && t.kind == TokenKind::Comment
        {
            let kind = ItemKind::Comment;
            let span = t.span;
            let item = Item { ctx, kind, span };
            self.items.push(item);
        }
        Ok(())
    }

    pub fn parse_expr(&mut self) -> ParseResult<()> {
        let ctx = Context::Expr;
        self.with_context(ctx, |p| {
            let i = p.items.len();
            p.parse_item(|p| p.parse_expr_item())?;

            let mut has_binary_op = false;
            while let Some(t) = p.peek_token()
                && t.is_binary_op(ctx, p.text)
            {
                p.parse_item(|p| p.parse_binary_op_item())?;
                p.parse_item(|p| p.parse_expr_item())?;
                has_binary_op = true;
            }
            if has_binary_op {
                let kind = ItemKind::BinaryOpExprs;
                let start = p.items[i].span.start;
                let end = p.last_span().end;
                let span = Span { start, end };
                let item = Item { ctx, kind, span };
                p.items.insert(i, item);
            }

            Ok(())
        })
    }

    fn parse_binary_op_item(&mut self) -> ParseResult<ItemKind> {
        let ctx = self.context()?;
        let t = self.token()?;
        if t.is_binary_op(ctx, self.text) {
            todo!()
        }
        Ok(ItemKind::BinaryOp)
    }

    fn parse_expr_item(&mut self) -> ParseResult<ItemKind> {
        let t = self.token()?;
        match t.kind {
            TokenKind::Comment => panic!("bug"),
            TokenKind::Integer => {
                self.next_token();
                Ok(ItemKind::Integer)
            }
            TokenKind::Atom => {
                self.next_token();
                Ok(ItemKind::Atom)
            }
            TokenKind::Variable => {
                self.next_token();
                Ok(ItemKind::Variable)
            }
            TokenKind::Float => {
                self.next_token();
                Ok(ItemKind::Float)
            }
            TokenKind::Char => {
                self.next_token();
                Ok(ItemKind::Char)
            }
            TokenKind::String => {
                self.next_token();
                Ok(ItemKind::String)
            }
            TokenKind::SigilString => {
                self.next_token();
                Ok(ItemKind::SigilString)
            }
            TokenKind::Keyword | TokenKind::Symbol => {
                self.next_token();
                // Handle as appropriate for your grammar
                todo!("Define handling for Keyword and Symbol tokens")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> ParseResult<Vec<Item>> {
        let tokens = crate::token::tokenize(text).expect("tokenization failed");
        let mut parser = Parser::new(text, tokens);
        parser.with_context(Context::Expr, |p| {
            while !p.is_eof() {
                p.parse_expr()?;
            }
            Ok(())
        })?;
        Ok(parser.items)
    }

    #[test]
    fn parse_integers() {
        let input = " 42 ";
        let items = parse(input).expect("parse failed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, ItemKind::Integer);
        assert_eq!(items[0].text(input), "42");
    }
}
