use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric0, char, line_ending, one_of, satisfy, space0, space1},
    combinator::{eof, map, opt, peek, recognize},
    error::{make_error, ErrorKind, ParseError, VerboseError},
    error_position,
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, preceded, terminated, tuple},
    Err, IResult,
};

use nom_locate::{position, LocatedSpan};

use crate::{
    ast::{
        tree::{self, *},
        NodeId,
    },
    diagnostics::Diagnostic,
    ty::{FuncType, PrimitiveType, StructType, Type},
};

pub type Parser<'a> = LocatedSpan<&'a str, ParserCtx>;

type Res<T, U> = IResult<T, U, VerboseError<T>>;

pub mod parsing_context;
pub mod source_file;
pub mod span;
pub mod span2;

pub use parsing_context::ParsingCtx;
pub use source_file::SourceFile;
pub use span::Span as OldSpan;
pub use span2::Span;

#[cfg(test)]
mod tests;

// TODO:
// - add support for escaped string
// - fix typing (check every types)

pub fn accepted_operator_chars() -> Vec<char> {
    return vec!['+', '-', '/', '*', '|', '<', '>', '=', '!', '$', '@', '&'];
}

#[derive(Debug, Clone)]
pub struct ParserCtx {
    files: HashMap<PathBuf, SourceFile>,
    cur_file_path: PathBuf,
    identities: BTreeMap<NodeId, Span>,
    operators_list: HashMap<String, u8>,
    block_indent: usize,
    first_indent: Option<usize>,
    next_node_id: NodeId,
    structs: HashMap<String, Type>,
}

impl ParserCtx {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            files: HashMap::new(),
            cur_file_path: file_path,
            identities: BTreeMap::new(),
            operators_list: HashMap::new(),
            block_indent: 0,
            first_indent: None,
            next_node_id: 0,
            structs: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn new_with_operators(file_path: PathBuf, operators: HashMap<String, u8>) -> Self {
        Self {
            files: HashMap::new(),
            cur_file_path: file_path,
            identities: BTreeMap::new(),
            operators_list: operators,
            block_indent: 0,
            first_indent: None,
            next_node_id: 0,
            structs: HashMap::new(),
        }
    }

    pub fn new_from(&self, name: &str) -> Self {
        Self {
            files: HashMap::new(),
            cur_file_path: self
                .cur_file_path
                .parent()
                .unwrap()
                .join(name.to_owned() + ".rk"),
            identities: BTreeMap::new(),
            operators_list: HashMap::new(),
            block_indent: 0,
            first_indent: None,
            next_node_id: self.next_node_id,
            structs: HashMap::new(),
        }
    }

    pub fn new_identity(&mut self, span: Span) -> NodeId {
        let node_id = self.next_node_id;

        self.next_node_id += 1;

        self.identities.insert(node_id, span);

        node_id
    }

    pub fn current_file_path(&self) -> &PathBuf {
        &self.cur_file_path
    }

    pub fn operators(&self) -> &HashMap<String, u8> {
        &self.operators_list
    }

    pub fn add_operator(&mut self, op: String, prec: u8) {
        self.operators_list.insert(op, prec);
    }

    pub fn identities(&self) -> BTreeMap<NodeId, Span> {
        self.identities.clone()
    }

    pub fn operators_list(&self) -> HashMap<String, u8> {
        self.operators_list.clone()
    }

    pub fn files(&self) -> HashMap<PathBuf, SourceFile> {
        self.files.clone()
    }
}

pub fn create_parser(s: &str) -> Parser<'_> {
    LocatedSpan::new_extra(s, ParserCtx::new(PathBuf::from("")))
}

pub fn create_parser_with_filename(s: &str, path: PathBuf) -> Parser<'_> {
    LocatedSpan::new_extra(s, ParserCtx::new(path))
}

pub fn parse_root(input: Parser) -> Res<Parser, Root> {
    // TODO: move eof check in parse_mod
    map(terminated(parse_mod, eof), Root::new)(input)
}

pub fn parse_mod(input: Parser) -> Res<Parser, Mod> {
    map(
        many1(terminated(
            parse_top_level,
            many1(preceded(opt(parse_comment), line_ending)),
        )),
        Mod::new,
    )(input)
}

