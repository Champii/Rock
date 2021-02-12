mod codegen_context;

use codegen_context::*;
use inkwell::context::Context;

use crate::{hir::Root, Config};

pub fn generate(config: &Config, hir: &Root) {
    let context = Context::create();
    let builder = context.create_builder();

    let mut codegen_ctx = CodegenContext::new(&context, &hir);
    codegen_ctx.lower_hir(hir, &builder);

    codegen_ctx.module.verify().unwrap();

    if config.show_ir {
        codegen_ctx.module.print_to_stderr();
    }

    codegen_ctx
        .module
        .write_bitcode_to_path(&std::path::Path::new("./out.ir"));
}
