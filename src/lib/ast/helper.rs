use crate::infer::*;

use super::{AstPrint, Identity};

#[allow(unused_macros)]
#[macro_use]
macro_rules! base_class {
    ($class:tt, $trait:tt, $method:ident, $ctx:tt, [ $($attr:ident),* ]) => {
        impl $trait for crate::ast::$class {
            fn $method(&self, ctx: &mut $ctx) {
                $(
                    self.$attr.$method(ctx);
                )*
            }
        }
    };
}

#[macro_use]
macro_rules! visitable_class {
    ($class:tt, $trait:tt, $method:ident, $ctx:tt, [ $($attr:ident),* ], $alt_macro:tt) => {
        $alt_macro!($class, $trait, $method, $ctx, [ $($attr),* ]);
    };
}

#[macro_use]
macro_rules! visitable_enum {
    ($enum:tt, $trait:tt, $method:ident, $ctx:tt, [ $($attr:tt ( $($var: ident),* )),* ]) => {
        impl $trait for crate::ast::$enum {
            fn $method(&self, ctx: &mut $ctx) {
                match self {
                    $(
                        crate::ast::$enum::$attr($($var),*) => {
                            $($var.$method(ctx));*
                        },
                    )*
                        #[allow(unreachable_patterns)]
                    _ => unreachable!(),
                }
            }
        }
    };
}

#[macro_use]
macro_rules! visitable_vec {
    ($trait:tt, $method:ident, $ctx:tt) => {
        impl<T: $trait> $trait for Vec<T> {
            fn $method(&self, ctx: &mut $ctx) {
                for x in self {
                    x.$method(ctx);
                }
            }
        }
    };
}

#[allow(unused_macros)]
#[macro_use]
macro_rules! visitable_string {
    ($trait:tt, $method:ident, $ctx:tt) => {
        impl<T: $trait> $trait for String {
            fn $method(&self, ctx: &mut $ctx) {
                for x in self {
                    x.$method(ctx);
                }
            }
        }
    };
}

#[macro_use]
macro_rules! predef_trait_visitor {
    ($trait:tt, $method:ident, $ctx:tt) => {
        predef_trait_visitor!($trait, $method, $ctx, base_class);
    };
    ($trait:tt, $method:ident, $ctx:tt, $alt_macro:tt) => {
        #[macro_use]
        macro_rules! $method {
            (struct, $class:tt, $attrs:tt) => {
                visitable_class!($class, $trait, $method, $ctx, $attrs, $alt_macro);
            };
            (struct, $class:tt, $attrs:tt, $override_macro:tt) => {
                visitable_class!($class, $trait, $method, $ctx, $attrs, $override_macro);
            };
            (enum, $enum:tt, $attrs:tt) => {
                visitable_enum!($enum, $trait, $method, $ctx, $attrs);
            };
        }

        visitable_vec!($trait, $method, $ctx);

        base_trait_impl!($method);
    };
}

#[macro_use]
macro_rules! base_trait_impl {
    ($name:tt) => {
        $name!(struct, SourceFile, [top_levels]);
        $name!(struct, TopLevel, [kind]);
        $name!(enum, TopLevelKind, [Function(x)]);
        // $name!(struct, FunctionDecl, [arguments, body]);
        // $name!(struct, ArgumentDecl, []);
        $name!(struct, Body, [stmt]);
        $name!(struct, Statement, [kind]);
        $name!(enum, StatementKind, [Expression(x)]);
        $name!(struct, Expression, [kind]);
        $name!(
            enum,
            ExpressionKind,
            [UnaryExpr(unary), BinopExpr(unary, op, expr)]
        );
        $name!(enum, UnaryExpr, [PrimaryExpr(p), UnaryExpr(op, unary)]);
        $name!(enum, PrimaryExpr, [PrimaryExpr(op, s)]);
        $name!(struct, Operand, [kind]);
        $name!(enum, SecondaryExpr, [Arguments(v)]);
        $name!(struct, Argument, [arg]);
        $name!(
            enum,
            OperandKind,
            [Literal(l), Identifier(i), Expression(e)]
        );
    };
}

pub trait HasName {
    fn get_name(&self) -> String;
}

#[macro_use]
macro_rules! generate_has_name {
    ($class:tt) => {
        impl HasName for $class {
            fn get_name(&self) -> String {
                self.name.clone().to_string()
            }
        }
    };
}

pub trait ClassName {
    fn class_name() -> String;
    fn class_name_self(&self) -> String;
}

impl<T> ClassName for T
where
    T: Default,
    T: core::fmt::Debug,
{
    fn class_name() -> String {
        let name = format!("{:?}", T::default());

        let names = name.split::<_>(" ").collect::<Vec<&str>>();
        let name = names.get(0).unwrap();

        name.to_string()
    }

    fn class_name_self(&self) -> String {
        T::class_name()
    }
}

pub trait HasIdentity: AstPrint {}

pub trait GetIdentity: AstPrint {
    fn get_identity(&self) -> Identity;
}

#[macro_use]
macro_rules! visitable_constraint_class {
    ($class:tt, $trait:tt, $method:ident, $ctx:tt, [ $($attr:ident),* ]) => {
        impl $trait for crate::ast::$class {
            fn $method(&self, ctx: &mut $ctx) -> TypeId {
                // println!("CONSTRAINT {}", stringify!($class));
                let self_type_id = ctx.get_type_id(self.identity.clone()).unwrap();

                $(
                    let child_id = self.$attr.$method(ctx);

                    ctx.add_constraint(Constraint::Eq(self_type_id, child_id));
                )*

                self_type_id
            }
        }
    };
}

#[macro_use]
macro_rules! visitable_constraint_enum {
    ($enum:tt, $trait:tt, $method:ident, $ctx:tt, [ $($attr:tt ( $($var: ident),* )),* ]) => {
        impl $trait for crate::ast::$enum {
            fn $method(&self, ctx: &mut $ctx) -> TypeId {
                match self {
                    $(
                        crate::ast::$enum::$attr($($var),*) => {
                            $($var.$method(ctx))*
                        },
                    )*
                        #[allow(unreachable_patterns)]
                    _ => unreachable!(),
                }
            }
        }
    };
}

#[macro_use]
macro_rules! visitable_constraint_vec {
    ($trait:tt, $method:ident, $ctx:tt) => {
        impl<T: $trait> $trait for Vec<T> {
            fn $method(&self, ctx: &mut $ctx) -> TypeId {
                for _ in self.iter().map(|x| x.$method(ctx)).collect::<Vec<_>>() {}

                0
            }

            fn constrain_vec(&self, ctx: &mut $ctx) -> Vec<TypeId> {
                self.iter().map(|x| x.$method(ctx)).collect()
            }
        }
    };
}

visitable_constraint_vec!(ConstraintGen, constrain, InferBuilder);

// #[macro_use]
// macro_rules! impl_parse {
//     ($class:tt, {
//         $( $field:ident ( $( box $attr_box:ident ),* ) ),*
//     }) => {
//         impl Parse for $class {
//             fn parse(ctx: &mut Parser) -> Result<$class, Error> {
//                 $(
//                     {
//                         return Ok($class::$field($(
//                             $(Box::new(try_or_restore!($attr::parse(ctx), ctx)))?
//                         ),*));
//                     }
//                 )*
//             }
//         }
//     };
// }
