use colored::*;
use std::fmt;

use super::Type;

#[derive(Clone, Eq, Serialize, Deserialize)]
pub struct FuncType {
    pub arguments: Vec<Type>,
    pub ret: Box<Type>,
}

impl std::hash::Hash for FuncType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.arguments.hash(state);
        self.ret.hash(state);
    }
}

impl PartialEq for FuncType {
    fn eq(&self, other: &Self) -> bool {
        self.arguments.eq(&other.arguments) && self.ret.eq(&other.ret)
    }
}

impl Default for FuncType {
    fn default() -> Self {
        Self {
            arguments: vec![],
            ret: Box::new(Type::Undefined(0)),
        }
    }
}

impl fmt::Debug for FuncType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self
            .arguments
            .iter()
            .map(|arg| format!("{:?}", arg))
            .chain(vec![format!("{:?}", self.ret)].into_iter())
            .collect::<Vec<_>>()
            .join(&" -> ".magenta().to_string());

        write!(f, "{}{}{}", "(".green(), s, ")".green(),)
    }
}

impl FuncType {
    pub fn new(arguments: Vec<Type>, ret: Type) -> Self {
        Self {
            arguments: arguments.into_iter().collect(),
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

    pub fn get_mangled_name(&self, name: String) -> String {
        let mut prefixes = self
            .arguments
            .iter()
            .map(|arg| arg.to_string())
            .collect::<Vec<_>>();

        prefixes.push(self.ret.to_string());

        format!("{}_{}", name, prefixes.join("_"))
    }

    pub fn apply_forall_types(&self, orig: &[Type], dest: &[Type]) -> Self {
        assert_eq!(orig.len(), dest.len());

        let (dest, orig): (Vec<_>, Vec<_>) = dest
            .iter()
            .zip(orig)
            .filter_map(|(dest_t, orig_t)| match dest_t {
                Type::ForAll(_) => None,
                _ => Some((dest_t.clone(), orig_t.clone())),
            })
            .unzip();

        let applied_args = self
            .arguments
            .iter()
            .map(|arg_t| {
                match orig
                    .iter()
                    .enumerate()
                    .find(|(_, orig_t)| **orig_t == *arg_t)
                {
                    Some((i, _orig_t)) => dest[i].clone(),
                    None => arg_t.clone(),
                }
            })
            .collect();

        let applied_ret = match orig
            .iter()
            .enumerate()
            .find(|(_, orig_t)| **orig_t == *self.ret)
        {
            Some((i, _orig_t)) => Box::new(dest[i].clone()),
            None => self.ret.clone(),
        };

        Self {
            arguments: applied_args,
            ret: applied_ret,
        }
    }

    fn collect_forall_types(&self, arguments: Vec<Type>, ret: Type) -> (Vec<Type>, Vec<Type>) {
        let (mut orig, mut dest): (Vec<_>, Vec<_>) = self
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(i, arg_t)| -> Option<(Type, Type)> {
                if !arg_t.is_forall() {
                    warn!("Trying to apply type to a not forall");

                    return None;
                }

                arguments.get(i).map(|t| (arg_t.clone(), t.clone()))
            })
            .unzip();

        if !ret.is_forall() {
            warn!("Trying to apply type to a not forall");
        }

        // FIXME: must remplace all occurences of ret
        orig.push(*self.ret.clone());
        dest.push(ret);

        (orig, dest)
    }

    fn collect_partial_forall_types(
        &self,
        arguments: &[Option<Type>],
        ret: Option<Type>,
    ) -> (Vec<Type>, Vec<Type>) {
        let (mut orig, mut dest): (Vec<_>, Vec<_>) = self
            .arguments
            .iter()
            .enumerate()
            .filter_map(|(i, arg_t)| -> Option<(Type, Type)> {
                if !arg_t.is_forall() {
                    warn!("Trying to apply type to a not forall");

                    return None;
                }

                arguments
                    .get(i)?
                    .as_ref()
                    .map(|t| (arg_t.clone(), t.clone()))
            })
            .unzip();

        if let Some(t) = ret {
            if !t.is_forall() {
                warn!("Trying to apply type to a not forall");
            }

            // FIXME: must remplace all occurences of ret
            orig.push(*self.ret.clone());
            dest.push(t);
        }

        (orig, dest)
    }

    pub fn apply_types(&self, arguments: Vec<Type>, ret: Type) -> Self {
        let mut resolved = self.clone();

        resolved.arguments = self
            .arguments
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                if let Type::Func(f_t) = arg {
                    f_t.merge_with(&arguments.get(i).unwrap().as_func_type())
                        .into()
                } else {
                    arg.clone()
                }
            })
            .collect::<Vec<_>>();

