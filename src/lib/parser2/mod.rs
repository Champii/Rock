use std::collections::HashMap;

use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::{
    alphanumeric0, alphanumeric1, char, line_ending, one_of, satisfy, space0, space1,
};
use nom::combinator::{eof, map, opt, recognize};
use nom::error::{Error, ErrorKind};
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::{error::ParseError, Err, IResult};

use crate::ast::identity2::Identity;
use crate::ast::tree2::*;
use crate::ast::NodeId;
use crate::parser::span2::Span;
use crate::ty::{FuncType, Type};

use nom_locate::{position, LocatedSpan};
pub type Parser<'a> = LocatedSpan<&'a str, ParserCtx>;

#[cfg(test)]
mod tests;

// TODO:
// - add support for comments
// - add support for string literals
// - add support for array literals
// - add support for struct declarations
// - add support for struct literals
// - add support for module declarations

#[derive(Debug, Clone)]
pub struct ParserCtx {
    cur_file_path: PathBuf,
    identities: Vec<Identity>,
    operators_list: HashMap<String, u8>,
    block_indent: usize,
}

impl ParserCtx {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            cur_file_path: file_path,
            identities: Vec::new(),
            operators_list: HashMap::new(),
            block_indent: 0,
        }
    }

    #[cfg(test)]
    pub fn new_with_operators(file_path: PathBuf, operators: HashMap<String, u8>) -> Self {
        Self {
            cur_file_path: file_path,
            identities: Vec::new(),
            operators_list: operators,
            block_indent: 0,
        }
    }

    pub fn new_identity(&mut self, span: Span) -> NodeId {
        let node_id = self.identities.len() as NodeId;

        self.identities.push(Identity::new(node_id, span));

        node_id
    }

    pub fn current_file_path(&self) -> &PathBuf {
        &self.cur_file_path
    }

    // pub fn identities(&self) -> &Vec<Identity> {
    //     &self.identities
    // }

    pub fn operators(&self) -> &HashMap<String, u8> {
        &self.operators_list
    }

    pub fn add_operator(&mut self, op: String, prec: u8) {
        self.operators_list.insert(op, prec);
    }
}

pub fn parse_root(input: Parser) -> IResult<Parser, Root> {
    map(terminated(parse_mod, eof), Root::new)(input)
}

pub fn parse_mod(input: Parser) -> IResult<Parser, Mod> {
    map(
        many1(terminated(parse_top_level, many1(line_ending))),
        Mod::new,
    )(input)
}

pub fn parse_top_level(input: Parser) -> IResult<Parser, TopLevel> {
    alt((
        preceded(
            terminated(tag("extern"), space1),
            map(parse_prototype, TopLevel::new_prototype),
        ),
        parse_infix,
        map(parse_use, TopLevel::new_use),
        map(parse_struct_decl, TopLevel::new_struct),
        map(parse_fn, TopLevel::new_function),
    ))(input)
}

pub fn parse_struct_decl(input: Parser) -> IResult<Parser, StructDecl> {
    map(
        tuple((
            terminated(tag("struct"), space1),
            parse_type,
            many0(line_ending),
            separated_list0(
                terminated(line_ending, space1),
                preceded(parse_block_indent, parse_prototype),
            ),
        )),
        |(tag, name, _, defs)| {
            let (_input, node_id) = new_identity(input.clone(), &tag);

            StructDecl::new(node_id, name, defs)
        },
    )(input.clone())
}

pub fn parse_use(input: Parser) -> IResult<Parser, Use> {
    preceded(
        terminated(tag("use"), space1),
        map(parse_identifier_path, Use::new),
    )(input)
}

