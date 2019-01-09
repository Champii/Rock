use llvm::analysis::*;
use llvm::core::*;
use llvm::execution_engine::*;
use llvm::target::*;
use llvm::target_machine::*;
use llvm::transforms::ipo::*;
use llvm::transforms::scalar::*;
use llvm::*;

use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;

use super::ast::*;
use super::scope::Scopes;

pub struct Context {
    pub scopes: Scopes<*mut LLVMValue>,
    pub functions: Scopes<*mut LLVMValue>,
    pub arguments: Scopes<*mut LLVMValue>,
    pub module: *mut LLVMModule,
    pub context: *mut LLVMContext,
    pub builder: *mut LLVMBuilder,
}

pub fn get_type(t: Box<Type>) -> *mut LLVMType {
    unsafe {
        match t.as_ref() {
            Type::Name(t) => match t.as_ref() {
                "Void" => LLVMVoidType(),
                "Bool" => LLVMInt1Type(),
                "Int" => LLVMInt32Type(),
                "Int32" => LLVMInt32Type(),
                "Int8" => LLVMInt8Type(),
                "String" => LLVMPointerType(LLVMInt8Type(), 0),
                _ => LLVMInt32Type(),
            },
            Type::Array(t) => LLVMPointerType(get_type(t.clone()), 0),
        }
    }
}

pub struct Builder {
    pub context: Context,
    pub source: SourceFile,
}

impl Builder {
    pub fn new(file_name: &str, source: SourceFile) -> Builder {
        unsafe {
            let ctx = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(file_name.as_ptr() as *const _, ctx);
            let builder = LLVMCreateBuilderInContext(ctx);

            let mut context = Context {
                context: ctx,
                module,
                builder,
                scopes: Scopes::new(),
                functions: Scopes::new(),
                arguments: Scopes::new(),
            };

            add_memcpy(&mut context);

            Builder { source, context }
        }
    }

    pub fn build(&mut self) {
        self.source.build(&mut self.context);

        unsafe {
            LLVMDisposeBuilder(self.context.builder);
            // LLVMDumpModule(self.context.module);

            // let mut err = ptr::null_mut();

            // LLVMVerifyModule(
            //     self.context.module,
            //     LLVMVerifierFailureAction::LLVMPrintMessageAction,
            //     &mut err,
            // );
        }
    }

    pub fn write(&mut self, filename: &str) {
        unsafe {
            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargets();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllAsmParsers();
            LLVM_InitializeAllAsmPrinters();

            let pass_manager = LLVMCreateFunctionPassManagerForModule(self.context.module);

            LLVMInitializeFunctionPassManager(pass_manager);

            LLVMAddConstantMergePass(pass_manager);
            LLVMAddDeadArgEliminationPass(pass_manager);
            LLVMAddDeadStoreEliminationPass(pass_manager);
            LLVMAddInstructionCombiningPass(pass_manager);
            // LLVMAddMemorySanitizerPass(pass_manager);
            LLVMAddReassociatePass(pass_manager);

            LLVMFinalizeFunctionPassManager(pass_manager);
            LLVMAddVerifierPass(pass_manager);

            let triple = LLVMGetDefaultTargetTriple();

            let target = LLVMGetFirstTarget();

            let generic = "generic\0";
            let empty = "\0";

            let machine = LLVMCreateTargetMachine(
                target,
                triple,
                generic.as_ptr() as *const _,
                empty.as_ptr() as *const _,
                LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );

            LLVMAddAnalysisPasses(machine, pass_manager);

            let mut error_str = ptr::null_mut();

            let res = LLVMTargetMachineEmitToFile(
                machine,
                self.context.module,
                filename.as_ptr() as *mut _,
                LLVMCodeGenFileType::LLVMObjectFile,
                &mut error_str,
            );

            if res == 1 {
                println!("Cannot generate file {:?}", CStr::from_ptr(error_str));
            }
        }
    }

