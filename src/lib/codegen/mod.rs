mod codegen_context;

use codegen_context::*;
use inkwell::context::Context;

use crate::{diagnostics::Diagnostic, hir::Root, parser::ParsingCtx, Config};

pub fn generate(
    config: &Config,
    parsing_ctx: ParsingCtx,
    hir: Root,
) -> Result<ParsingCtx, Diagnostic> {
    let context = Context::create();
    let builder = context.create_builder();

    let mut codegen_ctx = CodegenContext::new(&context, parsing_ctx, &hir);
    if codegen_ctx.lower_hir(&hir, &builder).is_err() {
        codegen_ctx.parsing_ctx.return_if_error()?;
    }

    match codegen_ctx.module.verify() {
        Ok(_) => (),
        Err(e) => {
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

    Ok(codegen_ctx.parsing_ctx)
}
