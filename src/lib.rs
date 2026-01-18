pub mod command_parse;
pub mod fs;
pub mod item;
pub mod parse;
pub mod rule_dont_use_nested_cases;
pub mod token;

#[derive(Debug)]
pub enum ParseError {
    Tokenize(erl_tokenize::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Tokenize(e) => write!(f, "Tokenization error: {}", e),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Tokenize(e) => Some(e),
        }
    }
}

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

pub type CheckResult<T = ()> = Result<T, Error>;
pub type CheckError = ParseError;

#[derive(Debug)]
pub struct Ast {
    pub text: String,
    pub items: Vec<item::Item>,
}

impl Ast {
    pub fn root(&self) -> item::ItemView<'_> {
        item::ItemView::new(&self.items, 0)
    }

    pub fn text(&self, span: item::Span) -> &str {
        span.text(&self.text)
    }

    pub fn item_views(&self) -> impl Iterator<Item = item::ItemView<'_>> {
        (0..self.items.len()).map(|i| item::ItemView::new(&self.items, i))
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
