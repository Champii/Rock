use std::collections::HashMap;

use crate::{
    ast::{tree::*, visit::*, NodeId},
    diagnostics::Diagnostic,
    helpers::scopes::*,
    infer::trait_solver::TraitSolver,
    parser::span::Span,
    parser::ParsingCtx,
    resolver::ResolutionMap,
};

#[derive(Debug)]
pub struct ResolveCtx<'a> {
    pub parsing_ctx: &'a mut ParsingCtx,
    pub scopes: HashMap<IdentifierPath, Scopes<String, NodeId>>, // <ModPath, ModScopes>
    pub cur_scope: IdentifierPath,
    pub resolutions: ResolutionMap<NodeId>,
    pub trait_solver: TraitSolver,
}

impl<'a> ResolveCtx<'a> {
    pub fn run(&mut self, root: &'a mut Root) {
        self.visit_root(root);
    }

    /// TODO: GitHub Issue #150
    /// Could use #![feature(map_try_insert)]
    /// <<: Ensures unique entry into the map.
    pub fn add_to_current_scope(&mut self, name: String, node_id: NodeId) {
        if let Some(ref mut scopes) = self.scopes.get_mut(&self.cur_scope) {
            match scopes.add(name, node_id) {
                Ok(_) => { },
                Err(_err) => { unimplemented!{} },
            }
        }
    }

    pub fn add_to_struct_scope(&mut self, struct_name: String, name: String, node_id: NodeId) {
        let mut struct_scope_name = self.cur_scope.clone();
        struct_scope_name.path.push(Identifier::new(struct_name, 0));

        if let Some(ref mut scopes) = self.scopes.get_mut(&struct_scope_name) {
            match scopes.add(name.clone(), node_id) {
                Ok(_) => { },
                Err(_err) => { unimplemented!{} }, 
            }
        }
    }


    pub fn new_struct(&mut self, name: Identifier) {
        let mut struct_scope_name = self.cur_scope.clone();
        struct_scope_name.path.push(name);

        self.scopes.insert(struct_scope_name.clone(), Scopes::new());
    }

    pub fn import_struct_scope(&mut self, struct_name: Identifier) {
        let mut struct_scope_name = self.cur_scope.clone();
        struct_scope_name.path.push(struct_name);

        let struct_scope = if let Some(struct_scope) = self.scopes.get(&struct_scope_name) {
            struct_scope.clone()
        } else {
            return;
        };

        self.scopes
            .get_mut(&self.cur_scope)
            .unwrap()
            .extend(struct_scope.scopes.last().unwrap());
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

    pub fn get(&mut self, name: String) -> Option<NodeId> {
        match self.scopes.get_mut(&self.cur_scope) {
            Some(ref mut scopes) => scopes.get(name),
            None => None,
        }
    }

    pub fn get_span(&self, node_id: NodeId) -> Span {
        self.parsing_ctx.identities.get(&node_id).unwrap().clone()
    }
}

impl<'a> Visitor<'a> for ResolveCtx<'a> {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first
        for top in &m.top_levels {
            match &top {
                TopLevel::Extern(p) => {
                    self.add_to_current_scope((*p.name).clone(), p.node_id);
                }
                TopLevel::FnSignature(_p) => {
                    // What to do about #150 ?
                    //
                    // We must allow for fn signatures but this issue will forbid
                    // entries that share the same name in the same scope.
                    //
                    // We must ensure that a fn decl exists for every fn signature
                }
                TopLevel::Use(_u) => (),
                TopLevel::Trait(t) => {
                    self.new_struct(Identifier::new(t.name.get_name(), 0));

                    for proto in &t.defs {
                        self.add_to_struct_scope(
                            t.name.get_name(),
                            (*proto.name).clone(),
                            proto.node_id,
                        );
                    }
                }
                TopLevel::Struct(s) => {
                    self.new_struct(s.name.clone());

                    self.add_to_current_scope(s.name.name.clone(), s.name.node_id);

                    s.defs.iter().for_each(|p| {
                        self.add_to_struct_scope(s.name.name.clone(), (*p.name).clone(), p.node_id);
                    })
                }
                TopLevel::Impl(i) => {
                    self.trait_solver
                        .add_implementor(i.name.clone(), i.name.clone());

                    if !i.types.is_empty() {
                        self.trait_solver
                            .add_implementor(i.types.first().unwrap().clone(), i.name.clone());
                    }

                    self.trait_solver.add_impl(i);

                    for proto in &i.defs {
                        let mut proto = proto.clone();

                        proto.mangle(&i.types.iter().map(|t| t.get_name()).collect::<Vec<_>>());

                        self.add_to_struct_scope(
                            i.name.get_name(),
                            (*proto.name).clone(),
                            proto.node_id,
                        );
                    }
                }
                TopLevel::Mod(_, _m) => (),
                TopLevel::Infix(_, _) => (),
                TopLevel::Function(f) => {
                    self.add_to_current_scope((*f.name).clone(), f.node_id);
                }
            }
        }

