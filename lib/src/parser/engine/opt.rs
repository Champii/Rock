use super::{parser_trait::Parser, IResult, Input};

pub struct Opt<Parser> {
    parser: Parser,
}

impl<Parser> Opt<Parser> {
    pub fn new(parser: Parser) -> Self {
        Self { parser }
    }
}

impl<O, P> Parser for Opt<P>
where
    P: Parser<Output = O>,
{
    type Output = Option<O>;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        if let Ok((tokens, output)) = self.parser.process(tokens) {
            Ok((tokens, Some(output)))
        } else {
            Ok((tokens, None))
        }
    }
}

pub fn opt<P>(parser: P) -> Opt<P> {
    Opt { parser }
}
