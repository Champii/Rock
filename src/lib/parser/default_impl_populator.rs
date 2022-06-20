use std::collections::BTreeMap;

use crate::{ast::visit_mut::*, ast::*};

#[derive(Debug)]
pub struct  DefaultImplPopulator {
    pub traits: BTreeMap<String, Trait>,
}

impl<'a> VisitorMut<'a> for DefaultImplPopulator {
    fn visit_trait(&mut self, trait_: &mut Trait) {
        self.traits.insert(trait_.name.get_name(), trait_.clone());
    }

    fn visit_impl(&mut self, i: &'a mut Impl) {
        if i.types.is_empty() {
            return;
        }

        let trait_name = i.name.get_name();
        let trait_ = self.traits.get(&trait_name).unwrap();

        i.defs.extend(trait_.default_impl.clone());
    }
}

pub fn populate_default_impl(root: &mut Root) {
    DefaultImplPopulator {
        traits: BTreeMap::new(),
    }
    .visit_root(root);
}
