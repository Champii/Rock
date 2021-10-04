use colored::*;
use rustyline::{error::ReadlineError, Editor};

use inkwell::{
    builder::Builder,
    context::Context,
    targets::{InitializationConfig, Target},
    OptimizationLevel,
};

use crate::{
    parser::{ParsingCtx, SourceFile},
    ty::Type,
    Config,
};

use super::codegen_context::CodegenContext;

pub fn interpret<'a, 'ctx>(
    _codegen_ctx: &'a mut CodegenContext<'ctx>,
    _builder: &'ctx Builder,
    rock_config: &Config,
) {
    println!(
        "{}{} {}{}\n{}\n\n{}\n",
        "Rock".green(),
        ":".bright_black(),
        "v".bright_cyan(),
        env!("CARGO_PKG_VERSION").cyan(),
        "----".bright_black(),
        "Type ':?' for help".bright_black()
    );

    let config = InitializationConfig::default();

    Target::initialize_native(&config).unwrap();

    let mut commands = vec![];
    let mut toplevels = vec![];

    let mut rl = Editor::<()>::new();

    if rl.load_history("history.txt").is_err() {}

    loop {
        let readline = rl.readline(&"> ".yellow().to_string());

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

    let mut get_type = false;

    if line.starts_with(":?") {
        print_help();

        return;
    }

    if line.starts_with(":t ") {
        get_type = true;

        line.replace_range(0..3, "");
    }

    if line.starts_with("print ") {
        line.replace_range(0..6, "");
    }

    let mut is_top_level = false;

    // FIXME: dirty hack to know if this is a function
    let line_parts = line.split("=").collect::<Vec<_>>();
    if line_parts.len() > 1
        && !line_parts[0].starts_with("let ")
        && line_parts[0].split(" ").count() > 1
    {
        is_top_level = true;
    } else {
        if line.starts_with("use ") || line.starts_with("mod ") {
            is_top_level = true;
        }
    }

    if is_top_level {
        top_levels.push(line.clone());
    } else {
        if !get_type {
            commands.push("  ".to_owned() + &line);
        }
    }

    let mut parsing_ctx = ParsingCtx::new(&config);

    let src =
        SourceFile::from_expr(top_levels.join("\n"), commands.join("\n"), !is_top_level).unwrap();

    parsing_ctx.add_file(&src);

    let hir = match crate::parse_str(&mut parsing_ctx, config) {
        Ok(hir) => hir,
        Err(e) => {
            println!("ERROR {:?}", e);

            if is_top_level {
                top_levels.pop();
            } else {
                if !get_type {
                    commands.pop();
                }
            }

            return;
        }
    };

    if get_type {
        if is_top_level {
            top_levels.pop();
        } else {
            // commands.pop();
        }
    }

    if get_type {
        let t = hir
            .get_function_by_name(line.split(" ").nth(0).unwrap())
            .map(|f| format!("{:?}", Type::from(f.signature.clone())))
            .or_else(|| Some("UNKNOWN".red().to_string()))
            // .or_else(|| {
            //     let custom_fn = hir.get_function_by_name("custom").unwrap();
            //     let custom_body = hir.get_body(&custom_fn.body_id).unwrap();
            //     let hir_id = custom_body.body.get_hir_id();
            //     Some(hir.node_types.get(&hir_id).unwrap().clone())
            // })
            .unwrap();

        println!("{}: {}", line, t);

        return;
    }

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

pub fn print_help() {
    println!(
        "\n{}\n\n  {} : {}\n  {} : {}\n",
        "Help:".bright_green(),
        ":t".bright_yellow(),
        "Print the type of an expression".bright_black(),
        ":?".bright_yellow(),
        "Print This help".bright_black(),
    );
}
