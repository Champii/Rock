use std::fmt;

use crate::ids::Idx;
use crate::types::{FunctionSafety, Type};

pub struct TypeDisplay<'a> {
    ty: &'a Type,
}

pub fn display_type(ty: &Type) -> TypeDisplay<'_> {
    TypeDisplay { ty }
}

pub fn write_type(ty: &Type, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match ty {
        Type::I8 => write!(f, "I8"),
        Type::I16 => write!(f, "I16"),
        Type::I32 => write!(f, "I32"),
        Type::I64 => write!(f, "I64"),
        Type::U8 => write!(f, "U8"),
        Type::U16 => write!(f, "U16"),
        Type::U32 => write!(f, "U32"),
        Type::U64 => write!(f, "U64"),
        Type::F32 => write!(f, "F32"),
        Type::F64 => write!(f, "F64"),
        Type::Bool => write!(f, "Bool"),
        Type::Str => write!(f, "Str"),
        Type::Char => write!(f, "Char"),
        Type::Unit => write!(f, "()"),
        Type::Never => write!(f, "!"),
        Type::Slice(inner) => write!(f, "[{}]", display_type(inner)),
        Type::Array(inner, len) => write!(f, "[{}; {}]", display_type(inner), len),
        Type::Tuple(elems) => {
            write!(f, "(")?;
            for (index, elem) in elems.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", display_type(elem))?;
            }
            write!(f, ")")
        }
        Type::Function {
            params,
            ret,
            safety,
            ..
        } => {
            if matches!(safety, FunctionSafety::Unsafe) {
                write!(f, "unsafe ")?;
            }
            for (index, arg) in params.iter().enumerate() {
                if index > 0 {
                    write!(f, " -> ")?;
                }
                write!(f, "{}", display_type(arg))?;
            }
            if !params.is_empty() {
                write!(f, " -> ")?;
            }
            write!(f, "{}", display_type(ret))
        }
        Type::Struct { id, args } => {
            write!(f, "struct#{}::{}", id.crate_id.0, id.local.0)?;
            write_type_args(args, f)
        }
        Type::Enum { id, args } => {
            write!(f, "enum#{}::{}", id.crate_id.0, id.local.0)?;
            write_type_args(args, f)
        }
        Type::Reference { mutable, inner } => {
            if *mutable {
                write!(f, "&mut {}", display_type(inner))
            } else {
                write!(f, "&{}", display_type(inner))
            }
        }
        Type::Pointer(inner) => write!(f, "*{}", display_type(inner)),
        Type::TypeVar(id) => write!(f, "?T{}", id.raw()),
        Type::Generic(param) => write!(
            f,
            "generic#{}::{}.{}",
            param.owner.crate_id.0, param.owner.local.0, param.index
        ),
        Type::Projection {
            ty,
            trait_id,
            assoc_type,
            trait_args,
        } => {
            write!(
                f,
                "<{} as trait#{}::{}",
                display_type(ty),
                trait_id.crate_id.0,
                trait_id.local.0
            )?;
            write_type_args(trait_args, f)?;
            write!(
                f,
                ">::assoc#{}::{}.{}",
                assoc_type.owner.crate_id.0, assoc_type.owner.local.0, assoc_type.assoc_type_id.0
            )
        }
        Type::Constructor { id, flavor } => write!(
            f,
            "constructor[{flavor:?}]#{}::{}",
            id.crate_id.0, id.local.0
        ),
        Type::Apply { constructor, args } => {
            write!(f, "{}", display_type(constructor))?;
            write_type_args(args, f)
        }
        Type::Lambda { params, body } => {
            write!(f, "\\")?;
            for (index, kind) in params.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{kind}")?;
            }
            write!(f, " -> {}", display_type(body))
        }
        Type::BoundVar { depth, index, kind } => write!(f, "^{depth}.{index}:{kind}"),
        Type::Error => write!(f, "<error>"),
    }
}

fn write_type_args(args: &[Type], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if args.is_empty() {
        return Ok(());
    }

    write!(f, "<")?;
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}", display_type(arg))?;
    }
    write!(f, ">")
}

impl fmt::Display for TypeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_type(self.ty, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AssocTypeId, CrateId, DefId, LocalDefId, TypeVarId};
    use crate::types::{AssociatedTypeKey, GenericParamId};

    fn def_id(index: u32) -> DefId {
        DefId::new(CrateId(0), LocalDefId(index))
    }

    #[test]
    fn type_display_service_preserves_existing_output() {
        let generic = GenericParamId {
            owner: def_id(4),
            index: 1,
        };
        let projection = Type::Projection {
            ty: Box::new(Type::Generic(generic)),
            trait_id: def_id(5),
            assoc_type: AssociatedTypeKey {
                owner: def_id(5),
                assoc_type_id: AssocTypeId(2),
            },
            trait_args: vec![Type::I64, Type::Bool],
        };

        let cases = vec![
            (Type::I8, "I8"),
            (Type::U64, "U64"),
            (Type::F32, "F32"),
            (Type::Bool, "Bool"),
            (Type::Str, "Str"),
            (Type::Char, "Char"),
            (Type::Unit, "()"),
            (Type::Never, "!"),
            (Type::Slice(Box::new(Type::I64)), "[I64]"),
            (Type::Array(Box::new(Type::Bool), 4), "[Bool; 4]"),
            (Type::Tuple(vec![Type::I64, Type::Bool]), "(I64, Bool)"),
            (
                Type::function(vec![Type::I64, Type::Bool], Type::Unit),
                "I64 -> Bool -> ()",
            ),
            (
                Type::Struct {
                    id: def_id(1),
                    args: vec![Type::I64],
                },
                "struct#0::1<I64>",
            ),
            (
                Type::Enum {
                    id: def_id(2),
                    args: vec![Type::Bool],
                },
                "enum#0::2<Bool>",
            ),
            (
                Type::Reference {
                    mutable: false,
                    inner: Box::new(Type::Str),
                },
                "&Str",
            ),
            (
                Type::Reference {
                    mutable: true,
                    inner: Box::new(Type::I64),
                },
                "&mut I64",
            ),
            (Type::Pointer(Box::new(Type::I64)), "*I64"),
            (Type::TypeVar(TypeVarId(7)), "?T7"),
            (Type::Generic(generic), "generic#0::4.1"),
            (
                projection,
                "<generic#0::4.1 as trait#0::5<I64, Bool>>::assoc#0::5.2",
            ),
            (Type::Error, "<error>"),
        ];

        for (ty, expected) in cases {
            assert_eq!(display_type(&ty).to_string(), expected);
            assert_eq!(ty.to_string(), expected);
        }
    }
}
