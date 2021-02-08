use regex::Regex;

mod lexer;
mod parser;
mod parsing_context;
mod span;
mod token;

pub use lexer::*;
pub use parser::*;
pub use parsing_context::*;
pub use span::*;
pub use token::*;

use crate::diagnostics::{Diagnostic, Diagnostics};

// pub fn preprocess(input: String) -> String {
//     // Add a '.' after a '@' if it is followed by some word
//     // This is a dirty trick to ditch having to modiffy the parser for that sugar
//     let re = Regex::new(r"@(\w)").unwrap();
//     let out = re.replace_all(&input, "@.$1");

//     out.to_string()
// }

pub fn parse(ctx: &mut ParsingCtx) -> Result<(crate::ast::Root, Vec<Token>), Diagnostic> {
    // let preprocessed = preprocess(input.clone());

    // let input: Vec<char> = ctx.get_current_file().chars().collect();

    let tokens = Lexer::new(ctx).collect();

    if ctx.diagnostics.must_stop {
        ctx.print_diagnostics();

        std::process::exit(-1);
    }

    let ast = match Parser::new(tokens.clone(), ctx).run_root() {
        Ok(ast) => ast,
        Err(e) => {
            ctx.print_diagnostics();

            if ctx.diagnostics.must_stop {
                std::process::exit(-1);

                unreachable!();
            }

            return Err(e);
        }
    };

    Ok((ast, tokens))
}

// TODO: Deduplicate
pub fn parse_mod(
    name: String,
    ctx: &mut ParsingCtx,
) -> Result<(crate::ast::Mod, Vec<Token>), Diagnostic> {
    // let preprocessed = preprocess(input.clone());

    // let input: Vec<char> = ctx.get_current_file().chars().collect();

    ctx.resolve_and_add_file(name);

    let tokens = Lexer::new(ctx).collect();

    if ctx.diagnostics.must_stop {
        ctx.print_diagnostics();

        std::process::exit(-1);
    }

    let ast = match Parser::new(tokens.clone(), ctx).run_mod() {
        Ok(ast) => ast,
        Err(e) => {
            ctx.print_diagnostics();

            if ctx.diagnostics.must_stop {
                std::process::exit(-1);

                unreachable!();
            }

            return Err(e);
        }
    };

    Ok((ast, tokens))
}
