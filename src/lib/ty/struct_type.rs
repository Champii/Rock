use std::{collections::BTreeMap, fmt};

use colored::*;

use crate::{ast, hir};

use super::Type;

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructType {
    pub name: String,
    pub defs: BTreeMap<String, Box<Type>>,
    pub fields_order: Vec<String>,
}

impl StructType {
    pub fn ordered_defs(&self) -> Vec<(String, Box<Type>)> {
        self.fields_order
            .iter()
            .map(|name| (name.clone(), self.defs.get(name).unwrap().clone()))
            .collect()
    }
}

impl fmt::Debug for StructType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.name.yellow(),
            "{".green(),
            self.defs
                .iter()
                .map(|(n, b)| format!("{}: {:?}", n, b))
                .collect::<Vec<_>>()
                .join(", "),
            "}".green(),
        )
    }
}

impl From<&ast::tree::StructDecl> for StructType {
    fn from(s: &ast::tree::StructDecl) -> Self {
        s.into()
    }
}

impl From<ast::tree::StructDecl> for StructType {
    fn from(s: ast::tree::StructDecl) -> Self {
        StructType {
            name: s.name.to_string(),
            fields_order: s.defs.iter().map(|f| f.name.to_string()).collect(),
            defs: s
                .defs
                .iter()
                .map(|proto| {
                    if proto.signature.arguments.is_empty() {
                        (proto.name.name.clone(), proto.signature.ret.clone())
                    } else {
                        (
                            proto.name.name.clone(),
                            Box::new(proto.signature.clone().into()),
                        )
                    }
                })
                .collect(),
        }
    }
}

impl From<hir::StructDecl> for StructType {
    fn from(s: hir::StructDecl) -> Self {
        StructType {
            name: s.name.name,
            fields_order: s.defs.iter().map(|f| f.name.to_string()).collect(),
            defs: s
                .defs
                .iter()
                .map(|proto| {
                    if proto.signature.arguments.is_empty() {
                        (proto.name.name.clone(), proto.signature.ret.clone())
                    } else {
                        (
                            proto.name.name.clone(),
                            Box::new(proto.signature.clone().into()),
                        )
                    }
                })
                .collect(),
        }
    }
}

impl StructType {
    // pub fn
}
