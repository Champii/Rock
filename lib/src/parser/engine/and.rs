use super::{parser_trait::Parser, IResult, Input};

pub struct And<Parser1, Parser2> {
    parser1: Parser1,
    parser2: Parser2,
}

impl<Parser1, Parser2> And<Parser1, Parser2> {
    pub fn new(parser1: Parser1, parser2: Parser2) -> Self {
        Self { parser1, parser2 }
    }
}

impl<O, Parser1, Parser2, T> Parser for And<Parser1, Parser2>
where
    Parser1: Parser<Output = O>,
    Parser2: Parser<Output = T>,
{
    type Output = (O, T);

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let (tokens, output1) = match self.parser1.process(tokens) {
            Ok(result) => result,
            Err(e) => {
                super::track_error(&e);
                return Err(e);
            }
        };
        let (tokens, output2) = match self.parser2.process(tokens) {
            Ok(result) => result,
            Err(e) => {
                super::track_error(&e);
                return Err(e);
            }
        };
        Ok((tokens, (output1, output2)))
    }
}
