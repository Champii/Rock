use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
};

use crate::{
    ast::*,
    hir::{self, Arena, FnBodyId, HirId},
    NodeId,
};

use super::{hir_map::HirMap, return_placement::ReturnInserter, InfixDesugar};

pub struct AstLoweringContext {
    hir_map: HirMap,
    unused: Vec<NodeId>,
    top_levels: BTreeMap<HirId, hir::TopLevel>,
    modules: BTreeMap<HirId, hir::Mod>,
    bodies: BTreeMap<FnBodyId, hir::FnBody>,
    operators_list: HashMap<String, u8>,
    traits: HashMap<Type, hir::Trait>,
    trait_methods: HashMap<String, HashMap<TypeSignature, hir::FunctionDecl>>,
}

impl AstLoweringContext {
    pub fn new(operators_list: HashMap<String, u8>, unused: Vec<NodeId>) -> Self {
        Self {
            unused,
            hir_map: HirMap::new(),
            top_levels: BTreeMap::new(),
            modules: BTreeMap::new(),
            bodies: BTreeMap::new(),
            traits: HashMap::new(),
            trait_methods: HashMap::new(),
            operators_list,
        }
    }

    pub fn lower_root(&mut self, root: &Root) -> hir::Root {
        self.lower_mod(&root.r#mod);

        let mut hir = hir::Root {
            arena: Arena::new(),
            hir_map: self.hir_map.clone(),
            resolutions: root.resolutions.lower_resolution_map(&self.hir_map),
            node_types: BTreeMap::new(),
            types: BTreeMap::new(),
            top_levels: self.top_levels.clone(),
            modules: self.modules.clone(),
            bodies: self.bodies.clone(),
            traits: self.traits.clone(),
            trait_methods: self.trait_methods.clone(),
            trait_call_to_mangle: HashMap::new(),
            unused: vec![],
            spans: root.spans.clone(),
        };

        hir.arena = hir::collect_arena(&hir);

        hir
    }

    pub fn lower_mod(&mut self, r#mod: &Mod) -> hir::HirId {
        let id = self.hir_map.next_hir_id(r#mod.identity.clone());

        let r#mod = hir::Mod {
            hir_id: id.clone(),
            top_levels: r#mod
                .top_levels
                .iter()
                .map(|t| self.lower_top_level(&t))
                .collect(),
        };

        self.modules.insert(id.clone(), r#mod);

        id
    }

    pub fn lower_top_level(&mut self, top_level: &TopLevel) -> hir::HirId {
        let id = self.hir_map.next_hir_id(top_level.identity.clone());

        match &top_level.kind {
            TopLevelKind::Prototype(p) => {
                let mut top_level = hir::TopLevel {
                    kind: hir::TopLevelKind::Prototype(self.lower_prototype(&p)),
                    hir_id: id,
                };

                let child_id = top_level.get_child_hir();
                top_level.hir_id = child_id.clone();

                self.top_levels.insert(child_id.clone(), top_level);
                child_id
            }
            TopLevelKind::Use(_u) => id,
            TopLevelKind::Trait(t) => {
                self.lower_trait(t);

                id
            }
            TopLevelKind::Impl(i) => {
                self.lower_impl(&i);

                id
            }
            TopLevelKind::Infix(_, _) => id,
            TopLevelKind::Function(f) => {
                if self.unused.contains(&f.identity.node_id) {
                    id
                } else {
                    let mut top_level = hir::TopLevel {
                        kind: hir::TopLevelKind::Function(self.lower_function_decl(&f)),
                        hir_id: id,
                    };

                    let child_id = top_level.get_child_hir();
                    top_level.hir_id = child_id.clone();

                    self.top_levels.insert(child_id.clone(), top_level);
                    child_id
                }
            }
            TopLevelKind::Mod(_name, mod_) => self.lower_mod(&mod_),
        }
    }

    pub fn lower_trait(&mut self, t: &Trait) -> hir::Trait {
        let hir_t = hir::Trait {
            name: t.name.clone(),
            types: t.types.clone(),
            defs: t
                .defs
                .iter()
                .map(|proto| self.lower_prototype(&proto))
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

            hir_f.mangle(&types);

            let body = self.bodies.get_mut(&hir_f.body_id).unwrap();
            body.mangle(&types);

            let r#trait = self.traits.get(&i.name).unwrap();

            let type_sig = r#trait
                .defs
                .iter()
                .find(|proto| *proto.name == *hir_f.name)
                .unwrap()
                .signature
                .clone();

            let type_sig = type_sig.apply_types(&r#trait.types, &i.types);

            hir_f.signature = type_sig.clone();

            let fn_decls = self
                .trait_methods
                .entry(hir_f.name.name.clone())
                .or_insert(HashMap::new());

            let _hir_id = self.hir_map.next_hir_id(f.identity.clone());

            (*fn_decls).insert(type_sig, hir_f);
        }
    }

    pub fn lower_prototype(&mut self, p: &Prototype) -> hir::Prototype {
        let id = self.hir_map.next_hir_id(p.identity.clone());
        let ident = self.lower_identifier(&p.name);

        hir::Prototype {
            name: ident,
            signature: p.signature.clone(),
            hir_id: id,
        }
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) -> hir::FunctionDecl {
        let body_id = FnBodyId::next();
        let id = self.hir_map.next_hir_id(f.identity.clone());
        let ident = self.lower_identifier(&f.name);

        let body = self.lower_fn_body(&f.body, ident.clone(), body_id.clone());

        self.bodies.insert(body_id.clone(), body);

        hir::FunctionDecl {
            name: ident,
            mangled_name: None,
            arguments: f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(&arg))
                .collect(),
            body_id,
            signature: f.signature.clone(),
            hir_id: id,
        }
    }

    pub fn lower_argument_decl(&mut self, argument: &ArgumentDecl) -> hir::ArgumentDecl {
        let id = self.hir_map.next_hir_id(argument.identity.clone());

        hir::ArgumentDecl {
            name: hir::Identifier {
                hir_id: id,
                name: argument.name.clone(),
            },
        }
    }

    pub fn lower_fn_body(
        &mut self,
        fn_body: &Body,
        name: hir::Identifier,
        body_id: FnBodyId,
    ) -> hir::FnBody {
        let body = ReturnInserter { body: &fn_body }.run();

        let body = self.lower_body(&body);

        hir::FnBody {
            id: body_id,
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
            kind: match &*stmt.kind {
                StatementKind::Expression(e) => {
                    Box::new(hir::StatementKind::Expression(self.lower_expression(&e)))
                }
                StatementKind::If(e) => Box::new(hir::StatementKind::If(self.lower_if(&e))),
                StatementKind::Assign(a) => {
                    Box::new(hir::StatementKind::Assign(self.lower_assign(&a)))
                }
            },
        }
    }

    pub fn lower_assign(&mut self, assign: &Assign) -> hir::Assign {
        hir::Assign {
            name: self.lower_identifier(&assign.name),
            value: self.lower_expression(&assign.value),
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> hir::Expression {
        match &expr.kind {
            ExpressionKind::UnaryExpr(unary) => self.lower_unary(&unary),
            ExpressionKind::NativeOperation(op, left, right) => {
                self.lower_native_operation(&op, &left, &right)
            }
            ExpressionKind::BinopExpr(_unary, _op, _expr22) => {
                let mut infix = InfixDesugar::new(self.operators_list.clone());

                self.lower_expression(&infix.desugar(expr))
            }
            ExpressionKind::Return(expr) => hir::Expression {
                kind: Box::new(hir::ExpressionKind::Return(self.lower_expression(&*expr))),
            },
        }
    }

    pub fn lower_if(&mut self, r#if: &If) -> hir::If {
        hir::If {
            hir_id: self.hir_map.next_hir_id(r#if.identity.clone()),
            predicat: self.lower_expression(&r#if.predicat),
            body: self.lower_body(&r#if.body),
            else_: r#if.else_.as_ref().map(|e| Box::new(self.lower_else(&e))),
        }
    }

    pub fn lower_else(&mut self, r#else: &Else) -> hir::Else {
        match r#else {
            Else::If(e) => hir::Else::If(self.lower_if(e)),
            Else::Body(b) => hir::Else::Body(self.lower_body(b)),
        }
    }

    pub fn lower_unary(&mut self, unary: &UnaryExpr) -> hir::Expression {
        match &unary {
            UnaryExpr::PrimaryExpr(primary) => self.lower_primary(&primary),
            _ => unimplemented!(),
        }
    }

    pub fn lower_primary(&mut self, primary: &PrimaryExpr) -> hir::Expression {
        if primary.secondaries.is_none() {
            return self.lower_operand(&primary.op);
        }

        hir::Expression::new_function_call(hir::FunctionCall {
            hir_id: self.hir_map.next_hir_id(primary.identity.clone()),
            op: self.lower_operand(&primary.op),
            args: primary
                .secondaries
                .clone()
                .unwrap()
                .iter()
                .map(|sec| self.lower_secondary(&sec))
                .flatten() // FIXME: This is bad, we mix secondaries with arguments and we flatten.
                .collect(),
        })
    }

    pub fn lower_operand(&mut self, operand: &Operand) -> hir::Expression {
        match &operand.kind {
            OperandKind::Literal(l) => hir::Expression::new_literal(self.lower_literal(&l)),
            OperandKind::Identifier(i) => {
                hir::Expression::new_identifier_path(self.lower_identifier_path(&i))
            }
            OperandKind::Expression(e) => self.lower_expression(&**e),
        }
    }

    pub fn lower_secondary(&mut self, secondary: &SecondaryExpr) -> Vec<hir::Expression> {
        match secondary {
            SecondaryExpr::Arguments(args) => {
                args.iter().map(|arg| self.lower_unary(&arg.arg)).collect()
            }
        }
    }

    pub fn lower_literal(&mut self, lit: &Literal) -> hir::Literal {
        let hir_id = self.hir_map.next_hir_id(lit.identity.clone());

        hir::Literal {
            hir_id,
            kind: match &lit.kind {
                LiteralKind::Number(n) => hir::LiteralKind::Number(*n),
                LiteralKind::Float(f) => hir::LiteralKind::Float(*f),
                LiteralKind::String(s) => hir::LiteralKind::String(s.clone()),
                LiteralKind::Bool(b) => hir::LiteralKind::Bool(*b),
            },
        }
    }

    pub fn lower_identifier_path(&mut self, path: &IdentifierPath) -> hir::IdentifierPath {
        hir::IdentifierPath {
            path: path.path.iter().map(|i| self.lower_identifier(i)).collect(),
        }
    }

    pub fn lower_identifier(&mut self, id: &Identifier) -> hir::Identifier {
        let hir_id = self.hir_map.next_hir_id(id.identity.clone());

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
        let hir_id = self.hir_map.next_hir_id(op.identity.clone());

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
            NativeOperatorKind::IGT => hir::NativeOperatorKind::IGT,
            NativeOperatorKind::IGE => hir::NativeOperatorKind::IGE,
            NativeOperatorKind::ILT => hir::NativeOperatorKind::ILT,
            NativeOperatorKind::ILE => hir::NativeOperatorKind::ILE,
            NativeOperatorKind::FEq => hir::NativeOperatorKind::FEq,
            NativeOperatorKind::FGT => hir::NativeOperatorKind::FGT,
            NativeOperatorKind::FGE => hir::NativeOperatorKind::FGE,
            NativeOperatorKind::FLT => hir::NativeOperatorKind::FLT,
            NativeOperatorKind::FLE => hir::NativeOperatorKind::FLE,
            NativeOperatorKind::BEq => hir::NativeOperatorKind::BEq,
        };

        hir::NativeOperator { hir_id, kind }
    }
}
