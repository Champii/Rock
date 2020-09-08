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

pub struct Context {
    pub scopes: Scopes<*mut LLVMValue>,
    pub functions: Scopes<*mut LLVMValue>,
    pub classes: HashMap<String, (*mut LLVMType, Class)>, // type -> (...)
    pub arguments: Scopes<*mut LLVMValue>,
    pub module: *mut LLVMModule,
    pub context: *mut LLVMContext,
    pub builder: *mut LLVMBuilder,
}

pub fn get_type(t: Box<Type>, context: &mut Context) -> *mut LLVMType {
    unsafe {
        match t.as_ref() {
            Type::Primitive(p) => match p {
                PrimitiveType::Void => LLVMVoidType(),
                PrimitiveType::Bool => LLVMInt1Type(),
                PrimitiveType::Int8 => LLVMInt8Type(),
                PrimitiveType::Int16 => LLVMInt16Type(),
                PrimitiveType::Int | PrimitiveType::Int32 => LLVMInt32Type(),
                PrimitiveType::Int64 => LLVMInt64Type(),
                PrimitiveType::String(_) => LLVMPointerType(LLVMInt8Type(), 0),
                PrimitiveType::Array(t, _) => LLVMPointerType(get_type(t.clone(), context), 0),
                // _ => LLVMPointerType(context.classes.get(t).unwrap().clone().0, 0),
                // _ => context.classes.get(t).unwrap().clone().0,
            },
            Type::Class(c) => LLVMPointerType(context.classes.get(c).unwrap().clone().0, 0),
            Type::Proto(_) => panic!("Codegen: Type::Proto not implemented"),
            Type::FuncType(_) => panic!("Codegen: Type::FuncType not implemented"),
            Type::ForAll(_) => panic!("Codegen: Type::ForAll not implemented"),
            Type::Undefined(name) => panic!("Codegen: Type::Undefined({}) detected !", name),
        }
    }
}

pub struct Builder {
    pub context: Context,
    pub source: SourceFile,
    pub config: Config,
}

impl Builder {
    pub fn new(file_name: &str, source: SourceFile, config: Config) -> Builder {
        unsafe {
            let ctx = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(file_name.as_ptr() as *const _, ctx);
            let builder = LLVMCreateBuilderInContext(ctx);

            let context = Context {
                context: ctx,
                module,
                builder,
                scopes: Scopes::new(),
                functions: Scopes::new(),
                classes: HashMap::new(),
                arguments: Scopes::new(),
            };

            Builder {
                source,
                context,
                config,
            }
        }
    }

    pub fn build(&mut self) {
        self.source.build(&mut self.context);

        unsafe {
            LLVMDisposeBuilder(self.context.builder);

            if self.config.show_ir {
                LLVMDumpModule(self.context.module);

                let mut err = ptr::null_mut();

                LLVMVerifyModule(
                    self.context.module,
                    LLVMVerifierFailureAction::LLVMPrintMessageAction,
                    &mut err,
                );
            }
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
            // let mut ee = mem::uninitialized();
            let mut ee = mem::MaybeUninit::uninit().assume_init();
            let mut out = mem::zeroed();

            // robust code should check that these calls complete successfully
            // each of these calls is necessary to setup an execution engine which compiles to native
            // code
            LLVMLinkInMCJIT();

            if LLVM_InitializeNativeTarget() == 1 {
                panic!("WOOT1");
            }
            if LLVM_InitializeNativeAsmPrinter() == 1 {
                panic!("WOOT2");
            }

            let mut opts: LLVMMCJITCompilerOptions =
                mem::MaybeUninit::<LLVMMCJITCompilerOptions>::uninit().assume_init();

            LLVMInitializeMCJITCompilerOptions(
                &mut opts,
                mem::size_of::<LLVMMCJITCompilerOptions>(),
            );

            opts.CodeModel = LLVMCodeModel::LLVMCodeModelDefault;

            if LLVMCreateMCJITCompilerForModule(
                &mut ee,
                self.context.module,
                &mut opts,
                mem::size_of::<LLVMMCJITCompilerOptions>(),
                &mut out,
            ) == 1
            {
                panic!("WOOT3");
            };

            let addr = LLVMGetFunctionAddress(ee, func_name.as_ptr() as *const _);

            let f: extern "C" fn() -> u64 = mem::transmute(addr);

            let res = f();

            // Clean up the rest.
            LLVMDisposeExecutionEngine(ee);
            LLVMContextDispose(self.context.context);

            res
        }
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
            TopLevel::Class(class) => class.build(context),
            TopLevel::Function(fun) => fun.build(context),
            TopLevel::Prototype(fun) => fun.build(context),
            TopLevel::Mod(_) => None,
        };

