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
    ModuleFunctionCall,
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
}

fn get_span_and_children<'a>(items: &'a [Item], kind: ItemKind) -> Option<(Span, &'a [Item])> {
    let t = items.first().filter(|t| t.kind == kind)?;
    let children = &t.span.items(items)[1..];
    Some((t.span, children))
}

#[derive(Debug, Clone)]
pub struct ItemView<'a> {
    item: Item,
    children: &'a [Item],
}

impl<'a> ItemView<'a> {
    pub fn new(items: &'a [Item]) -> Option<Self> {
        let (&item, children) = items.split_first()?;
        Some(Self { item, children })
    }

    pub fn kind(&self) -> ItemKind {
        self.item.kind
    }

    pub fn span(&self) -> Span {
        self.item.span
    }
}

#[derive(Debug, Clone)]
pub struct ExprsView<'a> {
    span: Span,
    children: &'a [Item],
    position: usize,
}

impl<'a> ExprsView<'a> {
    pub fn new(items: &'a [Item]) -> Option<Self> {
        if items.is_empty() {
            return None;
        }
        let t = items.first()?;
        let span = Span::new(
            t.span.start,
            items.last().map(|i| i.span.end).unwrap_or(t.span.end),
        );
        Some(Self {
            span,
            children: items,
            position: 0,
        })
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

impl<'a> Iterator for ExprsView<'a> {
    type Item = ItemView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.children.len() {
            return None;
        }

        let t = &self.children[self.position];
        let child = t.span.items(self.children);
        self.position += child.len();
        ItemView::new(child)
    }
}

#[derive(Debug, Clone)]
pub struct BinaryOpExprsView<'a> {
    span: Span,
    children: &'a [Item],
}

impl<'a> BinaryOpExprsView<'a> {
    pub fn new(items: &'a [Item]) -> Option<Self> {
        get_span_and_children(items, ItemKind::BinaryOpExprs)
            .map(|(span, children)| Self { span, children })
    }

    pub fn span(&self) -> Span {
        self.span
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

#[derive(Debug, Clone)]
pub struct ModuleFunctionCallView<'a> {
    span: Span,
    children: &'a [Item],
}

impl<'a> ModuleFunctionCallView<'a> {
    pub fn new(items: &'a [Item]) -> Option<Self> {
        get_span_and_children(items, ItemKind::ModuleFunctionCall)
            .map(|(span, children)| Self { span, children })
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn module_name(&self) -> Option<ItemView<'_>> {
        ItemView::new(self.children)
    }

    pub fn function_name(&self) -> Option<ItemView<'_>> {
        let module_name_span = self.module_name()?.span();
        let remaining = &self.children[self
            .children
            .iter()
            .position(|t| t.span.start >= module_name_span.end)?..];
        ItemView::new(remaining)
    }

    pub fn args(&self) -> Option<ExprsView<'_>> {
        let fn_name_span = self.function_name()?.span();
        let remaining = &self.children[self
            .children
            .iter()
            .position(|t| t.span.start >= fn_name_span.end)?..];

        if remaining.is_empty() {
            return None;
        }

        ExprsView::new(remaining)
    }
}
