#[macro_use]
extern crate serde_derive;

extern crate bitflags;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate nom_locate;

use std::path::PathBuf;

#[macro_use]
mod helpers;

#[macro_use]
mod ast;
#[macro_use]
mod infer;

mod ast_lowering;
mod codegen;
pub mod diagnostics;
mod hir;
mod parser;
mod resolver;
mod tests;
mod ty;

use codegen::interpret;
use diagnostics::Diagnostic;
pub use helpers::config::Config;
use parser::{ParsingCtx, SourceFile};

pub fn compile_file(in_name: String, config: &Config) -> Result<(), Diagnostic> {
    let mut source_file = SourceFile::from_file(in_name)?;

    if config.std {
        source_file.content = "mod std\nuse std::prelude::(*)\n".to_owned() + &source_file.content;
    }

    source_file.mod_path = PathBuf::from("root");

    compile_str(&source_file, config)
}

pub fn compile_str(input: &SourceFile, config: &Config) -> Result<(), Diagnostic> {
    let mut parsing_ctx = ParsingCtx::new(config);

    parsing_ctx.add_file(input);

    let hir = parse_str(&mut parsing_ctx, config)?;

    if config.repl {
        interpret(hir, config)
    } else {
        generate_ir(hir, config)?;

        parsing_ctx.print_success_diagnostics();

        Ok(())
    }
}

pub fn parse_str(parsing_ctx: &mut ParsingCtx, config: &Config) -> Result<hir::Root, Diagnostic> {
    // Text to Ast
    debug!("    -> Parsing");
    let mut ast = parser::parse(parsing_ctx)?;

    // Name resolving
    debug!("    -> Resolving");
    resolver::resolve(&mut ast, parsing_ctx)?;

    // Lowering to HIR
    debug!("    -> Lowering to HIR");
    let mut hir = ast_lowering::lower_crate(&ast);

    // Infer Hir
    debug!("    -> Infer HIR");
    let new_hir = infer::infer(&mut hir, parsing_ctx, config)?;

    Ok(new_hir)
}

pub fn generate_ir(hir: hir::Root, config: &Config) -> Result<(), Diagnostic> {
    // Generate code
    debug!("    -> Lower to LLVM IR");
    codegen::generate(config, hir)?;

    Ok(())
}
