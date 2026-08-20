use std::path::Path;

use crate::ast::Program;
use crate::hir::{
    AcceptedHirBlock, AcceptedHirExpr, AcceptedHirFunction, HirCallTarget, HirExprKindFor,
    HirMethodLocation, HirSelectedMethodTarget, HirStmtFor, HirVarTarget,
};
use crate::ids::{DefId, HirLocalId};
use crate::infer::ResolvedHirProgram;
use crate::lexer::Span;
use crate::source_map::SourceSymbol;
use crate::types::Type;

#[derive(Debug)]
pub struct Analysis {
    pub ast: Program,
    pub hir: ResolvedHirProgram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    Function,
    Variable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    pub span: Span,
    pub contents: String,
    pub kind: HoverKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureInfo {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: usize,
}

#[derive(Debug, Clone)]
struct FunctionSignature {
    label: String,
    parameters: Vec<String>,
}

impl Analysis {
    pub fn hover(&self, path: &Path, offset: usize) -> Option<HoverInfo> {
        let (symbol, span) = self.hir.source_map.symbol_at(path, offset)?;
        match symbol {
            SourceSymbol::Definition(id) => {
                let signature = self.function_signature(id)?;
                Some(HoverInfo {
                    span,
                    contents: signature.label,
                    kind: HoverKind::Function,
                })
            }
            SourceSymbol::Local { owner, local } => {
                let (name, ty) = self.local_binding(owner, local)?;
                Some(HoverInfo {
                    span,
                    contents: format!("{}: {}", name, self.hir.display_type(&ty)),
                    kind: HoverKind::Variable,
                })
            }
            _ => None,
        }
    }

    pub fn signature(&self, path: &Path, offset: usize) -> Option<SignatureInfo> {
        let mut best = None;
        self.for_each_function(|function| {
            find_signature_in_block(self, &function.body, path, offset, &mut best)
        });
        best.map(|(_, signature)| signature)
    }

    fn function_signature(&self, id: DefId) -> Option<FunctionSignature> {
        if let Some((name, function)) = self.hir.program.function_by_id(id) {
            return Some(self.format_function_signature(name, function));
        }

        if let Some(location) = self.hir.program.indexes.methods_by_id.get(&id) {
            return match location {
                HirMethodLocation::TraitDefault {
                    trait_id,
                    method_name,
                    ..
                } => self
                    .hir
                    .program
                    .traits
                    .get(trait_id)
                    .and_then(|trait_def| trait_def.methods.get(method_name))
                    .map(|function| self.format_function_signature(method_name, function)),
                HirMethodLocation::ImplMethod {
                    impl_id,
                    method_name,
                    ..
                } => self
                    .hir
                    .program
                    .impls
                    .get(impl_id)
                    .and_then(|imp| imp.methods.get(method_name))
                    .map(|function| self.format_function_signature(method_name, function)),
            };
        }

        for trait_def in self.hir.program.traits.values() {
            if let Some(signature) = trait_def.signatures.values().find(|sig| sig.id == id) {
                let parameters = signature
                    .params
                    .iter()
                    .enumerate()
                    .map(|(index, ty)| format!("arg{}: {}", index + 1, self.hir.display_type(ty)))
                    .collect::<Vec<_>>();
                return Some(FunctionSignature {
                    label: format_signature(
                        &signature.name,
                        &parameters,
                        &self.hir.display_type(&signature.ret),
                    ),
                    parameters,
                });
            }
        }

        self.hir.program.externs.get(&id).map(|external| {
            let parameters = external
                .params
                .iter()
                .enumerate()
                .map(|(index, ty)| format!("arg{}: {}", index + 1, self.hir.display_type(ty)))
                .collect::<Vec<_>>();
            FunctionSignature {
                label: format_signature(
                    &external.name,
                    &parameters,
                    &self.hir.display_type(&external.ret),
                ),
                parameters,
            }
        })
    }

    fn format_function_signature(
        &self,
        name: &str,
        function: &AcceptedHirFunction,
    ) -> FunctionSignature {
        let parameters = function
            .params
            .iter()
            .map(|param| format!("{}: {}", param.name, self.hir.display_type(&param.ty)))
            .collect::<Vec<_>>();
        FunctionSignature {
            label: format_signature(
                name,
                &parameters,
                &self.hir.display_type(&function.ret_type),
            ),
            parameters,
        }
    }

