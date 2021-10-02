use std::io::{self, BufRead, Write};

use inkwell::{
    context::Context,
    targets::{InitializationConfig, Target},
    OptimizationLevel,
};

use crate::{
    ast::{self, Statement},
    hir,
    parser::{Parse, ParsingCtx, SourceFile},
    ty::Type,
    Config,
};

use super::codegen_context::CodegenContext;

pub fn interpret(codegen_ctx: &CodegenContext) {
    let config = InitializationConfig::default();

    Target::initialize_native(&config).unwrap();

    let stdin = io::stdin();

    let mut commands = vec![];
    let rock_config = Config::default();

    prompt();

    for line in stdin.lock().lines() {
        let line = line.as_ref().unwrap().clone();

        if line == "exit" || line == "quit" {
            break;
        }

        commands.push("  ".to_owned() + &line);

        let mut parsing_ctx = ParsingCtx::new(&rock_config);

        let src = SourceFile::from_expr(commands.join("\n")).unwrap();

        parsing_ctx.add_file(&src);

        let hir = match crate::parse_str(parsing_ctx, &Config::default()) {
            Ok(hir) => hir,
            Err(_) => {
                commands.pop();

                prompt();

                continue;
            }
        };

        let context = Context::create();
        let builder = context.create_builder();

        let mut codegen_ctx = CodegenContext::new(&context, &hir);

        if codegen_ctx.lower_hir(&hir, &builder).is_err() {
            // FIXME: have a movable `Diagnostics`
            // codegen_ctx.parsing_ctx.return_if_error()?;
        }

        match codegen_ctx.module.verify() {
            Ok(_) => (),
            Err(e) => {
                codegen_ctx.module.print_to_stderr();

                println!("Error: Bug in the generated IR:\n\n{}", e.to_string());

                return;
            }
        }

        let engine = codegen_ctx
            .module
            .create_jit_execution_engine(OptimizationLevel::None)
            .unwrap();

        unsafe {
            engine.run_function_as_main(engine.get_function_value("main").unwrap(), &[]);
        }

        prompt();
    }
}

fn prompt() {
    print!("> ");

    std::io::stdout().lock().flush().unwrap();
}
