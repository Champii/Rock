use crate::parser::engine::*;

pub struct Indented<P> {
    parser: P,
}

impl<O, P> Parser for Indented<P>
where
    P: Parser<Output = O>,
{
    type Output = O;

    fn process<'a>(&mut self, stream: Input<'a>) -> IResult<'a, Self::Output> {
        stream.with_indent(|stream| self.parser.process(stream))
    }
}

pub fn indented<P>(parser: P) -> Indented<P> {
    Indented { parser }
}
