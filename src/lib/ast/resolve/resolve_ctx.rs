use crate::{
    ast::resolve::ResolutionMap, ast::visit::*, ast::*, diagnostics::Diagnostic,
    parser::ParsingCtx, scopes::*, NodeId,
};

#[derive(Debug)]
pub struct ResolveCtx<'a> {
    pub parsing_ctx: &'a mut ParsingCtx,
    scopes: Scopes<String, Identity>,
    resolutions: ResolutionMap<NodeId>,
}

impl<'a> Visitor<'a> for ResolveCtx<'a> {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first
        for top in &m.top_levels {
            match &top.kind {
                TopLevelKind::Mod(_, _m) => (),
                TopLevelKind::Function(f) => {
                    self.scopes.add((*f.name).clone(), f.identity.clone());
                }
            }
        }

        walk_list!(self, visit_top_level, &m.top_levels);
    }

    fn visit_top_level(&mut self, top: &'a TopLevel) {
        match &top.kind {
            TopLevelKind::Function(f) => {
                self.scopes.push();

                walk_function_decl(self, f);

                self.scopes.pop();
            }
            TopLevelKind::Mod(_, m) => self.visit_mod(m),
        };
    }

    fn visit_argument_decl(&mut self, arg: &'a ArgumentDecl) {
        self.scopes.add(arg.name.clone(), arg.identity.clone());
    }

    fn visit_identifier(&mut self, id: &'a Identifier) {
        match self.scopes.get((*id).to_string()) {
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

pub fn resolve(root: &mut Root, parsing_ctx: &mut ParsingCtx) {
    let mut ctx = ResolveCtx {
        parsing_ctx,
        scopes: Scopes::new(),
        resolutions: ResolutionMap::default(),
    };

    ctx.visit_root(root);

    root.resolutions = ctx.resolutions;
}
