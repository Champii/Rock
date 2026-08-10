use std::fmt::{self, Write};

use crate::ast::*;

use super::{FormatContext, FormatNode};

impl FormatNode for Pattern {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        if let Some(binding) = &self.binding {
            binding.fmt_with(context, f)?;
            write!(f, " @ ")?;
        }

        self.kind.fmt_with(context, f)
    }
}

impl FormatNode for PatternKind {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        match self {
            PatternKind::Ident(ident) => ident.fmt_with(context, f),
            PatternKind::Literal(lit) => lit.fmt_with(context, f),
            PatternKind::Tuple(patterns) => {
                write!(f, "(")?;

                for (i, pattern) in patterns.iter().enumerate() {
                    pattern.fmt_with(context, f)?;

                    if i < patterns.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ")")
            }
            PatternKind::Array(patterns) => {
                write!(f, "[")?;

                for (i, pattern) in patterns.iter().enumerate() {
                    pattern.fmt_with(context, f)?;

                    if i < patterns.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "]")
            }
            PatternKind::Instance(inst) => inst.fmt_with(context, f),
            PatternKind::Nested(pattern) => {
                write!(f, "(")?;
                pattern.fmt_with(context, f)?;
                write!(f, ")")
            }
            PatternKind::Wildcard => write!(f, "_"),
            PatternKind::Reference { pattern, mutable } => {
                if *mutable {
                    write!(f, "&mut ")?;
                } else {
                    write!(f, "&")?;
                }
                pattern.fmt_with(context, f)
            }
        }
    }
}

impl FormatNode for IdentPattern {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        if self.mut_ {
            write!(f, "mut ")?;
        }

        self.name.fmt_with(context, f)
    }
}

impl FormatNode for ArrayPattern {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        match self {
            ArrayPattern::Pattern(pattern) => pattern.fmt_with(context, f),
            ArrayPattern::Rest(ident) => {
                write!(f, "..")?;
                ident.fmt_with(context, f)
            }
        }
    }
}

impl FormatNode for InstancePattern {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        self.name.fmt_with(context, f)?;
        self.args.fmt_with(context, f)
    }
}

impl FormatNode for FieldsPatternOrArgumentsPattern {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        match self {
            FieldsPatternOrArgumentsPattern::Fields(fields) => {
                if fields.is_empty() {
                    return Ok(());
                }

                for (i, field) in fields.iter().enumerate() {
                    write!(f, " ")?;
                    field.fmt_with(context, f)?;

                    if i < fields.len() - 1 {
                        write!(f, ",")?;
                    }
                }

                Ok(())
            }
            FieldsPatternOrArgumentsPattern::Arguments(args) => {
                if args.is_empty() {
                    return Ok(());
                }

                for (i, arg) in args.iter().enumerate() {
                    write!(f, " ")?;
                    arg.fmt_with(context, f)?;

                    if i < args.len() - 1 {
                        write!(f, ",")?;
                    }
                }

                Ok(())
            }
        }
    }
}

impl FormatNode for FieldPattern {
    fn fmt_with<W: Write>(&self, context: &mut FormatContext, f: &mut W) -> fmt::Result {
        self.name.fmt_with(context, f)?;
        write!(f, ": ")?;
        self.pattern.fmt_with(context, f)
    }
}