pub fn parse_infix(input: Parser) -> IResult<Parser, TopLevel> {
    let (mut input, (parsed_op, pred)) = preceded(
        terminated(tag("infix"), space1),
        tuple((
            terminated(many1(allowed_operator_chars), space1),
            parse_number,
        )),
    )(input)?;

    // let (input, node_id) = new_identity(input, &parsed_op);

    let op = parsed_op.join("");

    input.extra.add_operator(op.clone(), pred.as_i64() as u8);

    let op = Operator(Identifier {
        name: op,
        node_id: 0,
    });

    Ok((input, TopLevel::new_infix(op, pred.as_i64() as u8)))
}

pub fn parse_prototype(input: Parser) -> IResult<Parser, Prototype> {
    map(
        tuple((
            terminated(parse_identifier, delimited(space0, tag("::"), space0)),
            parse_signature,
        )),
        |(name, signature)| Prototype {
            node_id: name.node_id,
            name,
            signature,
        },
    )(input)
}

pub fn parse_fn(input: Parser) -> IResult<Parser, FunctionDecl> {
    map(
        tuple((
            terminated(
                tuple((parse_identifier, many0(preceded(space1, parse_identifier)))),
                delimited(space0, char('='), space0),
            ),
            parse_body,
        )),
        |((name, arguments), body)| FunctionDecl {
            name,
            body,
            signature: FuncType::from_args_nb(arguments.len()),
            arguments,
        },
    )(input)
}

pub fn parse_block_indent(input: Parser) -> IResult<Parser, usize> {
    map(space0, |indents: Parser| indents.fragment().len())(input)
}

pub fn parse_body(mut input: Parser) -> IResult<Parser, Body> {
    input.extra.block_indent += 2;

    let output = map(
        tuple((
            line_ending,
            separated_list1(many1(line_ending), parse_statement),
        )),
        |(_, stmts)| Body::new(stmts),
    )(input);

    output.map(|(mut input, body)| {
        input.extra.block_indent -= 2;

        (input, body)
    })
}

pub fn parse_statement(input: Parser) -> IResult<Parser, Statement> {
    let (input, indent) = parse_block_indent(input)?;

    if indent == input.extra.block_indent {
        alt((
            map(parse_if, Statement::new_if),
            map(parse_for, Statement::new_for),
            map(parse_expression, Statement::new_expression),
        ))(input)
    } else {
        Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )))
    }
}

pub fn parse_if(input: Parser) -> IResult<Parser, If> {
    map(
        tuple((
            terminated(tag("if"), space1),
            terminated(parse_expression, space0),
            parse_body,
            opt(tuple((line_ending, parse_else))),
        )),
        |(if_, cond, body, else_)| {
            let (_input, node_id) = new_identity(input.clone(), &if_);

            If::new(node_id, cond, body, else_.map(|(_, else_)| Box::new(else_)))
        },
    )(input.clone())
}

pub fn parse_else(input: Parser) -> IResult<Parser, Else> {
    let (input, indent) = parse_block_indent(input)?;

    if indent == input.extra.block_indent {
        alt((
            map(
                tuple((
                    terminated(tag("else"), space1),
                    terminated(parse_if, space0),
                )),
                |(_, if_)| Else::If(if_),
            ),
            map(
                tuple((
                    terminated(tag("else"), space0),
                    terminated(parse_body, space0),
                )),
                |(_, body)| Else::Body(body),
            ),
        ))(input)
    } else {
        Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )))
    }
}

pub fn parse_for(input: Parser) -> IResult<Parser, For> {
    alt((map(parse_for_in, For::In), map(parse_while, For::While)))(input)
}

pub fn parse_for_in(input: Parser) -> IResult<Parser, ForIn> {
    map(
        tuple((
            terminated(tag("for"), space1),
            terminated(parse_identifier, space0),
            terminated(tag("in"), space0),
            terminated(parse_expression, space0),
            parse_body,
        )),
        |(_, var, _, expr, body)| ForIn::new(var, expr, body),
    )(input)
}

pub fn parse_while(input: Parser) -> IResult<Parser, While> {
    map(
        tuple((
            terminated(tag("while"), space1),
            terminated(parse_expression, space0),
            parse_body,
        )),
        |(_, cond, body)| While::new(cond, body),
    )(input)
}

