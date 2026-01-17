pub mod command_parse;
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

pub type CheckResult<T = ()> = Result<T, CheckError>;
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

    pub fn item_views(&self) -> impl Iterator<Item = item::ItemView> {
        (0..self.items.len()).map(|i| item::ItemView::new(&self.items, i))
    }
}
