#![feature(associated_type_bounds, destructuring_assignment)]

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

use crate::helpers::config::PackageMetaData;
pub use crate::infer::*;
mod ast_lowering;
mod codegen;
pub mod diagnostics;
mod hir;
mod parser;
mod tests;

pub use crate::helpers::config::Config;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<(), Diagnostic> {
    parse_str(SourceFile::from_file(in_name)?, out_name, config)
}

pub fn parse_str(
    input: SourceFile,
    _output_name: String,
    config: Config,
) -> Result<(), Diagnostic> {
    let mut parsing_ctx = ParsingCtx::new(&config);

    parsing_ctx.add_file(input.clone());

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
    infer::infer(&mut hir, &mut parsing_ctx, &config)?;

    // Generate code
    debug!("    -> Lower to LLVM IR");
    codegen::generate(&config, &hir)?;

    debug!("    -> Save MetaData");
    PackageMetaData { hir }
        .store(&PathBuf::from("/tmp/test.serde"))
        .unwrap();

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
            file_path: PathBuf::from("./src/lib").join(config.project_config.entry_point.clone()),
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
