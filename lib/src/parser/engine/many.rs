use super::{parser_trait::Parser, IResult, Input, ParseError};

pub struct Many<P> {
    parser: P,
    at_least_one_result: bool,
}

impl<P> Parser for Many<P>
where
    P: Parser,
{
    type Output = Vec<P::Output>;

    fn process<'a>(&mut self, mut tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let mut output = Vec::new();
        let mut deepest_error: Option<ParseError> = None;

        loop {
            if tokens.is_empty() {
                break;
            }

            match self.parser.process(tokens) {
                Ok((new_tokens, t)) => {
                    tokens = new_tokens;
                    output.push(t);
                }
                Err(err) => {
                    // Track the deepest error we've encountered
                    deepest_error = Some(match deepest_error {
                        Some(prev_err) => prev_err.choose_better(err),
                        None => err,
                    });
                    break;
                }
            }
        }

        if self.at_least_one_result && output.is_empty() {
            // If we have a deepest error, use it; otherwise use ExpectedOneOrMore
            let span = tokens
                .seek()
                .map(|token| token.span)
                .unwrap_or_else(|_| tokens.eof_span());
            return Err(deepest_error.unwrap_or(ParseError::ExpectedOneOrMore(span)));
        }

        Ok((tokens, output))
    }
}

pub fn many<'a, T: Parser>(parser: T) -> Many<T> {
    Many {
        parser,
        at_least_one_result: false,
    }
}

pub fn many1<'a, T: Parser>(parser: T) -> Many<T> {
    Many {
        parser,
        at_least_one_result: true,
    }
}