        None
    }
}

impl IrBuilder for Class {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let mut attrs_types = vec![];

        for attr in self.attributes.clone() {
            attrs_types.push(get_type(Box::new(attr.t.clone().unwrap()), context));
        }

        unsafe {
            let t = LLVMStructType(attrs_types.as_ptr() as *mut _, attrs_types.len() as u32, 0);

            context.classes.insert(self.name.clone(), (t, self.clone()));
        }

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
            let i32t = LLVMInt32Type();
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.clone()), context);

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

        if !self.is_solved() {
            panic!("CODEGEN: FuncDecl is not solved {}", self.name);
        }

        unsafe {
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.t.clone().unwrap()), context);

                argts.push(t);
            }

            let function_type = llvm::core::LLVMFunctionType(
                get_type(Box::new(self.ret.clone().unwrap()), context),
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
                Some(Type::Primitive(p)) => match p {
                    PrimitiveType::Void => llvm::core::LLVMBuildRetVoid(context.builder),
                    _ => llvm::core::LLVMBuildRet(context.builder, res.unwrap()),
                },
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
        match &self.kind {
            StatementKind::If(if_) => if_.build(context),
            StatementKind::For(for_) => for_.build(context),
            StatementKind::Expression(expr) => expr.build(context),
            StatementKind::Assignation(assign) => assign.build(context),
        }
    }
}

impl IrBuilder for For {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            For::In(in_) => in_.build(context),
            For::While(while_) => while_.build(context),
        }
    }
}

impl IrBuilder for ForIn {
    fn build(&self, _context: &mut Context) -> Option<*mut LLVMValue> {
        panic!("ForIn: Uninplemented");
    }
}

impl IrBuilder for While {
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

            let for_body = LLVMAppendBasicBlockInContext(
                context.context,
                func,
                b"then\0".as_ptr() as *const _,
            );

            LLVMPositionBuilderAtEnd(context.builder, for_body);

            let body = self.body.build(context).unwrap();

            let res_block = LLVMAppendBasicBlockInContext(
                context.context,
                func,
                "final\0".as_ptr() as *const _,
            );

            let predicat2 = self.predicat.build(context).unwrap();

            let icmp2 = LLVMBuildICmp(
                context.builder,
                LLVMIntPredicate::LLVMIntNE,
                predicat2,
                LLVMConstInt(LLVMTypeOf(predicat), 0, 0),
                "\0".as_ptr() as *const _,
            );

            LLVMBuildCondBr(context.builder, icmp2, for_body, res_block);

            LLVMPositionBuilderAtEnd(context.builder, base_block);

            LLVMBuildCondBr(context.builder, icmp, for_body, res_block);

            LLVMPositionBuilderAtEnd(context.builder, res_block);

            Some(body)
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

