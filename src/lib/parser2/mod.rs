use std::collections::HashMap;
use std::path::PathBuf;

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while};
use nom::character::complete::{
    alphanumeric0, alphanumeric1, char, line_ending, multispace0, one_of, satisfy, space0, space1,
};
use nom::character::is_alphanumeric;
use nom::combinator::{map, map_res, opt, recognize};
use nom::error::{Error, ErrorKind};
use nom::multi::{many0, many1, separated_list0, separated_list1};
use nom::sequence::{delimited, preceded, separated_pair, terminated, tuple};
use nom::{Err, IResult};

use crate::ast::identity2::Identity;
use crate::ast::tree2::*;
use crate::ast::NodeId;
use crate::parser::span2::Span;
use crate::ty::{FuncType, Type};

use nom_locate::{position, LocatedSpan};
pub type Parser<'a> = LocatedSpan<&'a str, ParserCtx>;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct ParserCtx {
    cur_file_path: PathBuf,
    identities: Vec<Identity>,
    operators_list: HashMap<String, u8>,
}

impl ParserCtx {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            cur_file_path: file_path,
            identities: Vec::new(),
            operators_list: HashMap::new(),
        }
    }

    pub fn new_with_operators(file_path: PathBuf, operators: HashMap<String, u8>) -> Self {
        Self {
            cur_file_path: file_path,
            identities: Vec::new(),
            operators_list: operators,
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
    map(parse_mod, Root::new)(input)
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
        map(parse_fn, TopLevel::new_function),
    ))(input)
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

pub fn parse_body(input: Parser) -> IResult<Parser, Body> {
    map(many1(parse_statement), Body::new)(input)
}

pub fn parse_statement(input: Parser) -> IResult<Parser, Statement> {
    map(parse_expression, Statement::new_expression)(input)
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
    map(parse_operand, PrimaryExpr::new)(input)
}

pub fn parse_operand(input: Parser) -> IResult<Parser, Operand> {
    alt((
        map(parse_literal, Operand::new_literal),
        map(parse_identifier_path, Operand::new_identifier_path),
        map(
            delimited(
                delimited(space0, tag("("), space0),
                parse_expression,
                delimited(space0, tag(")"), space0),
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
