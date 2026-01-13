use erl_tokenize::PositionRange;

use crate::item::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn text(self, full_text: &str) -> &str {
        self.span.text(full_text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Atom,
    Variable,
    Integer,
    Float,
    Char,
    String,
    SigilString,
    Keyword,
    Symbol,
    Comment,
}

pub fn tokenize(text: &str) -> Result<Vec<Token>, erl_tokenize::Error> {
    let mut tokens = Vec::new();
    let tokenizer = erl_tokenize::Tokenizer::new(text);

    for token_result in tokenizer {
        let erl_token = token_result?;
        let span = Span {
            start: erl_token.start_position().offset(),
            end: erl_token.end_position().offset(),
        };

        let kind = match erl_token {
            erl_tokenize::Token::Atom(_) => TokenKind::Atom,
            erl_tokenize::Token::Variable(_) => TokenKind::Variable,
            erl_tokenize::Token::Integer(_) => TokenKind::Integer,
            erl_tokenize::Token::Float(_) => TokenKind::Float,
            erl_tokenize::Token::Char(_) => TokenKind::Char,
            erl_tokenize::Token::String(_) => TokenKind::String,
            erl_tokenize::Token::SigilString(_) => TokenKind::SigilString,
            erl_tokenize::Token::Keyword(_) => TokenKind::Keyword,
            erl_tokenize::Token::Symbol(_) => TokenKind::Symbol,
            erl_tokenize::Token::Comment(_) => TokenKind::Comment,
            erl_tokenize::Token::Whitespace(_) => continue,
        };

        tokens.push(Token { kind, span });
    }

    Ok(tokens)
}
