use crate::parser::*;

pub fn instance(stream: Input) -> IResult<Instance> {
    (
        type_path,
        preceded(
            TokenType::Eol.followed_by(empty_lines),
            indented(separated1(
                preceded(indent, (followed(ident, TokenType::Colon), expression)),
                TokenType::Eol.followed_by(empty_lines),
            )),
        )
        .or(separated1(
            (followed(ident, TokenType::Colon), expression),
            TokenType::Coma,
        ))
        .opt(),
    )
        .map(|(type_path, fields)| Instance {
            name: type_path,
            fields: fields.unwrap_or_default().into_iter().collect(),
        })
        .process(stream)
}
