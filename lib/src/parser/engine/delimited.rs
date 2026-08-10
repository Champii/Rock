use super::{parser_trait::Parser, IResult, Input};

pub struct Delimited<P, D1, D2> {
    delimiter1: D1,
    parser: P,
    delimiter2: D2,
}

impl<P, D1, D2> Parser for Delimited<P, D1, D2>
where
    P: Parser,
    D1: Parser,
    D2: Parser,
{
    type Output = P::Output;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let (tokens, _) = self.delimiter1.process(tokens)?;
        let (tokens, result) = self.parser.process(tokens)?;
        let (tokens, _) = self.delimiter2.process(tokens)?;

        Ok((tokens, result))
    }
}

pub fn delimited<P, D1, D2>(delimiter1: D1, parser: P, delimiter2: D2) -> Delimited<P, D1, D2> {
    Delimited {
        delimiter1,
        parser,
        delimiter2,
    }
}