        walk_list!(self, visit_top_level, &m.top_levels);
    }

    fn visit_for_in(&mut self, for_in: &'a ForIn) {
        self.visit_expression(&for_in.expr);

        self.add_to_current_scope(for_in.value.name.clone(), for_in.value.node_id);

        self.visit_body(&for_in.body);
    }

    fn visit_assign(&mut self, assign: &'a Assign) {
        self.visit_expression(&assign.value);

        match &assign.name {
            AssignLeftSide::Identifier(id) => {
                let ident = id.as_identifier().unwrap();

                if !assign.is_let {
                    if let Some(previous_assign_node_id) = self.get(ident.name.clone()) {
                        self.resolutions
                            .insert(ident.node_id, previous_assign_node_id);
                    }

                    self.visit_identifier(ident)
                }

                self.add_to_current_scope(ident.name.clone(), ident.node_id);
            }
            AssignLeftSide::Indice(expr) => self.visit_expression(expr),
            AssignLeftSide::Dot(expr) => self.visit_expression(expr),
        }
    }

    fn visit_trait(&mut self, trait_: &'a Trait) {
        self.push_scope();
        self.import_struct_scope(Identifier::new(trait_.name.get_name(), 0));

        walk_trait(self, trait_);

        self.pop_scope();
    }

    fn visit_impl(&mut self, impl_: &'a Impl) {
        self.push_scope();
        self.import_struct_scope(Identifier::new(impl_.name.get_name(), 0));

        walk_impl(self, impl_);

        self.pop_scope();
    }

    fn visit_top_level(&mut self, top: &'a TopLevel) {
        match &top {
            TopLevel::Extern(p) => self.visit_prototype(p),
            TopLevel::FnSignature(_p) => (), // TODO
            TopLevel::Use(u) => {
                self.visit_use(u);
            }
            TopLevel::Infix(_, _) => (),
            TopLevel::Trait(t) => self.visit_trait(t),
            TopLevel::Impl(i) => self.visit_impl(i),
            TopLevel::Struct(s) => self.visit_struct_decl(s),
            TopLevel::Function(f) => self.visit_function_decl(f),
            TopLevel::Mod(name, m) => {
                let current_mod = self.cur_scope.clone();

                self.new_mod(self.cur_scope.child(name.clone()));

                self.visit_mod(m);

                self.cur_scope = current_mod;
            }
        };
    }

    fn visit_struct_decl(&mut self, s: &'a StructDecl) {
        self.push_scope();
        self.import_struct_scope(s.name.clone());

        walk_struct_decl(self, s);

        self.pop_scope()
    }

    fn visit_struct_ctor(&mut self, s: &'a StructCtor) {
        match self.get(s.name.name.clone()) {
            Some(pointed) => self.resolutions.insert(s.name.node_id, pointed),
            None => self
                .parsing_ctx
                .diagnostics
                .push_error(Diagnostic::new_unknown_identifier(
                    self.get_span(s.name.node_id).into(),
                )),
        };

        self.visit_identifier(&s.name);

        walk_struct_ctor(self, s);
    }

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        self.push_scope();

        // self.visit_identifier(&f.name);

        for arg in &f.arguments {
            self.add_to_current_scope(arg.name.clone(), arg.node_id);
        }

        self.visit_body(&f.body);

        self.pop_scope();
    }

    fn visit_use(&mut self, r#use: &'a Use) {
        let ident = r#use.path.last_segment_ref();

        if r#use.path.path.len() == 1 {
            panic!("Unimplemented");
        }

        let mut mod_path = if r#use.path.has_root() {
            r#use.path.parent()
        } else {
            r#use.path.parent().prepend_mod(self.cur_scope.clone())
        };

        mod_path.resolve_supers();

        match self.scopes.get(&mod_path) {
            Some(scopes) => {
                if ident.name == "(*)" {
                    let scope = scopes.scopes.get(0).unwrap();

                    for (k, v) in &scope.clone() {
                        self.add_to_current_scope(k.clone(), v.clone());

                        // This is barbarian, we try each resolution if it match a scope name
                        // If so, we hard-copy the struct scope into the current
                        let mut struct_scope_name = mod_path.clone();
                        struct_scope_name.path.push(Identifier::new(k.clone(), 0));

                        if let Some(struct_scopes) =
                            self.scopes.get_mut(&struct_scope_name).cloned()
                        {
                            let mut struct_scope_name = self.cur_scope.clone();

                            struct_scope_name.path.push(Identifier::new(k.clone(), *v));

                            self.scopes.insert(struct_scope_name, struct_scopes);
                        }
                    }
                } else {
                    match scopes.get((*ident).to_string()) {
                        Some(pointed) => {
                            self.add_to_current_scope((*ident).name.clone(), pointed);
                        }
                        None => self.parsing_ctx.diagnostics.push_error(
                            Diagnostic::new_unknown_identifier(self.get_span(ident.node_id).into()),
                        ),
                    };
                }
            }

            None => self
                .parsing_ctx
                .diagnostics
                .push_error(Diagnostic::new_module_not_found(
                    self.get_span(ident.node_id).into(),
                    mod_path
                        .path
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>()
                        .join("/"),
                )),
        };
    }

    fn visit_identifier_path(&mut self, path: &'a IdentifierPath) {
        let ident = path.last_segment_ref();

        if path.path.len() == 1 {
            self.visit_identifier(ident);

            return;
        }

        let mut mod_path = path.parent().prepend_mod(self.cur_scope.clone());

        mod_path.resolve_supers();

        match self.scopes.get(&mod_path) {
            Some(scopes) => match scopes.get((*ident).to_string()) {
                Some(pointed) => self.resolutions.insert(ident.node_id, pointed),
                None => {
                    self.parsing_ctx
                        .diagnostics
                        .push_error(Diagnostic::new_unknown_identifier(
                            self.get_span(ident.node_id).into(),
                        ))
                }
            },

            // TODO: change to Unknown Mod diagnostic
            None => self
                .parsing_ctx
                .diagnostics
                .push_error(Diagnostic::new_module_not_found(
                    self.get_span(ident.node_id).into(),
                    mod_path
                        .path
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>()
                        .join("/"),
                )),
        };
    }

    fn visit_identifier(&mut self, id: &'a Identifier) {
        match self.get((*id).to_string()) {
            Some(pointed) => self.resolutions.insert(id.node_id, pointed),
            None => self
                .parsing_ctx
                .diagnostics
                .push_error(Diagnostic::new_unknown_identifier(
                    self.get_span(id.node_id).into(),
                )),
        };
    }

    fn visit_secondary_expr(&mut self, node: &'a SecondaryExpr) {
        match node {
            SecondaryExpr::Arguments(args) => walk_list!(self, visit_argument, args),
            SecondaryExpr::Indice(expr) => self.visit_expression(expr),
            SecondaryExpr::Dot(_) => (), // ignored for now, we don't have the means to typecheck deep properties
        }
    }
}
