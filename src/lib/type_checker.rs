use super::ast::*;
use super::context::*;
use super::error::*;

pub struct TypeChecker {
    pub ctx: Context,
    pub ast: SourceFile,
}

impl TypeChecker {
    pub fn new(ast: SourceFile) -> TypeChecker {
        TypeChecker {
            ctx: Context::new(),
            ast,
        }
    }

    pub fn infer(&mut self) -> Result<TypeInfer, Error> {
        self.ast.infer(&mut self.ctx)
    }
}

pub trait TypeInferer {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error>;
}