    pub fn run(&mut self, func_name: &str) -> u64 {
        unsafe {
            let mut ee = mem::uninitialized();
            let mut out = mem::zeroed();

            // robust code should check that these calls complete successfully
            // each of these calls is necessary to setup an execution engine which compiles to native
            // code
            LLVMLinkInMCJIT();
            LLVM_InitializeNativeTarget();
            LLVM_InitializeNativeAsmPrinter();

            // takes ownership of the module
            LLVMCreateExecutionEngineForModule(&mut ee, self.context.module, &mut out);

            let addr = LLVMGetFunctionAddress(ee, func_name.as_ptr() as *const _);

            let f: extern "C" fn() -> u64 = mem::transmute(addr);

            let res = f();

            println!("{}", res);

            // Clean up the rest.
            LLVMDisposeExecutionEngine(ee);
            LLVMContextDispose(self.context.context);

            res
        }
    }
}

fn add_memcpy(context: &mut Context) {
    unsafe {
        let mut args = [
            LLVMPointerType(LLVMIntType(8), 0),
            LLVMPointerType(LLVMIntType(8), 0),
            LLVMIntType(32),
            LLVMIntType(32),
            LLVMIntType(1),
        ];

        let ftMemcpy = LLVMFunctionType(LLVMVoidType(), args.as_mut_ptr(), 5, 0);

        let memcpy = LLVMAddFunction(
            context.module,
            "llvm.memcpy.p0i8.p0i8.i32".as_ptr() as *const _,
            ftMemcpy,
        );

        context.scopes.add("memcpy".to_string(), memcpy);
    }
}

pub trait IrBuilder {
    fn build(&self, ctx: &mut Context) -> Option<*mut LLVMValue>;
}

impl IrBuilder for SourceFile {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        for top in &self.top_levels {
            top.build(context);
        }

        None
    }
}

impl IrBuilder for TopLevel {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            TopLevel::Function(fun) => fun.build(context),
            TopLevel::Prototype(fun) => fun.build(context),
            TopLevel::Mod(_) => None,
        };

        None
    }
}

impl IrBuilder for Prototype {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let name_orig = self.name.clone().unwrap_or("nop".to_string());

        let mut name = name_orig.clone();

        name.push('\0');

        let name = name.as_str();

        unsafe {
            let i32t = LLVMInt32TypeInContext(context.context);
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.clone()));

                argts.push(t);
            }

            let function_type =
                llvm::core::LLVMFunctionType(i32t, argts.as_mut_ptr(), argts.len() as u32, 0);

            let function = llvm::core::LLVMAddFunction(
                context.module,
                name.as_ptr() as *const _,
                function_type,
            );

            context.scopes.add(name_orig.clone(), function);
            context.functions.add(name_orig, function);

            Some(function)
        }
    }
}

impl IrBuilder for FunctionDecl {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let name_orig = self.name.clone();

        let mut name = name_orig.clone();

        name.push('\0');

        let name = name.as_str();

        unsafe {
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.t.clone().unwrap()));

                argts.push(t);
            }

            let function_type = llvm::core::LLVMFunctionType(
                get_type(Box::new(self.ret.clone().unwrap())),
                argts.as_mut_ptr(),
                argts.len() as u32,
                0,
            );

            let function = llvm::core::LLVMAddFunction(
                context.module,
                name.as_ptr() as *const _,
                function_type,
            );

            context.scopes.add(name_orig.clone(), function);
            context.functions.add(name_orig, function);

            context.scopes.push();
            context.functions.push();
            context.arguments.push();

            let mut count = 0;
            for arg in &self.arguments {
                context
                    .scopes
                    .add(arg.name.clone(), LLVMGetParam(function, count));
                context
                    .arguments
                    .add(arg.name.clone(), LLVMGetParam(function, count));

                count += 1;
            }

            let bb = llvm::core::LLVMAppendBasicBlockInContext(
                context.context,
                function,
                b"entry\0".as_ptr() as *const _,
            );

            llvm::core::LLVMPositionBuilderAtEnd(context.builder, bb);

            let res = self.body.build(context);

            match &self.ret {
                Some(Type::Name(t)) => {
                    if *t == "Void".to_string() {
                        llvm::core::LLVMBuildRetVoid(context.builder)
                    } else {
                        llvm::core::LLVMBuildRet(context.builder, res.unwrap())
                    }
                }
                _ => llvm::core::LLVMBuildRet(context.builder, res.unwrap()),
            };

            context.scopes.pop();
            context.functions.pop();
            context.arguments.pop();

            return Some(function);
        }
    }
}

