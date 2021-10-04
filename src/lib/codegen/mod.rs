mod codegen_context;
mod interpreter;

use codegen_context::*;
use inkwell::context::Context;

use crate::{diagnostics::Diagnostic, hir::Root, Config};

pub fn generate(config: &Config, hir: Root) -> Result<(), Diagnostic> {
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

            return Err(Diagnostic::new_empty());
        }
    }

    if !config.no_optimize {
        codegen_ctx.optimize();
    }

    if config.show_ir {
        codegen_ctx.module.print_to_stderr();
    }

    if !codegen_ctx
        .module
        .write_bitcode_to_path(&config.build_folder.join("out.bc"))
    {
        panic!("CANNOT IR WRITE TO PATH");
    }

    Ok(())
}

pub fn interpret(hir: Root, config: &Config) -> Result<(), Diagnostic> {
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

            return Err(Diagnostic::new_empty());
        }
    }

    interpreter::interpret(&mut codegen_ctx, &builder, config);

    // if config.show_ir {
    //     codegen_ctx.module.print_to_stderr();
    // }

    Ok(())
}
