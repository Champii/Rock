use llvm::analysis::*;
use llvm::core::*;
use llvm::execution_engine::*;
use llvm::target::*;
use llvm::target_machine::*;
use llvm::transforms::ipo::*;
use llvm::transforms::scalar::*;
use llvm::*;

use std::collections::HashMap;
use std::ffi::CStr;
use std::mem;
use std::ptr;

use super::ast::*;
use super::scope::Scopes;
use super::Config;

pub struct IrContext {
    pub scopes: Scopes<*mut LLVMValue>,
    pub functions: Scopes<*mut LLVMValue>,
    // pub classes: HashMap<String, (*mut LLVMType, Class)>, // type -> (...)
    pub arguments: Scopes<*mut LLVMValue>,
    pub module: *mut LLVMModule,
    pub context: *mut LLVMContext,
    pub builder: *mut LLVMBuilder,
}

// pub fn get_type(t: Box<Type>, context: &mut IrContext) -> *mut LLVMType {
//     unsafe {
//         match t.as_ref() {
//             Type::Primitive(p) => match p {
//                 PrimitiveType::Void => LLVMVoidType(),
//                 PrimitiveType::Bool => LLVMInt1Type(),
//                 PrimitiveType::Int8 => LLVMInt8Type(),
//                 PrimitiveType::Int16 => LLVMInt16Type(),
//                 PrimitiveType::Int32 => LLVMInt32Type(),
//                 PrimitiveType::Int64 => LLVMInt64Type(),
//                 PrimitiveType::String(_) => LLVMPointerType(LLVMInt8Type(), 0),
//                 PrimitiveType::Array(t, _) => LLVMPointerType(get_type(t.clone(), context), 0),
//                 // _ => LLVMPointerType(context.classes.get(t).unwrap().clone().0, 0),
//                 // _ => context.classes.get(t).unwrap().clone().0,
//             },
//             Type::Class(c) => LLVMPointerType(context.classes.get(c).unwrap().clone().0, 0),
//             Type::Proto(proto) => get_type(Box::new(proto.ret.clone()), context),
//             Type::FuncType(_) => panic!("Codegen: Type::FuncType not implemented"),
//             Type::ForAll(_) => panic!("Codegen: Type::ForAll not implemented"),
//             Type::Undefined(name) => panic!("Codegen: Type::Undefined({}) detected !", name),
//         }
//     }
// }

pub struct Builder {
    pub context: IrContext,
    pub source: SourceFile,
    pub config: Config,
}

impl Builder {
    pub fn new(file_name: &str, source: SourceFile, config: Config) -> Builder {
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
                // classes: HashMap::new(),
                arguments: Scopes::new(),
            };

            Builder {
                source,
                context,
                config,
            }
        }
    }

    // pub fn build(&mut self) {
    //     self.source.build(&mut self.context);

    //     unsafe {
    //         LLVMDisposeBuilder(self.context.builder);

    //         if self.config.show_ir {
    //             LLVMDumpModule(self.context.module);

    //             let mut err = ptr::null_mut();

    //             LLVMVerifyModule(
    //                 self.context.module,
    //                 LLVMVerifierFailureAction::LLVMPrintMessageAction,
    //                 &mut err,
    //             );
    //         }
    //     }
    // }

    // pub fn write(&mut self, filename: &str) {
    //     unsafe {
    //         LLVM_InitializeAllTargetInfos();
    //         LLVM_InitializeAllTargets();
    //         LLVM_InitializeAllTargetMCs();
    //         LLVM_InitializeAllAsmParsers();
    //         LLVM_InitializeAllAsmPrinters();

    //         let pass_manager = LLVMCreateFunctionPassManagerForModule(self.context.module);

    //         LLVMInitializeFunctionPassManager(pass_manager);

    //         LLVMAddConstantMergePass(pass_manager);
    //         LLVMAddDeadArgEliminationPass(pass_manager);
    //         LLVMAddDeadStoreEliminationPass(pass_manager);
    //         LLVMAddInstructionCombiningPass(pass_manager);
    //         // LLVMAddMemorySanitizerPass(pass_manager);
    //         LLVMAddReassociatePass(pass_manager);

    //         LLVMFinalizeFunctionPassManager(pass_manager);
    //         LLVMAddVerifierPass(pass_manager);

    //         let triple = LLVMGetDefaultTargetTriple();

    //         let target = LLVMGetFirstTarget();

    //         let generic = "generic\0";
    //         let empty = "\0";

    //         let machine = LLVMCreateTargetMachine(
    //             target,
    //             triple,
    //             generic.as_ptr() as *const _,
    //             empty.as_ptr() as *const _,
    //             LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
    //             LLVMRelocMode::LLVMRelocDefault,
    //             LLVMCodeModel::LLVMCodeModelDefault,
    //         );

    //         LLVMAddAnalysisPasses(machine, pass_manager);

    //         let mut error_str = ptr::null_mut();

    //         let res = LLVMTargetMachineEmitToFile(
    //             machine,
    //             self.context.module,
    //             filename.as_ptr() as *mut _,
    //             LLVMCodeGenFileType::LLVMObjectFile,
    //             &mut error_str,
    //         );

    //         if res == 1 {
    //             println!("Cannot generate file {:?}", CStr::from_ptr(error_str));
    //         }
    //     }
    // }

    // pub fn run(&mut self, func_name: &str) -> u64 {
    //     unsafe {
    //         // let mut ee = mem::uninitialized();
    //         let mut ee = mem::MaybeUninit::uninit().assume_init();
    //         let mut out = mem::zeroed();

    //         // robust code should check that these calls complete successfully
    //         // each of these calls is necessary to setup an execution engine which compiles to native
    //         // code
    //         LLVMLinkInMCJIT();

    //         if LLVM_InitializeNativeTarget() == 1 {
    //             panic!("WOOT1");
    //         }
    //         if LLVM_InitializeNativeAsmPrinter() == 1 {
    //             panic!("WOOT2");
    //         }

    //         let mut opts: LLVMMCJITCompilerOptions =
    //             mem::MaybeUninit::<LLVMMCJITCompilerOptions>::uninit().assume_init();

    //         LLVMInitializeMCJITCompilerOptions(
    //             &mut opts,
    //             mem::size_of::<LLVMMCJITCompilerOptions>(),
    //         );

    //         opts.CodeModel = LLVMCodeModel::LLVMCodeModelDefault;

    //         if LLVMCreateMCJITCompilerForModule(
    //             &mut ee,
    //             self.context.module,
    //             &mut opts,
    //             mem::size_of::<LLVMMCJITCompilerOptions>(),
    //             &mut out,
    //         ) == 1
    //         {
    //             panic!("WOOT3");
    //         };

    //         let addr = LLVMGetFunctionAddress(ee, func_name.as_ptr() as *const _);

    //         let f: extern "C" fn() -> u64 = mem::transmute(addr);

    //         let res = f();

    //         // Clean up the rest.
    //         LLVMDisposeExecutionEngine(ee);
    //         LLVMContextDispose(self.context.context);

    //         res
    //     }
    // }
}

pub trait IrBuilder {
    fn build(&self, ctx: &mut IrContext) -> Option<*mut LLVMValue>;
}
