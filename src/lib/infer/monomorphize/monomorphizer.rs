use std::collections::{BTreeMap, HashMap};

use crate::{
    hir::visit_mut::*,
    hir::*,
    infer::Env,
    resolver::ResolutionMap,
    ty::{FuncType, Type},
};

#[derive(Debug)]
pub struct Monomorphizer<'a> {
    pub root: &'a mut Root,
    pub trans_resolutions: ResolutionMap<HirId>, // old_hir_id => new_hir_id
    pub new_resolutions: ResolutionMap<HirId>,
    pub old_ordered_resolutions: HashMap<HirId, Vec<HirId>>, // fn_call => [fn_decl]
    pub body_arguments: BTreeMap<FnBodyId, Vec<ArgumentDecl>>,
    pub generated_fn_hir_id: HashMap<(HirId, FuncType), HirId>, // (Old_fn_id, target_sig) => generated fn hir_id
    pub tmp_resolutions: BTreeMap<HirId, ResolutionMap<HirId>>,
    pub structs: HashMap<String, StructDecl>,
}

impl<'a> Monomorphizer<'a> {
    pub fn get_fn_envs_pairs(&self) -> Vec<(HirId, HashMap<FuncType, Env>)> {
        self.root
            .type_envs
            .get_inner()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
    }

