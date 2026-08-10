use crate::parser::{engine::*, Program};

use super::module_inline;

pub fn program(stream: Input) -> IResult<Program> {
    module_inline
        .map(|module| Program { module })
        .process(stream)
}
