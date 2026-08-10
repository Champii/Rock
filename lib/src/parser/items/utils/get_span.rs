use crate::{lexer::Span, parser::engine::*};

pub fn get_span(stream: Input) -> IResult<Span> {
    let span = stream.seek()?.span.clone();

    Ok((stream, span))
}