pub fn parse_top_level(input: Parser) -> Res<Parser, TopLevel> {
    alt((
        preceded(
            terminated(tag("extern"), space1),
            map(parse_prototype, TopLevel::new_prototype),
        ),
        parse_infix,
        map(parse_use, TopLevel::new_use),
        map(parse_struct_decl, TopLevel::new_struct),
        map(parse_trait, TopLevel::new_trait),
        map(parse_impl, TopLevel::new_impl),
        map(parse_fn, TopLevel::new_function),
        map(parse_mod_decl, |(name, mod_)| TopLevel::new_mod(name, mod_)),
    ))(input)
}

pub fn parse_comment(input: Parser) -> Res<Parser, ()> {
    let (input, _) = tuple((tag("#"), many0(satisfy(|c: char| c != '\n'))))(input)?;

    Ok((input, ()))
}

pub fn parse_eol(input: Parser) -> Res<Parser, ()> {
    let (input, _) = tuple((opt(parse_comment), line_ending))(input)?;

    Ok((input, ()))
}

pub fn parse_mod_decl(input: Parser) -> Res<Parser, (Identifier, Mod)> {
    let (mut input, mod_name) = preceded(terminated(tag("mod"), space1), parse_identifier)(input)?;

    let mut new_ctx = input.extra.new_from(&mod_name.name);
    let file_path = new_ctx.current_file_path().to_str().unwrap().to_string();

    let file = SourceFile::from_file(file_path).unwrap(); // FIXME: ERRORS ARE swallowed HERE
    new_ctx
        .files
        .insert(new_ctx.current_file_path().clone(), file);

    let content = std::fs::read_to_string(&new_ctx.current_file_path()).unwrap();

    let mut new_parser = Parser::new_extra(&content, new_ctx);

    use nom::Finish;
    // FIXME: Errors are swallowed here
    let (input2, mod_) = parse_mod(new_parser).finish().unwrap();

    // hydrate `input` with the new parser's operators
    // TODO: handle duplicate operators
    input
        .extra
        .operators_list
        .extend(input2.extra.operators_list);

    // extend identities
    input.extra.identities.extend(input2.extra.identities);
    input.extra.next_node_id = input2.extra.next_node_id;
    input.extra.files.extend(input2.extra.files);

    Ok((input, (mod_name, mod_)))
}

pub fn parse_trait(input: Parser) -> Res<Parser, Trait> {
    map(
        tuple((
            terminated(tag("trait"), space1),
            parse_type,
            many0(delimited(space1, parse_type, space0)),
            many0(line_ending),
            indent(separated_list1(
                line_ending,
                preceded(parse_block_indent, parse_prototype),
            )),
        )),
        |(_, name, types, _, defs)| Trait::new(name, types, defs),
    )(input)
}

pub fn parse_impl(input: Parser) -> Res<Parser, Impl> {
    map(
        tuple((
            terminated(tag("impl"), space1),
            parse_type,
            many0(delimited(space1, parse_type, space0)),
            many0(line_ending),
            indent(separated_list1(
                line_ending,
                preceded(parse_block_indent, parse_fn),
            )),
        )),
        |(_, name, types, _, defs)| Impl::new(name, types, defs),
    )(input)
}

pub fn parse_struct_decl(input: Parser) -> Res<Parser, StructDecl> {
    let (mut input, struct_decl) = map(
        tuple((
            terminated(tag("struct"), space1),
            parse_capitalized_identifier,
            many0(line_ending),
            indent(separated_list0(
                line_ending,
                preceded(parse_block_indent, parse_prototype),
            )),
        )),
        |(tag, name, _, defs)| StructDecl::new(name, defs),
    )(input)?;

    let struct_t: StructType = struct_decl.clone().into();

    input
        .extra
        .structs
        .insert(struct_decl.name.name.clone(), struct_t.into());

    Ok((input, struct_decl))
}

pub fn parse_use(input: Parser) -> Res<Parser, Use> {
    preceded(
        terminated(tag("use"), space1),
        map(
            tuple((parse_identity, parse_identifier_path)),
            |(node_id, ident)| Use::new(ident, node_id),
        ),
    )(input)
}

