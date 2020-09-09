use crate::Error;
use crate::Token;

use crate::ast::Expression;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::context::Context;
use crate::type_checker::TypeInferer;

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub t: Option<Type>,
    pub default: Option<Expression>,
    pub token: Token,
}

impl TypeInferer for Attribute {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Attribute ({:?})", self.token);

        if let Some(mut default) = self.default.clone() {
            let t = default.infer(ctx)?;

            debug!("Infered default type {:?} for attr {}", t, self.name);

            if let Some(t2) = self.t.clone() {
                if t2.get_name() != t.clone().unwrap().get_name() {
                    return Err(Error::new_empty());
                }
            }

            self.t = t.clone();

            Ok(t)
        } else if let Some(_) = self.t.clone() {
            debug!(
                "Already manualy defined type {:?} for attr {}",
                self.t, self.name
            );

            Ok(self.t.clone())
        } else {
            Err(Error::new_empty())
        }
    }
}
