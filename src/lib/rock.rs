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

// use crate::helpers::config::PackageMetaData;
pub use crate::infer::*;
mod ast_lowering;
mod codegen;
pub mod diagnostics;
mod hir;
mod parser;
mod tests;

pub use crate::helpers::config::Config;

pub fn parse_file(in_name: String, out_name: String, config: Config) -> Result<(), Diagnostic> {
    let mut source_file = SourceFile::from_file(in_name)?;

    source_file.mod_path = PathBuf::from("root");

    parse_str(source_file, out_name, config)
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
    let new_hir = infer::infer(&mut hir, &mut parsing_ctx, &config)?;

    // Generate code
    debug!("    -> Lower to LLVM IR");
    let parsing_ctx = codegen::generate(&config, parsing_ctx, new_hir)?;

    // debug!("    -> Save MetaData");
    // PackageMetaData { hir }
    //     .store(&PathBuf::from("/tmp/test.serde"))
    //     .unwrap();

    parsing_ctx.print_success_diagnostics();

    Ok(())
}

pub mod test {
    use super::*;
    use crate::{parser::SourceFile, Config};
    use std::{
        fs::{self, DirEntry},
        path::{Path, PathBuf},
        process::Command,
    };
    // one possible implementation of walking a directory only visiting files
    fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, cb)?;
                } else {
                    cb(&entry);
                }
            }
        }
        Ok(())
    }

    fn build(build_path: &PathBuf, input: String, config: Config) -> bool {
        println!("BUILD PATH: {:?}", build_path);
        println!("CONFIG: {:?}", config);
        let file = SourceFile {
            file_path: PathBuf::from("./src/lib").join(config.project_config.entry_point.clone()),
            mod_path: PathBuf::from("main"),
            content: input,
        };
        println!("FILE: {:?}", file);
        println!("ENV: {:?}", env!("PWD"));
        println!("DIR: {:?}", std::fs::read_dir(env!("PWD")));

        if let Err(_e) = parse_str(file, "main".to_string(), config.clone()) {
            return false;
        }
        visit_dirs(&PathBuf::from(env!("PWD")), &|file_path: &DirEntry| {
            println!(
                "{:?}",
                fs::metadata(file_path.path()).unwrap().permissions()
            )
        })
        .unwrap();

        let llc_cmd = Command::new("llc")
            .args(&[
                "--relocation-model=pic",
                build_path.join("out.ir").to_str().unwrap(),
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
                build_path.join("a.out").to_str().unwrap(),
                build_path.join("out.ir.s").to_str().unwrap(),
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
