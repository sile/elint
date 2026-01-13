#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExprKind {
    Variable,
    Atom,
    Integer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Position {
    pub start: usize,
    pub end: usize,
}
