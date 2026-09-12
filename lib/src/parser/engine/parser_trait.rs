use super::{and::And, map::Map, opt::Opt, or::Or, Followed, IResult, Input, ParseError};

pub trait Parser {
    type Output;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output>;

    fn or<Parser2>(self, parser2: Parser2) -> Or<Self, Parser2>
    where
        Parser2: Parser,
        Self: Sized,
    {
        Or::new(self, parser2)
    }

    fn map<F, T>(self, f: F) -> Map<Self, F>
    where
        F: Fn(Self::Output) -> T,
        Self: Sized,
    {
        Map::new(self, f)
    }

    fn and<Parser2, T>(self, parser2: Parser2) -> And<Self, Parser2>
    where
        Parser2: Parser<Output = T>,
        Self: Sized,
    {
        And::new(self, parser2)
    }

    fn opt(self) -> Opt<Self>
    where
        Self: Sized,
    {
        Opt::new(self)
    }

    fn followed_by<Parser2>(self, next: Parser2) -> Followed<Self, Parser2>
    where
        Self: Sized,
    {
        Followed::new(self, next)
    }

    fn debug(self) -> Map<Self, fn(Self::Output) -> Self::Output>
    where
        Self: Sized,
        <Self as Parser>::Output: std::fmt::Debug,
    {
        Map::new(self, move |output| {
            println!("{:#?}", output);
            output
        })
    }

    fn assert<F>(&mut self, mut f: F) -> impl FnMut(Input) -> IResult<Self::Output>
    where
        F: FnMut(&Self::Output) -> bool,
        Self: Sized,
    {
        move |input| {
            let (rest, output) = self.process(input)?;

            if f(&output) {
                Ok((rest, output))
            } else {
                Err(ParseError::AssertFailed)
            }
        }
    }
}