    fn local_binding(&self, owner: DefId, local: HirLocalId) -> Option<(String, Type)> {
        let function = self.function(owner)?;
        if let Some(param) = function.params.iter().find(|param| param.local_id == local) {
            return Some((param.name.clone(), param.ty.clone()));
        }
        find_local_in_block(&function.body, local)
    }

    fn function(&self, id: DefId) -> Option<&AcceptedHirFunction> {
        if let Some((_, function)) = self.hir.program.function_by_id(id) {
            return Some(function);
        }
        match self.hir.program.indexes.methods_by_id.get(&id)? {
            HirMethodLocation::TraitDefault {
                trait_id,
                method_name,
                ..
            } => self
                .hir
                .program
                .traits
                .get(trait_id)?
                .methods
                .get(method_name),
            HirMethodLocation::ImplMethod {
                impl_id,
                method_name,
                ..
            } => self
                .hir
                .program
                .impls
                .get(impl_id)?
                .methods
                .get(method_name),
        }
    }

    fn for_each_function(&self, mut visit: impl FnMut(&AcceptedHirFunction)) {
        self.hir.program.functions.values().for_each(&mut visit);
        for trait_def in self.hir.program.traits.values() {
            trait_def.methods.values().for_each(&mut visit);
        }
        for imp in self.hir.program.impls.values() {
            imp.methods.values().for_each(&mut visit);
        }
    }
}

fn format_signature(name: &str, parameters: &[String], ret: &str) -> String {
    if parameters.is_empty() {
        format!("{} = -> {}", name, ret)
    } else {
        format!("{} = {} -> {}", name, parameters.join(", "), ret)
    }
}

fn find_local_in_block(block: &AcceptedHirBlock, local: HirLocalId) -> Option<(String, Type)> {
    for statement in &block.stmts {
        match statement {
            HirStmtFor::Let {
                name,
                local_id,
                ty,
                value,
                ..
            } => {
                if *local_id == local {
                    return Some((name.clone(), ty.clone()));
                }
                if let Some(binding) = find_local_in_expr(value, local) {
                    return Some(binding);
                }
            }
            HirStmtFor::Expr(expr)
            | HirStmtFor::Return(Some(expr))
            | HirStmtFor::Break(Some(expr)) => {
                if let Some(binding) = find_local_in_expr(expr, local) {
                    return Some(binding);
                }
            }
            HirStmtFor::Return(None) | HirStmtFor::Break(None) | HirStmtFor::Continue => {}
        }
    }
    None
}

fn find_local_in_expr(expr: &AcceptedHirExpr, local: HirLocalId) -> Option<(String, Type)> {
    if let HirExprKindFor::ResolvedVar(reference) = &expr.kind {
        if reference.target == HirVarTarget::Local(local) {
            return Some((reference.name.clone(), expr.ty.clone()));
        }
    }

    let mut found = None;
    visit_expr_children(expr, &mut |child| {
        if found.is_some() {
            return;
        }
        found = match child {
            HirChild::Expr(child) => find_local_in_expr(child, local),
            HirChild::Block(block) => find_local_in_block(block, local),
        };
    });

    if found.is_none() {
        if let HirExprKindFor::Lambda { params, .. } = &expr.kind {
            found = params
                .iter()
                .find(|param| param.local_id == local)
                .map(|param| (param.name.clone(), param.ty.clone()));
        }
    }
    found
}

fn find_signature_in_block(
    analysis: &Analysis,
    block: &AcceptedHirBlock,
    path: &Path,
    offset: usize,
    best: &mut Option<(usize, SignatureInfo)>,
) {
    for statement in &block.stmts {
        match statement {
            HirStmtFor::Let { value, .. }
            | HirStmtFor::Expr(value)
            | HirStmtFor::Return(Some(value))
            | HirStmtFor::Break(Some(value)) => {
                find_signature_in_expr(analysis, value, path, offset, best)
            }
            HirStmtFor::Return(None) | HirStmtFor::Break(None) | HirStmtFor::Continue => {}
        }
    }
}

fn find_signature_in_expr(
    analysis: &Analysis,
    expr: &AcceptedHirExpr,
    path: &Path,
    offset: usize,
    best: &mut Option<(usize, SignatureInfo)>,
) {
    let call = match &expr.kind {
        HirExprKindFor::Call(callee, args, target) => {
            let definition = target
                .as_ref()
                .and_then(call_target_definition)
                .or_else(|| {
                    analysis
                        .hir
                        .source_map
                        .symbol_at(path, callee.span.start)
                        .and_then(|(symbol, _)| match symbol {
                            SourceSymbol::Definition(id) => Some(id),
                            _ => None,
                        })
                });
            definition.map(|id| (id, callee.span.start, args.as_slice()))
        }
        HirExprKindFor::MethodCall(receiver, _, args, _, target) => Some((
            method_target_definition(target),
            receiver.span.start,
            args.as_slice(),
        )),
        _ => None,
    };

    if let Some((definition, call_start, args)) = call {
        let call_end = args
            .last()
            .map(|argument| argument.span.end)
            .unwrap_or(expr.span.end)
            .max(expr.span.end);
        if expr.span.file_path == path && call_start <= offset && offset <= call_end {
            if let Some(signature) = analysis.function_signature(definition) {
                let active_parameter = args
                    .iter()
                    .position(|argument| offset <= argument.span.end)
                    .unwrap_or(args.len())
                    .min(signature.parameters.len().saturating_sub(1));
                let width = call_end.saturating_sub(call_start);
                if best
                    .as_ref()
                    .is_none_or(|(best_width, _)| width < *best_width)
                {
                    *best = Some((
                        width,
                        SignatureInfo {
                            label: signature.label,
                            parameters: signature.parameters,
                            active_parameter,
                        },
                    ));
                }
            }
        }
    }

    visit_expr_children(expr, &mut |child| match child {
        HirChild::Expr(child) => find_signature_in_expr(analysis, child, path, offset, best),
        HirChild::Block(block) => find_signature_in_block(analysis, block, path, offset, best),
    });
}

fn call_target_definition(target: &HirCallTarget) -> Option<DefId> {
    match target {
        HirCallTarget::Function(id) | HirCallTarget::Extern(id) => Some(*id),
        HirCallTarget::StaticMethod(target) => target.method.method_id(),
        HirCallTarget::Instance(_) | HirCallTarget::Local(_) | HirCallTarget::Intrinsic(_) => None,
    }
}

fn method_target_definition(target: &crate::hir::HirMethodCallTarget) -> DefId {
    match target.target {
        HirSelectedMethodTarget::ImplMethod { method_id, .. } => method_id,
        HirSelectedMethodTarget::TraitMethod { member_id, .. } => member_id,
    }
}

enum HirChild<'a> {
    Expr(&'a AcceptedHirExpr),
    Block(&'a AcceptedHirBlock),
}

fn visit_expr_children(expr: &AcceptedHirExpr, visit: &mut impl FnMut(HirChild<'_>)) {
    match &expr.kind {
        HirExprKindFor::ArrayLiteral(values) | HirExprKindFor::TupleLiteral(values) => {
            values.iter().for_each(|expr| visit(HirChild::Expr(expr)))
        }
        HirExprKindFor::ArrayRepeat(value, _)
        | HirExprKindFor::UnaryOp(_, value)
        | HirExprKindFor::FieldAccess(value, _, _)
        | HirExprKindFor::TupleIndex(value, _)
        | HirExprKindFor::Ref(_, value)
        | HirExprKindFor::Deref(value)
        | HirExprKindFor::Cast(value, _) => visit(HirChild::Expr(value)),
        HirExprKindFor::BinOp(_, left, right)
        | HirExprKindFor::Assign(left, right)
        | HirExprKindFor::Range(left, right) => {
            visit(HirChild::Expr(left));
            visit(HirChild::Expr(right));
        }
        HirExprKindFor::Call(callee, args, _) => {
            visit(HirChild::Expr(callee));
            args.iter().for_each(|expr| visit(HirChild::Expr(expr)));
        }
        HirExprKindFor::MethodCall(receiver, _, args, _, _) => {
            visit(HirChild::Expr(receiver));
            args.iter().for_each(|expr| visit(HirChild::Expr(expr)));
        }
        HirExprKindFor::Try { expr, .. } => visit(HirChild::Expr(expr)),
        HirExprKindFor::StructLiteral(_, _, fields) => fields
            .iter()
            .for_each(|field| visit(HirChild::Expr(&field.value))),
        HirExprKindFor::EnumVariant(_, _, args, _) | HirExprKindFor::Intrinsic { args, .. } => {
            args.iter().for_each(|expr| visit(HirChild::Expr(expr)))
        }
        HirExprKindFor::If {
            condition,
            then_branch,
            else_branch,
        } => {
            visit(HirChild::Expr(condition));
            visit(HirChild::Block(then_branch));
            if let Some(else_branch) = else_branch {
                visit(HirChild::Block(else_branch));
            }
        }
        HirExprKindFor::Match { scrutinee, arms } => {
            visit(HirChild::Expr(scrutinee));
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visit(HirChild::Expr(guard));
                }
                visit(HirChild::Block(&arm.body));
            }
        }
        HirExprKindFor::While { condition, body } => {
            visit(HirChild::Expr(condition));
            visit(HirChild::Block(body));
        }
        HirExprKindFor::For { iter, body, .. } => {
            visit(HirChild::Expr(iter));
            visit(HirChild::Block(body));
        }
        HirExprKindFor::Loop(body)
        | HirExprKindFor::Block(body)
        | HirExprKindFor::UnsafeBlock(body) => visit(HirChild::Block(body)),
        HirExprKindFor::Lambda { body, .. } => visit(HirChild::Block(body)),
        HirExprKindFor::IntLiteral(_)
        | HirExprKindFor::FloatLiteral(_)
        | HirExprKindFor::BoolLiteral(_)
        | HirExprKindFor::StringLiteral(_)
        | HirExprKindFor::CharLiteral(_)
        | HirExprKindFor::Unit
        | HirExprKindFor::Var(_)
        | HirExprKindFor::ResolvedVar(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{Config, SourceProvider};

    fn analyze(source: &str) -> super::Analysis {
        let path = PathBuf::from("/virtual/main.rk");
        crate::analyze(&Config {
            entry_file: path.clone(),
            source_providers: vec![SourceProvider::Virtual {
                path,
                text: source.to_string(),
            }],
            no_prelude: true,
            no_std: true,
            ..Config::default()
        })
        .unwrap()
    }

    #[test]
    fn analysis_reports_inferred_local_types_and_function_signatures() {
        let source = "id = value -> value\n\nmain = ->\n    answer = id 42\n    answer\n";
        let analysis = analyze(source);
        let path = PathBuf::from("/virtual/main.rk");

        let local_offset = source.rfind("answer").unwrap();
        let local = analysis.hover(&path, local_offset).unwrap();
        assert_eq!(local.contents, "answer: I64");

        let function_offset = source.find("id 42").unwrap();
        let function = analysis.hover(&path, function_offset).unwrap();
        assert!(function.contents.starts_with("id = value: "));

        let argument_offset = source.find("42").unwrap() + 1;
        let signature = analysis.signature(&path, argument_offset).unwrap();
        assert!(signature.label.starts_with("id = value: "));
        assert_eq!(signature.parameters.len(), 1);
        assert_eq!(signature.active_parameter, 0);
    }

    #[test]
    fn analysis_errors_keep_virtual_source_for_lsp_conversion() {
        let path = PathBuf::from("/virtual/error.rk");
        let source = "main = ->\n    `\n";
        let diagnostics = crate::analyze(&Config {
            entry_file: path.clone(),
            source_providers: vec![SourceProvider::Virtual {
                path,
                text: source.to_string(),
            }],
            no_prelude: true,
            no_std: true,
            ..Config::default()
        })
        .unwrap_err();

        assert!(diagnostics.0.iter().any(|diagnostic| diagnostic
            .source
            .as_ref()
            .is_some_and(|source| { source.text == "main = ->\n    `\n" })));
    }
}
