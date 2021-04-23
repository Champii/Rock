#![feature(const_fn, associated_type_bounds)]

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

use diagnostics::Diagnostic;
use parser::{ParsingCtx, SourceFile};

pub use crate::infer::*;
mod ast_lowering;
mod codegen;
mod diagnostics;
mod hir;
mod parser;
mod tests;

pub use crate::helpers::config::Config;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<(), Diagnostic> {
    parse_str(SourceFile::from_file(in_name), out_name, config)
}

pub fn parse_str(
    input: SourceFile,
    _output_name: String,
    config: Config,
) -> Result<(), Diagnostic> {
    let mut parsing_ctx = ParsingCtx::new(&config);

    parsing_ctx.add_file(input.clone());

    // Text to Ast
    info!("    -> Parsing");
    let mut ast = parser::parse_root(&mut parsing_ctx)?;

    // Name resolving
    info!("    -> Resolving");
    ast::resolve(&mut ast, &mut parsing_ctx)?;

    // Lowering to HIR
    info!("    -> Lowering to HIR");
    let mut hir = ast_lowering::lower_crate(&config, &ast);

    // Infer Hir
    info!("    -> Infer HIR");
    infer::infer(&mut hir);

    // Generate code
    info!("    -> Lower to LLVM IR");
    codegen::generate(&config, &hir);

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

    // TODO: compile in each separated folder OR make tests sync
    fn build(build_path: &PathBuf, input: String, config: Config) -> bool {
        let file = SourceFile {
            file_path: PathBuf::from("./src/lib/testcases/mods/main.rk"), // trick for module testing
            mod_path: PathBuf::from("main"),
            content: input,
        };

        if let Err(_e) = parse_str(file, "main".to_string(), config.clone()) {
            return false;
        }

        Command::new("llc")
            .args(&[
                "--relocation-model=pic",
                build_path.join("out.ir").to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute IR -> ASM");

        Command::new("clang")
            .args(&[
                "-o",
                build_path.join("a.out").to_str().unwrap(),
                build_path.join("out.ir.s").to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute ASM -> BINARY");

        true
    }

    pub fn run(path: &str, input: String, config: Config) -> i64 {
        let path = Path::new("./src/lib/").join(path);

        let build_path = path.with_extension("").join("build");

        let mut config = config.clone();
        config.build_folder = build_path.clone();

        fs::create_dir_all(build_path.clone()).unwrap();

        if !build(&build_path, input, config) {
            return -1;
        }

        let cmd = Command::new(build_path.join("a.out").to_str().unwrap())
            .output()
            .expect("failed to execute BINARY");

        fs::remove_dir_all(build_path.parent().unwrap()).unwrap();

        match cmd.status.code() {
            Some(code) => code.into(),
            None => -1,
        }
    }
}
