pub mod ast_expr;

pub fn parse_code(text: &str) -> Result<Module, ParseError> {
    let lexer = erl_tokenize::Lexer::new(text);

    let tokens: Result<Vec<_>, _> = lexer
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ParseError::Tokenize(e));

    let _tokens = tokens?;

    // TODO: Parse tokens into AST
    todo!()
}

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
pub struct Module;
