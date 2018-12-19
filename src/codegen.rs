use llvm::core::*;
// use llvm::execution_engine::*;
use llvm::target::*;
use llvm::target_machine::*;
use llvm::*;

use std::ptr;

use super::ast::*;
use super::scope::Scopes;

pub struct Context {
    pub scopes: Scopes,
    pub module: *mut LLVMModule,
    pub context: *mut LLVMContext,
    pub builder: *mut LLVMBuilder,
}

pub fn get_type(t: Box<Type>) -> *mut LLVMType {
    unsafe {
        match t.as_ref() {
            Type::Name(t) => match t.as_ref() {
                "Int" => LLVMInt32Type(),
                "" => LLVMInt32Type(),
                _ => LLVMInt32Type(),
            },
            Type::Array(t) => get_type(t.clone()),
        }
    }
}

pub trait IrBuilder {
    fn build(&self, ctx: &mut Context) -> Option<*mut LLVMValue>;
}

impl SourceFile {
    pub fn build(&self, filename: &str) -> Option<*mut LLVMValue> {
        unsafe {
            // Set up a context, module and builder in that context.
            let ctx = LLVMContextCreate();
            let module = LLVMModuleCreateWithNameInContext(b"sum\0".as_ptr() as *const _, ctx);
            let builder = LLVMCreateBuilderInContext(ctx);

            let mut context = Context {
                context: ctx,
                module,
                builder,
                scopes: Scopes::new(),
            };

            for top in &self.top_levels {
                top.build(&mut context);
            }

            LLVMDisposeBuilder(builder);
            LLVMDumpModule(module);

            LLVM_InitializeAllTargetInfos();
            LLVM_InitializeAllTargets();
            LLVM_InitializeAllTargetMCs();
            LLVM_InitializeAllAsmParsers();
            LLVM_InitializeAllAsmPrinters();

            let triple = LLVMGetDefaultTargetTriple();

            let mut target: *mut LLVMTarget = ptr::null_mut();

            let target: *mut *mut LLVMTarget = &mut target;

            let err = "";

            if LLVMGetTargetFromTriple(triple, target, err.as_ptr() as *mut _) == 1 {
                println!("Cannot get target {}", err);

                return None;
            }

            let generic = "generic\0";
            let empty = "\0";

            let machine = LLVMCreateTargetMachine(
                *target,
                triple,
                generic.as_ptr() as *const _,
                empty.as_ptr() as *const _,
                LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );

            let res = LLVMTargetMachineEmitToFile(
                machine,
                module,
                filename.as_ptr() as *mut _,
                LLVMCodeGenFileType::LLVMObjectFile,
                err.as_ptr() as *mut _,
            );

            if res == 1 {
                println!("Cannot generate file {}", err);
            }
        }

        None
    }
}

impl IrBuilder for TopLevel {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            TopLevel::Function(fun) => fun.build(context),
            TopLevel::Prototype(fun) => fun.build(context),
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
            // let mut argts = [i32t, i32t, i32t];
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

            context.scopes.add(name_orig, function);

            Some(function)
        }
    }
}

impl IrBuilder for FunctionDecl {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        let name_orig = self.name.clone().unwrap_or("nop".to_string());

        let mut name = name_orig.clone();

        name.push('\0');

        let name = name.as_str();

        unsafe {
            let i32t = LLVMInt32TypeInContext(context.context);
            // let mut argts = [i32t, i32t, i32t];
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.t.clone()));

                argts.push(t);
            }

            let function_type =
                llvm::core::LLVMFunctionType(i32t, argts.as_mut_ptr(), argts.len() as u32, 0);
            let function = llvm::core::LLVMAddFunction(
                context.module,
                name.as_ptr() as *const _,
                function_type,
            );

            context.scopes.add(name_orig, function);

            context.scopes.push();

            let mut count = 0;
            for arg in &self.arguments {
                context
                    .scopes
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

            llvm::core::LLVMBuildRet(context.builder, res.unwrap());

            context.scopes.pop();

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
            Statement::Expression(expr) => expr.build(context),
            Statement::Assignation(assign) => assign.build(context),
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
            Some(LLVMBuildAlloca(
                context.builder,
                get_type(Box::new(self.t.clone())),
                "".as_ptr() as *const _,
            ))
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
            Operand::Identifier(ident) => context.scopes.get(ident.clone()),
            _ => None,
        }
    }
}

impl IrBuilder for Literal {
    fn build(&self, context: &mut Context) -> Option<*mut LLVMValue> {
        match self {
            Literal::Number(num) => unsafe { Some(LLVMConstInt(LLVMInt32Type(), *num, 0)) },
            _ => None,
        }
    }
}
