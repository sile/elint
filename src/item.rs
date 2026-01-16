use crate::parse::{ParseError, ParseResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

impl Item {
    pub const fn new(kind: ItemKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn text(self, full_text: &str) -> &str {
        self.span.text(full_text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Variable,
    Atom,
    Integer,
    Float,
    Char,
    String,
    SigilString,
    Comment,
    BinaryOp,
    BinaryOpExprs,
    ModuleFunCall,
    Args,
    Module,
    FunDecl,
    FunClause,
    Guard,
    Body,
    Case, // TODO: CaseExpr
    CaseClauses,
    CaseClause,
    MaybeExpr,
    Clauses,
    ElseClause,
    Tuple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn text(self, full_text: &str) -> &str {
        &full_text[self.start..self.end]
    }

    pub fn items(self, items: &[Item]) -> &[Item] {
        assert!(!items.is_empty());
        assert_eq!(self, items[0].span);

        let mut n = items
            .binary_search_by_key(&self.end, |t| t.span.start)
            .unwrap_or_else(|i| i);
        n -= items[..n]
            .iter()
            .rev()
            .take_while(|t| t.span.start == self.end)
            .count();
        &items[..n]
    }

    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemView<'a> {
    items: &'a [Item],
    i: usize,
}

impl<'a> ItemView<'a> {
    pub fn new(items: &'a [Item], i: usize) -> Self {
        Self { items, i }
    }

    pub fn kind(&self) -> ItemKind {
        self.items[self.i].kind
    }

    pub fn span(&self) -> Span {
        self.items[self.i].span
    }

    pub fn text<'b>(&self, text: &'b str) -> &'b str {
        self.span().text(text)
    }

    pub fn items(&self) -> &'a [Item] {
        self.span().items(&self.items[self.i..])
    }

    pub fn start_index(&self) -> usize {
        self.i
    }

    pub fn end_index(&self) -> usize {
        self.i + self.items().len()
    }

    pub fn parent(&self) -> Option<Self> {
        let span = self.span();
        for (i, item) in self.items[..self.i].iter().rev().enumerate() {
            if item.span.contains(span) {
                return Some(Self::new(self.items, self.i - i - 1));
            }
        }
        None
    }

    pub fn children(&self) -> ItemsView<'a> {
        let start = self.i + 1;
        let end = self.end_index();
        ItemsView::new(self.items, start, end)
    }

    pub fn siblings(&self) -> ItemsView<'a> {
        let start = self.start_index();
        let end = self.parent().map_or(self.items.len(), |p| p.end_index());
        ItemsView::new(self.items, start, end)
    }

    pub fn expect_kind(&self, kind: ItemKind) -> ParseResult {
        if self.kind() == kind {
            Ok(())
        } else {
            Err(ParseError::new(self.span(), format!("TODO")))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemsView<'a> {
    items: &'a [Item],
    start: usize,
    end: usize,
}

impl<'a> ItemsView<'a> {
    fn new(items: &'a [Item], start: usize, end: usize) -> Self {
        Self { items, start, end }
    }
}

impl<'a> Iterator for ItemsView<'a> {
    type Item = ItemView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.start;
        let next = self.items.get(i).filter(|_| i < self.end)?;
        self.start += next.span.items(&self.items[i..]).len();
        Some(ItemView::new(self.items, i))
    }
}

#[derive(Debug, Clone)]
pub struct BinaryOpExprsView<'a>(ItemsView<'a>);

impl<'a> BinaryOpExprsView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::BinaryOpExprs)?;
        Ok(Self(item.children()))
    }

    pub fn exprs(&self) -> impl Iterator<Item = ItemView<'a>> {
        self.0.clone().step_by(2)
    }

    pub fn ops(&self) -> impl Iterator<Item = ItemView<'a>> {
        self.0.clone().skip(1).step_by(2)
    }
}

#[derive(Debug, Clone)]
pub struct ModuleFunCallView<'a>(ItemsView<'a>);

