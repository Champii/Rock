use std::collections::HashMap;

use crate::walk_list;
use crate::{
    ast::resolve::ResolutionMap, ast::visit::*, ast::*, diagnostics::Diagnostic,
    helpers::scopes::*, parser::ParsingCtx, NodeId,
};

#[derive(Debug)]
pub struct ResolveCtx<'a> {
    pub parsing_ctx: &'a mut ParsingCtx,
    pub scopes: HashMap<IdentifierPath, Scopes<String, Identity>>, // <ModPath, ModScopes>
    pub cur_scope: IdentifierPath,
    pub resolutions: ResolutionMap<NodeId>,
}

impl<'a> ResolveCtx<'a> {
    pub fn add_to_current_scope(&mut self, name: String, ident: Identity) {
        if let Some(ref mut scopes) = self.scopes.get_mut(&self.cur_scope) {
            scopes.add(name, ident);
        }
    }

    pub fn new_mod(&mut self, name: IdentifierPath) {
        self.scopes.insert(name.clone(), Scopes::new());

        self.cur_scope = name;
    }

    pub fn push_scope(&mut self) {
        if let Some(ref mut scopes) = self.scopes.get_mut(&self.cur_scope) {
            scopes.push();
        }
    }

    pub fn pop_scope(&mut self) {
        if let Some(ref mut scopes) = self.scopes.get_mut(&self.cur_scope) {
            scopes.pop();
        }
    }

    pub fn get(&mut self, name: String) -> Option<Identity> {
        match self.scopes.get_mut(&self.cur_scope) {
            Some(ref mut scopes) => scopes.get(name),
            None => None,
        }
    }
}

impl<'a> Visitor<'a> for ResolveCtx<'a> {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first
        for top in &m.top_levels {
            match &top.kind {
                TopLevelKind::Prototype(p) => {
                    self.add_to_current_scope((*p.name).clone(), p.identity.clone());
                }
                TopLevelKind::Use(_u) => (),
                TopLevelKind::Trait(t) => {
                    for proto in &t.defs {
                        self.add_to_current_scope((*proto.name).clone(), proto.identity.clone());
                    }
                }
                TopLevelKind::Impl(i) => {
                    // for proto in &i.defs {
                    //     self.add_to_current_scope((*proto.name).clone(), proto.identity.clone());
                    // }
                }
                TopLevelKind::Mod(_, _m) => (),
                TopLevelKind::Infix(_, _) => (),
                TopLevelKind::Function(f) => {
                    self.add_to_current_scope((*f.name).clone(), f.identity.clone());
                }
            }
        }

        walk_list!(self, visit_top_level, &m.top_levels);
    }

    fn visit_assign(&mut self, assign: &'a Assign) {
        self.add_to_current_scope(assign.name.name.clone(), assign.name.identity.clone());

        // self.visit_identifier(&assign.name);

        self.visit_expression(&assign.value);
    }

    fn visit_top_level(&mut self, top: &'a TopLevel) {
        match &top.kind {
            TopLevelKind::Prototype(p) => self.visit_prototype(&p),
            TopLevelKind::Use(u) => {
                self.visit_use(&u);
            }
            TopLevelKind::Infix(_, _) => (),
            TopLevelKind::Trait(t) => self.visit_trait(&t),
            TopLevelKind::Impl(i) => self.visit_impl(&i),
            TopLevelKind::Function(f) => self.visit_function_decl(&f),
            TopLevelKind::Mod(name, m) => {
                let current_mod = self.cur_scope.clone();

                self.new_mod(self.cur_scope.child(name.clone()));

                self.visit_mod(m);

                self.cur_scope = current_mod;
            }
        };
    }

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        self.push_scope();

        walk_function_decl(self, f);

        self.pop_scope();
    }

    fn visit_use(&mut self, r#use: &'a Use) {
        let ident = r#use.path.last_segment_ref();

        if r#use.path.path.len() == 1 {
            panic!("Unimplemented");
        }

        let mut mod_path = r#use.path.parent().prepend_mod(self.cur_scope.clone());

        mod_path.resolve_supers();

        match self.scopes.get(&mod_path) {
            Some(scopes) => match scopes.get((*ident).to_string()) {
                Some(pointed) => {
                    self.add_to_current_scope(r#use.path.last_segment().name, pointed);
                }
                None => self
                    .parsing_ctx
                    .diagnostics
                    .push(Diagnostic::new_unknown_identifier(
                        ident.identity.span.clone(),
                    )),
            },

            None => self
                .parsing_ctx
                .diagnostics
                .push(Diagnostic::new_module_not_found(
                    ident.identity.span.clone(),
                )),
        };
    }

    fn visit_argument_decl(&mut self, arg: &'a ArgumentDecl) {
        self.add_to_current_scope(arg.name.clone(), arg.identity.clone());
    }

    fn visit_identifier_path(&mut self, path: &'a IdentifierPath) {
        let ident = path.last_segment_ref();

        if path.path.len() == 1 {
            self.visit_identifier(ident);

            return;
        }

        let mut mod_path = path.parent().prepend_mod(self.cur_scope.clone());

        mod_path.resolve_supers();

        // println!("MOD {:#?}", self.scopes);
        // println!("NAME {:#?}", mod_path);

        match self.scopes.get(&mod_path) {
            Some(scopes) => match scopes.get((*ident).to_string()) {
                Some(pointed) => self
                    .resolutions
                    .insert(ident.identity.node_id, pointed.node_id),
                None => self
                    .parsing_ctx
                    .diagnostics
                    .push(Diagnostic::new_unknown_identifier(
                        ident.identity.span.clone(),
                    )),
            },

            // TODO: change to Unknown Mod diagnostic
            None => self
                .parsing_ctx
                .diagnostics
                .push(Diagnostic::new_module_not_found(
                    ident.identity.span.clone(),
                )),
        };
    }

    fn visit_identifier(&mut self, id: &'a Identifier) {
        match self.get((*id).to_string()) {
            Some(pointed) => self
                .resolutions
                .insert(id.identity.node_id, pointed.node_id),
            None => self
                .parsing_ctx
                .diagnostics
                .push(Diagnostic::new_unknown_identifier(id.identity.span.clone())),
        };
    }
}