                else_.build(context).unwrap();

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
        match &self.kind {
            ExpressionKind::BinopExpr(unary, op, expr) => {
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
                    Operator::DashEqual => unsafe {
                        LLVMBuildICmp(
                            context.builder,
                            LLVMIntPredicate::LLVMIntNE,
                            left,
                            right,
                            "\0".as_ptr() as *const _,
                        )
                    },
                    Operator::Less => unsafe {
                        LLVMBuildICmp(
                            context.builder,
                            LLVMIntPredicate::LLVMIntSLT,
                            left,
                            right,
                            "\0".as_ptr() as *const _,
                        )
                    },
                    Operator::LessOrEqual => unsafe {
                        LLVMBuildICmp(
                            context.builder,
                            LLVMIntPredicate::LLVMIntSLE,
                            left,
                            right,
                            "\0".as_ptr() as *const _,
                        )
                    },
                    Operator::More => unsafe {
                        LLVMBuildICmp(
                            context.builder,
                            LLVMIntPredicate::LLVMIntSGT,
                            left,
                            right,
                            "\0".as_ptr() as *const _,
                        )
                    },
                    Operator::MoreOrEqual => unsafe {
                        LLVMBuildICmp(
                            context.builder,
                            LLVMIntPredicate::LLVMIntSGE,
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
            ExpressionKind::UnaryExpr(unary) => unary.build(context),
        }
    }
}

impl IrBuilder for Assignation {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        unsafe {
            let ptr = if !self.name.has_secondaries() {
                let name = self.name.get_identifier().unwrap();

                if let Some(val) = context.scopes.get(name.clone()) {
                    val
                } else {
                    if let Some(t) = &self.t {
                        if let Some(_) = context.classes.get(&t.get_name()) {
                            let val = self.value.build(context).unwrap();

                            context.scopes.add(name.clone(), val);

                            return Some(val);
                        }
                    }

                    let mut alloc_name = "alloc_".to_string() + &name.clone();

                    alloc_name.push('\0');

                    let alloc = LLVMBuildAlloca(
                        context.builder,
                        get_type(Box::new(self.t.clone().unwrap()), context),
                        alloc_name.as_ptr() as *const _,
                    );

                    context.scopes.add(name.clone(), alloc);

                    alloc
                }
            } else {
                let ptr = self.name.build_no_load(context).unwrap();

                ptr
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
            UnaryExpr::UnaryExpr(_op, unary) => unary.build(context),
        }
    }
}

impl PrimaryExpr {
    fn build_no_load(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            PrimaryExpr::PrimaryExpr(operand, vec) => {
                let mut op = operand.build(context);

                if vec.len() == 0 {
                    return op;
                }

                let mut last = vec.first().unwrap().clone();
                let mut is_first = true;

                for second in vec {
                    if !is_first {
                        if let SecondaryExpr::Selector(_) = second {
                            op = if let SecondaryExpr::Selector(_) = last {
                                unsafe {
                                    Some(LLVMBuildLoad(
                                        context.builder,
                                        op.clone().unwrap(),
                                        b"\0".as_ptr() as *const _,
                                    ))
                                }
                            } else {
                                op
                            };
                        }
                    }

                    op = second.build_with(context, op.clone().unwrap());

                    last = second.clone();

                    is_first = false;
                }

                op
            }
        }
    }
}

impl IrBuilder for PrimaryExpr {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let res = self.build_no_load(context);

        match self {
            PrimaryExpr::PrimaryExpr(_, vec) => {
                if vec.len() == 0 {
                    return res;
                }

                let last_second = vec.last().unwrap();

                if let SecondaryExpr::Arguments(_) = last_second {
                    return res;
                }

                unsafe {
                    let op = LLVMBuildLoad(
                        context.builder,
                        res.clone().unwrap(),
                        b"\0".as_ptr() as *const _,
                    );

                    Some(op)
                }
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

            SecondaryExpr::Index(expr) => {
                let idx = expr.build(context).unwrap();

                unsafe {
                    let mut indices = [idx];

                    let ptr_elem = LLVMBuildGEP(
                        context.builder,
                        op,
                        indices.as_mut_ptr(),
                        1,
                        b"\0".as_ptr() as *const _,
                    );

                    Some(ptr_elem)
                }
            }

            SecondaryExpr::Selector(sel) => unsafe {
                let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
                let idx = LLVMConstInt(LLVMInt32Type(), sel.class_offset as u64, 0);

                let mut indices = [zero, idx];

                if let Some(f) = context.functions.get(sel.full_name.clone()) {
                    return Some(f);
                }

                let ptr_elem = LLVMBuildGEP(
                    context.builder,
                    op,
                    indices.as_mut_ptr(),
                    2,
                    b"\0".as_ptr() as *const _,
                );

                Some(ptr_elem)
            },
        }
    }
}

impl IrBuilder for Operand {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match &self.kind {
            OperandKind::Literal(lit) => lit.build(context),
            OperandKind::Identifier(ident) => {
                if let Some(args) = context.arguments.get(ident.clone()) {
                    return Some(args);
                }

                if let Some(func) = context.functions.get(ident.clone()) {
                    return Some(func);
                }

                if let Some(ptr) = context.scopes.get(ident.clone()) {
                    unsafe {
                        let mut ident = ident.clone();

                        ident.push('\0');

                        if let Some(Type::Class(_)) = &self.t {
                            return Some(ptr);
                        }

                        Some(LLVMBuildLoad(
                            context.builder,
                            ptr,
                            ident.as_ptr() as *const _,
                        ))
                    }
                } else {
                    panic!("Unknown identifier {}", ident);
                }
            }
            OperandKind::ClassInstance(ci) => {
                if let Some(class_ty) = context.classes.get(&ci.name.clone()) {
                    unsafe {
                        let res = LLVMBuildAlloca(
                            context.builder,
                            class_ty.0.clone(),
                            "\0".as_ptr() as *const _,
                        );

                        let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);

                        for attr in ci.class.attributes.clone() {
                            let class_attr = ci.class.get_attribute(attr.name.clone()).unwrap();

                            let (val, i) = match ci.get_attribute(attr.name.clone()) {
                                None => (class_attr.0.default.unwrap(), class_attr.1), // handle error here
                                Some((attr, _i)) => (attr.default.unwrap(), class_attr.1), // and here
                            };

                            let idx = LLVMConstInt(LLVMInt32Type(), i as u64, 0);
                            let mut indices = [zero, idx];

                            let ptr_elem = LLVMBuildGEP(
                                context.builder,
                                res,
                                indices.as_mut_ptr(),
                                2,
                                b"\0".as_ptr() as *const _,
                            );

                            let val_res = if val.is_identifier() {
                                println!("VAL IDENT {:?}", val);
                                let ident = val.get_identifier().unwrap();
                                let t = class_attr.0.t.clone().unwrap();
                                if let Type::Class(_) = t {
                                    context.scopes.get(ident).unwrap()
                                } else {
                                    val.build(context).unwrap()
                                }
                            } else {
                                val.build(context).unwrap()
                            };

                            LLVMBuildStore(context.builder, val_res, ptr_elem);
                        }

                        Some(res)
                    }
                } else {
                    panic!("Unknown class {}", ci.name);
                }
            }
            OperandKind::Array(arr) => arr.build(context),
            OperandKind::Expression(expr) => expr.build(context),
        }
    }
}

