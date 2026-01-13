use crate::item::Item;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseContext {
    Expr,
}

#[derive(Debug)]
pub struct Parser {
    pub ctx: ParseContext,
    pub items: Vec<Item>,
}
