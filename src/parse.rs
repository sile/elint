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

#[derive(Debug)]
pub struct Parser<'text> {
    pub text: &'text str,
    pub items: Vec<Item>,
    pub comments: Vec<Item>,
    pub tokens: Vec<Token>,
    pub token_i: usize,
}

impl<'text> Parser<'text> {
    pub fn new(text: &'text str, mut tokens: Vec<Token>) -> Self {
        let mut comments = Vec::new();
        for t in tokens.iter().filter(|t| t.kind == TokenKind::Comment) {
            comments.push(Item::new(ItemKind::Comment, t.span));
        }
        tokens.retain(|t| t.kind != TokenKind::Comment);

        Self {
            items: Vec::new(),
            comments: Vec::new(),
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

    pub fn next_token(&mut self) -> ParseResult<Token> {
        let t = self.token()?;
        self.token_i += 1;
        Ok(t)
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

    pub fn parse_item<F>(&mut self, f: F) -> ParseResult
    where
        F: Fn(&mut Self) -> ParseResult<ItemKind>,
    {
        let i = self.items.len();
        let start = self.next_span().start;
        let kind = f(self)?;
        let end = self.last_span().end;

        // TODO: negative span check

        let span = Span { start, end };
        let item = Item { kind, span };
        self.items.insert(i, item);

        Ok(())
    }

    pub fn parse_expr(&mut self) -> ParseResult<()> {
        let i = self.items.len();
        self.parse_expr_item()?;

        let mut has_binary_op = false;
        while let Some(t) = self.peek_token()
            && t.is_binary_op(self.text)
        {
            self.parse_binary_op_item()?;
            self.parse_expr_item()?;
            has_binary_op = true;
        }
        if has_binary_op {
            let start = self.items[i].span.start;
            let end = self.last_span().end;
            self.insert_item(i, ItemKind::BinaryOpExprs, Span::new(start, end));
        }

        Ok(())
    }

    fn push_item(&mut self, kind: ItemKind, span: Span) {
        self.items.push(Item::new(kind, span));
    }

    fn insert_item(&mut self, i: usize, kind: ItemKind, span: Span) {
        self.items.insert(i, Item::new(kind, span));
    }

    fn parse_binary_op_item(&mut self) -> ParseResult<()> {
        let t = self.next_token()?;
        if !t.is_binary_op(self.text) {
            return Err(ParseError::new(t.span, "expected binary operator"));
        }
        self.push_item(ItemKind::BinaryOp, t.span);
        Ok(())
    }

    fn parse_expr_item(&mut self) -> ParseResult<()> {
        //let i = self.items.len();
        self.parse_base_expr_item()?;
        if self.is_next_symbol(":") {
            //    self.parse_mfa_call(expr)
            todo!()
        } else {
            Ok(())
        }
    }

    fn is_next_symbol(&self, name: &str) -> bool {
        self.peek_token()
            .is_some_and(|t| t.kind == TokenKind::Symbol && t.text(self.text) == name)
    }

    fn parse_base_expr_item(&mut self) -> ParseResult<()> {
        let t = self.next_token()?;
        match t.kind {
            TokenKind::Comment => panic!("bug"),
            TokenKind::Integer => self.push_item(ItemKind::Integer, t.span),
            TokenKind::Atom => self.push_item(ItemKind::Atom, t.span),
            TokenKind::Variable => self.push_item(ItemKind::Variable, t.span),
            TokenKind::Float => self.push_item(ItemKind::Float, t.span),
            TokenKind::Char => self.push_item(ItemKind::Char, t.span),
            TokenKind::String => self.push_item(ItemKind::String, t.span),
            TokenKind::SigilString => self.push_item(ItemKind::SigilString, t.span),
            TokenKind::Keyword | TokenKind::Symbol => {
                // Handle as appropriate for your grammar
                todo!("Define handling for Keyword and Symbol tokens")
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> ParseResult<Vec<Item>> {
        let tokens = crate::token::tokenize(text).expect("tokenization failed");
        let mut parser = Parser::new(text, tokens);
        while !parser.is_eof() {
            parser.parse_expr()?;
        }
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

    #[test]
    fn parse_binary_op_exprs() {
        let input = "1 + 2 * 3";
        let items = parse(input).expect("parse failed");
        assert_eq!(items.len(), 6);

        assert_eq!(items[0].kind, ItemKind::BinaryOpExprs);
        assert_eq!(items[0].text(input), input);

        assert_eq!(items[1].kind, ItemKind::Integer);
        assert_eq!(items[1].text(input), "1");

        assert_eq!(items[2].kind, ItemKind::BinaryOp);
        assert_eq!(items[2].text(input), "+");

        assert_eq!(items[3].kind, ItemKind::Integer);
        assert_eq!(items[3].text(input), "2");

        assert_eq!(items[4].kind, ItemKind::BinaryOp);
        assert_eq!(items[4].text(input), "*");

        assert_eq!(items[5].kind, ItemKind::Integer);
        assert_eq!(items[5].text(input), "3");

        let view = crate::item::BinaryOpExprsView::new(&items).expect("bug");
        assert_eq!(view.count(), 5);
    }
}
