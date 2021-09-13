use crate::{hir::visit_mut::*, hir::*, walk_list};
use std::collections::HashMap;

use super::call_solver::Bindings;

#[derive(Debug)]
pub struct Monomorphizer<'a> {
    pub bindings: Bindings, // fc_call => prefixes
    pub root: &'a mut Root,
}

impl<'a> Monomorphizer<'a> {
    pub fn run(&mut self) -> Root {
        let fresh_top_levels = self
            .bindings
            .clone()
            .iter()
            .map(|(proto_id, (sig, calls))| {
                calls
                    .into_iter()
                    .map(|fn_call| {
                        let f = self.root.arena.get(&proto_id).unwrap();

                        if let HirNode::FunctionDecl(f) = f {
                            let mut new_f = f.clone();

                            self.visit_function_decl(&mut new_f);

                            let fn_body = self.root.bodies.get(&new_f.body_id).unwrap();

                            let mut new_fn_body = fn_body.clone();

                            new_f.body_id = FnBodyId::next();
                            new_fn_body.id = new_f.body_id.clone();

                            self.visit_fn_body(&mut new_fn_body);

                            // self.root.bodies.insert(new_fn_body.id.clone(), new_fn_body.clone());

                            (new_f, new_fn_body)
                        } else {
                            panic!("Not a function decl");
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let mut new_root = Root::default();

        let (tops, bodies) = fresh_top_levels
            .into_iter()
            .map(|(f, body)| {
                let top = TopLevel {
                    hir_id: f.get_hir_id(),
                    kind: TopLevelKind::Function(f),
                };

                ((top.hir_id.clone(), top), (body.id.clone(), body.clone()))
            })
            .unzip();

        new_root.top_levels = tops;
        new_root.bodies = bodies;

        let mut main = self.root.get_function_by_name("main").unwrap();
        self.visit_function_decl(&mut main);

        let mut main_body = self.root.bodies.get(&main.body_id).unwrap().clone();

        main_body.id = FnBodyId::next();
        main.body_id = main_body.id.clone();

        self.visit_fn_body(&mut main_body);

        new_root.top_levels.insert(
            main.hir_id.clone(),
            TopLevel {
                hir_id: main.hir_id.clone(),
                kind: TopLevelKind::Function(main),
            },
        );
        new_root.bodies.insert(main_body.id.clone(), main_body);

        println!("{:#?}", new_root);

        new_root
    }
}

impl<'a, 'b> VisitorMut<'a> for Monomorphizer<'b> {
    fn visit_function_decl(&mut self, f: &'a mut FunctionDecl) {
        f.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(f.hir_id.clone())
            .unwrap();

        walk_function_decl(self, f);
    }

    // FIXME: missing IF, assign, etc etc

    fn visit_identifier(&mut self, id: &'a mut Identifier) {
        // TODO: update resolution map

        id.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(id.hir_id.clone())
            .unwrap();

        walk_identifier(self, id);
    }
}

pub fn monomophize(bindings: Bindings, root: &mut Root) -> Root {
    Monomorphizer { root, bindings }.run()
}