impl<'a> ModuleFunCallView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::ModuleFunCall)?;
        Ok(Self(item.children()))
    }

    pub fn module_name(&self) -> ItemView<'a> {
        self.0.clone().next().expect("bug")
    }

    pub fn function_name(&self) -> ItemView<'a> {
        self.0.clone().nth(1).expect("bug")
    }

    pub fn args(&self) -> ItemsView<'a> {
        self.0.clone().nth(2).expect("bug").children()
    }
}

#[derive(Debug, Clone)]
pub struct FunDeclView<'a>(ItemsView<'a>);

impl<'a> FunDeclView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::FunDecl)?;
        Ok(Self(item.children()))
    }

    pub fn clauses(&self) -> impl Iterator<Item = FunClauseView<'a>> {
        self.0.clone().map(|t| FunClauseView::new(t).expect("bug"))
    }
}

#[derive(Debug, Clone)]
pub struct FunClauseView<'a>(ItemsView<'a>);

impl<'a> FunClauseView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::FunClause)?;
        Ok(Self(item.children()))
    }

    pub fn name(&self) -> ItemView<'a> {
        self.0.clone().next().expect("bug")
    }

    pub fn args(&self) -> ItemsView<'a> {
        self.0.clone().nth(1).expect("bug").children()
    }

    pub fn guard(&self) -> ItemsView<'a> {
        self.0.clone().nth(2).expect("bug").children()
    }

    pub fn body(&self) -> ItemsView<'a> {
        self.0.clone().nth(3).expect("bug").children()
    }
}

#[derive(Debug, Clone)]
pub struct CaseView<'a>(ItemsView<'a>);

impl<'a> CaseView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::Case)?;
        Ok(Self(item.children()))
    }

    pub fn expr(&self) -> ItemView<'a> {
        self.0.clone().next().expect("bug")
    }

    pub fn clauses(&self) -> impl Iterator<Item = CaseClauseView<'a>> {
        self.0
            .clone()
            .nth(1)
            .expect("bug")
            .children()
            .map(|t| CaseClauseView::new(t).expect("bug"))
    }
}

#[derive(Debug, Clone)]
pub struct CaseClauseView<'a>(ItemsView<'a>);

impl<'a> CaseClauseView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::CaseClause)?;
        Ok(Self(item.children()))
    }

    pub fn pattern(&self) -> ItemView<'a> {
        self.0.clone().next().expect("bug")
    }

    pub fn guard(&self) -> ItemsView<'a> {
        self.0.clone().nth(1).expect("bug").children()
    }

    pub fn body(&self) -> ItemsView<'a> {
        self.0.clone().nth(2).expect("bug").children()
    }
}

#[derive(Debug, Clone)]
pub struct MaybeExprView<'a>(ItemsView<'a>);

impl<'a> MaybeExprView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::MaybeExpr)?;
        Ok(Self(item.children()))
    }

    pub fn body(&self) -> ItemsView<'a> {
        self.0.clone().next().expect("bug").children()
    }

    pub fn clauses(&self) -> impl Iterator<Item = ElseClauseView<'a>> {
        self.0
            .clone()
            .nth(1)
            .expect("bug")
            .children()
            .map(|t| ElseClauseView::new(t).expect("bug"))
    }
}

#[derive(Debug, Clone)]
pub struct ElseClauseView<'a>(ItemsView<'a>);

impl<'a> ElseClauseView<'a> {
    pub fn new(item: ItemView<'a>) -> ParseResult<Self> {
        item.expect_kind(ItemKind::ElseClause)?;
        Ok(Self(item.children()))
    }

    pub fn pattern(&self) -> ItemView<'a> {
        self.0.clone().next().expect("bug")
    }

    pub fn guard(&self) -> ItemsView<'a> {
        self.0.clone().nth(1).expect("bug").children()
    }

    pub fn body(&self) -> ItemsView<'a> {
        self.0.clone().nth(2).expect("bug").children()
    }
}
