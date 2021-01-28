use inkwell::context::Context;
use inkwell::module::Module;

use crate::hir::*;

pub struct CodegenContext<'a> {
    pub context: &'a Context,
    pub module: Module<'a>,
}

impl<'a> CodegenContext<'a> {
    pub fn new(context: &'a Context) -> Self {
        let module: Module<'a> = context.create_module("mod");

        Self { context, module }
    }

    pub fn lower_hir(&mut self, root: &Root) {
        for (_, item) in &root.top_levels {
            match &item.kind {
                TopLevelKind::Function(f) => self.lower_function_decl(&f),
            }
        }

        for (_, body) in &root.bodies {
            self.lower_body(&body);
        }

        self.module.print_to_stderr();
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) {
        let void_type = self.context.void_type();
        let fn_type = void_type.fn_type(&[], false);
        let function = self.module.add_function(&f.name.name, fn_type, None);
    }

    pub fn lower_body(&mut self, body: &Body) {
        //
        match self.module.get_function(&body.name.name) {
            Some(f) => {
                let basic_block = self.context.append_basic_block(f, "entry");
            }
            None => (),
        }
    }
}
