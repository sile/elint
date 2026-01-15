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
pub struct ItemView2<'a> {
    items: &'a [Item],
    i: usize,
}

impl<'a> ItemView2<'a> {
    pub fn new(items: &'a [Item], i: usize) -> Self {
        Self { items, i }
    }

    pub fn kind(&self) -> ItemKind {
        self.items[self.i].kind
    }

    pub fn span(&self) -> Span {
        self.items[self.i].span
    }

    pub fn items(&self) -> &'a [Item] {
        self.span().items(&self.items[self.i..])
    }

    pub fn parent(&self) -> Option<Self> {
        todo!()
    }

    // ancestors, next_siblings, prev_siblings, children
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

    pub fn children(&self) -> ItemsView<'a> {
        ItemsView::new(self.children)
    }

    pub fn siblings(&self) -> ItemsView<'a> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct ItemsView<'a> {
    items: &'a [Item],
}

impl<'a> ItemsView<'a> {
    pub fn new(items: &'a [Item]) -> Self {
        Self { items }
    }
}

impl<'a> Iterator for ItemsView<'a> {
    type Item = ItemView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let head = self.items.first()?;
        let head_items = head.span.items(self.items);
        self.items = &self.items[head_items.len()..];
        ItemView::new(head_items)
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

    pub fn module_name(&self) -> ItemView<'_> {
        ItemView::new(self.children).expect("bug")
    }

    pub fn function_name(&self) -> ItemView<'_> {
        self.module_name().siblings().next().expect("bug")
    }

    pub fn args(&self) -> ItemsView<'_> {
        self.function_name().siblings()
    }
}
