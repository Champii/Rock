use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::{resolve::ResolutionMap, Type, TypeSignature},
    hir::visit_mut::*,
    hir::*,
};

#[derive(Debug)]
pub struct Monomorphizer<'a> {
    pub root: &'a mut Root,
    pub trans_resolutions: ResolutionMap<HirId>, // old_hir_id => new_hir_id
    pub new_resolutions: ResolutionMap<HirId>,
    pub old_ordered_resolutions: HashMap<HirId, Vec<HirId>>, // fn_call => [fn_decl]
    pub body_arguments: BTreeMap<FnBodyId, Vec<ArgumentDecl>>,
    pub generated_fn_hir_id: HashMap<(HirId, TypeSignature), HirId>, // (Old_fn_id, target_sig) => generated fn hir_id
    pub tmp_resolutions: BTreeMap<HirId, ResolutionMap<HirId>>,
}

impl<'a> Monomorphizer<'a> {
    pub fn run(&mut self) -> Root {
        println!("ENV MONO {:#?}", self.root.type_envs);
        let prototypes = self
            .root
            .top_levels
            .iter()
            .filter(|top| match &top.kind {
                TopLevelKind::Prototype(_) => true,
                _ => false,
            })
            .cloned()
            .collect::<Vec<_>>();

        for top in &prototypes {
            if let TopLevelKind::Prototype(p) = &top.kind {
                self.root
                    .node_types
                    .insert(p.hir_id.clone(), p.signature.to_func_type());
                self.root
                    .node_types
                    .insert(p.name.hir_id.clone(), p.signature.to_func_type());
            }
        }

        let fresh_top_levels = self
            .root
            .type_envs
            .clone()
            .get_inner()
            .iter()
            .map(|(proto_id, sig_map)| {
                let f_decls = sig_map
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

                                self.visit_function_decl(&mut new_f);

                                self.generated_fn_hir_id
                                    .insert((proto_id.clone(), sig.clone()), new_f.hir_id.clone());

                                self.trans_resolutions
                                    .insert(old_f.hir_id.clone(), new_f.hir_id.clone());

                                Some((new_f, sig))
                            }
                            HirNode::Prototype(p) => {
                                self.generated_fn_hir_id
                                    .insert((proto_id.clone(), sig.clone()), p.hir_id.clone());
                                None
                            }
                            _ => {
                                panic!("Not a function decl");
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                f_decls
                    .into_iter()
                    .map(|(mut new_f, sig)| {
                        self.root
                            .type_envs
                            .set_current_fn((proto_id.clone(), sig.clone()));

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
        new_root.top_levels.extend(prototypes);

        new_root.bodies = bodies;

        new_root.resolutions = self.new_resolutions.clone();
        new_root.arena = crate::hir::collect_arena(&new_root);
        new_root.hir_map = self.root.hir_map.clone();
        new_root.spans = self.root.spans.clone();
        new_root.node_types = self.root.node_types.clone();

        new_root
    }

    pub fn duplicate_hir_id(&mut self, old_hir_id: &HirId) -> HirId {
        let new_hir_id = self
            .root
            .hir_map
            .duplicate_hir_mapping(old_hir_id.clone())
            .unwrap();

        new_hir_id
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
    }

    fn visit_function_decl(&mut self, f: &'a mut FunctionDecl) {
        let old_f_hir_id = f.hir_id.clone();
        let old_f_name_hir_id = f.name.hir_id.clone();

        f.hir_id = self.duplicate_hir_id(&f.hir_id);
        f.name.hir_id = self.duplicate_hir_id(&f.name.hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_f_hir_id) {
            self.root.node_types.insert(f.hir_id.clone(), t.clone());
        }
        if let Some(t) = self.root.type_envs.get_type(&old_f_name_hir_id) {
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
                        self.trans_resolutions
                            .get(existing_pointer)
                            .map(|new_pointer_id| {
                                self.trans_resolutions.get(existing_pointee).map(
                                    |new_pointee_id| {
                                        // println!(
                                        //     "NEW RESO FRON TRANS {:?} {:?}",
                                        //     new_pointer_id, new_pointee_id
                                        // );
                                        self.new_resolutions
                                            .insert(new_pointer_id.clone(), new_pointee_id.clone());
                                    },
                                );
                            });
                    });
            });

        self.trans_resolutions = save_trans;
    }

    fn visit_if(&mut self, r#if: &'a mut If) {
        let old_if_id = r#if.hir_id.clone();

        r#if.hir_id = self.duplicate_hir_id(&mut r#if.hir_id);

        if let Some(t) = self.root.type_envs.get_type(&old_if_id) {
            self.root.node_types.insert(r#if.hir_id.clone(), t.clone());
        }

        self.trans_resolutions
            .insert(old_if_id, r#if.hir_id.clone());

        self.visit_expression(&mut r#if.predicat);

        self.visit_body(&mut r#if.body);

        if let Some(e) = &mut r#if.else_ {
            match &mut **e {
                Else::Body(b) => {
                    self.visit_body(b);
                }
                Else::If(i) => {
                    self.visit_if(i);
                }
            }
        }
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
            HirNode::FunctionDecl(_) => {
                self.new_resolutions.insert(
                    fc.op.get_hir_id(),
                    self.generated_fn_hir_id
                        .get(&(
                            self.resolve_rec(&old_fc_op).unwrap(),
                            fc.to_type_signature(&self.root.node_types),
                        ))
                        .unwrap()
                        .clone(),
                );

                self.trans_resolutions.remove(&old_fc_op);
            }
            HirNode::Prototype(p) => {
                let f_type = self.root.type_envs.get_type(&old_fc_op).unwrap();

                if let Type::FuncType(f_type) = f_type {
                    // Traits
                    if let Some(f) = self
                        .root
                        .get_trait_method((*p.name).clone(), &f_type.to_type_signature())
                    {
                        self.new_resolutions.insert(
                            fc.op.get_hir_id(),
                            self.trans_resolutions.get(&f.hir_id).unwrap(),
                        );
                    // Extern Prototypes
                    } else {
                        self.new_resolutions
                            .insert(fc.op.get_hir_id(), self.resolve_rec(&old_fc_op).unwrap());
                    }
                }

                self.trans_resolutions.remove(&old_fc_op);
            }
            _ => {
                // FIXME: This may be bad
                self.new_resolutions
                    .insert(fc.op.get_hir_id(), self.resolve_rec(&old_fc_op).unwrap());
            }
        }

        for (i, arg) in fc.args.iter().enumerate() {
            if let Type::FuncType(f) = self.root.node_types.get(&arg.get_hir_id()).unwrap() {
                if let Some(reso) = self.resolve(&old_fc_args.get(i).unwrap()) {
                    self.new_resolutions.insert(
                        arg.get_hir_id(),
                        self.generated_fn_hir_id
                            .get(&(reso, f.to_type_signature()))
                            .unwrap()
                            .clone(),
                    );
                } else {
                    println!("NO RESO FOR {:#?}", arg.get_hir_id())
                }

                self.trans_resolutions.remove(&old_fc_args.get(i).unwrap());
            }
        }
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