pub fn parse_infix(input: Parser) -> Res<Parser, TopLevel> {
    let (mut input, (parsed_op, pred)) = preceded(
        terminated(tag("infix"), space1),
        tuple((
            terminated(many1(allowed_operator_chars), space1),
            parse_number,
        )),
    )(input)?;

    let (input, pos) = position(input)?;

    let (mut input, node_id) = new_identity(input, &pos);

    let op = parsed_op.join("");

    input.extra.add_operator(op.clone(), pred.as_i64() as u8);

    let op = Operator(Identifier { name: op, node_id });

    Ok((input, TopLevel::new_infix(op, pred.as_i64() as u8)))
}

pub fn parse_identifier_or_operator(input: Parser) -> Res<Parser, Identifier> {
    alt((parse_identifier, map(parse_operator, |op| op.0)))(input)
}

pub fn parse_prototype(input: Parser) -> Res<Parser, Prototype> {
    map(
        tuple((
            parse_identity,
            terminated(
                parse_identifier_or_operator,
                delimited(space0, tag("::"), space0),
            ),
            parse_signature,
        )),
        |(node_id, name, signature)| Prototype {
            node_id,
            name,
            signature,
        },
    )(input)
}

pub fn parse_fn(input: Parser) -> Res<Parser, FunctionDecl> {
    map(
        tuple((
            parse_identity,
            terminated(
                tuple((
                    parse_identifier_or_operator,
                    many0(preceded(space1, parse_identifier)),
                )),
                delimited(space0, char('='), space0),
            ),
            parse_body,
        )),
        |(node_id, (name, arguments), body)| FunctionDecl {
            node_id,
            name,
            body,
            signature: FuncType::from_args_nb(arguments.len()), // FIXME: Should not generate random signature
            arguments,
        },
    )(input)
}

fn indent<'a, O, E, F>(mut parser: F) -> impl FnMut(Parser<'a>) -> IResult<Parser<'a>, O, E>
where
    F: nom::Parser<Parser<'a>, O, E>,
{
    move |mut input: Parser<'a>| {
        if let Some(indent) = input.extra.first_indent {
            input.extra.block_indent += indent;
        }

        let (mut input, output) = parser.parse(input)?;

        if let Some(indent) = input.extra.first_indent {
            input.extra.block_indent -= indent;
        }

        Ok((input, output))
    }
}

pub fn parse_block_indent(input: Parser) -> Res<Parser, usize> {
    let (mut input, indent) = space1(input)?;
    let indent_len = indent.fragment().len();

    if input.extra.first_indent == None {
        input.extra.first_indent = Some(indent_len);
        input.extra.block_indent = indent_len;
    }

    if indent_len == input.extra.block_indent {
        Ok((input, indent_len))
    } else {
        Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )))
    }
}

pub fn parse_body(mut input: Parser) -> Res<Parser, Body> {
    let (input, opt_eol) = opt(line_ending)(input)?; // NOTE: should not fail

    if opt_eol.is_some() {
        indent(map(
            separated_list1(
                many1(parse_eol),
                preceded(parse_block_indent, parse_statement),
            ),
            Body::new,
        ))(input)
    } else {
        map(parse_statement, |stmt| Body::new(vec![stmt]))(input)
    }
}

pub fn parse_statement(input: Parser) -> Res<Parser, Statement> {
    alt((
        map(parse_if, Statement::new_if),
        map(parse_for, Statement::new_for),
        map(parse_assign, Statement::new_assign),
        map(parse_expression, Statement::new_expression),
    ))(input)
}

pub fn parse_if(input: Parser) -> Res<Parser, If> {
    map(
        tuple((
            parse_identity,
            terminated(tag("if"), space1),
            terminated(parse_expression, space0),
            many1(line_ending),
            parse_then,
            parse_body,
            opt(tuple((line_ending, parse_else))),
        )),
        |(node_id, if_, cond, _, _, body, else_)| {
            If::new(node_id, cond, body, else_.map(|(_, else_)| Box::new(else_)))
        },
    )(input.clone())
}

