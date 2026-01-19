pub mod command_parse;
pub mod expect;
pub mod fs;
pub mod item;
pub mod parse;
pub mod rule_element_bif;
pub mod rule_nested_cases;
pub mod token;

pub use item::Span;

#[derive(Debug)]
pub struct Error {
    pub span: item::Span,
    pub message: String,
}

impl Error {
    pub fn new(span: item::Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    pub fn fix_span(mut self, span: item::Span) -> Self {
        self.span = span;
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:?})", self.message, self.span)
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct Ast {
    pub text: String,
    pub items: Vec<item::Item>,
}

impl Ast {
    pub fn root(&self) -> item::ItemView<'_> {
        item::ItemView::new(&self.text, &self.items, 0)
    }

    pub fn text(&self, span: item::Span) -> &str {
        span.text(&self.text)
    }

    pub fn item_views(&self) -> impl Iterator<Item = item::ItemView<'_>> {
        (0..self.items.len()).map(|i| item::ItemView::new(&self.text, &self.items, i))
    }

    pub fn is_atom(&self, t: item::ItemView, name: &str) -> bool {
        t.kind() == item::ItemKind::Atom && self.text(t.span()) == name
    }

    pub fn is_tagged_tuple(&self, t: item::ItemView, tag: &str) -> bool {
        t.kind() == item::ItemKind::Tuple
            && t.children()
                .next()
                .is_some_and(|t| self.text(t.span()) == tag)
    }
}

#[derive(Debug)]
pub struct Rule {
    pub name: &'static str,
    pub text: &'static str,
    pub check: fn(&Ast) -> Vec<Span>,
}

impl Rule {
    pub const fn new(name: &'static str, text: &'static str, check: fn(&Ast) -> Vec<Span>) -> Self {
        Self { name, text, check }
    }
}

pub const RULES: &[Rule] = &[rule_nested_cases::RULE, rule_element_bif::RULE];
