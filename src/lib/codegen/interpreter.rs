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
    let mut print_ir = false;
    let mut print_hir = false;

    if line.starts_with(":?") {
        print_help();

        return;
    }

    if line.starts_with(":i") {
        print_ir = true;
    }

    if line.starts_with(":h") {
        print_hir = true;
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
    let line_parts = line.split('=').collect::<Vec<_>>();
    if line_parts.len() > 1
        && !line_parts[0].starts_with("let ")
        && line_parts[0].split(' ').filter(|x| !x.is_empty()).count() > 1
    {
        is_top_level = true;
    } else if line.starts_with("use ") || line.starts_with("mod ") {
        is_top_level = true;
    }

    if is_top_level {
        top_levels.push(line.clone());
    } else if !get_type && !print_ir && !print_hir {
        commands.push("  ".to_owned() + &line);
    }

    let mut parsing_ctx = ParsingCtx::new(config);

    let src =
        SourceFile::from_expr(top_levels.join("\n"), commands.join("\n"), !is_top_level).unwrap();

    parsing_ctx.add_file(&src);

    let hir = match crate::parse_str(&mut parsing_ctx, config) {
        Ok(hir) => hir,
        Err(_e) => {
            if is_top_level || print_ir || print_hir {
                top_levels.pop();
            } else if !get_type && !print_ir && !print_hir {
                commands.pop();
            }

            return;
        }
    };

    if print_hir {
        hir.print();

        return;
    }

    if get_type {
        if is_top_level || print_ir {
            top_levels.pop();
        } else {
            // commands.pop();
        }
    }

    if get_type {
        let t = hir
            .get_function_by_name(line.split(' ').next().unwrap())
            .map(|f| format!("{:?}", Type::from(f.signature)))
            .or_else(|| Some("UNKNOWN".red().to_string()))
            .unwrap();

        println!("{}: {}", line, t);

        return;
    }

    let context = Context::create();
    let builder = context.create_builder();

    let mut codegen_ctx = CodegenContext::new(&context, &hir);

    if codegen_ctx.lower_hir(&hir, &builder).is_err() {}

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

    if config.show_ir || print_ir {
        codegen_ctx.module.print_to_stderr();
    }

    if print_ir {
        return;
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
        "\n{}\n\n  {} : {}\n  {} : {}\n  {} : {}\n  {} : {}\n",
        "Help:".bright_green(),
        ":h".bright_yellow(),
        "Print the HIR".bright_black(),
        ":i".bright_yellow(),
        "Print the IR".bright_black(),
        ":t".bright_yellow(),
        "Print the type of an expression".bright_black(),
        ":?".bright_yellow(),
        "Print this help".bright_black(),
    );
}