impl IrBuilder for Argument {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        self.arg.build(context)
    }
}

impl IrBuilder for Body {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let mut last = None;

        for stmt in &self.stmts {
            last = stmt.build(context);
        }

        last
    }
}

impl IrBuilder for Statement {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Statement::If(if_) => if_.build(context),
            Statement::Expression(expr) => expr.build(context),
            Statement::Assignation(assign) => assign.build(context),
        }
    }
}

impl IrBuilder for If {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        unsafe {
            let func = LLVMGetLastFunction(context.module);

            let base_block = LLVMGetLastBasicBlock(func);

            let predicat = self.predicat.build(context).unwrap();

            let icmp = LLVMBuildICmp(
                context.builder,
                LLVMIntPredicate::LLVMIntNE,
                predicat,
                LLVMConstInt(LLVMTypeOf(predicat), 0, 0),
                "\0".as_ptr() as *const _,
            );

            let if_body = LLVMAppendBasicBlockInContext(
                context.context,
                func,
                b"then\0".as_ptr() as *const _,
            );

            LLVMPositionBuilderAtEnd(context.builder, if_body);

            let body = self.body.build(context).unwrap();

            let final_block = LLVMAppendBasicBlockInContext(
                context.context,
                func,
                "final\0".as_ptr() as *const _,
            );

            LLVMBuildBr(context.builder, final_block);

            let res_block = if let Some(else_) = self.else_.clone() {
                LLVMPositionBuilderAtEnd(context.builder, final_block);

                let else_body = LLVMAppendBasicBlockInContext(
                    context.context,
                    func,
                    b"else\0".as_ptr() as *const _,
                );

                LLVMPositionBuilderAtEnd(context.builder, else_body);

                let res = else_.build(context).unwrap();

                LLVMBuildBr(context.builder, final_block);

                LLVMMoveBasicBlockAfter(final_block, else_body);

                else_body
            } else {
                final_block
            };

            LLVMPositionBuilderAtEnd(context.builder, base_block);

            LLVMBuildCondBr(context.builder, icmp, if_body, res_block);

            LLVMPositionBuilderAtEnd(context.builder, final_block);

            Some(body)
        }
    }
}

impl IrBuilder for Else {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Else::If(if_) => if_.build(context),
            Else::Body(body) => body.build(context),
        }
    }
}

impl IrBuilder for Expression {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Expression::BinopExpr(unary, op, expr) => {
                let left = unary.build(context).unwrap();
                let right = expr.build(context).unwrap();

                Some(match op {
                    Operator::Add => unsafe {
                        LLVMBuildAdd(context.builder, left, right, b"\0".as_ptr() as *const _)
                    },
                    Operator::EqualEqual => unsafe {
                        LLVMBuildICmp(
                            context.builder,
                            LLVMIntPredicate::LLVMIntEQ,
                            left,
                            right,
                            "\0".as_ptr() as *const _,
                        )
                    },
                    _ => unsafe {
                        LLVMBuildAdd(context.builder, left, right, b"\0".as_ptr() as *const _)
                    },
                })
            }
            Expression::UnaryExpr(unary) => unary.build(context),
        }
    }
}

impl IrBuilder for Assignation {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        unsafe {
            let ptr = if let Some(val) = context.scopes.get(self.name.clone()) {
                val
            } else {
                let mut alloc_name = "alloc_".to_string() + &self.name.clone();

                alloc_name.push('\0');

                let alloc = LLVMBuildAlloca(
                    context.builder,
                    get_type(Box::new(self.t.clone().unwrap())),
                    alloc_name.as_ptr() as *const _,
                );

                context.scopes.add(self.name.clone(), alloc);

                alloc
            };

