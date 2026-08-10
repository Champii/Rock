use crate::parser::{engine::*, items::primitives, Literal, LiteralKind};

use super::{array, repeat_array, utils::get_span};

pub fn literal(stream: Input) -> IResult<Literal> {
    let (stream, span) = get_span(stream)?;

    primitives::boolean
        .map(LiteralKind::Bool)
        .or(primitives::int.map(LiteralKind::Number))
        .or(primitives::float.map(LiteralKind::Float))
        .or(repeat_array.map(|(value, len)| LiteralKind::ArrayRepeat {
            value: Box::new(value),
            len,
        }))
        .or(array.map(LiteralKind::Array))
        .or(primitives::string.map(LiteralKind::String))
        .or(primitives::char.map(LiteralKind::Char))
        .map(|kind| Literal {
            kind,
            span: span.clone(),
        })
        .process(stream)
}
