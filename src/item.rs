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
}

#[derive(Debug, Clone)]
pub struct BinaryOpExprsView<'a> {
    items: &'a [Item],
    i: usize,
}

impl<'a> BinaryOpExprsView<'a> {
    pub fn new(items: &'a [Item]) -> Option<Self> {
        let t = items.first()?;
        (t.kind == ItemKind::BinaryOpExprs).then_some(Self { items, i: 1 })
    }

    fn span_end(&self) -> usize {
        self.items[0].span.end
    }
}

impl<'a> Iterator for BinaryOpExprsView<'a> {
    type Item = &'a [Item];

    fn next(&mut self) -> Option<Self::Item> {
        let t = self.items.get(self.i)?;
        if self.span_end() <= t.span.start {
            return None;
        }

        todo!()
    }
}
