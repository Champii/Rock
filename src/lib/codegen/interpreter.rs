use colored::*;
use inkwell::{
    builder::Builder,
    context::Context,
    targets::{InitializationConfig, Target},
    OptimizationLevel,
};
use rustyline::{error::ReadlineError, Editor};

use crate::{
    parser::{ParsingCtx, SourceFile},
    Config,
};

use super::codegen_context::CodegenContext;

pub fn interpret<'a, 'ctx>(
    _codegen_ctx: &'a mut CodegenContext<'ctx>,
    _builder: &'ctx Builder,
    rock_config: &Config,
) {
    println!(
        "{}{} {}{}\n{}\n",
        "Rock".green(),
        ":".bright_black(),
        "v".bright_cyan(),
        env!("CARGO_PKG_VERSION").cyan(),
        "----".bright_black()
    );

    let config = InitializationConfig::default();

    Target::initialize_native(&config).unwrap();

    let mut commands = vec![];
    let mut toplevels = vec![];

    let mut rl = Editor::<()>::new();

    if rl.load_history("history.txt").is_err() {}

    loop {
        let readline = rl.readline(&"> ".bright_black());

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());

                if line == "exit" || line == "quit" {
                    break;
                }

                process_line(line, &mut commands, &mut toplevels, rock_config);
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    rl.save_history("history.txt").unwrap();
}

fn process_line(
    mut line: String,
    commands: &mut Vec<String>,
    top_levels: &mut Vec<String>,
    config: &Config,
) {
    if line.is_empty() {
        return;
    }

    if line.starts_with("print ") {
        line.replace_range(0..6, "");
    }

    if line.starts_with("use ") || line.starts_with("mod ") {
        top_levels.push(line.clone());
    } else {
        commands.push("  ".to_owned() + &line);
    }

    let mut parsing_ctx = ParsingCtx::new(&config);

    let src = SourceFile::from_expr(commands.join("\n"), top_levels.join("\n")).unwrap();

    parsing_ctx.add_file(&src);

    let hir = match crate::parse_str(&mut parsing_ctx, config) {
        Ok(hir) => hir,
        Err(_) => {
            commands.pop();

            return;
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

    if !config.no_optimize {
        codegen_ctx.optimize();
    }

    if config.show_ir {
        codegen_ctx.module.print_to_stderr();
    }

    let engine = codegen_ctx
        .module
        .create_jit_execution_engine(OptimizationLevel::None)
        .unwrap();

    unsafe {
        engine.run_function_as_main(engine.get_function_value("main").unwrap(), &[]);
    }
}
