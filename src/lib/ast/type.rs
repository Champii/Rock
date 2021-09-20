use colored::*;
use std::fmt;

use crate::ast::PrimitiveType;
// use crate::ast::Prototype;

#[derive(Clone, Hash, Eq, Serialize, Deserialize)]
pub enum Type {
    Primitive(PrimitiveType),
    // Proto(Box<Prototype>),
    FuncType(FuncType),
    Class(String),
    Trait(String),
    ForAll(String), // TODO
    Undefined(u64),
}

impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self.get_name() == other.get_name()
    }
}

impl Type {
    pub fn int64() -> Self {
        Self::Primitive(PrimitiveType::Int64)
    }

    pub fn forall(t: &str) -> Self {
        Self::ForAll(String::from(t))
    }

    pub fn get_name(&self) -> String {
        match self {
            Self::Primitive(p) => p.get_name(),
            // Self::Proto(p) => p.name.clone().unwrap_or(String::new()),
            Self::FuncType(f) => f.name.clone(),
            Self::Class(c) => c.clone(),
            Self::Trait(t) => t.clone(),
            Self::ForAll(n) => String::from(n),
            Self::Undefined(s) => s.to_string(),
        }
    }

    pub fn is_forall(&self) -> bool {
        matches!(self, Self::ForAll(_x))
    }
}

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::FuncType(f) => format!("{:?}", f),
            _ => self.get_name().cyan().to_string(),
        };

        write!(f, "{}", s)
    }
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuncType {
    pub name: String,
    pub arguments: Vec<Box<Type>>,
    pub ret: Box<Type>,
}

impl fmt::Debug for FuncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {} {}{}",
            "(".green(),
            self.name.yellow(),
            "::".red(),
            self.to_type_signature(),
            ")".green(),
        )
    }
}

impl FuncType {
    pub fn new(name: String, arguments: Vec<Type>, ret: Type) -> Self {
        Self {
            name,
            arguments: arguments.into_iter().map(Box::new).collect(),
            ret: Box::new(ret),
        }
    }

    pub fn to_prefixes(&self) -> Vec<String> {
        self.arguments
            .iter()
            .cloned()
            .map(|arg| arg.to_string())
            .chain(vec![self.ret.to_string()].into_iter())
            .collect()
    }

    pub fn to_type_signature(&self) -> TypeSignature {
        let mut sig = TypeSignature::default().with_ret(*self.ret.clone());

        sig.args = self.arguments.iter().map(|arg| *arg.clone()).collect();

        sig
    }

    pub fn get_mangled_name(&self) -> String {
        let mut prefixes = self
            .arguments
            .iter()
            .map(|arg| arg.to_string())
            .collect::<Vec<_>>();

        prefixes.push(self.ret.to_string());

        format!("{}_{}", self.name, prefixes.join("_"))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSignature {
    pub args: Vec<Type>,
    pub ret: Type,
    next_free_forall_type: usize,
}

impl fmt::Display for TypeSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .args
            .iter()
            .map(|arg| format!("{:?}", arg))
            .chain(vec![format!("{:?}", self.ret)].into_iter())
            .collect::<Vec<_>>()
            .join(&" -> ".magenta().to_string());

        write!(f, "{}", s)
    }
}

impl Default for TypeSignature {
    fn default() -> Self {
        Self {
            args: vec![],
            ret: Type::Undefined(0),
            next_free_forall_type: 0,
        }
    }
}

