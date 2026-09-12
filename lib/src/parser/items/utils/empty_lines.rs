use crate::{
    lexer::TokenType,
    parser::{engine::*, indent_token},
};

pub fn empty_lines(stream: Input) -> IResult<usize> {
    many((indent_token, TokenType::Eol))
        .map(|x| x.len())
        .process(stream)
}

/// Like empty_lines, but also accepts standalone Eol tokens (e.g., from stripped comments)
pub fn empty_lines_permissive(stream: Input) -> IResult<usize> {
    many(
        (indent_token, TokenType::Eol)
            .map(|_| ())
            .or(TokenType::Eol.map(|_| ())),
    )
    .map(|x| x.len())
    .process(stream)
}
