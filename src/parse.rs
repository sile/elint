use crate::item::{Item, ItemKind, Span};
use crate::token::{Token, TokenKind};
use std::backtrace::Backtrace;

#[derive(Debug)]
pub struct ParseError {
    pub span: Span,
    pub reason: String,
    pub backtrace: Backtrace,
}

impl ParseError {
    pub fn new<T>(span: Span, reason: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            span,
            reason: reason.into(),
            backtrace: Backtrace::force_capture(), // TODO
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse Error: {} ({:?})\nBacktrace:\n{}",
            self.reason, self.span, self.backtrace
        )
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
            comments,
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

    fn next_empty_span(&self) -> Span {
        let start = self.next_span().start;
        Span::new(start, start)
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

    pub fn parse_module(&mut self) -> ParseResult {
        while !self.is_eof() {
            if self.is_next_symbol("-") {
                self.parse_attr()?;
            } else {
                self.parse_fun_decl()?;
            }
        }
        let span = Span::new(0, self.text.len());
        self.insert_item(0, ItemKind::Module, span);
        Ok(())
    }

    pub fn parse_attr(&mut self) -> ParseResult {
        let (_i, start) = self.span_start();
        self.expect_symbol("-")?;
        while !self.expect_optional_symbol(".") {
            self.next_token()?;
        }
        let span = self.span_finish(start);
        self.push_item(ItemKind::Attr, span);
        Ok(())
    }

    pub fn parse_fun_decl(&mut self) -> ParseResult {
        let (i, start) = self.span_start();
        self.parse_fun_clause()?;
        while self.is_next_symbol(";") {
            self.expect_symbol(";")?;
            self.parse_fun_clause()?;
        }
        let end = self.expect_symbol(".")?.end;

        let span = Span::new(start, end);
        self.insert_item(i, ItemKind::FunDecl, span);
        Ok(())
    }

    fn span_start(&self) -> (usize, usize) {
        (self.items.len(), self.next_span().start)
    }

    fn span_finish(&self, start: usize) -> Span {
        Span::new(start, self.last_span().end)
    }

    pub fn parse_body(&mut self) -> ParseResult<()> {
        let (i, start) = self.span_start();

        self.parse_expr()?;
        while self.is_next_symbol(",") {
            let _ = self.next_token();
            self.parse_expr()?;
        }

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::Body, span);
        Ok(())
    }