impl TypeSignature {
    pub fn apply_forall_types(&self, orig: &Vec<Type>, dest: &Vec<Type>) -> Self {
        let mut orig = orig.clone();

        let dest = dest
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if let Type::ForAll(_) = t {
                    orig.remove(i);

                    None
                } else {
                    Some(t)
                }
            })
            .collect::<Vec<_>>();

        let applied_args = self
            .args
            .iter()
            .map(
                |arg_t| match orig.iter().enumerate().find(|(_, orig_t)| *orig_t == arg_t) {
                    Some((i, _orig_t)) => dest[i].clone(),
                    None => arg_t.clone(),
                },
            )
            .collect();

        let applied_ret = match orig
            .iter()
            .enumerate()
            .find(|(_, orig_t)| **orig_t == self.ret)
        {
            Some((i, _orig_t)) => dest[i].clone(),
            None => self.ret.clone(),
        };

        Self {
            next_free_forall_type: 0,
            args: applied_args,
            ret: applied_ret,
        }
    }

    pub fn apply_types(&self, args: Vec<Type>, ret: Type) -> Self {
        let mut orig = vec![];
        let mut dest = vec![];

        self.args.iter().enumerate().for_each(|(i, arg_t)| {
            if !arg_t.is_forall() {
                warn!("Trying to apply type to a not forall");

                return;
            }

            if let Some(t) = args.get(i) {
                orig.push(arg_t.clone());
                dest.push(t.clone());
            }
        });

        if !ret.is_forall() {
            // panic!("Trying to apply type to a not forall")
            warn!("Trying to apply type to a not forall");
        }

        // FIXME: must remplace all occurences of ret
        orig.push(self.ret.clone());
        dest.push(ret.clone());

        self.apply_forall_types(&orig, &dest)
    }

    pub fn apply_partial_types(&self, args: &Vec<Option<Type>>, ret: Option<Type>) -> Self {
        let mut orig = vec![];
        let mut dest = vec![];

        self.args.iter().enumerate().for_each(|(i, arg_t)| {
            if !arg_t.is_forall() {
                warn!("Trying to apply type to a not forall");

                return;
            }

            if let Some(t) = args.get(i).unwrap() {
                orig.push(arg_t.clone());
                dest.push(t.clone());
            }
        });

        if let Some(t) = ret {
            if !t.is_forall() {
                // panic!("Trying to apply type to a not forall")
                warn!("Trying to apply type to a not forall");
            }

            // FIXME: must remplace all occurences of ret
            orig.push(self.ret.clone());
            dest.push(t.clone());
        }

        self.apply_forall_types(&orig, &dest)
    }

    pub fn apply_partial_types_mut(&mut self, args: &Vec<Option<Type>>, ret: Option<Type>) {
        let mut orig = vec![];
        let mut dest = vec![];

        self.args.iter_mut().enumerate().for_each(|(i, arg_t)| {
            if !arg_t.is_forall() {
                warn!("Trying to apply type to a not forall");

                return;
            }

            if let Some(t) = args.get(i).unwrap() {
                orig.push(arg_t.clone());
                dest.push(t.clone());
            }
        });

        if let Some(t) = ret {
            if !t.is_forall() {
                // panic!("Trying to apply type to a not forall")
                warn!("Trying to apply type to a not forall");
            }

            // FIXME: must remplace all occurences of ret
            orig.push(self.ret.clone());
            dest.push(t.clone());
        }

        *self = self.apply_forall_types(&orig, &dest);
    }

    pub fn from_args_nb(nb: usize) -> Self {
        let mut new = Self::default();

        new.args = (0..nb).map(|_| new.get_next_available_forall()).collect();
        new.ret = new.get_next_available_forall();

        new
    }

    fn get_next_available_forall(&mut self) -> Type {
        let t = Type::ForAll(
            ('a'..'z')
                .nth(self.next_free_forall_type)
                .unwrap()
                .to_string(),
        );

        self.next_free_forall_type += 1;

        t
    }

    pub fn is_solved(&self) -> bool {
        self.are_args_solved() && !self.ret.is_forall()
    }

    pub fn are_args_solved(&self) -> bool {
        !self.args.iter().any(|arg| arg.is_forall())
    }

    pub fn with_ret(mut self, ret: Type) -> Self {
        self.ret = ret;

        self
    }

    pub fn to_func_type(&self) -> Type {
        Type::FuncType(FuncType::new(
            String::new(),
            self.args.clone(),
            self.ret.clone(),
        ))
    }

    pub fn merge_with(&self, other: &Self) -> Self {
        self.apply_types(other.args.clone(), other.ret.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_type_signature() {
        let sig = TypeSignature::from_args_nb(2);

        assert_eq!(sig.args[0], Type::forall("a"));
        assert_eq!(sig.args[1], Type::forall("b"));
        assert_eq!(sig.ret, Type::forall("c"));
    }

    #[test]
    fn apply_forall_types() {
        let sig = TypeSignature::from_args_nb(2);

        let res = sig.apply_forall_types(&vec![Type::forall("b")], &vec![Type::int64()]);

        assert_eq!(res.args[0], Type::forall("a"));
        assert_eq!(res.args[1], Type::int64());
        assert_eq!(res.ret, Type::forall("c"));
    }

    #[test]
    fn apply_types() {
        let sig = TypeSignature::from_args_nb(2);

        let res = sig.apply_types(vec![Type::int64()], Type::int64());

        assert_eq!(res.args[0], Type::int64());
        assert_eq!(res.args[1], Type::forall("b"));
        assert_eq!(res.ret, Type::int64());
    }

    #[test]
    fn apply_partial_types() {
        let sig = TypeSignature::from_args_nb(2);

        let res = sig.apply_partial_types(&vec![None, Some(Type::int64())], Some(Type::int64()));

        assert_eq!(res.args[0], Type::forall("a"));
        assert_eq!(res.args[1], Type::int64());
        assert_eq!(res.ret, Type::int64());
    }
}
