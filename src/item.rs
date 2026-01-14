use crate::parse::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Item {
    pub ctx: Context,
    pub kind: ItemKind,
    pub span: Span,
}

impl Item {
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
}

#[derive(Debug, Clone)]
pub struct BinaryOpExprsView<'a> {
    children: &'a [Item],
}

impl<'a> BinaryOpExprsView<'a> {
    pub fn new(items: &'a [Item]) -> Option<Self> {
        let t = items.first()?;
        if t.kind != ItemKind::BinaryOpExprs {
            return None;
        }

        let children = &t.span.items(items)[1..];
        Some(Self { children })
    }
}

impl<'a> Iterator for BinaryOpExprsView<'a> {
    type Item = &'a [Item];

    fn next(&mut self) -> Option<Self::Item> {
        let t = self.children.first()?;
        let child = t.span.items(self.children);
        self.children = &self.children[child.len()..];
        Some(child)
    }
}