pub fn parse_assign(input: Parser) -> IResult<Parser, Assign> {
    map(
        tuple((
            opt(terminated(tag("let"), space1)),
            terminated(parse_assign_left_side, space0),
            terminated(tag("="), space0),
            terminated(parse_expression, space0),
        )),
        |(opt_let, var, _, expr)| Assign::new(var, expr, opt_let.is_some()),
    )(input)
}

pub fn parse_assign_left_side(input: Parser) -> IResult<Parser, AssignLeftSide> {
    map(parse_expression, |expr| {
        if expr.is_dot() {
            AssignLeftSide::Dot(expr)
        } else if expr.is_indice() {
            AssignLeftSide::Indice(expr)
        } else if expr.is_identifier() {
            AssignLeftSide::Identifier(expr)
        } else {
            panic!("Invalid left side of assignment: {:?}", expr);
        }
    })(input)
}

pub fn parse_expression(input: Parser) -> IResult<Parser, Expression> {
    alt((
        map(
            tuple((
                parse_unary,
                delimited(space0, parse_operator, space0),
                parse_expression,
            )),
            |(l, op, r)| Expression::new_binop(l, op, r),
        ),
        map(parse_unary, Expression::new_unary),
    ))(input)
}

pub fn parse_unary(input: Parser) -> IResult<Parser, UnaryExpr> {
    map(parse_primary, UnaryExpr::new_primary)(input)
}

pub fn parse_primary(input: Parser) -> IResult<Parser, PrimaryExpr> {
    map(
        tuple((parse_operand, many0(parse_secondary))),
        |(op, secs)| PrimaryExpr::new(op, secs),
    )(input)
}

pub fn parse_secondary(input: Parser) -> IResult<Parser, SecondaryExpr> {
    alt((
        map(parse_indice, SecondaryExpr::Indice),
        map(parse_dot, SecondaryExpr::Dot),
        map(parse_arguments, SecondaryExpr::Arguments),
    ))(input)
}

pub fn parse_arguments(input: Parser) -> IResult<Parser, Arguments> {
    alt((
        map(
            tuple((
                terminated(tag("("), space0),
                separated_list0(tuple((space0, tag(","), space0)), parse_argument),
                terminated(tag(")"), space0),
            )),
            |(_, args, _)| args,
        ),
        map(
            tuple((
                space1,
                separated_list1(
                    tuple((space0, terminated(tag(","), space0), space0)),
                    parse_argument,
                ),
            )),
            |(_, args)| args,
        ),
    ))(input)
}

pub fn parse_argument(input: Parser) -> IResult<Parser, Argument> {
    map(parse_unary, Argument::new)(input)
}

pub fn parse_indice(input: Parser) -> IResult<Parser, Box<Expression>> {
    map(
        tuple((
            terminated(tag("["), space0),
            terminated(parse_expression, space0),
            terminated(tag("]"), space0),
        )),
        |(_, index, _)| Box::new(index),
    )(input)
}

pub fn parse_dot(input: Parser) -> IResult<Parser, Identifier> {
    map(
        tuple((
            terminated(tag("."), space0),
            terminated(parse_identifier, space0),
        )),
        |(_, ident)| ident,
    )(input)
}

pub fn parse_operand(input: Parser) -> IResult<Parser, Operand> {
    alt((
        map(parse_literal, Operand::new_literal),
        map(parse_identifier_path, Operand::new_identifier_path),
        map(
            delimited(
                terminated(tag("("), space0),
                parse_expression,
                terminated(space0, tag(")")),
            ),
            Operand::new_expression,
        ),
    ))(input)
}

pub fn parse_identifier_path(input: Parser) -> IResult<Parser, IdentifierPath> {
    map(
        separated_list1(tag("::"), parse_identifier),
        IdentifierPath::new,
    )(input)
}

