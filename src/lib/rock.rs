#![feature(associated_type_bounds, destructuring_assignment, derive_default_enum)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;
#[macro_use]
extern crate concat_idents;

#[macro_use]
pub mod ast;
#[macro_use]
pub mod infer;
#[macro_use]
pub mod helpers;

use std::path::PathBuf;

use diagnostics::Diagnostic;
use parser::{ParsingCtx, SourceFile};

mod ast_lowering;
mod codegen;
pub mod diagnostics;
mod hir;
mod parser;
mod tests;
mod ty;

pub use crate::helpers::config::Config;

pub fn parse_file(in_name: String, config: &Config) -> Result<(), Diagnostic> {
    let mut source_file = SourceFile::from_file(in_name)?;

    source_file.mod_path = PathBuf::from("root");

    parse_str(&source_file, config)
}

pub fn parse_str(input: &SourceFile, config: &Config) -> Result<(), Diagnostic> {
    let mut parsing_ctx = ParsingCtx::new(config);

    parsing_ctx.add_file(input);

    // Text to Ast
    debug!("    -> Parsing");
    let mut ast = parser::parse_root(&mut parsing_ctx)?;

    // Name resolving
    debug!("    -> Resolving");
    ast::resolve(&mut ast, &mut parsing_ctx)?;

    // Lowering to HIR
    debug!("    -> Lowering to HIR");
    let mut hir = ast_lowering::lower_crate(&ast);

    // Infer Hir
    debug!("    -> Infer HIR");
    let new_hir = infer::infer(&mut hir, &mut parsing_ctx, config)?;

    // Generate code
    debug!("    -> Lower to LLVM IR");
    let parsing_ctx = codegen::generate(config, parsing_ctx, new_hir)?;

    parsing_ctx.print_success_diagnostics();

    Ok(())
}

pub mod test {
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

        if let Err(_e) = parse_str(&file, &config) {
            return false;
        }

        let llc_cmd = Command::new("llc")
            .args(&[
                "--relocation-model=pic",
                config.build_folder.join("out.ir").to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute IR -> ASM");

        match llc_cmd.status.code() {
            Some(code) => {
                if code != 0 {
                    println!(
                        "BUG: Cannot compile to ir: \n{}",
                        String::from_utf8(llc_cmd.stderr).unwrap()
                    );

                    return false;
                }
            }
            None => println!(
                "\nError running: \n{}",
                String::from_utf8(llc_cmd.stderr).unwrap()
            ),
        }

        let clang_cmd = Command::new("clang")
            .args(&[
                "-o",
                config.build_folder.join("a.out").to_str().unwrap(),
                config.build_folder.join("out.ir.s").to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute ASM -> BINARY");

        match clang_cmd.status.code() {
            Some(code) => {
                if code != 0 {
                    println!(
                        "BUG: Cannot compile to binary: {}",
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
