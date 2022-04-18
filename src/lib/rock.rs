#![feature(associated_type_bounds, derive_default_enum)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate bitflags;

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

// mod ast_lowering;
mod ast_lowering2;
mod codegen;
pub mod diagnostics;
mod hir;
mod parser;
mod parser2;
mod resolver;
mod resolver2;
mod tests;
mod ty;

use ast::ast_print::AstPrintContext;
use codegen::interpret;
use diagnostics::Diagnostic;
pub use helpers::config::Config;
use nom::Finish;
use nom_locate::LocatedSpan;
use parser::{ParsingCtx, SourceFile};

use crate::parser2::ParserCtx;

pub fn compile_file(in_name: String, config: &Config) -> Result<(), Diagnostic> {
    let mut source_file = SourceFile::from_file(in_name)?;

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
    let content = &parsing_ctx.get_current_file().content;

    let parser = LocatedSpan::new_extra(
        content.as_str(),
        ParserCtx::new(parsing_ctx.get_current_file().file_path.clone()),
    );

    let ast_test = parser2::parse_root(parser).finish();

    let hir = match ast_test {
        Ok((ctx, mut ast)) => {
            parsing_ctx.identities = ctx.extra.identities();
            parsing_ctx.files = ctx.extra.files();

            ast.operators_list = ctx.extra.operators_list();
            ast.spans = ctx.extra.identities().into_iter().collect();

            // Debug ast
            if parsing_ctx.config.show_ast {
                ast.print();
            }

            debug!("    -> Resolving");
            resolver2::resolve(&mut ast, parsing_ctx)?;

            // Lowering to HIR
            debug!("    -> Lowering to HIR");
            let mut hir = ast_lowering2::lower_crate(&ast);

            // Debug hir
            if parsing_ctx.config.show_hir {
                hir.print();
            }

            // Infer Hir
            debug!("    -> Infer HIR");
            let new_hir = infer::infer(&mut hir, parsing_ctx, config)?;

            Ok(new_hir)
        }
        Err(e) => {
            let diagnostic = Diagnostic::from(e);

            parsing_ctx.diagnostics.push_error(diagnostic.clone());

            Err(diagnostic)
        }
    }?;

    parsing_ctx.return_if_error()?;

    /* // Text to Ast
    debug!("    -> Parsing");
    let mut ast = parser::parse_root(parsing_ctx)?;

    // Name resolving
    debug!("    -> Resolving");
    resolver::resolve(&mut ast, parsing_ctx)?;

    // Lowering to HIR
    debug!("    -> Lowering to HIR");
    let mut hir = ast_lowering::lower_crate(&ast);

    // Infer Hir
    debug!("    -> Infer HIR");
    let new_hir = infer::infer(&mut hir, parsing_ctx, config)?; */

    // Ok(new_hir)
    Ok(hir)
}

pub fn generate_ir(hir: hir::Root, config: &Config) -> Result<(), Diagnostic> {
    // Generate code
    debug!("    -> Lower to LLVM IR");
    codegen::generate(config, hir)?;

    Ok(())
}
mod test {
    use super::*;
    use crate::{parser::SourceFile, Config};
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    fn build(input: String, config: Config) -> bool {
        let file = SourceFile {
            file_path: PathBuf::from("src/lib").join(&config.project_config.entry_point),
            mod_path: PathBuf::from("main"),
            content: input,
        };

        if let Err(_e) = compile_str(&file, &config) {
            return false;
        }

        let clang_cmd = Command::new("clang")
            .args(&[
                config.build_folder.join("out.bc").to_str().unwrap(),
                "-o",
                config.build_folder.join("a.out").to_str().unwrap(),
            ])
            .output()
            .expect("failed to compile to ir");

        match clang_cmd.status.code() {
            Some(code) => {
                if code != 0 {
                    println!(
                        "BUG: Cannot compile: \n{}",
                        String::from_utf8(clang_cmd.stderr).unwrap()
                    );

                    return false;
                }
            }
            None => println!(
                "\nError running: \n{}",
                String::from_utf8(clang_cmd.stderr).unwrap()
            ),
        }

        true
    }

    pub fn run(path: &str, input: String, config: Config) -> (i64, String) {
        let path = Path::new("src/lib/").join(path);

        let build_path = path.parent().unwrap().join("build");

        let mut config = config;
        config.build_folder = build_path;

        fs::create_dir_all(config.build_folder.clone()).unwrap();

        if !build(input, config.clone()) {
            return (-1, String::new());
        }

        let cmd = Command::new(config.build_folder.join("a.out").to_str().unwrap())
            .output()
            .expect("failed to execute BINARY");

        let stdout = String::from_utf8(cmd.stderr).unwrap();

        fs::remove_dir_all(config.build_folder).unwrap();

        match cmd.status.code() {
            Some(code) => (code.into(), stdout),
            None => (-1, stdout),
        }
    }
}