    pub fn run(&mut self) -> Root {
        let prototypes = self
            .root
            .top_levels
            .iter()
            .filter(|top| match &top.kind {
                TopLevelKind::Extern(p) => p.signature.is_solved(),
                _ => false,
            })
            .cloned()
            .collect::<Vec<_>>();

        for top in &prototypes {
            if let TopLevelKind::Extern(p) = &top.kind {
                let f_type: Type = p.signature.clone().into();

                self.root
                    .node_types
                    .insert(p.hir_id.clone(), f_type.clone());
                self.root.node_types.insert(p.name.hir_id.clone(), f_type);
            }
        }

        let fresh_top_levels_flat = self
            .get_fn_envs_pairs()
            .into_iter()
            .map(|(proto_id, sig_map)| {
                sig_map
                    .into_iter()
                    .filter_map(|(sig, _env)| {
                        let f = self.root.arena.get(&proto_id).unwrap();

                        match f {
                            HirNode::FunctionDecl(f) => {
                                let old_f = f.clone();
                                let mut new_f = f.clone();

                                self.root
                                    .type_envs
                                    .set_current_fn((proto_id.clone(), sig.clone()));

                                new_f.signature = sig.clone();

                                self.visit_function_decl(&mut new_f);

                                self.generated_fn_hir_id
                                    .insert((proto_id.clone(), sig.clone()), new_f.hir_id.clone());

                                self.trans_resolutions
                                    .insert(old_f.hir_id, new_f.hir_id.clone());

                                Some((proto_id.clone(), new_f, sig))
                            }
                            HirNode::Prototype(p) => {
                                self.generated_fn_hir_id
                                    .insert((proto_id.clone(), sig), p.hir_id.clone());

                                None
                            }
                            _ => {
                                panic!("Not a function decl");
                            }
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        let fresh_top_levels =
            fresh_top_levels_flat
                .into_iter()
                .map(|(proto_id, mut new_f, sig)| {
                    self.root.type_envs.set_current_fn((proto_id, sig));

                    let fn_body = self.root.bodies.get(&new_f.body_id).unwrap();

                    let mut new_fn_body = fn_body.clone();

                    new_f.body_id = self.root.hir_map.next_body_id();
                    new_fn_body.id = new_f.body_id.clone();

                    self.body_arguments
                        .insert(new_f.body_id.clone(), new_f.arguments.clone());

                    self.visit_fn_body(&mut new_fn_body);

                    new_fn_body.name = new_f.name.clone();
                    new_fn_body.fn_id = new_f.hir_id.clone();

                    new_f.arguments = self.body_arguments.get(&new_f.body_id).unwrap().clone();

                    (new_f, new_fn_body)
                });

        let mut new_root = Root::default();

        let (tops, bodies) = fresh_top_levels
            .map(|(f, body)| {
                let top = TopLevel {
                    kind: TopLevelKind::Function(f),
                };

                (top, (body.id.clone(), body))
            })
            .unzip();

        new_root.top_levels = tops;
        new_root.top_levels.extend(prototypes);

        new_root.bodies = bodies;

        new_root.resolutions = self.new_resolutions.clone();
        new_root.arena = crate::hir::collect_arena(&new_root);
        new_root.hir_map = self.root.hir_map.clone();
        new_root.spans = self.root.spans.clone();
        new_root.structs = self.structs.clone(); // TODO: monomorphize that
        new_root.node_types = self.root.node_types.clone();
        new_root.trait_solver = self.root.trait_solver.clone();

        new_root
    }

    pub fn duplicate_hir_id(&mut self, old_hir_id: &HirId) -> HirId {
        self.root
            .hir_map
            .duplicate_hir_mapping(old_hir_id.clone())
            .unwrap()
    }

    pub fn resolve(&self, id: &HirId) -> Option<HirId> {
        self.root.resolutions.get(id).or_else(|| {
            self.tmp_resolutions
                .get(&self.root.type_envs.get_current_fn().0)?
                .get(id)
        })
    }

    pub fn resolve_rec(&self, id: &HirId) -> Option<HirId> {
        self.resolve(id)
            .and_then(|reso| self.resolve_rec(&reso).or(Some(reso)))
    }
}

impl<'a, 'b> VisitorMut<'a> for Monomorphizer<'b> {
    fn visit_literal(&mut self, literal: &'a mut Literal) {
        let old_hir_id = literal.hir_id.clone();

        literal.hir_id = self.duplicate_hir_id(&literal.hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root
                .node_types
                .insert(literal.hir_id.clone(), t.clone());
        }

        if let LiteralKind::Array(arr) = &mut literal.kind {
            self.visit_array(arr);
        }
    }

    fn visit_prototype(&mut self, _p: &'a mut Prototype) {}

    fn visit_function_decl(&mut self, f: &'a mut FunctionDecl) {
        let old_f_hir_id = f.hir_id.clone();

        f.hir_id = self.duplicate_hir_id(&f.hir_id);
        f.name.hir_id = self.duplicate_hir_id(&f.name.hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_f_hir_id) {
            self.root.node_types.insert(f.hir_id.clone(), t.clone());
            self.root
                .node_types
                .insert(f.name.hir_id.clone(), t.clone());
        }
    }

    fn visit_fn_body(&mut self, fn_body: &'a mut FnBody) {
        let save_trans = self.trans_resolutions.clone();

        let mut args = self.body_arguments.get(&fn_body.id).unwrap().clone();

        args.iter_mut()
            .for_each(|arg| self.visit_argument_decl(arg));

        self.body_arguments.insert(fn_body.id.clone(), args);

        fn_body.name.hir_id = self.duplicate_hir_id(&fn_body.name.hir_id);

        walk_fn_body(self, fn_body);

        self.trans_resolutions
            .get_map()
            .iter()
            .for_each(|(old_pointer_id, _new_pointee_id)| {
                self.root
                    .resolutions
                    .get_map()
                    .iter()
                    .filter(|(pointer, _pointee)| *pointer == old_pointer_id)
                    .for_each(|(existing_pointer, existing_pointee)| {
                        if let Some(new_pointer_id) = self.trans_resolutions.get(existing_pointer) {
                            if let Some(new_pointee_id) =
                                self.trans_resolutions.get(existing_pointee)
                            {
                                self.new_resolutions.insert(new_pointer_id, new_pointee_id);
                            }
                        }
                    });
            });

        self.trans_resolutions = save_trans;
    }

    fn visit_if(&mut self, r#if: &'a mut If) {
        let old_if_id = r#if.hir_id.clone();

        r#if.hir_id = self.duplicate_hir_id(&r#if.hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_if_id) {
            self.root.node_types.insert(r#if.hir_id.clone(), t.clone());
        }

        self.trans_resolutions
            .insert(old_if_id, r#if.hir_id.clone());

        self.visit_expression(&mut r#if.predicat);

        self.visit_body(&mut r#if.body);
    }

    // FIXME: missing IF, assign, etc etc
    fn visit_function_call(&mut self, fc: &'a mut FunctionCall) {
        let old_fc = fc.get_hir_id();
        let old_fc_op = fc.op.get_hir_id();
        fc.hir_id = self.duplicate_hir_id(&fc.hir_id);
        let old_fc_args = fc
            .args
            .iter()
            .map(|arg| arg.get_hir_id())
            .collect::<Vec<_>>();

        walk_function_call(self, fc);

        if let Some(t) = self.root.type_envs.get_type(&old_fc) {
            self.root.node_types.insert(fc.hir_id.clone(), t.clone());
        }

        match self
            .root
            .arena
            .get(&self.resolve(&old_fc_op).unwrap())
            .unwrap()
        {
            HirNode::FunctionDecl(f) => {
                if let Some(generated_fn) = self.generated_fn_hir_id.get(&(
                    self.resolve_rec(&old_fc_op).unwrap(),
                    fc.to_func_type(&self.root.node_types),
                )) {
                    self.new_resolutions
                        .insert(fc.op.get_hir_id(), generated_fn.clone());
                } else {
                    // This is a dirty duplicate of the `Prototype` branch below
                    if let Some(f) = self.root.get_trait_method(
                        (*f.name).clone(),
                        &fc.to_func_type(&self.root.node_types),
                    ) {
                        if let Some(trans_res) = self.trans_resolutions.get(&f.hir_id) {
                            self.new_resolutions.insert(fc.op.get_hir_id(), trans_res);
                        } else {
                            panic!(
                                "NO TRANS RES FOR TRAIT {:#?} {:#?} {:#?}",
                                self.trans_resolutions,
                                fc.op.get_hir_id(),
                                f.hir_id,
                            );
                        }
                    } else {
                    }
                    // panic!("BUG: Cannot find function from signature");
                }

                self.trans_resolutions.remove(&old_fc_op);
            }
            HirNode::Prototype(p) => {
                let f_type = self.root.type_envs.get_type(&old_fc_op).unwrap();

                if let Type::Func(f_type) = f_type {
                    // Traits
                    if let Some(f) = self.root.get_trait_method((*p.name).clone(), f_type) {
                        if let Some(trans_res) = self.trans_resolutions.get(&f.hir_id) {
                            self.new_resolutions.insert(fc.op.get_hir_id(), trans_res);
                        } else {
                            panic!(
                                "NO TRANS RES FOR TRAIT {:#?} {:#?} {:#?}",
                                self.trans_resolutions,
                                fc.op.get_hir_id(),
                                f.hir_id,
                            );
                        }
                    // Extern Prototypes
                    } else {
                        self.new_resolutions
                            .insert(fc.op.get_hir_id(), self.resolve_rec(&old_fc_op).unwrap());
                    }
                }

                self.trans_resolutions.remove(&old_fc_op);
            }
            _ => {}
        }

        for (i, arg) in fc.args.iter().enumerate() {
            if let Type::Func(f) = self.root.node_types.get(&arg.get_hir_id()).unwrap() {
                if let Some(reso) = self.resolve(old_fc_args.get(i).unwrap()) {
                    self.new_resolutions.insert(
                        arg.get_hir_id(),
                        self.generated_fn_hir_id
                            .get(&(reso, f.clone()))
                            .unwrap()
                            .clone(),
                    );
                } else {
                    println!("NO RESO FOR {:#?}", arg.get_hir_id())
                }

                self.trans_resolutions.remove(old_fc_args.get(i).unwrap());
            }
        }
    }

    fn visit_indice(&mut self, indice: &'a mut Indice) {
        let old_hir_id = indice.hir_id.clone();

        indice.hir_id = self.duplicate_hir_id(&old_hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root
                .node_types
                .insert(indice.hir_id.clone(), t.clone());
        }

        self.trans_resolutions
            .insert(old_hir_id, indice.hir_id.clone());

        self.visit_expression(&mut indice.op);
        self.visit_expression(&mut indice.value);
    }

    fn visit_dot(&mut self, dot: &'a mut Dot) {
        let old_hir_id = dot.hir_id.clone();

        dot.hir_id = self.duplicate_hir_id(&old_hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root.node_types.insert(dot.hir_id.clone(), t.clone());
        }

        self.trans_resolutions
            .insert(old_hir_id, dot.hir_id.clone());

        self.visit_expression(&mut dot.op);
        self.visit_identifier(&mut dot.value);
    }

    fn visit_struct_decl(&mut self, s: &'a mut StructDecl) {
        let old_hir_id = s.name.hir_id.clone();

        s.name.hir_id = self.duplicate_hir_id(&old_hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root
                .node_types
                .insert(s.name.hir_id.clone(), t.clone());
        }

        self.trans_resolutions
            .insert(old_hir_id, s.name.hir_id.clone());

        s.defs.iter().for_each(|p| {
            let t = *s
                .to_type()
                .as_struct_type()
                .defs
                .get(&p.name.name)
                .unwrap()
                .clone();
            self.root.node_types.insert(p.get_hir_id(), t.clone());
            self.root.node_types.insert(p.name.get_hir_id(), t);
        });

        self.structs.insert(s.name.name.clone(), s.clone());
    }

    fn visit_struct_ctor(&mut self, s: &'a mut StructCtor) {
        let old_hir_id = s.name.hir_id.clone();

        let mut s_decl = self.root.structs.get(&s.name.name).unwrap().clone();

        // TODO: Do that once
        self.visit_struct_decl(&mut s_decl);

        s.name.hir_id = self.duplicate_hir_id(&old_hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root
                .node_types
                .insert(s.name.hir_id.clone(), t.clone());
        }

        self.trans_resolutions
            .insert(old_hir_id, s.name.hir_id.clone());

        s.defs = s
            .defs
            .iter_mut()
            .map(|(old_k, def)| {
                let mut k = old_k.clone();
                self.visit_identifier(&mut k);
                let old_def_id = def.get_hir_id();
                self.visit_expression(def);

                if let Type::Func(ft) = self.root.node_types.get(&def.get_hir_id()).unwrap() {
                    if let Some(reso) = self.resolve(&old_def_id) {
                        if let HirNode::FunctionDecl(_f2) = self.root.arena.get(&reso).unwrap() {
                            self.new_resolutions.insert(
                                def.get_hir_id(),
                                self.generated_fn_hir_id
                                    .get(&(reso, ft.clone()))
                                    .unwrap()
                                    .clone(),
                            );

                            self.trans_resolutions.remove(&old_def_id);
                        } else {
                        }
                    }
                }
                (k, def.clone())
            })
            .collect();
    }

    fn visit_identifier(&mut self, id: &'a mut Identifier) {
        let old_hir_id = id.hir_id.clone();

        id.hir_id = self.duplicate_hir_id(&old_hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root.node_types.insert(id.hir_id.clone(), t.clone());
        }

        self.trans_resolutions.insert(old_hir_id, id.hir_id.clone());
    }

    fn visit_native_operator(&mut self, op: &'a mut NativeOperator) {
        let old_hir_id = op.hir_id.clone();

        op.hir_id = self.duplicate_hir_id(&old_hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_hir_id) {
            self.root.node_types.insert(op.hir_id.clone(), t.clone());
        }

        self.trans_resolutions.insert(old_hir_id, op.hir_id.clone());
    }
}
