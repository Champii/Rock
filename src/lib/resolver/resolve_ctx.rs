use std::collections::HashMap;

use crate::{
    ast::{tree::*, visit::*, NodeId},
    diagnostics::Diagnostic,
    helpers::scopes::*,
    parser::span2::Span as Span2,
    parser::ParsingCtx,
    resolver::ResolutionMap,
};

#[derive(Debug)]
pub struct ResolveCtx<'a> {
    pub parsing_ctx: &'a mut ParsingCtx,
    pub scopes: HashMap<IdentifierPath, Scopes<String, NodeId>>, // <ModPath, ModScopes>
    pub cur_scope: IdentifierPath,
    pub resolutions: ResolutionMap<NodeId>,
}

impl<'a> ResolveCtx<'a> {
    pub fn add_to_current_scope(&mut self, name: String, node_id: NodeId) {
        if let Some(ref mut scopes) = self.scopes.get_mut(&self.cur_scope) {
            scopes.add(name, node_id);
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

    pub fn get(&mut self, name: String) -> Option<NodeId> {
        match self.scopes.get_mut(&self.cur_scope) {
            Some(ref mut scopes) => scopes.get(name),
            None => None,
        }
    }

    pub fn get_span2(&self, node_id: NodeId) -> Span2 {
        self.parsing_ctx.identities.get(&node_id).unwrap().clone()
    }
}

impl<'a> Visitor<'a> for ResolveCtx<'a> {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first
        for top in &m.top_levels {
            match &top {
                TopLevel::Prototype(p) => {
                    self.add_to_current_scope((*p.name).clone(), p.node_id);
                }
                TopLevel::Use(_u) => (),
                TopLevel::Trait(t) => {
                    for proto in &t.defs {
                        self.add_to_current_scope((*proto.name).clone(), proto.node_id);
                    }
                }
                TopLevel::Struct(s) => {
                    self.add_to_current_scope(s.name.name.clone(), s.name.node_id);

                    s.defs.iter().for_each(|p| {
                        self.add_to_current_scope((*p.name).clone(), p.node_id);
                    })
                }
                TopLevel::Impl(i) => {
                    for proto in &i.defs {
                        let mut proto = proto.clone();

                        proto.mangle(&i.types.iter().map(|t| t.get_name()).collect::<Vec<_>>());

                        self.add_to_current_scope((*proto.name).clone(), proto.node_id);
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

    // fn visit_for(&mut self, for_loop: &'a For) {
    //     self.visit_expression(&for_loop.expr);
    //     self.visit_body(&for_loop.body);
    // }

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

    fn visit_top_level(&mut self, top: &'a TopLevel) {
        match &top {
            TopLevel::Prototype(p) => self.visit_prototype(p),
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
                    self.get_span2(s.name.node_id).into(),
                )),
        };

        self.visit_identifier(&s.name);

        walk_struct_ctor(self, s);
    }

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        self.push_scope();

        self.visit_identifier(&f.name);

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

        let mut mod_path = r#use.path.parent().prepend_mod(self.cur_scope.clone());

        mod_path.resolve_supers();

        match self.scopes.get(&mod_path) {
            Some(scopes) => {
                if ident.name == "(*)" {
                    let scope = scopes.scopes.get(0).unwrap();

                    for (k, v) in &scope.items.clone() {
                        self.add_to_current_scope(k.clone(), v.clone());
                    }
                } else {
                    match scopes.get((*ident).to_string()) {
                        Some(pointed) => {
                            self.add_to_current_scope((*ident).name.clone(), pointed);
                        }
                        None => self.parsing_ctx.diagnostics.push_error(
                            Diagnostic::new_unknown_identifier(
                                self.get_span2(ident.node_id).into(),
                            ),
                        ),
                    };
                }
            }

            None => self
                .parsing_ctx
                .diagnostics
                .push_error(Diagnostic::new_module_not_found(
                    self.get_span2(ident.node_id).into(),
                    mod_path
                        .path
                        .iter()
                        .map(|p| p.name.clone())
                        .collect::<Vec<_>>()
                        .join("/"),
                )),
        };
    }

    /* fn visit_argument_decl(&mut self, arg: &'a ArgumentDecl) {
           self.add_to_current_scope(arg.name.clone(), arg.node_id);
       }
    */
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
                            self.get_span2(ident.node_id).into(),
                        ))
                }
            },

            // TODO: change to Unknown Mod diagnostic
            None => self
                .parsing_ctx
                .diagnostics
                .push_error(Diagnostic::new_module_not_found(
                    self.get_span2(ident.node_id).into(),
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
                    self.get_span2(id.node_id).into(),
                )),
        };
    }
}
