use crate::parser::{engine::*, Ident};

use super::{ident_token, operator};

pub fn ident(stream: Input) -> IResult<Ident> {
    ident_token
        .or(operator.map(|op| Ident {
            name: op.value.clone(),
            span: op.span.clone(),
        }))
        .process(stream)
}
