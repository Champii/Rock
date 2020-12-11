use super::ast::*;
use super::context::*;
use super::error::*;
use super::type_checker::*;

pub struct Generator {
    ast: SourceFile,
    ctx: Context,
}

impl Generator {
    pub fn new(ast: SourceFile, ctx: Context) -> Generator {
        Generator { ast, ctx }
    }

    fn insert_toplevel_at(&mut self, i: usize, f: FunctionDecl) {
        self.ctx
            .scopes
            .add(f.name.clone(), Some(Type::FuncType(Box::new(f.clone()))));

        let i = if i > self.ast.top_levels.len() {
            self.ast.top_levels.len()
        } else {
            i
        };

        self.ast.top_levels.insert(i, TopLevel::Function(f.clone()));
    }

    pub fn generate(&mut self) -> Result<SourceFile, Error> {
        let main_scope = self.ctx.scopes.scopes.first().unwrap();

        let mut i = 0;
        let items = &mut main_scope.get_ordered().clone();

        for func in items {
            if let Some(Type::FuncType(ref mut f)) = func {
                self.ast.top_levels = self
                    .ast
                    .top_levels
                    .iter()
                    .filter(|top| {
                        if let TopLevel::Function(fu) = top {
                            fu.name != f.name
                        } else {
                            true
                        }
                    })
                    .cloned()
                    .collect();

                if f.name == "main" {
                    f.infer(&mut self.ctx).unwrap();

                    self.insert_toplevel_at(i, *f.clone());

                    continue;
                }

                if !f.is_solved() {
                    let ctx_save = self.ctx.clone();

                    if let Some(calls) = ctx_save.calls.get(&f.name) {
                        for (_, call) in calls {
                            let mut new_f = f.clone();

                            new_f.apply_types(f.ret.clone(), call.clone());
                            new_f.infer(&mut self.ctx).unwrap();
                            new_f.apply_name(call.clone());

                            self.insert_toplevel_at(i, *new_f.clone());
                        }
                    }

                    self.ctx = ctx_save;
                } else {
                    f.infer(&mut self.ctx).unwrap();
                    f.apply_name_self();

                    self.insert_toplevel_at(i, *f.clone());
                }
            }
            i += 1;
        }

        self.ast.generate(&mut self.ctx)?;

        Ok(self.ast.clone())
    }
}

pub trait Generate {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error>;
}