impl IrBuilder for Literal {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Literal::Number(num) => unsafe { Some(LLVMConstInt(LLVMInt32Type(), *num, 0)) },
            Literal::String(s) => s.build(context),
            Literal::Bool(b) => unsafe { Some(LLVMConstInt(LLVMInt1Type(), b.clone(), 0)) },
        }
    }
}

impl IrBuilder for Array {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let mut res = vec![];

        for item in &self.items {
            res.push(item.build(context).unwrap());
        }

        let t = get_type(Box::new(self.t.clone().unwrap()), context);

        unsafe {
            let pointer = LLVMBuildAlloca(
                context.builder,
                LLVMArrayType(LLVMGetElementType(t), res.len() as u32),
                b"\0".as_ptr() as *const _,
            );

            let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
            let mut indices = [zero, zero];

            let ptr_elem = LLVMBuildGEP(
                context.builder,
                pointer,
                indices.as_mut_ptr(),
                2,
                b"\0".as_ptr() as *const _,
            );

            let mut i = 0;

            for item in res {
                let idx = LLVMConstInt(LLVMInt32Type(), i, 0);
                let mut indices = [zero, idx];

                let ptr_elem = LLVMBuildGEP(
                    context.builder,
                    pointer,
                    indices.as_mut_ptr(),
                    2,
                    b"\0".as_ptr() as *const _,
                );

                LLVMBuildStore(context.builder, item, ptr_elem);

                i += 1;
            }

            let ptr8 = LLVMBuildBitCast(context.builder, ptr_elem, t, b"\0".as_ptr() as *const _);

            Some(ptr8)
        }
    }
}

impl IrBuilder for String {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let mut me = self.clone();

        me.push('\0');

        unsafe {
            let t = LLVMPointerType(LLVMInt8Type(), 0);

            let pointer = LLVMBuildAlloca(
                context.builder,
                LLVMArrayType(LLVMGetElementType(t), me.len() as u32),
                b"\0".as_ptr() as *const _,
            );

            let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
            let mut indices = [zero, zero];

            let ptr_elem = LLVMBuildGEP(
                context.builder,
                pointer,
                indices.as_mut_ptr(),
                2,
                b"\0".as_ptr() as *const _,
            );

            let mut i = 0;

            for item in me.bytes() {
                let idx = LLVMConstInt(LLVMInt32Type(), i, 0);
                let mut indices = [zero, idx];

                let ptr_elem = LLVMBuildGEP(
                    context.builder,
                    pointer,
                    indices.as_mut_ptr(),
                    2,
                    b"\0".as_ptr() as *const _,
                );

                let idx = LLVMConstInt(LLVMInt8Type(), item as u64, 0);

                LLVMBuildStore(context.builder, idx, ptr_elem);

                i += 1;
            }

            let ptr8 = LLVMBuildBitCast(context.builder, ptr_elem, t, b"\0".as_ptr() as *const _);

            Some(ptr8)
        }
    }
}
