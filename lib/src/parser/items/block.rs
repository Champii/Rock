use crate::{
    lexer::TokenType,
    parser::{engine::*, Block},
};

use super::{disallow_multiline_operators, empty_lines, indent, statement};

pub fn block(stream: Input) -> IResult<Block> {
    preceded(
        TokenType::Eol,
        indented(separated1(
            preceded(indent, statement),
            TokenType::Eol.followed_by(empty_lines),
        )),
    )
    .or(disallow_multiline_operators(statement).map(|statement| vec![statement]))
    .map(|statements| Block { statements })
    .process(stream)
    .map_err(|e| e.with_context("block"))
}
