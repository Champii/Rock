use crate::parser::{
    engine::*, separated1, IdentOrType, IdentifierPath, Path, TokenType, TypePath,
};

use super::{ident, parse_type, parse_type_path_head};

fn path_type(stream: Input) -> IResult<crate::ast::ParseType> {
    (TokenType::OpenParen, TokenType::CloseParen)
        .map(|_| crate::ast::ParseType::Tuple(vec![]))
        .or((TokenType::OpenParen, parse_type, TokenType::CloseParen).map(|(_, ty, _)| ty))
        .or(parse_type_path_head)
        .process(stream)
}

fn is_named_path_segment(segment: &IdentOrType) -> bool {
    matches!(
        segment,
        IdentOrType::Ident(_)
            | IdentOrType::Type(crate::ast::ParseType::Type(_))
            | IdentOrType::Type(crate::ast::ParseType::Application(_))
    )
}

pub fn path(stream: Input) -> IResult<Path> {
    ident_path
        .map(Path::Ident)
        .or(type_path.map(Path::Type))
        .process(stream)
}

pub fn ident_path(stream: Input) -> IResult<IdentifierPath> {
    let (stream, path) = separated1(ident_or_type, TokenType::DoubleColon)
        .map(|idents| IdentifierPath { path: idents })
        .process(stream)?;

    if path.path.len() > 1 && !path.path.iter().all(is_named_path_segment) {
        return Err(ParseError::Fail);
    }

    if let Some(last) = path.path.last() {
        if let IdentOrType::Ident(_) = last {
            return Ok((stream, path));
        } else {
            return Err(ParseError::Fail);
        }
    }

    Err(ParseError::Fail)
}

// the same, but finished with a type
pub fn type_path(stream: Input) -> IResult<TypePath> {
    let (stream, path) = separated1(ident_or_type, TokenType::DoubleColon)
        .map(|idents| TypePath { path: idents })
        .process(stream)?;

    if path.path.len() > 1 && !path.path.iter().all(is_named_path_segment) {
        return Err(ParseError::Fail);
    }

    if let Some(last) = path.path.last() {
        if let IdentOrType::Type(_) = last {
            return Ok((stream, path));
        } else {
            return Err(ParseError::Fail);
        }
    }

    Err(ParseError::Fail)
}

pub fn ident_or_type(stream: Input) -> IResult<IdentOrType> {
    ident
        .map(IdentOrType::Ident)
        .or(path_type.map(IdentOrType::Type))
        .process(stream)
}
