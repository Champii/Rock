use crate::{ast::resolve::ResolutionMap, hir::visit_mut::*, hir::*, walk_list, InferState};
use std::collections::{BTreeMap, HashMap};

use super::call_solver::Bindings;

#[derive(Debug)]
pub struct Monomorphizer<'a> {
    pub bindings: Bindings, // fc_call => prefixes
    pub root: &'a mut Root,
    pub old_resolutions: ResolutionMap<HirId>, // new_hir_id => old_hir_id
    pub new_resolutions: ResolutionMap<HirId>,
    pub tmp_ordered_resolutions: HashMap<HirId, Vec<HirId>>, // fn_call => [fn_decl]
    pub body_arguments: BTreeMap<FnBodyId, Vec<ArgumentDecl>>,
}

impl<'a> Monomorphizer<'a> {
    pub fn run(&mut self) -> Root {
        let fresh_top_levels = self
            .bindings
            .clone()
            .iter()
            .map(|(proto_id, (sig, calls))| {
                let f_decls = calls
                    .into_iter()
                    .map(|fn_call| {
                        let f = self.root.arena.get(&proto_id).unwrap();

                        if let HirNode::FunctionDecl(f) = f {
                            let old_f = f.clone();
                            let mut new_f = f.clone();

                            self.visit_function_decl(&mut new_f);

                            self.tmp_ordered_resolutions
                                .entry(fn_call.call_hir_id.clone())
                                .or_insert_with(|| vec![])
                                .push(new_f.hir_id.clone());

                            (new_f, old_f)
                        } else {
                            panic!("Not a function decl");
                        }
                    })
                    .collect::<Vec<_>>();

                f_decls
                    .into_iter()
                    .map(|(mut new_f, old_f)| {
                        let fn_body = self.root.bodies.get(&new_f.body_id).unwrap();

                        let mut new_fn_body = fn_body.clone();

                        new_f.body_id = FnBodyId::next();
                        new_fn_body.id = new_f.body_id.clone();

                        self.body_arguments
                            .insert(new_f.body_id.clone(), new_f.arguments.clone());

                        self.visit_fn_body(&mut new_fn_body);

                        new_f.arguments = self.body_arguments.get(&new_f.body_id).unwrap().clone();

                        (new_f, new_fn_body)
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
                    kind: TopLevelKind::Function(f),
                };

                (top, (body.id.clone(), body))
            })
            .unzip();

        new_root.top_levels = tops;
        new_root.bodies = bodies;

        let mut main = self.root.get_function_by_name("main").unwrap();
        self.visit_function_decl(&mut main);

        let mut main_body = self.root.bodies.get(&main.body_id).unwrap().clone();

        main_body.id = FnBodyId::next();
        main.body_id = main_body.id.clone();

        self.body_arguments.insert(main.body_id.clone(), vec![]);

        self.visit_fn_body(&mut main_body);

        new_root.top_levels.push(TopLevel {
            kind: TopLevelKind::Function(main),
        });

        println!("NEW RESO {:#?}", self.new_resolutions);

        new_root.bodies.insert(main_body.id.clone(), main_body);

        new_root.resolutions = self.new_resolutions.clone();
        new_root.arena = crate::hir::collect_arena(&new_root);
        new_root.hir_map = self.root.hir_map.clone();
        new_root.spans = self.root.spans.clone();

        new_root
    }
}

impl<'a, 'b> VisitorMut<'a> for Monomorphizer<'b> {
    fn visit_literal(&mut self, literal: &'a mut Literal) {
        literal.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(literal.hir_id.clone())
            .unwrap();
    }

    fn visit_function_decl(&mut self, f: &'a mut FunctionDecl) {
        f.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(f.hir_id.clone())
            .unwrap();

        f.name.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(f.name.hir_id.clone())
            .unwrap();

        // for arg in f.arguments.iter_mut() {
        //     arg.name.hir_id = self
        //         .root
        //         .hir_map
        //         .duplicate_hir_mapping(arg.name.hir_id.clone())
        //         .unwrap();
        // }
    }

    fn visit_fn_body(&mut self, fn_body: &'a mut FnBody) {
        self.old_resolutions.clear();

        let mut args = self.body_arguments.get(&fn_body.id).unwrap().clone();

        args.iter_mut()
            .for_each(|arg| self.visit_argument_decl(arg));

        self.body_arguments.insert(fn_body.id.clone(), args);

        walk_fn_body(self, fn_body);

        self.old_resolutions
            .get_map()
            .iter()
            .for_each(|(old_pointer_id, new_pointee_id)| {
                self.root
                    .resolutions
                    .get_map()
                    .iter()
                    .filter(|(pointer, pointee)| {
                        *pointer == old_pointer_id // || *pointee == old_pointer_id
                    })
                    .for_each(|(existing_pointer, existing_pointee)| {
                        self.old_resolutions
                            .get(existing_pointer)
                            .map(|new_pointer_id| {
                                self.old_resolutions
                                    .get(existing_pointee)
                                    .map(|new_pointee_id| {
                                        println!(
                                            "NEW RESOLUTION!!!!! {:?} => {:?}",
                                            new_pointer_id, new_pointee_id
                                        );
                                        self.new_resolutions
                                            .insert(new_pointer_id.clone(), new_pointee_id.clone());
                                    });
                            });
                    });
            });

        println!("OLD ROOT RESO {:#?}", self.root.resolutions);
        println!("OLD RESO {:#?}", self.old_resolutions);
    }

    // FIXME: missing IF, assign, etc etc
    fn visit_function_call(&mut self, fc: &'a mut FunctionCall) {
        // TODO: update resolution map

        let new_pointed_fn_id = self
            .tmp_ordered_resolutions
            .get_mut(&fc.op.get_hir_id())
            .unwrap()
            .remove(0);

        fc.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(fc.hir_id.clone())
            .unwrap();

        walk_function_call(self, fc);

        self.new_resolutions
            .insert(fc.op.get_hir_id(), new_pointed_fn_id.clone());
    }

    // fn visit_argument_decl(&mut self, arg: &'a mut ArgumentDecl) {
    //     let old_hir_id = arg.hir_id.clone();

    //     arg.hir_id = self
    //         .root
    //         .hir_map
    //         .duplicate_hir_mapping(old_hir_id.clone())
    //         .unwrap();

    //     // FIXME: ignore already set reso
    //     println!(
    //         "ADD OLD RESO {} {:?} => {:?}",
    //         arg.name, old_hir_id, arg.hir_id
    //     );

    //     self.old_resolutions.insert(old_hir_id, arg.hir_id.clone());
    // }

    fn visit_identifier(&mut self, id: &'a mut Identifier) {
        let old_hir_id = id.hir_id.clone();

        id.hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(old_hir_id.clone())
            .unwrap();

        // FIXME: ignore already set reso
        println!(
            "ADD OLD RESO {} {:?} => {:?}",
            id.name, old_hir_id, id.hir_id
        );

        self.old_resolutions.insert(old_hir_id, id.hir_id.clone());

        // walk_identifier(self, id);
    }
}

pub fn monomophize(root: &mut Root) -> Root {
    let mut protos = super::proto_collector::collect_prototypes(root);
    let calls = super::call_collector::collect_calls(root);

    let bindings = super::call_solver::solve_calls(protos, calls, root);

    Monomorphizer {
        root,
        bindings,
        old_resolutions: ResolutionMap::default(),
        new_resolutions: ResolutionMap::default(),
        tmp_ordered_resolutions: HashMap::new(),
        body_arguments: BTreeMap::new(),
    }
    .run()
}
