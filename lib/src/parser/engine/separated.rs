use super::{parser_trait::Parser, IResult, Input};

pub struct Separated<P, D> {
    parser: P,
    delimiter: D,
    at_least_one_result: bool,
    trailing_delimiter: bool,
}

impl<P, D> Parser for Separated<P, D>
where
    P: Parser,
    D: Parser,
{
    type Output = Vec<P::Output>;

    fn process<'a>(&mut self, tokens: Input<'a>) -> IResult<'a, Self::Output> {
        let mut remaining_tokens = tokens;
        let mut items = Vec::new();
        // let mut diagnostics = Diagnostics::default();

        let mut remaining_tokens_with_delim = tokens;

        loop {
            if remaining_tokens.is_empty() {
                if !self.trailing_delimiter {
                    remaining_tokens = remaining_tokens_with_delim;
                }
                break;
            }

            let (new_remaining_tokens, item) = match self.parser.process(remaining_tokens) {
                Ok((new_remaining_tokens, item)) => (new_remaining_tokens, item),
                Err(e) => {
                    // Track the error
                    super::track_error(&e);
                    if !self.trailing_delimiter {
                        remaining_tokens = remaining_tokens_with_delim;
                    }
                    break;
                }
            };

            remaining_tokens = new_remaining_tokens;
            remaining_tokens_with_delim = new_remaining_tokens;

            items.push(item);

            if let Ok((new_remaining_tokens, _)) =
                self.delimiter.process(remaining_tokens_with_delim)
            {
                remaining_tokens = new_remaining_tokens;
            } else {
                break;
            }
        }

        if self.at_least_one_result && items.is_empty() {
            return Err(super::ParseError::ExpectedOneOrMore);
        }

        Ok((remaining_tokens, items))
    }
}

pub fn separated<P, D>(parser: P, delimiter: D) -> Separated<P, D> {
    Separated {
        parser,
        delimiter,
        at_least_one_result: false,
        trailing_delimiter: false,
    }
}

pub fn separated1<P, D>(parser: P, delimiter: D) -> Separated<P, D> {
    Separated {
        parser,
        delimiter,
        at_least_one_result: true,
        trailing_delimiter: false,
    }
}

pub fn separated_trailing<P, D>(parser: P, delimiter: D) -> Separated<P, D> {
    Separated {
        parser,
        delimiter,
        at_least_one_result: false,
        trailing_delimiter: true,
    }
}

pub fn separated1_trailing<P, D>(parser: P, delimiter: D) -> Separated<P, D> {
    Separated {
        parser,
        delimiter,
        at_least_one_result: true,
        trailing_delimiter: true,
    }
}
