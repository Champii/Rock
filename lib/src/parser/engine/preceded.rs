use super::{parser_trait::Parser, IResult, Input};

pub struct Preceded<Parser1, Parser2> {
    precedent: Parser1,
    parser: Parser2,
}

impl<Parser1, Parser2> Preceded<Parser1, Parser2> {
    pub fn new(precedent: Parser1, parser: Parser2) -> Self {
        Self { precedent, parser }
    }
}

impl<O1, O2, Parser1, Parser2> Parser for Preceded<Parser1, Parser2>
where
    Parser1: Parser<Output = O1>,
    Parser2: Parser<Output = O2>,
{
    type Output = Parser2::Output;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let (tokens, _) = match self.precedent.process(tokens) {
            Ok(result) => result,
            Err(e) => {
                super::track_error(&e);
                return Err(e);
            }
        };
        match self.parser.process(tokens) {
            Ok(result) => Ok(result),
            Err(e) => {
                super::track_error(&e);
                Err(e)
            }
        }
    }
}

pub fn preceded<Parser1, Parser2>(
    precedent: Parser1,
    parser: Parser2,
) -> Preceded<Parser1, Parser2> {
    Preceded::new(precedent, parser)
}