pub fn parse_then(input: Parser) -> Res<Parser, ()> {
    // NOTE: This is a tweek for then block that are at indent 0 (i.e. in the test files)
    let (input, indent) = if input.extra.first_indent.is_some() && input.extra.block_indent > 0 {
        parse_block_indent(input)?
    } else {
        (input, 0)
    };

    if indent == input.extra.block_indent {
        let (input, _) = terminated(tag("then"), space0)(input)?;

        Ok((input, ()))
    } else {
        Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )))
    }
}

pub fn parse_else(input: Parser) -> Res<Parser, Else> {
    // NOTE: This is a tweek for else block that are at indent 0 (i.e. in the test files)
    let (input, indent) = if input.extra.first_indent.is_some() && input.extra.block_indent > 0 {
        parse_block_indent(input)?
    } else {
        (input, 0)
    };

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

pub fn parse_for(input: Parser) -> Res<Parser, For> {
    alt((map(parse_for_in, For::In), map(parse_while, For::While)))(input)
}

pub fn parse_for_in(input: Parser) -> Res<Parser, ForIn> {
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

pub fn parse_while(input: Parser) -> Res<Parser, While> {
    map(
        tuple((
            terminated(tag("while"), space1),
            terminated(parse_expression, space0),
            parse_body,
        )),
        |(_, cond, body)| While::new(cond, body),
    )(input)
}

pub fn parse_assign(input: Parser) -> Res<Parser, Assign> {
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

pub fn parse_assign_left_side(input: Parser) -> Res<Parser, AssignLeftSide> {
    let (input, expr) = parse_expression(input)?;

    let res = if expr.is_dot() {
        AssignLeftSide::Dot(expr)
    } else if expr.is_indice() {
        AssignLeftSide::Indice(expr)
    } else if expr.is_identifier() {
        AssignLeftSide::Identifier(expr)
    } else {
        return Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )));
    };

    Ok((input, res))
}

pub fn parse_expression(input: Parser) -> Res<Parser, Expression> {
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
        map(parse_struct_ctor, Expression::new_struct_ctor),
        map(parse_native_operator, |(op, id1, id2)| {
            Expression::new_native_operator(op, id1, id2)
        }),
        // TODO: Return
    ))(input)
}

pub fn parse_native_operator(
    input: Parser,
) -> Res<Parser, (NativeOperator, Identifier, Identifier)> {
    map(
        tuple((
            preceded(
                tag("~"),
                alt((
                    tag("IAdd"),
                    tag("ISub"),
                    tag("IMul"),
                    tag("IDiv"),
                    tag("FAdd"),
                    tag("FSub"),
                    tag("FMul"),
                    tag("FDiv"),
                    tag("IEq"),
                    tag("Igt"),
                    tag("Ige"),
                    tag("Ilt"),
                    tag("Ile"),
                    tag("FEq"),
                    tag("Fgt"),
                    tag("Fge"),
                    tag("Flt"),
                    tag("Fle"),
                    tag("BEq"),
                    tag("Len"),
                )),
            ),
            preceded(space1, parse_identifier),
            preceded(space1, parse_identifier),
        )),
        |(tag, id1, id2)| {
            let (input, node_id) = new_identity(input.clone(), &tag);
            (
                NativeOperator::new(node_id, NativeOperatorKind::from_str(tag.fragment())),
                id1,
                id2,
            )
        },
    )(input.clone())
}

pub fn parse_struct_ctor(input: Parser) -> Res<Parser, StructCtor> {
    map(
        tuple((
            // parse_identity,
            terminated(parse_capitalized_identifier, line_ending),
            indent(separated_list0(
                line_ending,
                preceded(
                    parse_block_indent,
                    tuple((
                        terminated(parse_identifier, delimited(space0, tag(":"), space0)),
                        parse_expression,
                    )),
                ),
            )),
        )),
        |(name, decls)| StructCtor::new(name, decls.into_iter().collect()),
    )(input)
}

pub fn parse_unary(input: Parser) -> Res<Parser, UnaryExpr> {
    map(parse_primary, UnaryExpr::new_primary)(input)
}

pub fn parse_primary(input: Parser) -> Res<Parser, PrimaryExpr> {
    map(
        tuple((parse_identity, parse_operand, many0(parse_secondary))),
        |(node_id, op, secs)| PrimaryExpr::new(node_id, op, secs),
    )(input)
}