            let val = self.value.build(context).unwrap();

            LLVMBuildStore(context.builder, val, ptr);

            Some(val)
        }
    }
}

impl IrBuilder for UnaryExpr {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.build(context),
            UnaryExpr::UnaryExpr(op, unary) => unary.build(context),
        }
    }
}

impl IrBuilder for PrimaryExpr {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            PrimaryExpr::PrimaryExpr(operand, vec) => {
                let op = operand.build(context);

                if vec.len() == 0 {
                    return op;
                }

                let op = op.unwrap();

                let second = vec.first().unwrap();

                second.build_with(context, op)
            }
        }
    }
}

impl SecondaryExpr {
    pub fn build_with(&self, context: &mut Context, op: *mut LLVMValue) -> Option<*mut LLVMValue> {
        match self {
            SecondaryExpr::Arguments(args) => {
                let mut res = vec![];

                for arg in args {
                    res.push(arg.build(context).unwrap());
                }

                unsafe {
                    Some(LLVMBuildCall(
                        context.builder,
                        op,
                        res.as_mut_ptr(),
                        res.len() as u32,
                        b"\0".as_ptr() as *const _,
                    ))
                }
            }
            _ => None,
        }
    }
}

impl IrBuilder for Operand {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Operand::Literal(lit) => lit.build(context),
            Operand::Identifier(ident) => {
                if let Some(func) = context.arguments.get(ident.clone()) {
                    return Some(func);
                }

                if let Some(func) = context.functions.get(ident.clone()) {
                    return Some(func);
                }

                if let Some(ptr) = context.scopes.get(ident.clone()) {
                    unsafe {
                        let mut ident = ident.clone();

                        ident.push('\0');

                        Some(LLVMBuildLoad(
                            context.builder,
                            ptr,
                            ident.as_ptr() as *const _,
                        ))
                    }
                } else {
                    panic!("Unknown identifier {}", ident);
                    // None
                }
            }
            Operand::Expression(expr) => expr.build(context),
        }
    }
}

impl IrBuilder for Literal {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Literal::Number(num) => unsafe { Some(LLVMConstInt(LLVMInt32Type(), *num, 0)) },
            Literal::String(s) => s.build(context),
            Literal::Bool(b) => unsafe { Some(LLVMConstInt(LLVMInt1Type(), b.clone(), 0)) },
            _ => None,
        }
    }
}

impl IrBuilder for String {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let mut me = self.clone();

        me.push('\0');

        unsafe {
            let s = LLVMBuildGlobalStringPtr(
                context.builder,
                me.as_ptr() as *const i8,
                b"\0".as_ptr() as *const _,
            );

            let pointer = LLVMBuildArrayAlloca(
                context.builder,
                LLVMInt8Type(),
                s,
                b"\0".as_ptr() as *const _,
            );

            let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
            let mut indices = [zero];

            let ptr_elem = LLVMBuildGEP(
                context.builder,
                pointer,
                indices.as_mut_ptr(),
                1,
                b"\0".as_ptr() as *const _,
            );

            let ptr8 = LLVMBuildBitCast(
                context.builder,
                ptr_elem,
                LLVMPointerType(LLVMInt8Type(), 0),
                b"\0".as_ptr() as *const _,
            );
            let gptr8 = LLVMBuildBitCast(
                context.builder,
                s,
                LLVMPointerType(LLVMInt8Type(), 0),
                b"\0".as_ptr() as *const _,
            );

            let mut args = [
                ptr8,
                gptr8,
                LLVMConstInt(LLVMInt32Type(), me.len() as u64, 0),
                LLVMConstInt(LLVMIntType(32), 1, 0),
                LLVMConstInt(LLVMIntType(1), 1, 0),
            ];

            LLVMBuildCall(
                context.builder,
                context.scopes.get("memcpy".to_string()).unwrap(),
                args.as_mut_ptr(),
                5,
                b"\0".as_ptr() as *const _,
            );

            Some(ptr8)
        }
    }
}