    pub fn parse_guard(&mut self) -> ParseResult {
        let (i, start) = self.span_start();
        if !self.expect_optional_keyword("when") {
            self.push_item(ItemKind::Guard, self.next_empty_span());
            return Ok(());
        }

        // TODO
        self.parse_body()?;

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::Guard, span);
        Ok(())
    }

    pub fn parse_fun_clause(&mut self) -> ParseResult<()> {
        let i = self.items.len();
        let start = self.parse_atom()?.start;
        self.parse_args()?;
        self.parse_guard()?;
        self.expect_symbol("->")?;
        self.parse_body()?;
        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::FunClause, span);
        Ok(())
    }

    pub fn parse_atom(&mut self) -> ParseResult<Span> {
        let t = self.next_token()?;
        if t.kind != TokenKind::Atom {
            return Err(ParseError::new(t.span, "not an atom token"));
        }
        self.push_item(ItemKind::Atom, t.span);
        Ok(t.span)
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

    fn insert_item2(&mut self, i: usize, kind: ItemKind) {
        let start = self.items[i].span.start;
        let span = self.span_finish(start);
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

    // TODO: rename
    fn parse_expr_item(&mut self) -> ParseResult<()> {
        let i = self.items.len();
        self.parse_base_expr_item()?;
        if self.is_next_symbol(":") {
            self.parse_module_fun_call(i)
        } else if self.is_next_symbol("(") {
            self.parse_fun_call(i)
        } else if self.is_next_symbol("=") {
            self.parse_match(i)
        } else if self.is_next_symbol("?=") {
            self.parse_maybe_match(i)
        } else {
            Ok(())
        }
    }

    fn parse_args(&mut self) -> ParseResult<Span> {
        let (i, start) = self.span_start();
        self.expect_symbol("(")?;
        if !self.is_next_symbol(")") {
            loop {
                self.parse_expr()?;
                if !self.is_next_symbol(",") {
                    break;
                }
                let _ = self.next_token()?;
            }
        }
        let last = self.expect_symbol(")")?;

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::Args, span);
        Ok(last)
    }

    fn parse_binary(&mut self) -> ParseResult {
        let (_i, start) = self.span_start();
        self.expect_symbol("<<")?;
        let mut level = 1; // TODO
        while level > 0 {
            if self.is_next_symbol("<<") {
                level += 1;
            } else if self.is_next_symbol(">>") {
                level -= 1;
            }
            self.next_token()?;
        }

        let span = self.span_finish(start);
        self.push_item(ItemKind::Binary, span);
        Ok(())
    }

    fn parse_tuple<F>(&mut self, f: F) -> ParseResult
    where
        F: Fn(&mut Self) -> ParseResult,
    {
        let (i, start) = self.span_start();
        self.expect_symbol("{")?;
        while !self.is_next_symbol("}") {
            f(self)?;
            if !self.expect_optional_symbol(",") {
                break;
            }
        }
        self.expect_symbol("}")?;

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::Tuple, span);
        Ok(())
    }

    fn parse_module_fun_call(&mut self, module_item_i: usize) -> ParseResult {
        let _ = self.next_token()?; // ':'
        self.parse_base_expr_item()?; // function name
        let last = self.parse_args()?; // (...)

        let start = self.items[module_item_i].span.start;
        let span = Span::new(start, last.end);
        self.insert_item(module_item_i, ItemKind::ModuleFunCall, span);
        Ok(())
    }

    fn parse_fun_call(&mut self, i: usize) -> ParseResult {
        self.parse_args()?; // (...)
        self.insert_item2(i, ItemKind::FunCall);
        Ok(())
    }

    fn parse_maybe_match(&mut self, i: usize) -> ParseResult {
        let _ = self.next_token()?; // '?='
        self.parse_expr()?;
        self.insert_item2(i, ItemKind::MaybeMatch);
        Ok(())
    }

    fn parse_match(&mut self, i: usize) -> ParseResult {
        let _ = self.next_token()?; // '='
        self.parse_expr()?;
        self.insert_item2(i, ItemKind::Match);
        Ok(())
    }

    fn expect_optional_symbol(&mut self, name: &str) -> bool {
        if self.is_next_symbol(name) {
            self.token_i += 1;
            true
        } else {
            false
        }
    }

    fn expect_optional_keyword(&mut self, name: &str) -> bool {
        if self.is_next_keyword(name) {
            self.token_i += 1;
            true
        } else {
            false
        }
    }

    fn expect_symbol(&mut self, name: &str) -> ParseResult<Span> {
        let t = self.next_token()?;
        if t.kind == TokenKind::Symbol && t.text(self.text) == name {
            Ok(t.span)
        } else {
            Err(ParseError::new(
                t.span,
                format!("expected symbol '{}', got '{}'", name, t.text(self.text)),
            ))
        }
    }

    fn expect_keyword(&mut self, name: &str) -> ParseResult<Span> {
        let t = self.next_token()?;
        if t.kind == TokenKind::Keyword && t.text(self.text) == name {
            Ok(t.span)
        } else {
            Err(ParseError::new(
                t.span,
                format!("expected keyword '{}', got '{}'", name, t.text(self.text)),
            ))
        }
    }

    fn is_next_symbol(&self, name: &str) -> bool {
        self.peek_token()
            .is_some_and(|t| t.kind == TokenKind::Symbol && t.text(self.text) == name)
    }

    fn is_next_keyword(&self, name: &str) -> bool {
        self.peek_token()
            .is_some_and(|t| t.kind == TokenKind::Keyword && t.text(self.text) == name)
    }

    fn parse_maybe_expr(&mut self) -> ParseResult {
        let (i, start) = self.span_start();
        self.expect_keyword("maybe")?;
        self.parse_body()?;
        if !self.is_next_keyword("end") {
            self.expect_keyword("else")?;
            self.parse_else_clauses()?;
        }
        self.expect_keyword("end")?;

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::MaybeExpr, span);
        Ok(())
    }

    fn parse_else_clauses(&mut self) -> ParseResult<()> {
        let (i, start) = self.span_start();
        loop {
            self.parse_else_clause()?;
            if !self.is_next_symbol(";") {
                break;
            }
            let _ = self.next_token()?; // consume ';'
        }
        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::Clauses, span);
        Ok(())
    }

    fn parse_else_clause(&mut self) -> ParseResult<()> {
        let (i, start) = self.span_start();

        self.parse_pattern()?; // pattern
        // TODO: guard (optional)
        self.push_item(ItemKind::Guard, self.next_empty_span());
        self.expect_symbol("->")?;
        self.parse_body()?; // clause body

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::ElseClause, span);
        Ok(())
    }

    fn parse_case(&mut self) -> ParseResult {
        let (i, start) = self.span_start();
        self.expect_keyword("case")?;
        self.parse_expr()?;
        self.expect_keyword("of")?;
        self.parse_case_clauses()?;
        self.expect_keyword("end")?;

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::Case, span);
        Ok(())
    }

    fn parse_case_clauses(&mut self) -> ParseResult<()> {
        let (i, start) = self.span_start();
        loop {
            self.parse_case_clause()?;
            if !self.is_next_symbol(";") {
                break;
            }
            let _ = self.next_token()?; // consume ';'
        }
        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::CaseClauses, span);
        Ok(())
    }

    fn parse_pattern(&mut self) -> ParseResult {
        self.parse_expr() // TODO
    }

    fn parse_case_clause(&mut self) -> ParseResult<()> {
        let (i, start) = self.span_start();

        self.parse_pattern()?; // pattern
        // TODO: guard (optional)
        self.push_item(ItemKind::Guard, self.next_empty_span());
        self.expect_symbol("->")?;
        self.parse_body()?; // clause body

        let span = self.span_finish(start);
        self.insert_item(i, ItemKind::CaseClause, span);
        Ok(())
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
            TokenKind::Keyword => {
                self.token_i -= 1;
                match t.text(self.text) {
                    "case" => self.parse_case()?,
                    "maybe" => self.parse_maybe_expr()?,
                    t => return Err(ParseError::new(self.next_span(), format!("TODO: {t:?}"))),
                }
            }
            TokenKind::Symbol => {
                self.token_i -= 1;
                match t.text(self.text) {
                    "{" => self.parse_tuple(|p| p.parse_expr())?,
                    "<<" => self.parse_binary()?,
                    t => return Err(ParseError::new(self.next_span(), format!("TODO: {t:?}"))),
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::item::ItemView;

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

        let view = crate::item::BinaryOpExprsView::new(ItemView::new(&items, 0)).expect("bug");
        assert_eq!(view.exprs().count(), 3);
        assert_eq!(view.ops().count(), 2);
    }

    #[test]
    fn parse_module_fun_call() {
        let input = "foo:bar()";
        let items = parse(input).expect("parse failed");

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].kind, ItemKind::ModuleFunCall);
        assert_eq!(items[0].text(input), "foo:bar()");
        assert_eq!(items[1].kind, ItemKind::Atom);
        assert_eq!(items[1].text(input), "foo");
        assert_eq!(items[2].kind, ItemKind::Atom);
        assert_eq!(items[2].text(input), "bar");
        assert_eq!(items[3].kind, ItemKind::Args);
        assert_eq!(items[3].text(input), "()");

        let view = crate::item::ModuleFunCallView::new(ItemView::new(&items, 0)).expect("bug");
        assert_eq!(view.module_name().kind(), ItemKind::Atom);
        assert_eq!(view.module_name().text(input), "foo");
        assert_eq!(view.function_name().kind(), ItemKind::Atom);
        assert_eq!(view.function_name().text(input), "bar");
        assert_eq!(view.args().count(), 0);

        let input = "foo:bar(42, X)";
        let items = parse(input).expect("parse failed");
        let view = crate::item::ModuleFunCallView::new(ItemView::new(&items, 0)).expect("bug");
        assert_eq!(view.module_name().text(input), "foo");
        assert_eq!(view.function_name().text(input), "bar");

        let args: Vec<_> = view.args().collect();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].kind(), ItemKind::Integer);
        assert_eq!(args[0].text(input), "42");
        assert_eq!(args[1].kind(), ItemKind::Variable);
        assert_eq!(args[1].text(input), "X");
    }

    #[test]
    fn parse_fun_decl() {
        let input = "foo(X) -> X.";
        let tokens = crate::token::tokenize(input).expect("tokenization failed");
        let mut parser = Parser::new(input, tokens);
        parser.parse_fun_decl().expect("parse failed");

        let items = &parser.items;
        assert!(!items.is_empty());
        assert_eq!(items[0].kind, ItemKind::FunDecl);
        assert_eq!(items[0].text(input), input);

        let view = crate::item::FunDeclView::new(ItemView::new(items, 0)).expect("bug");
        let clauses: Vec<_> = view.clauses().collect();
        assert_eq!(clauses.len(), 1);

        assert_eq!(clauses[0].name().text(input), "foo");
        assert_eq!(clauses[0].args().count(), 1);
        assert_eq!(clauses[0].body().count(), 1);
    }

    #[test]
    fn parse_case_expr() {
        let input = "case X of 42 -> ok; _ -> error end";
        let items = parse(input).expect("parse failed");

        assert!(!items.is_empty());
        assert_eq!(items[0].kind, ItemKind::Case);
        assert_eq!(items[0].text(input), input);

        let view = crate::item::CaseView::new(ItemView::new(&items, 0)).expect("bug");

        // Check the expression being matched
        assert_eq!(view.expr().kind(), ItemKind::Variable);
        assert_eq!(view.expr().text(input), "X");

        // Check the case clauses
        let clauses: Vec<_> = view.clauses().collect();
        assert_eq!(clauses.len(), 2);

        // First clause: 42 -> ok
        assert_eq!(clauses[0].pattern().kind(), ItemKind::Integer);
        assert_eq!(clauses[0].pattern().text(input), "42");
        assert_eq!(clauses[0].body().count(), 1);

        // Second clause: _ -> error
        assert_eq!(clauses[1].pattern().kind(), ItemKind::Variable);
        assert_eq!(clauses[1].pattern().text(input), "_");
        assert_eq!(clauses[1].body().count(), 1);
    }

    #[test]
    fn parse_maybe_expr() {
        let input = "maybe X else _ -> error end";
        let items = parse(input).expect("parse failed");

        assert!(!items.is_empty());
        assert_eq!(items[0].kind, ItemKind::MaybeExpr);
        assert_eq!(items[0].text(input), input);

        let view = crate::item::MaybeExprView::new(ItemView::new(&items, 0)).expect("bug");

        // Check the body
        let body_items: Vec<_> = view.body().collect();
        assert_eq!(body_items.len(), 1);
        assert_eq!(body_items[0].kind(), ItemKind::Variable);
        assert_eq!(body_items[0].text(input), "X");

        // Check the else clauses
        let clauses: Vec<_> = view.clauses().collect();
        assert_eq!(clauses.len(), 1);

        // First clause: _ -> error
        assert_eq!(clauses[0].pattern().kind(), ItemKind::Variable);
        assert_eq!(clauses[0].pattern().text(input), "_");
        assert_eq!(clauses[0].body().count(), 1);
    }

    #[test]
    fn parse_tuple() {
        let input = "{42, X, ok}";
        let items = parse(input).expect("parse failed");

        assert!(!items.is_empty());
        assert_eq!(items[0].kind, ItemKind::Tuple);
        assert_eq!(items[0].text(input), input);

        let view = crate::item::ItemView::new(&items, 0);
        let tuple_items: Vec<_> = view.children().collect();
        assert_eq!(tuple_items.len(), 3);

        assert_eq!(tuple_items[0].kind(), ItemKind::Integer);
        assert_eq!(tuple_items[0].text(input), "42");

        assert_eq!(tuple_items[1].kind(), ItemKind::Variable);
        assert_eq!(tuple_items[1].text(input), "X");

        assert_eq!(tuple_items[2].kind(), ItemKind::Atom);
        assert_eq!(tuple_items[2].text(input), "ok");
    }
}