pub fn parse_secondary(input: Parser) -> Res<Parser, SecondaryExpr> {
    alt((
        map(parse_indice, SecondaryExpr::Indice),
        map(parse_dot, SecondaryExpr::Dot),
        map(parse_arguments, SecondaryExpr::Arguments),
    ))(input)
}

pub fn parse_arguments(input: Parser) -> Res<Parser, Arguments> {
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

pub fn parse_argument(input: Parser) -> Res<Parser, Argument> {
    map(parse_unary, Argument::new)(input)
}

pub fn parse_indice(input: Parser) -> Res<Parser, Box<Expression>> {
    map(
        tuple((
            terminated(tag("["), space0),
            terminated(parse_expression, space0),
            terminated(tag("]"), space0),
        )),
        |(_, index, _)| Box::new(index),
    )(input)
}

pub fn parse_dot(input: Parser) -> Res<Parser, Identifier> {
    map(
        tuple((
            terminated(tag("."), space0),
            terminated(parse_identifier, space0),
        )),
        |(_, ident)| ident,
    )(input)
}

pub fn parse_operand(input: Parser) -> Res<Parser, Operand> {
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

pub fn parse_identifier_path(input: Parser) -> Res<Parser, IdentifierPath> {
    map(
        separated_list1(
            tag("::"),
            alt((
                map(tuple((parse_identity, tag("(*)"))), |(node_id, _)| {
                    Identifier::new("(*)".to_string(), node_id)
                }),
                parse_identifier,
            )),
        ),
        IdentifierPath::new,
    )(input)
}

pub fn parse_capitalized_identifier(input: Parser) -> Res<Parser, Identifier> {
    let (input, (node_id, txt)) = tuple((parse_identity, parse_capitalized_text))(input)?;

    Ok((
        input,
        Identifier {
            name: txt.to_string(),
            node_id,
        },
    ))
}

pub fn parse_identifier(input: Parser) -> Res<Parser, Identifier> {
    let (input, ident_parsed) =
        recognize(many1(one_of("abcdefghijklmnopqrstuvwxyz_0123456789")))(input)?;

    let (input, node_id) = new_identity(input, &ident_parsed);

    Ok((
        input,
        Identifier {
            name: ident_parsed.to_string(),
            node_id,
        },
    ))
}

pub fn parse_operator(input: Parser) -> Res<Parser, Operator> {
    let (input, parsed_op) = recognize(many1(one_of(LocatedSpan::new(
        // We parse any accepted operators chars here, and then check if it is a valid operator later
        crate::parser::accepted_operator_chars()
            .iter()
            .cloned()
            .collect::<String>()
            .as_str(),
    ))))(input)?;

    if parsed_op.to_string() == "=" {
        return Err(Err::Error(error_position!(input, ErrorKind::Eof)));
    }

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

pub fn parse_literal(input: Parser) -> Res<Parser, Literal> {
    alt((
        parse_bool,
        parse_float,
        parse_number,
        parse_array,
        parse_string,
    ))(input)
}

pub fn parse_string(input: Parser) -> Res<Parser, Literal> {
    map(
        tuple((
            parse_identity,
            terminated(tag("\""), space0),
            recognize(take_while(|c: char| c != '"')),
            terminated(tag("\""), space0),
        )),
        |(node_id, _, s, _)| Literal::new_string(String::from(*s.fragment()), node_id),
    )(input)
}

pub fn parse_array(input: Parser) -> Res<Parser, Literal> {
    map(
        tuple((
            parse_identity,
            terminated(tag("["), space0),
            separated_list0(
                tuple((space0, terminated(tag(","), space0), space0)),
                parse_expression,
            ),
            terminated(tag("]"), space0),
        )),
        |(node_id, _, elements, _)| {
            Literal::new_array(Array::new(elements.into_iter().collect()), node_id)
        },
    )(input)
}

pub fn parse_bool(input: Parser) -> Res<Parser, Literal> {
    let (input, bool_parsed) = alt((tag("true"), tag("false")))(input)?;

    let num: bool = bool_parsed
        .parse()
        .map_err(|_| Err::Error(make_error(input.clone(), ErrorKind::Alpha)))?;

    let (input, node_id) = new_identity(input, &bool_parsed);

    Ok((input, Literal::new_bool(num, node_id)))
}

pub fn parse_float(input: Parser) -> Res<Parser, Literal> {
    let (input, float_parsed) =
        recognize(tuple((parse_number, char('.'), opt(parse_number))))(input)?;

    let num: f64 = float_parsed
        .parse()
        .map_err(|_| Err::Error(make_error(input.clone(), ErrorKind::Digit)))?;

    let (input, node_id) = new_identity(input, &float_parsed);

    Ok((input, Literal::new_float(num, node_id)))
}

pub fn parse_number(input: Parser) -> Res<Parser, Literal> {
    let (input, parsed) = take_while(is_digit)(input)?;

    let num: i64 = parsed
        .parse()
        .map_err(|_| Err::Error(make_error(input.clone(), ErrorKind::Digit)))?;

    let (input, node_id) = new_identity(input, &parsed);

    Ok((input, Literal::new_number(num, node_id)))
}

// Types

pub fn parse_signature(input: Parser) -> Res<Parser, FuncType> {
    let (input, parsed) = tuple((
        parse_type,
        many0(preceded(delimited(space0, tag("->"), space0), parse_type)),
    ))(input)?;

    let mut types = vec![parsed.0];

    types.extend(parsed.1);

    let ret = types.pop().unwrap();

    Ok((input, FuncType::new(types, ret)))
}

pub fn parse_type(input: Parser) -> Res<Parser, Type> {
    let (input, ty) = alt((
        map(
            terminated(
                one_of("abcdefghijklmnopqrstuvwxyz"),
                peek(alt((space1, line_ending, eof))),
            ),
            |c| Type::ForAll(String::from(c)),
        ),
        map(delimited(tag("["), parse_type, tag("]")), |t| {
            Type::Primitive(PrimitiveType::Array(
                Box::new(t),
                0, // FIXME
            ))
        }),
        map(
            alt((
                map(tag("Bool"), |_| PrimitiveType::Bool),
                map(tag("Int64"), |_| PrimitiveType::Int64),
                map(tag("Float64"), |_| PrimitiveType::Float64),
                map(tag("String"), |_| PrimitiveType::String),
            )),
            |t| Type::from(t),
        ),
        map(parse_struct_type, Type::Struct),
        map(parse_capitalized_text, Type::Trait),
    ))(input)?;

    Ok((input, ty))
}

pub fn parse_struct_type(input: Parser) -> Res<Parser, StructType> {
    let (input, name) = parse_capitalized_text(input)?;

    let ty = if let Some(struct_t) = input.extra.structs.get(&name) {
        struct_t.as_struct_type()
    } else {
        return Err(nom::Err::Error(ParseError::from_error_kind(
            input,
            ErrorKind::Tag,
        )));
    };

    Ok((input, ty))
}

pub fn parse_capitalized_text(input: Parser) -> Res<Parser, String> {
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

fn parse_identity(input: Parser) -> Res<Parser, NodeId> {
    let (input, pos) = position(input)?;

    let (input, node_id) = new_identity(input, &pos);

    Ok((input, node_id))
}

pub fn allowed_operator_chars(input: Parser) -> Res<Parser, String> {
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

pub fn parse(parsing_ctx: &mut ParsingCtx) -> Result<tree::Root, Diagnostic> {
    use nom::Finish;
    use nom_locate::LocatedSpan;

    let content = &parsing_ctx.get_current_file().content;

    let parser = LocatedSpan::new_extra(
        content.as_str(),
        ParserCtx::new(parsing_ctx.get_current_file().file_path.clone()),
    );

    let ast = parse_root(parser).finish();

    let mut ast = match ast {
        Ok((ctx, mut ast)) => {
            parsing_ctx.identities = ctx.extra.identities();
            parsing_ctx.files = ctx.extra.files();

            ast.operators_list = ctx.extra.operators_list();
            ast.spans = ctx.extra.identities().into_iter().collect();

            // Debug ast
            if parsing_ctx.config.show_ast {
                ast.print();
            }

            Ok(ast)
        }
        Err(e) => {
            let diagnostic = Diagnostic::from(e);

            parsing_ctx.diagnostics.push_error(diagnostic.clone());

            Err(diagnostic)
        }
    }?;

    parsing_ctx.return_if_error()?;

    Ok(ast)
}
