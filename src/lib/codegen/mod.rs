mod codegen_context;

use codegen_context::*;
use inkwell::context::Context;

use crate::{
    diagnostics::Diagnostic,
    hir::{HirId, Identifier, Prototype, Root, TopLevel, TopLevelKind},
    ty::{FuncType, PrimitiveType, Type},
    Config,
};

pub fn add_builtins(hir: &mut Root) {
    let mut builtins = Vec::new();

    builtins.push(TopLevel {
        kind: TopLevelKind::Extern(Prototype {
            name: Identifier {
                name: "malloc".to_string(),
                hir_id: HirId(9999996), // FIXME: what if this hir_id already exists ?
            },
            signature: FuncType {
                arguments: vec![Type::Primitive(PrimitiveType::Int64)],
                ret: Box::new(Type::Primitive(PrimitiveType::String)),
            },
            hir_id: HirId(9999997), // FIXME: bis
        }),
    });

    builtins.push(TopLevel {
        kind: TopLevelKind::Extern(Prototype {
            name: Identifier {
                name: "free".to_string(),
                hir_id: HirId(9999998), // FIXME: what if this hir_id already exists ?
            },
            signature: FuncType {
                arguments: vec![Type::Primitive(PrimitiveType::String)],
                ret: Box::new(Type::Primitive(PrimitiveType::Int64)),
            },
            hir_id: HirId(9999999), // FIXME: bis
        }),
    });

    hir.top_levels.append(&mut builtins);
}

pub fn generate(config: &Config, mut hir: Root) -> Result<(), Diagnostic> {
    let context = Context::create();
    let builder = context.create_builder();

    add_builtins(&mut hir);

    let mut codegen_ctx = CodegenContext::new(&context, &hir);

    if let Err(e) = codegen_ctx.lower_hir(&hir, &builder) {
        // FIXME: have a movable `Diagnostics`
        // codegen_ctx.parsing_ctx.return_if_error()?;
        codegen_ctx.module.print_to_stderr();
        panic!("GEN ERROR {:#?}", e);
    }

    // codegen_ctx.module.print_to_stderr();

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
        // optimize further
        // FIXME: find a way to organize the pass-manager better to avoid the double pass
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
