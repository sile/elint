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

#[derive(Debug)]
pub struct Module;
