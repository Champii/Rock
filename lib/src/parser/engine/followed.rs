use super::{parser_trait::Parser, IResult, Input};

pub struct Followed<Parser1, Parser2> {
    parser: Parser1,
    next: Parser2,
}

impl<Parser1, Parser2> Followed<Parser1, Parser2> {
    pub fn new(parser: Parser1, next: Parser2) -> Self {
        Self { parser, next }
    }
}

impl<O1, O2, Parser1, Parser2> Parser for Followed<Parser1, Parser2>
where
    Parser1: Parser<Output = O1>,
    Parser2: Parser<Output = O2>,
{
    type Output = Parser1::Output;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let (tokens, res) = match self.parser.process(tokens) {
            Ok(result) => result,
            Err(e) => {
                super::track_error(&e);
                return Err(e);
            }
        };

        let (tokens, _) = match self.next.process(tokens) {
            Ok(result) => result,
            Err(e) => {
                super::track_error(&e);
                return Err(e);
            }
        };

        Ok((tokens, res))
    }
}

pub fn followed<Parser1, Parser2>(parser: Parser1, next: Parser2) -> Followed<Parser1, Parser2> {
    Followed::new(parser, next)
}
