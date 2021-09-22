mod codegen_context;

use codegen_context::*;
use inkwell::context::Context;

use crate::{diagnostics::Diagnostic, hir::Root, parser::ParsingCtx, Config};

pub fn generate<'a>(
    config: &Config,
    parsing_ctx: ParsingCtx,
    hir: Root,
) -> Result<ParsingCtx, Diagnostic> {
    let context = Context::create();
    let builder = context.create_builder();

    let mut codegen_ctx = CodegenContext::new(&context, parsing_ctx, &hir);
    if let Err(_) = codegen_ctx.lower_hir(&hir, &builder) {
        codegen_ctx.parsing_ctx.return_if_error()?;
    }

    if config.show_ir {
        codegen_ctx.module.print_to_stderr();
    }

    match codegen_ctx.module.verify() {
        Ok(_) => (),
        Err(e) => {
            println!("Error: Bug in the generated IR:\n\n{}", e.to_string());

            return Err(Diagnostic::new_empty());
        }
    }

    if !codegen_ctx
        .module
        .write_bitcode_to_path(&config.build_folder.join("out.ir"))
    {
        panic!("CANNOT IR WRITE TO PATH");
    }

    Ok(codegen_ctx.parsing_ctx)
}