pub fn parse_identifier(input: Parser) -> IResult<Parser, Identifier> {
    let (input, ident_parsed) = alphanumeric1(input)?;

    let (input, node_id) = new_identity(input, &ident_parsed);

    Ok((
        input,
        Identifier {
            name: ident_parsed.to_string(),
            node_id,
        },
    ))
}

pub fn parse_operator(input: Parser) -> IResult<Parser, Operator> {
    let (input, parsed_op) = one_of(LocatedSpan::new(
        input
            .extra
            .operators()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join("")
            .as_str(),
    ))(input)?;

    let (input, pos) = position(input)?;

    let (input, node_id) = new_identity(input, &pos);

    Ok((
        input,
        Operator(Identifier {
            name: parsed_op.to_string(),
            node_id,
        }),
    ))
}

pub fn parse_literal(input: Parser) -> IResult<Parser, Literal> {
    alt((parse_bool, parse_float, parse_number))(input)
}

pub fn parse_bool(input: Parser) -> IResult<Parser, Literal> {
    let (input, bool_parsed) = alt((tag("true"), tag("false")))(input)?;

    let num: bool = bool_parsed
        .parse()
        .map_err(|_| Err::Error(Error::new(input.clone(), ErrorKind::Alpha)))?;

    let (input, node_id) = new_identity(input, &bool_parsed);

    Ok((input, Literal::new_bool(num, node_id)))
}

pub fn parse_float(input: Parser) -> IResult<Parser, Literal> {
    let (input, float_parsed) =
        recognize(tuple((parse_number, char('.'), opt(parse_number))))(input)?;

    let num: f64 = float_parsed
        .parse()
        .map_err(|_| Err::Error(Error::new(input.clone(), ErrorKind::Float)))?;

    let (input, node_id) = new_identity(input, &float_parsed);

    Ok((input, Literal::new_float(num, node_id)))
}

pub fn parse_number(input: Parser) -> IResult<Parser, Literal> {
    let (input, parsed) = take_while(is_digit)(input)?;

    let num: i64 = parsed
        .parse()
        .map_err(|_| Err::Error(Error::new(input.clone(), ErrorKind::Digit)))?;

    let (input, node_id) = new_identity(input, &parsed);

    Ok((input, Literal::new_number(num, node_id)))
}

// Types

pub fn parse_signature(input: Parser) -> IResult<Parser, FuncType> {
    let (input, parsed) = tuple((
        parse_type,
        many0(preceded(delimited(space0, tag("->"), space0), parse_type)),
    ))(input)?;

    let mut types = vec![parsed.0];

    types.extend(parsed.1);

    let ret = types.pop().unwrap();

    Ok((
        input,
        FuncType::from_args_nb(types.len()).apply_types(types, ret),
    ))
}

pub fn parse_type(input: Parser) -> IResult<Parser, Type> {
    let (input, parsed) = parse_capitalized_text(input)?;

    let ty = Type::from(parsed);

    Ok((input, ty))
}

pub fn parse_capitalized_text(input: Parser) -> IResult<Parser, String> {
    let (input, parsed) = tuple((satisfy(char::is_uppercase), alphanumeric0))(input)?;

    let txt =
        format!("{}", parsed.0) + &String::from_utf8(parsed.1.bytes().collect::<Vec<_>>()).unwrap();

    Ok((input, txt))
}

// Helpers

fn is_digit(c: char) -> bool {
    c.is_numeric()
}

fn new_identity<'a>(mut input: Parser<'a>, parsed: &Parser<'a>) -> (Parser<'a>, NodeId) {
    let node_id = input.extra.new_identity(Span::from(parsed.clone()));

    (input, node_id)
}

pub fn allowed_operator_chars(input: Parser) -> IResult<Parser, String> {
    let (input, c) = one_of(LocatedSpan::new(
        crate::parser::accepted_operator_chars()
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("")
            .as_str(),
    ))(input)?;

    Ok((input, c.to_string()))
}
