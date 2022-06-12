use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::{tree::*, NodeId},
    hir::{self, Arena, FnBodyId, HirId},
    infer::Envs,
    ty::*,
};

use super::{hir_map::HirMap, InfixDesugar};

pub struct AstLoweringContext {
    hir_map: HirMap,
    top_levels: Vec<hir::TopLevel>,
    bodies: BTreeMap<FnBodyId, hir::FnBody>,
    operators_list: HashMap<String, u8>,
    traits: HashMap<Type, hir::Trait>,
    trait_methods: BTreeMap<String, HashMap<FuncType, hir::FunctionDecl>>,
    struct_methods: BTreeMap<HirId, HashMap<FuncType, hir::FunctionDecl>>,
    structs: HashMap<String, hir::StructDecl>,
}

impl AstLoweringContext {
    pub fn new(operators_list: HashMap<String, u8>) -> Self {
        Self {
            hir_map: HirMap::new(),
            top_levels: Vec::new(),
            bodies: BTreeMap::new(),
            traits: HashMap::new(),
            trait_methods: BTreeMap::new(),
            struct_methods: BTreeMap::new(),
            structs: HashMap::new(),
            operators_list,
        }
    }

    pub fn lower_root(&mut self, root: &Root) -> hir::Root {
        self.lower_mod(&root.r#mod);

        let mut hir = hir::Root {
            arena: Arena::new(),
            hir_map: self.hir_map.clone(),
            resolutions: root.resolutions.lower_resolution_map(&self.hir_map),
            type_envs: Envs::default(),
            node_types: BTreeMap::new(),
            top_levels: self.top_levels.clone(),
            bodies: self.bodies.clone(),
            traits: self.traits.clone(),
            trait_methods: self.trait_methods.clone(),
            struct_methods: self.struct_methods.clone(),
            spans: root.spans.clone(),
            structs: self.structs.clone(),
            trait_solver: root.trait_solver.clone(),
        };

        hir.arena = hir::collect_arena(&hir);

        hir
    }

    pub fn lower_mod(&mut self, r#mod: &Mod) {
        r#mod
            .top_levels
            .iter()
            .for_each(|t| self.lower_top_level(t));
    }

    pub fn lower_top_level(&mut self, top_level: &TopLevel) {
        match &top_level {
            TopLevel::Prototype(p) => {
                let top_level = hir::TopLevel {
                    kind: hir::TopLevelKind::Prototype(self.lower_prototype(p)),
                };

                self.top_levels.push(top_level);
            }
            TopLevel::Function(f) => {
                let top_level = hir::TopLevel {
                    kind: hir::TopLevelKind::Function(self.lower_function_decl(f)),
                };

                self.top_levels.push(top_level);
            }
            TopLevel::Trait(t) => {
                self.lower_trait(t);
            }
            TopLevel::Struct(s) => {
                self.lower_struct_decl(s);
            }
            TopLevel::Impl(i) => {
                self.lower_impl(i);
            }
            TopLevel::Mod(_name, mod_) => self.lower_mod(mod_),
            TopLevel::Infix(_, _) => (),
            TopLevel::Use(_u) => (),
        };
    }

    pub fn lower_struct_decl(&mut self, s: &StructDecl) -> hir::StructDecl {
        let hir_t = hir::StructDecl {
            name: self.lower_identifier(&s.name),
            defs: s
                .defs
                .iter()
                .map(|proto| self.lower_prototype(proto))
                .collect(),
        };

        self.structs.insert(s.name.to_string(), hir_t.clone());

        hir_t
    }

    pub fn lower_trait(&mut self, t: &Trait) -> hir::Trait {
        let hir_t = hir::Trait {
            name: t.name.clone(),
            types: t.types.clone(),
            defs: t
                .defs
                .iter()
                .map(|proto| self.lower_prototype(proto))
                .collect(),
        };

        self.traits.insert(t.name.clone(), hir_t.clone());

        hir_t
    }

    pub fn lower_impl(&mut self, i: &Impl) {
        for f in &i.defs {
            let mut hir_f = self.lower_function_decl(f);

            let mut types = vec![i.name.get_name()];
            types.extend(i.types.iter().map(|t| t.get_name()));

            let body = self.bodies.get_mut(&hir_f.body_id).unwrap();
            body.mangle(&types);

            let type_sig = if let Some(r#trait) = self.traits.get(&i.name) {
                let type_sig = r#trait
                    .defs
                    .iter()
                    .find(|proto| *proto.name == *hir_f.name)
                    .unwrap()
                    .signature
                    .clone();

                let type_sig = type_sig.apply_forall_types(&r#trait.types, &i.types);

                type_sig
            } else {
                f.signature.clone()
            };

            hir_f.signature = type_sig.clone();

            let fn_decls = self
                .struct_methods
                .entry(hir_f.name.hir_id.clone())
                .or_insert_with(HashMap::new);

            let _hir_id = self.hir_map.next_hir_id(f.node_id);
            hir_f.hir_id = _hir_id;

            (*fn_decls).insert(type_sig.clone(), hir_f.clone());

            if self.traits.get(&i.name).is_some() {
                self.trait_methods
                    .entry(hir_f.name.name.clone())
                    .or_insert_with(HashMap::new)
                    .insert(type_sig, hir_f);
            }
        }
    }

    pub fn lower_prototype(&mut self, p: &Prototype) -> hir::Prototype {
        let id = self.hir_map.next_hir_id(p.node_id);
        let ident = self.lower_identifier(&p.name);

        hir::Prototype {
            name: ident,
            signature: p.signature.clone(),
            hir_id: id,
        }
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) -> hir::FunctionDecl {
        let body_id = self.hir_map.next_body_id();
        let id = self.hir_map.next_hir_id(f.node_id);
        let ident = self.lower_identifier(&f.name);

        let body = self.lower_fn_body(&f.body, ident.clone(), body_id.clone(), id.clone());

        self.bodies.insert(body_id.clone(), body);

        hir::FunctionDecl {
            name: ident,
            mangled_name: None,
            arguments: f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(arg))
                .collect(),
            body_id,
            signature: f.signature.clone(),
            hir_id: id,
        }
    }

    pub fn lower_argument_decl(&mut self, identifier: &Identifier) -> hir::ArgumentDecl {
        let id = self.hir_map.next_hir_id(identifier.node_id);

        hir::ArgumentDecl {
            name: hir::Identifier {
                hir_id: id,
                name: identifier.name.clone(),
            },
        }
    }

    pub fn lower_fn_body(
        &mut self,
        fn_body: &Body,
        name: hir::Identifier,
        body_id: FnBodyId,
        fn_id: HirId,
    ) -> hir::FnBody {
        let body = self.lower_body(&fn_body);

        hir::FnBody {
            id: body_id,
            fn_id,
            name,
            mangled_name: None,
            body,
        }
    }

    pub fn lower_body(&mut self, body: &Body) -> hir::Body {
        hir::Body {
            stmts: body
                .stmts
                .iter()
                .map(|stmt| self.lower_statement(stmt))
                .collect(),
        }
    }

    pub fn lower_statement(&mut self, stmt: &Statement) -> hir::Statement {
        hir::Statement {
            kind: match &*stmt {
                Statement::Expression(e) => {
                    Box::new(hir::StatementKind::Expression(self.lower_expression(e)))
                }
                Statement::If(e) => Box::new(hir::StatementKind::If(self.lower_if_chain(e))),
                Statement::Assign(a) => Box::new(hir::StatementKind::Assign(self.lower_assign(a))),
                Statement::For(f) => Box::new(hir::StatementKind::For(self.lower_for(f))),
            },
        }
    }

    pub fn lower_for(&mut self, for_loop: &For) -> hir::For {
        match for_loop {
            For::In(i) => hir::For::In(self.lower_for_in(i)),
            For::While(w) => hir::For::While(self.lower_while(w)),
        }
    }

    pub fn lower_for_in(&mut self, for_in: &ForIn) -> hir::ForIn {
        hir::ForIn {
            value: self.lower_identifier(&for_in.value),
            expr: self.lower_expression(&for_in.expr),
            body: self.lower_body(&for_in.body),
        }
    }

    pub fn lower_while(&mut self, while_loop: &While) -> hir::While {
        hir::While {
            predicat: self.lower_expression(&while_loop.predicat),
            body: self.lower_body(&while_loop.body),
        }
    }

    pub fn lower_assign_left_side(&mut self, assign_left: &AssignLeftSide) -> hir::AssignLeftSide {
        match assign_left {
            AssignLeftSide::Identifier(id) => {
                hir::AssignLeftSide::Identifier(self.lower_identifier(id.as_identifier().unwrap()))
            }
            AssignLeftSide::Indice(indice) => {
                let expr_hir = self.lower_expression(indice);
                let res = match &*expr_hir.kind {
                    hir::ExpressionKind::Dot(dot) => hir::AssignLeftSide::Dot(dot.clone()),
                    hir::ExpressionKind::Indice(indice) => {
                        hir::AssignLeftSide::Indice(indice.clone())
                    }
                    _ => unimplemented!(
                        "Assign left hand side can be Identifiers, Indices or dot notation, found {:#?}", expr_hir
                    ),
                };

                res
            }
            AssignLeftSide::Dot(dot) => {
                let expr_hir = self.lower_expression(dot);
                let res = match &*expr_hir.kind {
                    hir::ExpressionKind::Dot(dot) => hir::AssignLeftSide::Dot(dot.clone()),
                    hir::ExpressionKind::Indice(indice) => hir::AssignLeftSide::Indice(indice.clone()),
                    _ => unimplemented!(
                        "Assign left hand side can be Identifiers, Indices or dot notation, found {:#?}", expr_hir
                    ),
                };

                res
            }
        }
    }

    pub fn lower_assign(&mut self, assign: &Assign) -> hir::Assign {
        hir::Assign {
            name: self.lower_assign_left_side(&assign.name),
            value: self.lower_expression(&assign.value),
            is_let: assign.is_let,
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> hir::Expression {
        match &expr {
            Expression::UnaryExpr(unary) => self.lower_unary(unary),
            Expression::StructCtor(s) => self.lower_struct_ctor(s),
            Expression::NativeOperation(op, left, right) => {
                self.lower_native_operation(op, left, right)
            }
            Expression::BinopExpr(_unary, _op, _expr22) => {
                let mut infix = InfixDesugar::new(self.operators_list.clone());

                self.lower_expression(&infix.desugar(expr))
            }
            Expression::Return(expr) => hir::Expression {
                kind: Box::new(hir::ExpressionKind::Return(self.lower_expression(&*expr))),
            },
        }
    }

    pub fn lower_struct_ctor(&mut self, s: &StructCtor) -> hir::Expression {
        hir::Expression::new_struct_ctor(hir::StructCtor {
            name: self.lower_identifier(&s.name),
            defs: s
                .defs
                .iter()
                .map(|(k, expr)| (self.lower_identifier(k), self.lower_expression(expr)))
                .collect(),
        })
    }

    pub fn lower_if_chain(&mut self, r#if: &If) -> hir::IfChain {
        let flat_if = r#if.get_flat();

        let flat_hir_if = flat_if.iter().map(|(node_id, predicat, body)| {
            let body = self.lower_body(body);

            hir::If {
                hir_id: self.hir_map.next_hir_id(*node_id),
                predicat: self.lower_expression(predicat),
                body,
            }
        });

        hir::IfChain {
            ifs: flat_hir_if.collect(),
            else_body: r#if.last_else().map(|body| self.lower_body(body)),
        }
    }

    pub fn lower_unary(&mut self, unary: &UnaryExpr) -> hir::Expression {
        match &unary {
            UnaryExpr::PrimaryExpr(primary) => self.lower_primary(primary),
            _ => unimplemented!(),
        }
    }

    pub fn lower_primary(&mut self, primary: &PrimaryExpr) -> hir::Expression {
        if primary.secondaries.is_none() {
            return self.lower_operand(&primary.op);
        }

        let mut expr = self.lower_operand(&primary.op);

        for secondary in &primary.secondaries.clone().unwrap() {
            let mut expr2 = self.lower_secondary(expr.clone(), secondary, primary.node_id);

            // If the caller is a dot notation, we need to inject self as first argument
            if let hir::ExpressionKind::Dot(dot) = &mut *expr.kind {
                if let hir::ExpressionKind::FunctionCall(fc) = &mut *expr2.kind {
                    fc.args.insert(0, dot.op.clone());
                }
            }

            expr = expr2;
        }

        expr
    }

    pub fn lower_operand(&mut self, operand: &Operand) -> hir::Expression {
        match &operand {
            Operand::Literal(l) => hir::Expression::new_literal(self.lower_literal(l)),
            Operand::Identifier(i) => {
                hir::Expression::new_identifier_path(self.lower_identifier_path(i))
            }
            Operand::Expression(e) => self.lower_expression(&**e),
        }
    }

    pub fn lower_secondary(
        &mut self,
        op: hir::Expression,
        secondary: &SecondaryExpr,
        node_id: NodeId,
    ) -> hir::Expression {
        match secondary {
            SecondaryExpr::Arguments(args) => {
                hir::Expression::new_function_call(hir::FunctionCall {
                    hir_id: self.hir_map.next_hir_id(node_id.clone()),
                    op,
                    args: args.iter().map(|arg| self.lower_unary(&arg.arg)).collect(),
                })
            }
            SecondaryExpr::Indice(expr) => hir::Expression::new_indice(hir::Indice {
                hir_id: self.hir_map.next_hir_id(node_id.clone()),
                op,
                value: self.lower_expression(expr),
            }),
            SecondaryExpr::Dot(expr) => hir::Expression::new_dot(hir::Dot {
                hir_id: self.hir_map.next_hir_id(node_id.clone()),
                op,
                value: self.lower_identifier(expr),
            }),
        }
    }

    pub fn lower_literal(&mut self, lit: &Literal) -> hir::Literal {
        let hir_id = self.hir_map.next_hir_id(lit.node_id);

        hir::Literal {
            hir_id,
            kind: match &lit.kind {
                LiteralKind::Number(n) => hir::LiteralKind::Number(*n),
                LiteralKind::Float(f) => hir::LiteralKind::Float(*f),
                LiteralKind::String(s) => hir::LiteralKind::String(s.clone()),
                LiteralKind::Bool(b) => hir::LiteralKind::Bool(*b),
                LiteralKind::Array(arr) => hir::LiteralKind::Array(self.lower_array(arr)),
            },
        }
    }

    pub fn lower_array(&mut self, arr: &Array) -> hir::Array {
        hir::Array {
            values: arr
                .values
                .iter()
                .map(|expr| self.lower_expression(expr))
                .collect(),
        }
    }

    pub fn lower_identifier_path(&mut self, path: &IdentifierPath) -> hir::IdentifierPath {
        hir::IdentifierPath {
            path: path.path.iter().map(|i| self.lower_identifier(i)).collect(),
        }
    }

    pub fn lower_identifier(&mut self, id: &Identifier) -> hir::Identifier {
        let hir_id = self.hir_map.next_hir_id(id.node_id);

        hir::Identifier {
            hir_id,
            name: id.name.clone(),
        }
    }

    pub fn lower_native_operation(
        &mut self,
        op: &NativeOperator,
        left: &Identifier,
        right: &Identifier,
    ) -> hir::Expression {
        hir::Expression::new_native_operation(
            self.lower_native_operator(op),
            self.lower_identifier(left),
            self.lower_identifier(right),
        )
    }

    pub fn lower_native_operator(&mut self, op: &NativeOperator) -> hir::NativeOperator {
        let hir_id = self.hir_map.next_hir_id(op.node_id);

        let kind = match op.kind {
            NativeOperatorKind::IAdd => hir::NativeOperatorKind::IAdd,
            NativeOperatorKind::ISub => hir::NativeOperatorKind::ISub,
            NativeOperatorKind::IMul => hir::NativeOperatorKind::IMul,
            NativeOperatorKind::IDiv => hir::NativeOperatorKind::IDiv,
            NativeOperatorKind::FAdd => hir::NativeOperatorKind::FAdd,
            NativeOperatorKind::FSub => hir::NativeOperatorKind::FSub,
            NativeOperatorKind::FMul => hir::NativeOperatorKind::FMul,
            NativeOperatorKind::FDiv => hir::NativeOperatorKind::FDiv,
            NativeOperatorKind::IEq => hir::NativeOperatorKind::IEq,
            NativeOperatorKind::Igt => hir::NativeOperatorKind::Igt,
            NativeOperatorKind::Ige => hir::NativeOperatorKind::Ige,
            NativeOperatorKind::Ilt => hir::NativeOperatorKind::Ilt,
            NativeOperatorKind::Ile => hir::NativeOperatorKind::Ile,
            NativeOperatorKind::FEq => hir::NativeOperatorKind::FEq,
            NativeOperatorKind::Fgt => hir::NativeOperatorKind::Fgt,
            NativeOperatorKind::Fge => hir::NativeOperatorKind::Fge,
            NativeOperatorKind::Flt => hir::NativeOperatorKind::Flt,
            NativeOperatorKind::Fle => hir::NativeOperatorKind::Fle,
            NativeOperatorKind::BEq => hir::NativeOperatorKind::BEq,
            NativeOperatorKind::Len => hir::NativeOperatorKind::Len,
        };

        hir::NativeOperator { hir_id, kind }
    }
}
