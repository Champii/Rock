use crate::parser::*;

pub fn parse_condition(stream: Input) -> IResult<Condition> {
    disallow_multiline_fn_call((pattern.followed_by(TokenType::Equal).opt(), expression))
        .map(|(pattern, expression)| Condition {
            pattern,
            expression,
        })
        .process(stream)
}

pub fn parse_if(stream: Input) -> IResult<If> {
    (
        TokenType::Keyword("if".to_string()),
        parse_condition,
        // 'then' keyword is optional but if present, must have inline body on same line
        // Two valid forms:
        // 1. if <cond> then <inline-stmt> else <inline-stmt>
        // 2. if <cond>
        //        then <inline-stmt>
        //        else <inline-stmt>
        // The 'then' keyword can be on a new line with indentation, but the body must be inline
        (
            // Try: newline + optional empty lines + indent + then + inline statement
            (
                TokenType::Eol,
                empty_lines,
                indent_token,
                TokenType::Keyword("then".to_string()),
            )
                .map(|_| ())
                // Or: just 'then' on the same line
                .or(TokenType::Keyword("then".to_string()).map(|_| ()))
        )
        .opt(),
        block,
        // Handle optional newline and indentation before 'else'
        // Only consume the newline if it's actually followed by 'else'
        // This prevents consuming newlines that are part of the surrounding block structure
        (
            TokenType::Eol,
            empty_lines,
            indent_token,
            seek(TokenType::Keyword("else".to_string())),
        )
            .map(|_| ())
            .opt(),
        parse_else.opt(),
    )
        .map(|(_, condition, _, then, _, else_)| If {
            condition,
            then,
            else_,
        })
        .process(stream)
        .map_err(|e| e.with_context("if expression"))
}

pub fn parse_else(stream: Input) -> IResult<Else> {
    preceded(
        TokenType::Keyword("else".to_string()),
        parse_if
            .map(Box::new)
            .map(Else::If)
            .or(block.map(Else::Block)),
    )
    .process(stream)
}
