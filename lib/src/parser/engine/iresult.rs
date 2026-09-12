use super::{parse_error::ParseError, Input};

pub type IResult<'a, Output> = Result<(Input<'a>, Output), ParseError>;
