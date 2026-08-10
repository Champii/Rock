use super::{parse_error::ParseError, parser_trait::Parser, IResult, Input};

pub struct Or<Parser1, Parser2> {
    parser1: Parser1,
    parser2: Parser2,
}

impl<Parser1, Parser2> Or<Parser1, Parser2> {
    pub fn new(parser1: Parser1, parser2: Parser2) -> Self {
        Self { parser1, parser2 }
    }
}

impl<O, Parser1, Parser2> Parser for Or<Parser1, Parser2>
where
    Parser1: Parser<Output = O>,
    Parser2: Parser<Output = O>,
{
    type Output = Parser1::Output;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        match self.parser1.process(tokens) {
            Ok((tokens, output)) => Ok((tokens, output)),
            Err(ParseError::HardError(msg, span)) => Err(ParseError::HardError(msg, span)),
            Err(err1) => {
                // Track the error from the first parser
                super::track_error(&err1);

                match self.parser2.process(tokens) {
                    Ok((tokens, output)) => Ok((tokens, output)),
                    Err(err2) => {
                        // Track the error from the second parser
                        super::track_error(&err2);

                        // Return the better of the two errors
                        Err(err1.choose_better(err2))
                    }
                }
            }
        }
    }
}
