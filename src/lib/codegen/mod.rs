mod codegen_context;

use codegen_context::*;
use inkwell::context::Context;

use crate::{diagnostics::Diagnostic, hir::Root, Config};

pub fn generate(config: &Config, hir: &Root) -> Result<(), Diagnostic> {
    let context = Context::create();
    let builder = context.create_builder();

    let mut codegen_ctx = CodegenContext::new(&context, &hir);
    codegen_ctx.lower_hir(hir, &builder);

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

    codegen_ctx
        .module
        .write_bitcode_to_path(&config.build_folder.join("out.ir"));

    Ok(())
}