        resolved.ret = if let Type::Func(f_t) = &*self.ret {
            Box::new(f_t.merge_with(&ret.as_func_type()).into())
        } else {
            self.ret.clone()
        };

        let (orig, dest) = resolved.collect_forall_types(arguments, ret);

        resolved.apply_forall_types(&orig, &dest)
    }

    pub fn apply_partial_types(&self, arguments: &[Option<Type>], ret: Option<Type>) -> Self {
        let mut resolved = self.clone();

        resolved.arguments = self
            .arguments
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                if let Type::Func(f_t) = arg {
                    let inner = arguments.get(i).unwrap().as_ref().unwrap().as_func_type();

                    f_t.merge_with(&inner).into()
                } else {
                    arg.clone()
                }
            })
            .collect::<Vec<_>>();

        resolved.ret = if let Type::Func(f_t) = &*self.ret {
            Box::new(f_t.merge_with(&ret.as_ref().unwrap().as_func_type()).into())
        } else {
            self.ret.clone()
        };

        let (orig, dest) = resolved.collect_partial_forall_types(arguments, ret);

        resolved.apply_forall_types(&orig, &dest)
    }

    pub fn from_args_nb(nb: usize) -> Self {
        let mut new = Self::default();
        let mut forall_generator = 'a'..'z';

        new.arguments = forall_generator
            .clone()
            .take(nb)
            .map(|n| Type::ForAll(n.to_string()))
            .collect();

        new.ret = Box::new(Type::ForAll(forall_generator.nth(nb).unwrap().to_string()));

        new
    }

    pub fn is_solved(&self) -> bool {
        self.are_args_solved() && self.ret.is_solved()
    }

    pub fn are_args_solved(&self) -> bool {
        !self.arguments.iter().any(|arg| !arg.is_solved())
    }

    pub fn with_ret(mut self, ret: Type) -> Self {
        self.ret = Box::new(ret);

        self
    }

    pub fn merge_with(&self, other: &Self) -> Self {
        self.apply_types(
            other.arguments.iter().map(|b| (*b).clone()).collect(),
            *other.ret.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_type_signature() {
        let sig = FuncType::from_args_nb(2);

        assert_eq!(sig.arguments[0], Type::forall("a"));
        assert_eq!(sig.arguments[1], Type::forall("b"));
        assert_eq!(*sig.ret, Type::forall("c"));
    }

    #[test]
    fn apply_forall_types() {
        let sig = FuncType::from_args_nb(2);

        let res = sig.apply_forall_types(&[Type::forall("b")], &[Type::int64()]);

        assert_eq!(res.arguments[0], Type::forall("a"));
        assert_eq!(res.arguments[1], Type::int64());
        assert_eq!(*res.ret, Type::forall("c"));
    }

    #[test]
    fn apply_types() {
        let sig = FuncType::from_args_nb(2);

        let res = sig.apply_types(vec![Type::int64()], Type::int64());

        assert_eq!(res.arguments[0], Type::int64());
        assert_eq!(res.arguments[1], Type::forall("b"));
        assert_eq!(*res.ret, Type::int64());
    }

    #[test]
    fn apply_partial_types() {
        let sig = FuncType::from_args_nb(2);

        let res = sig.apply_partial_types(&[None, Some(Type::int64())], Some(Type::int64()));

        assert_eq!(res.arguments[0], Type::forall("a"));
        assert_eq!(res.arguments[1], Type::int64());
        assert_eq!(*res.ret, Type::int64());
    }
}
