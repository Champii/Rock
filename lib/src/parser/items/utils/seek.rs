use crate::parser::engine::*;

/* // Execute parser F if token type is found but does not consume it
pub fn seek<'a, T, F>(token_type: TokenType, parser: F) -> impl Fn(Input<'a>) -> IResult<'a, T>
where
    F: Fn(Input<'a>) -> IResult<'a, T>,
{
    move |stream| {
        let (_, token) = stream.consume()?;

        if token.token_type == token_type {
            parser(stream)
        } else {
            Err(ParseError::UnexpectedToken(
                token_type.discriminant().to_string(),
                token.clone(),
            ))
        }
    }
} */

// Execute parser F but return the original stream
pub fn seek<T, F>(mut parser: F) -> impl FnMut(Input<'_>) -> IResult<'_, T>
where
    F: Parser<Output = T>,
{
    move |stream| {
        let (_, res) = parser.process(stream)?;

        Ok((stream, res))
    }
}
