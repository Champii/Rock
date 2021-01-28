use llvm::core::*;
use llvm::*;

use crate::ast::*;
use crate::scope::Scopes;
use crate::Config;

pub struct IrContext {
    pub scopes: Scopes<*mut LLVMValue>,
    pub functions: Scopes<*mut LLVMValue>,
    pub arguments: Scopes<*mut LLVMValue>,
    pub module: *mut LLVMModule,
    pub context: *mut LLVMContext,
    pub builder: *mut LLVMBuilder,
}

pub struct Builder {
    pub context: IrContext,
    pub source: Root,
    pub config: Config,
}

impl Builder {
    pub fn new(file_name: &str, source: Root, config: Config) -> Builder {
        unsafe {
            let ctx = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(file_name.as_ptr() as *const _, ctx);
            let builder = LLVMCreateBuilderInContext(ctx);

            let context = IrContext {
                context: ctx,
                module,
                builder,
                scopes: Scopes::new(),
                functions: Scopes::new(),
                arguments: Scopes::new(),
            };

            Builder {
                source,
                context,
                config,
            }
        }
    }
}

pub trait IrBuilder {
    fn build(&self, ctx: &mut IrContext) -> Option<*mut LLVMValue>;
}
