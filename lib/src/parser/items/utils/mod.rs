mod consume_tokens_until;
mod debug;
mod empty_lines;
mod get_span;
mod parenthesis;
mod seek;

pub use consume_tokens_until::*;
pub use debug::*;
pub use empty_lines::*;
pub use get_span::*;
pub use parenthesis::*;
pub use seek::*;

#[cfg(test)]
pub use crate::lexer::Token;

#[cfg(test)]
pub fn lex_test(input: &str) -> Vec<Token> {
    use crate::lexer::Lexer;

    let mut tokens = Lexer::new(std::path::PathBuf::new(), input)
        .unwrap()
        .with_newline_at_end(false)
        .collect()
        .unwrap();

    //ignore indent
    tokens.remove(0);

    //ignore EOF
    tokens.pop();

    tokens
}

// Same function, but keep the newline at the end to specifically test toplevel functions
#[cfg(test)]
pub fn lex_test_toplevel(input: &str) -> Vec<Token> {
    use crate::lexer::Lexer;

    let mut tokens = Lexer::new(std::path::PathBuf::new(), input)
        .unwrap()
        .collect()
        .unwrap();

    //ignore EOF
    tokens.pop();

    tokens
}
