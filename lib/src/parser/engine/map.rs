use super::{parser_trait::Parser, IResult, Input};

pub struct Map<Parser, F> {
    parser: Parser,
    f: F,
}

impl<Parser, F> Map<Parser, F> {
    pub fn new(parser: Parser, f: F) -> Self {
        Self { parser, f }
    }
}

impl<O, P, F, T> Parser for Map<P, F>
where
    P: Parser<Output = O>,
    F: Fn(O) -> T,
{
    type Output = T;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let (tokens, output) = match self.parser.process(tokens) {
            Ok(result) => result,
            Err(e) => {
                super::track_error(&e);
                return Err(e);
            }
        };
        Ok((tokens, (self.f)(output)))
    }
}
