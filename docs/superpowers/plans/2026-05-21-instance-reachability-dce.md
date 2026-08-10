# Instance Reachability DCE Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace HIR/name-based dead-code elimination authority with a reachability pass over `MonomorphizedProgram.instances`.

**Architecture:** Add `dce::prune_unreachable_instances(&mut MonomorphizedProgram)` as the compiler DCE authority after monomorphization. The pass roots at the `main` instance, traverses `HirVarTarget::Instance` edges plus narrow direct-function compatibility edges, prunes `InstanceRecord`s before codegen, and leaves product metadata/generic bodies product-ID based.

**Tech Stack:** Rust 2021, `rock-lib`, HIR `HirExprKind`, mono `InstanceId`/`InstanceRecord`, `BTreeMap`/`BTreeSet`, product artifacts, focused Cargo unit and integration tests.

**2026-05-25 completion update:** The scoped Task 16 implementation is complete and verified. The compile pipeline uses `prune_unreachable_instances` after monomorphization, DCE follows explicit `InstanceId`, direct `Function(DefId)`, trait/impl method target, receiver field-method, and object-backed edges, and source-name/backend-symbol reachability aliases are not used for instance retention. Additional fixes landed for imported custom operators so operator callees lower to `ResolvedVar(Function(DefId))`, static impl calls and function values with resolved method targets specialize by `DefId`, specialized static impl constructors preserve function ABI instead of receiver-method ABI, product artifact validation accepts only static impl method IDs as function targets with callable-name validation, and product emission skips exact duplicate method bodies without hiding legitimate function/method ID collisions. Final verification: `cargo fmt --all && cargo fmt --all --check && git diff --check && cargo test -p rock-lib > /tmp/rock-lib-task16-final.log 2>&1`; the log shows `1251` unit tests passed with `1` ignored, `276` integration tests passed, parser integration passed, and doctests passed.

---

## Scope

This plan implements `docs/superpowers/specs/2026-05-21-instance-reachability-dce-design.md`.

In scope:
- Instance reachability over `MonomorphizedProgram.instances`.
- Pipeline integration after mono and before codegen/link-record attachment.
- Instance-edge traversal for calls, function values, nested expressions, and object-backed declaration leaves.
- Narrow compatibility mapping from direct `Function(DefId)` or name-based concrete function refs to zero-substitution instances.
- Product link records consuming the pruned instance set.

Out of scope:
- Product artifact schema changes.
- Serializing `InstanceId` into products.
- MIR codegen.
- Backend symbol redesign.
- Reconstructing targetless method dispatch in DCE.

---

## File Structure

- Modify `lib/src/dce.rs`: add instance-reachability DCE implementation, report type, helper traversal, and focused unit tests; keep legacy HIR/name helpers as legacy tests only.
- Modify `lib/src/lib.rs`: run instance DCE after mono; add tests that link-record attachment observes the pruned instance set.
- Modify `lib/tests/integration.rs`: add pipeline regressions for emitted LLVM IR and keep Task 15 regressions green.

---

### Task 1: Instance DCE Roots And Basic Pruning

**Files:**
- Modify: `lib/src/dce.rs`

- [ ] **Step 1: Write failing tests for main retention and unused instance removal**

Update the existing test imports in `#[cfg(test)] mod tests` in `lib/src/dce.rs`:

```rust
use std::collections::HashMap;

use crate::mono::{
    InstanceId, InstanceOrigin, InstanceRecord, MonomorphizedProgram,
};
```

Replace the existing `use super::{count_dead_functions, prune_dead_functions};` with:

```rust
use super::{count_dead_functions, prune_dead_functions, prune_unreachable_instances};
```

Add these helpers to the same test module:

```rust
fn instance_record(
    id: InstanceId,
    origin: InstanceOrigin,
    source_name: &str,
    backend_symbol: &str,
    body: Option<HirFunction>,
    provided_by_object: bool,
) -> InstanceRecord {
    InstanceRecord {
        id,
        origin,
        substitution: Vec::new(),
        source_name: source_name.to_string(),
        backend_symbol: backend_symbol.to_string(),
        declared: body.clone(),
        body,
        provided_by_object,
        is_specialization: false,
    }
}

fn monomorphized_with_instances(instances: Vec<InstanceRecord>) -> MonomorphizedProgram {
    let mut functions = HashMap::new();
    for record in &instances {
        if let Some(function) = record.body.as_ref().or(record.declared.as_ref()) {
            if matches!(record.origin, InstanceOrigin::Function(_)) {
                functions.insert(record.source_name.clone(), function.clone());
            }
        }
    }
    MonomorphizedProgram {
        program: HirProgram::from_parts(
            functions,
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            Vec::new(),
            Vec::new(),
        ),
        instances: instances
            .into_iter()
            .map(|record| (record.id, record))
            .collect(),
    }
}

fn unit_return_function(id: DefId, name: &str) -> HirFunction {
    let mut function = empty_function(id, name);
    function.body.stmts.push(HirStmt::Return(Some(HirExpr {
        kind: HirExprKind::Unit,
        ty: Type::Unit,
        span: Span::default(),
    })));
    function
}
```

Add these tests:

```rust
#[test]
fn prune_unreachable_instances_keeps_main_instance() {
    let main_id = DefId::new(CrateId(0), LocalDefId(10));
    let main_instance = InstanceId(0);
    let mut program = monomorphized_with_instances(vec![instance_record(
        main_instance,
        InstanceOrigin::Function(main_id),
        "main",
        "main",
        Some(unit_return_function(main_id, "main")),
        false,
    )]);

    let report = prune_unreachable_instances(&mut program);

    assert_eq!(report.retained_instances, 1);
    assert_eq!(report.removed_instances, 0);
    assert!(program.instances.contains_key(&main_instance));
}

#[test]
fn prune_unreachable_instances_removes_unreachable_function_instance() {
    let main_id = DefId::new(CrateId(0), LocalDefId(10));
    let unused_id = DefId::new(CrateId(0), LocalDefId(11));
    let main_instance = InstanceId(0);
    let unused_instance = InstanceId(1);
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(unit_return_function(main_id, "main")),
            false,
        ),
        instance_record(
            unused_instance,
            InstanceOrigin::Function(unused_id),
            "unused",
            "unused",
            Some(unit_return_function(unused_id, "unused")),
            false,
        ),
    ]);

    let report = prune_unreachable_instances(&mut program);

    assert_eq!(report.retained_instances, 1);
    assert_eq!(report.removed_instances, 1);
    assert!(program.instances.contains_key(&main_instance));
    assert!(!program.instances.contains_key(&unused_instance));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib prune_unreachable_instances_keeps_main_instance -- --exact`

Expected: FAIL because `prune_unreachable_instances` and `InstanceDceReport` do not exist.

Run: `cargo test -p rock-lib prune_unreachable_instances_removes_unreachable_function_instance -- --exact`

Expected: FAIL because `prune_unreachable_instances` and `InstanceDceReport` do not exist.

- [ ] **Step 3: Add the report and root-only pruning implementation**

In `lib/src/dce.rs`, update the module docs at the top:

```rust
//! Dead code elimination.
//!
//! The compiler DCE authority is instance reachability over monomorphized
//! callable records. The older HIR/name helpers remain for legacy coverage.
```

Add imports near the top:

```rust
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ids::{DefId, InstanceId};
use crate::mono::{InstanceOrigin, MonomorphizedProgram};
```

Replace the existing `use std::collections::{HashMap, HashSet};` with the combined import above.

Add this public report and function before the legacy `prune_dead_functions`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InstanceDceReport {
    pub removed_instances: usize,
    pub retained_instances: usize,
    pub missing_instance_edges: Vec<InstanceId>,
}

pub fn prune_unreachable_instances(program: &mut MonomorphizedProgram) -> InstanceDceReport {
    let original_len = program.instances.len();
    let root_ids = instance_roots(program);
    let reachable = root_ids.into_iter().collect::<BTreeSet<_>>();

    program.instances.retain(|id, _| reachable.contains(id));

    InstanceDceReport {
        removed_instances: original_len.saturating_sub(program.instances.len()),
        retained_instances: program.instances.len(),
        missing_instance_edges: Vec::new(),
    }
}

fn instance_roots(program: &MonomorphizedProgram) -> Vec<InstanceId> {
    let main_def = program
        .program
        .function_by_name("main")
        .map(|(_, function)| function.id);

    program
        .instances
        .iter()
        .filter_map(|(id, record)| {
            let is_main_name = record.source_name == "main" || record.backend_symbol == "main";
            let is_main_origin = main_def.is_some_and(|main_def| {
                record.origin == InstanceOrigin::Function(main_def)
            });
            (is_main_name || is_main_origin).then_some(*id)
        })
        .collect()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p rock-lib prune_unreachable_instances_keeps_main_instance -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib prune_unreachable_instances_removes_unreachable_function_instance -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/dce.rs
git commit -m "add instance dce roots"
```

---

### Task 2: Instance Edge Traversal

**Files:**
- Modify: `lib/src/dce.rs`

- [ ] **Step 1: Write failing tests for instance edges, function values, and missing edges**

Add this helper to `#[cfg(test)] mod tests` in `lib/src/dce.rs`:

```rust
fn function_returning_expr(id: DefId, name: &str, value: HirExpr) -> HirFunction {
    let mut function = empty_function(id, name);
    function.body.stmts.push(HirStmt::Return(Some(value)));
    function
}

fn instance_ref_expr(name: &str, instance_id: InstanceId) -> HirExpr {
    HirExpr {
        kind: HirExprKind::ResolvedVar(HirVarRef {
            name: name.to_string(),
            target: HirVarTarget::Instance(instance_id),
        }),
        ty: Type::Function(Vec::new(), Box::new(Type::Unit)),
        span: Span::default(),
    }
}
```

Add these tests:

```rust
#[test]
fn prune_unreachable_instances_follows_direct_instance_call_edge() {
    let main_id = DefId::new(CrateId(0), LocalDefId(20));
    let helper_id = DefId::new(CrateId(0), LocalDefId(21));
    let main_instance = InstanceId(0);
    let helper_instance = InstanceId(1);
    let main_body = function_returning_expr(
        main_id,
        "main",
        HirExpr {
            kind: HirExprKind::Call(
                Box::new(instance_ref_expr("helper", helper_instance)),
                Vec::new(),
            ),
            ty: Type::Unit,
            span: Span::default(),
        },
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        instance_record(
            helper_instance,
            InstanceOrigin::Function(helper_id),
            "helper",
            "helper",
            Some(unit_return_function(helper_id, "helper")),
            false,
        ),
    ]);

    let report = prune_unreachable_instances(&mut program);

    assert_eq!(report.retained_instances, 2);
    assert!(program.instances.contains_key(&main_instance));
    assert!(program.instances.contains_key(&helper_instance));
}

#[test]
fn prune_unreachable_instances_follows_function_value_instance_edge() {
    let main_id = DefId::new(CrateId(0), LocalDefId(30));
    let helper_id = DefId::new(CrateId(0), LocalDefId(31));
    let main_instance = InstanceId(0);
    let helper_instance = InstanceId(1);
    let main_body = function_returning_expr(
        main_id,
        "main",
        instance_ref_expr("helper", helper_instance),
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        instance_record(
            helper_instance,
            InstanceOrigin::Function(helper_id),
            "helper",
            "helper",
            Some(unit_return_function(helper_id, "helper")),
            false,
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(program.instances.contains_key(&helper_instance));
}

#[test]
fn prune_unreachable_instances_records_missing_instance_edges_without_name_fallback() {
    let main_id = DefId::new(CrateId(0), LocalDefId(40));
    let real_helper_id = DefId::new(CrateId(0), LocalDefId(41));
    let main_instance = InstanceId(0);
    let real_helper_instance = InstanceId(1);
    let missing_instance = InstanceId(99);
    let main_body = function_returning_expr(
        main_id,
        "main",
        instance_ref_expr("real_helper", missing_instance),
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        instance_record(
            real_helper_instance,
            InstanceOrigin::Function(real_helper_id),
            "real_helper",
            "real_helper",
            Some(unit_return_function(real_helper_id, "real_helper")),
            false,
        ),
    ]);

    let report = prune_unreachable_instances(&mut program);

    assert_eq!(report.missing_instance_edges, vec![missing_instance]);
    assert!(!program.instances.contains_key(&real_helper_instance));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rock-lib prune_unreachable_instances_follows_direct_instance_call_edge -- --exact`

Expected: FAIL because only root instances are retained.

Run: `cargo test -p rock-lib prune_unreachable_instances_follows_function_value_instance_edge -- --exact`

Expected: FAIL because only root instances are retained.

Run: `cargo test -p rock-lib prune_unreachable_instances_records_missing_instance_edges_without_name_fallback -- --exact`

Expected: FAIL because missing edges are not reported.

- [ ] **Step 3: Implement traversal over instance bodies**

Replace `prune_unreachable_instances` in `lib/src/dce.rs` with this worklist implementation:

```rust
pub fn prune_unreachable_instances(program: &mut MonomorphizedProgram) -> InstanceDceReport {
    let original_len = program.instances.len();
    let indexes = InstanceReachabilityIndexes::new(program);
    let mut reachable = BTreeSet::new();
    let mut worklist = instance_roots(program);
    let mut missing_instance_edges = BTreeSet::new();

    while let Some(instance_id) = worklist.pop() {
        if !reachable.insert(instance_id) {
            continue;
        }

        let Some(record) = program.instances.get(&instance_id) else {
            missing_instance_edges.insert(instance_id);
            continue;
        };
        let Some(body) = record.body.as_ref() else {
            continue;
        };

        let mut edges = Vec::new();
        collect_instance_edges_block(&body.body, &indexes, &mut edges, &mut missing_instance_edges);
        worklist.extend(edges);
    }

    program.instances.retain(|id, _| reachable.contains(id));

    InstanceDceReport {
        removed_instances: original_len.saturating_sub(program.instances.len()),
        retained_instances: program.instances.len(),
        missing_instance_edges: missing_instance_edges.into_iter().collect(),
    }
}
```

Add this private index type after `instance_roots`:

```rust
struct InstanceReachabilityIndexes {
    zero_substitution_functions: BTreeMap<DefId, InstanceId>,
    zero_substitution_names: BTreeMap<String, InstanceId>,
    known_instances: BTreeSet<InstanceId>,
}

impl InstanceReachabilityIndexes {
    fn new(program: &MonomorphizedProgram) -> Self {
        let mut zero_substitution_functions = BTreeMap::new();
        let mut zero_substitution_names = BTreeMap::new();
        let mut known_instances = BTreeSet::new();

        for (id, record) in &program.instances {
            known_instances.insert(*id);
            if record.substitution.is_empty() {
                if let InstanceOrigin::Function(def_id) = record.origin {
                    zero_substitution_functions.insert(def_id, *id);
                    zero_substitution_names.insert(record.source_name.clone(), *id);
                }
            }
        }

        Self {
            zero_substitution_functions,
            zero_substitution_names,
            known_instances,
        }
    }

    fn function_instance(&self, def_id: DefId) -> Option<InstanceId> {
        self.zero_substitution_functions.get(&def_id).copied()
    }

    fn named_instance(&self, name: &str) -> Option<InstanceId> {
        self.zero_substitution_names.get(name).copied()
    }
}
```

Add traversal helpers after the index type:

```rust
fn collect_instance_edges_block(
    block: &HirBlock,
    indexes: &InstanceReachabilityIndexes,
    out: &mut Vec<InstanceId>,
    missing: &mut BTreeSet<InstanceId>,
) {
    for stmt in &block.stmts {
        collect_instance_edges_stmt(stmt, indexes, out, missing);
    }
}

fn collect_instance_edges_stmt(
    stmt: &HirStmt,
    indexes: &InstanceReachabilityIndexes,
    out: &mut Vec<InstanceId>,
    missing: &mut BTreeSet<InstanceId>,
) {
    match stmt {
        HirStmt::Let { value, .. } | HirStmt::Expr(value) => {
            collect_instance_edges_expr(value, indexes, out, missing);
        }
        HirStmt::Return(Some(value)) | HirStmt::Break(Some(value)) => {
            collect_instance_edges_expr(value, indexes, out, missing);
        }
        HirStmt::Return(None) | HirStmt::Break(None) | HirStmt::Continue => {}
    }
}

fn collect_instance_edges_expr(
    expr: &HirExpr,
    indexes: &InstanceReachabilityIndexes,
    out: &mut Vec<InstanceId>,
    missing: &mut BTreeSet<InstanceId>,
) {
    match &expr.kind {
        HirExprKind::Var(name) => {
            if let Some(instance_id) = indexes.named_instance(name) {
                out.push(instance_id);
            }
        }
        HirExprKind::ResolvedVar(reference) => match reference.target {
            HirVarTarget::Instance(instance_id) => {
                if indexes.known_instances.contains(&instance_id) {
                    out.push(instance_id);
                } else {
                    missing.insert(instance_id);
                }
            }
            HirVarTarget::Function(def_id) => {
                if let Some(instance_id) = indexes.function_instance(def_id) {
                    out.push(instance_id);
                }
            }
            HirVarTarget::Extern(_) => {}
        },
        HirExprKind::Call(callee, args) => {
            collect_instance_edges_expr(callee, indexes, out, missing);
            for arg in args {
                collect_instance_edges_expr(arg, indexes, out, missing);
            }
        }
        HirExprKind::MethodCall(recv, _, args, _, _) => {
            collect_instance_edges_expr(recv, indexes, out, missing);
            for arg in args {
                collect_instance_edges_expr(arg, indexes, out, missing);
            }
        }
        HirExprKind::BinOp(_, lhs, rhs)
        | HirExprKind::Range(lhs, rhs)
        | HirExprKind::Assign(lhs, rhs)
        | HirExprKind::Index(lhs, rhs) => {
            collect_instance_edges_expr(lhs, indexes, out, missing);
            collect_instance_edges_expr(rhs, indexes, out, missing);
        }
        HirExprKind::UnaryOp(_, inner)
        | HirExprKind::FieldAccess(inner, _, _)
        | HirExprKind::TupleIndex(inner, _)
        | HirExprKind::Ref(_, inner)
        | HirExprKind::Deref(inner)
        | HirExprKind::Cast(inner, _) => {
            collect_instance_edges_expr(inner, indexes, out, missing);
        }
        HirExprKind::StructLiteral(_, _, fields) => {
            for field in fields {
                collect_instance_edges_expr(&field.value, indexes, out, missing);
            }
        }
        HirExprKind::EnumVariant(_, _, values, _)
        | HirExprKind::ArrayLiteral(values)
        | HirExprKind::TupleLiteral(values) => {
            for value in values {
                collect_instance_edges_expr(value, indexes, out, missing);
            }
        }
        HirExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_instance_edges_expr(condition, indexes, out, missing);
            collect_instance_edges_block(then_branch, indexes, out, missing);
            if let Some(else_branch) = else_branch {
                collect_instance_edges_block(else_branch, indexes, out, missing);
            }
        }
        HirExprKind::Match { scrutinee, arms } => {
            collect_instance_edges_expr(scrutinee, indexes, out, missing);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_instance_edges_expr(guard, indexes, out, missing);
                }
                collect_instance_edges_block(&arm.body, indexes, out, missing);
            }
        }
        HirExprKind::While { condition, body } => {
            collect_instance_edges_expr(condition, indexes, out, missing);
            collect_instance_edges_block(body, indexes, out, missing);
        }
        HirExprKind::For { iter, body, .. } => {
            collect_instance_edges_expr(iter, indexes, out, missing);
            collect_instance_edges_block(body, indexes, out, missing);
        }
        HirExprKind::Loop(body) | HirExprKind::Block(body) => {
            collect_instance_edges_block(body, indexes, out, missing);
        }
        HirExprKind::Lambda { body, .. } => {
            collect_instance_edges_block(body, indexes, out, missing);
        }
        HirExprKind::Intrinsic { args, .. } => {
            for arg in args {
                collect_instance_edges_expr(arg, indexes, out, missing);
            }
        }
        HirExprKind::IntLiteral(_)
        | HirExprKind::FloatLiteral(_)
        | HirExprKind::BoolLiteral(_)
        | HirExprKind::StringLiteral(_)
        | HirExprKind::CharLiteral(_)
        | HirExprKind::Unit => {}
    }
}
```

- [ ] **Step 4: Run focused traversal tests**

Run: `cargo test -p rock-lib prune_unreachable_instances_follows_direct_instance_call_edge -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib prune_unreachable_instances_follows_function_value_instance_edge -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib prune_unreachable_instances_records_missing_instance_edges_without_name_fallback -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/dce.rs
git commit -m "trace instance dce edges"
```

---

### Task 3: Object-Backed And Generic Specialization Reachability

**Files:**
- Modify: `lib/src/dce.rs`

- [ ] **Step 1: Write failing tests for object-backed and specialized instances**

Add this helper to `#[cfg(test)] mod tests` in `lib/src/dce.rs`:

```rust
fn specialized_instance_record(
    id: InstanceId,
    origin: InstanceOrigin,
    source_name: &str,
    backend_symbol: &str,
    body: Option<HirFunction>,
) -> InstanceRecord {
    InstanceRecord {
        id,
        origin,
        substitution: vec![Type::I64],
        source_name: source_name.to_string(),
        backend_symbol: backend_symbol.to_string(),
        declared: body.clone(),
        body,
        provided_by_object: false,
        is_specialization: true,
    }
}
```

Add these tests:

```rust
#[test]
fn prune_unreachable_instances_keeps_reachable_generic_specialization() {
    let main_id = DefId::new(CrateId(0), LocalDefId(50));
    let identity_id = DefId::new(CrateId(0), LocalDefId(51));
    let main_instance = InstanceId(0);
    let specialization = InstanceId(1);
    let main_body = function_returning_expr(
        main_id,
        "main",
        HirExpr {
            kind: HirExprKind::Call(
                Box::new(instance_ref_expr("identity", specialization)),
                vec![HirExpr {
                    kind: HirExprKind::IntLiteral(1),
                    ty: Type::I64,
                    span: Span::default(),
                }],
            ),
            ty: Type::I64,
            span: Span::default(),
        },
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        specialized_instance_record(
            specialization,
            InstanceOrigin::Function(identity_id),
            "identity",
            "identity_mono_0",
            Some(unit_return_function(identity_id, "identity")),
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(program.instances.contains_key(&specialization));
}

#[test]
fn prune_unreachable_instances_removes_unused_generic_specialization() {
    let main_id = DefId::new(CrateId(0), LocalDefId(60));
    let identity_id = DefId::new(CrateId(0), LocalDefId(61));
    let main_instance = InstanceId(0);
    let specialization = InstanceId(1);
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(unit_return_function(main_id, "main")),
            false,
        ),
        specialized_instance_record(
            specialization,
            InstanceOrigin::Function(identity_id),
            "identity",
            "identity_mono_0",
            Some(unit_return_function(identity_id, "identity")),
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(!program.instances.contains_key(&specialization));
}

#[test]
fn prune_unreachable_instances_keeps_reachable_object_backed_instance() {
    let main_id = DefId::new(CrateId(0), LocalDefId(70));
    let dep_id = DefId::new(CrateId(1), LocalDefId(71));
    let main_instance = InstanceId(0);
    let object_instance = InstanceId(1);
    let main_body = function_returning_expr(
        main_id,
        "main",
        HirExpr {
            kind: HirExprKind::Call(
                Box::new(instance_ref_expr("dep::print", object_instance)),
                Vec::new(),
            ),
            ty: Type::Unit,
            span: Span::default(),
        },
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        instance_record(
            object_instance,
            InstanceOrigin::Function(dep_id),
            "dep::print",
            "dep__print",
            Some(unit_return_function(dep_id, "dep::print")),
            true,
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(program.instances.contains_key(&object_instance));
}

#[test]
fn prune_unreachable_instances_removes_unreachable_object_backed_instance() {
    let main_id = DefId::new(CrateId(0), LocalDefId(80));
    let dep_id = DefId::new(CrateId(1), LocalDefId(81));
    let main_instance = InstanceId(0);
    let object_instance = InstanceId(1);
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(unit_return_function(main_id, "main")),
            false,
        ),
        instance_record(
            object_instance,
            InstanceOrigin::Function(dep_id),
            "dep::print",
            "dep__print",
            Some(unit_return_function(dep_id, "dep::print")),
            true,
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(!program.instances.contains_key(&object_instance));
}
```

- [ ] **Step 2: Run tests to verify they fail where behavior is incomplete**

Run: `cargo test -p rock-lib prune_unreachable_instances_keeps_reachable_generic_specialization -- --exact`

Expected: PASS if Task 2 traversal already handles this edge; if it fails, the failure should show the specialization was pruned.

Run: `cargo test -p rock-lib prune_unreachable_instances_removes_unused_generic_specialization -- --exact`

Expected: PASS if Task 1 pruning already handles this case.

Run: `cargo test -p rock-lib prune_unreachable_instances_keeps_reachable_object_backed_instance -- --exact`

Expected: PASS if Task 2 traversal already handles object-backed leaves.

Run: `cargo test -p rock-lib prune_unreachable_instances_removes_unreachable_object_backed_instance -- --exact`

Expected: PASS if Task 1 pruning already removes non-root leaves.

These tests may pass immediately because Task 2's instance-edge traversal is generic. Record the result; do not change production code only to force a red state.

- [ ] **Step 3: Add compatibility edge tests for direct `Function(DefId)` and `Var(name)` refs**

Add these tests:

```rust
#[test]
fn prune_unreachable_instances_maps_direct_function_target_to_zero_substitution_instance() {
    let main_id = DefId::new(CrateId(0), LocalDefId(90));
    let helper_id = DefId::new(CrateId(0), LocalDefId(91));
    let main_instance = InstanceId(0);
    let helper_instance = InstanceId(1);
    let main_body = function_returning_expr(
        main_id,
        "main",
        HirExpr {
            kind: HirExprKind::ResolvedVar(HirVarRef {
                name: "wrong_display_name".to_string(),
                target: HirVarTarget::Function(helper_id),
            }),
            ty: Type::Function(Vec::new(), Box::new(Type::Unit)),
            span: Span::default(),
        },
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        instance_record(
            helper_instance,
            InstanceOrigin::Function(helper_id),
            "helper",
            "helper",
            Some(unit_return_function(helper_id, "helper")),
            false,
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(program.instances.contains_key(&helper_instance));
}

#[test]
fn prune_unreachable_instances_maps_name_var_to_zero_substitution_instance() {
    let main_id = DefId::new(CrateId(0), LocalDefId(100));
    let helper_id = DefId::new(CrateId(0), LocalDefId(101));
    let main_instance = InstanceId(0);
    let helper_instance = InstanceId(1);
    let main_body = function_returning_expr(
        main_id,
        "main",
        HirExpr {
            kind: HirExprKind::Var("helper".to_string()),
            ty: Type::Function(Vec::new(), Box::new(Type::Unit)),
            span: Span::default(),
        },
    );
    let mut program = monomorphized_with_instances(vec![
        instance_record(
            main_instance,
            InstanceOrigin::Function(main_id),
            "main",
            "main",
            Some(main_body),
            false,
        ),
        instance_record(
            helper_instance,
            InstanceOrigin::Function(helper_id),
            "helper",
            "helper",
            Some(unit_return_function(helper_id, "helper")),
            false,
        ),
    ]);

    prune_unreachable_instances(&mut program);

    assert!(program.instances.contains_key(&helper_instance));
}
```

- [ ] **Step 4: Run compatibility tests**

Run: `cargo test -p rock-lib prune_unreachable_instances_maps_direct_function_target_to_zero_substitution_instance -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib prune_unreachable_instances_maps_name_var_to_zero_substitution_instance -- --exact`

Expected: PASS.

- [ ] **Step 5: Run all focused instance DCE tests**

Run: `cargo test -p rock-lib prune_unreachable_instances -- --nocapture`

Expected: PASS for all `prune_unreachable_instances_*` tests.

- [ ] **Step 6: Commit**

```bash
git add lib/src/dce.rs
git commit -m "cover instance dce reachability cases"
```

---

### Task 4: Pipeline Integration And Product Link Records

**Files:**
- Modify: `lib/src/lib.rs`
- Modify: `lib/src/dce.rs`

- [ ] **Step 1: Write failing tests for product link records using pruned instances**

Add imports to the existing `#[cfg(test)] mod tests` in `lib/src/lib.rs` if they are not already present:

```rust
use crate::hir::{HirBlock, HirExpr, HirExprKind, HirFunction, HirProgram, HirStmt};
use crate::ids::{CrateId, DefId, InstanceId, LocalDefId};
use crate::mono::{InstanceOrigin, InstanceRecord, MonomorphizedProgram};
use crate::products::{CompilerProducts, ProductCrateIdentity, ProductDefId};
use crate::types::Type;
```

Add these helpers to the `lib/src/lib.rs` test module:

```rust
fn link_test_function(id: DefId, name: &str) -> HirFunction {
    HirFunction {
        id,
        name: name.to_string(),
        qualified_name: None,
        generic_params: Vec::new(),
        generic_param_ids: Vec::new(),
        generic_bounds: std::collections::HashMap::new(),
        params: Vec::new(),
        ret_type: Type::Unit,
        body: HirBlock {
            stmts: vec![HirStmt::Return(Some(HirExpr {
                kind: HirExprKind::Unit,
                ty: Type::Unit,
                span: Default::default(),
            }))],
            ty: Type::Unit,
        },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    }
}

fn link_test_instance(id: InstanceId, function: &HirFunction, name: &str) -> InstanceRecord {
    InstanceRecord {
        id,
        origin: InstanceOrigin::Function(function.id),
        substitution: Vec::new(),
        source_name: name.to_string(),
        backend_symbol: name.to_string(),
        declared: None,
        body: Some(function.clone()),
        provided_by_object: false,
        is_specialization: false,
    }
}

fn link_test_products(functions: Vec<(&str, HirFunction)>) -> CompilerProducts {
    let program = HirProgram::from_parts(
        functions
            .into_iter()
            .map(|(name, function)| (name.to_string(), function))
            .collect(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    let resolved = crate::infer::ResolvedHirProgram::new(
        program,
        crate::collect::resolver::ResolverTables::default(),
        crate::ids::CrateId(0),
        crate::ids::IdGen::default(),
        crate::type_context::TypeContext::new(),
        crate::hir::HirTypeIds::default(),
        std::collections::BTreeSet::new(),
    );
    CompilerProducts::from_resolved_hir(
        ProductCrateIdentity::local("app".to_string()),
        &resolved,
        Vec::new(),
        std::collections::BTreeMap::new(),
        crate::products::ProductSourceFingerprint::default(),
        crate::products::ProductLinkData::default(),
    )
}
```

Add this test:

```rust
#[test]
fn product_link_records_follow_pruned_instance_set() {
    let main_id = DefId::new(CrateId(0), LocalDefId(1));
    let unused_id = DefId::new(CrateId(0), LocalDefId(2));
    let main = link_test_function(main_id, "main");
    let unused = link_test_function(unused_id, "unused");
    let mut monomorphized = MonomorphizedProgram {
        program: HirProgram::from_parts(
            std::collections::HashMap::from([
                ("main".to_string(), main.clone()),
                ("unused".to_string(), unused.clone()),
            ]),
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
            Vec::new(),
            Vec::new(),
        ),
        instances: std::collections::BTreeMap::from([
            (InstanceId(0), link_test_instance(InstanceId(0), &main, "main")),
            (InstanceId(1), link_test_instance(InstanceId(1), &unused, "unused")),
        ]),
    };
    crate::dce::prune_unreachable_instances(&mut monomorphized);
    let mut products = Some(link_test_products(vec![
        ("main", main.clone()),
        ("unused", unused.clone()),
    ]));

    attach_product_link_records(&mut products, &monomorphized, Some("app"));

    let products = products.expect("products should be present");
    assert!(products
        .link
        .records
        .contains_key(&ProductDefId::from(main_id)));
    assert!(!products
        .link
        .records
        .contains_key(&ProductDefId::from(unused_id)));
}
```

- [ ] **Step 2: Run test to verify current link-record behavior fails before pipeline integration if needed**

Run: `cargo test -p rock-lib product_link_records_follow_pruned_instance_set -- --exact`

Expected: PASS once Task 1 DCE exists because the test calls DCE directly. If it fails, fix the helper or product-ID construction before moving on.

- [ ] **Step 3: Integrate instance DCE into the compile pipeline**

In `lib/src/lib.rs`, replace:

```rust
let monomorphized = mono::monomorphize_with_crates(hir, crate_ctx);
```

with:

```rust
let mut monomorphized = mono::monomorphize_with_crates(hir, crate_ctx);
let _dce_report = dce::prune_unreachable_instances(&mut monomorphized);
```

Keep `codegen.compile_program(&monomorphized)` and `attach_product_link_records(&mut products, &monomorphized, ...)` using the same pruned value.

- [ ] **Step 4: Run focused pipeline/link tests**

Run: `cargo test -p rock-lib product_link_records_follow_pruned_instance_set -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib prune_unreachable_instances -- --nocapture`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/lib.rs lib/src/dce.rs
git commit -m "run instance dce before codegen"
```

---

### Task 5: LLVM And Integration Regressions

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Add integration helpers for LLVM IR assertions**

If no reusable helper already exists near `test_generic_function_argument_app_ir_does_not_define_stdlib_object_symbols`, add this helper near that test:

```rust
fn compile_to_llvm_ir(source: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = test_temp_dir(id);
    std::fs::create_dir_all(&dir).unwrap();
    let source_path = dir.join(format!("test_{}.rk", id));
    std::fs::write(&source_path, source).unwrap();
    let mut config = test_config(source_path.clone(), dir.clone());
    config.no_link = true;
    config.emit_llvm = true;
    config.emit_object = Some(dir.join(format!("test_{}.o", id)));

    rock_lib::compile(&config).expect("Compilation failed");

    let module_name = source_path.file_stem().unwrap().to_string_lossy();
    let llvm = std::fs::read_to_string(dir.join(format!("{}.ll", module_name)))
        .expect("Expected LLVM IR output");
    let _ = std::fs::remove_dir_all(&dir);
    llvm
}
```

- [ ] **Step 2: Write failing integration tests for unused function and specialization pruning**

Add these tests near the existing generic instance call-edge tests:

```rust
#[test]
fn test_instance_dce_does_not_emit_unused_current_function() {
    let llvm = compile_to_llvm_ir(
        r#"
used = -> 1
unused = -> 2

main = ->
    (used!).println!
    0
"#,
    );

    let defined_symbols: Vec<&str> = llvm
        .lines()
        .filter(|line| line.starts_with("define "))
        .collect();
    assert!(
        defined_symbols.iter().any(|line| line.contains("@used")),
        "expected used function definition in LLVM IR: {defined_symbols:?}"
    );
    assert!(
        !defined_symbols.iter().any(|line| line.contains("@unused")),
        "unused function should not be defined in LLVM IR: {defined_symbols:?}"
    );
}

#[test]
fn test_instance_dce_does_not_emit_unused_generic_specialization() {
    let llvm = compile_to_llvm_ir(
        r#"
identity = x -> x

use_i64 = -> identity 7
use_i32 = -> identity 8i32

main = ->
    (use_i64!).println!
    0
"#,
    );

    let defined_symbols: Vec<&str> = llvm
        .lines()
        .filter(|line| line.starts_with("define "))
        .collect();
    let identity_specializations: Vec<&str> = defined_symbols
        .iter()
        .copied()
        .filter(|line| line.contains("@identity_mono_"))
        .collect();
    assert_eq!(
        identity_specializations.len(),
        1,
        "only the reachable identity specialization should be defined: {defined_symbols:?}"
    );
    assert!(
        !defined_symbols.iter().any(|line| line.contains("@use_i32")),
        "unused caller should not be defined in LLVM IR: {defined_symbols:?}"
    );
}
```

- [ ] **Step 3: Run tests to verify they fail before pipeline DCE is active**

Run: `cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_current_function -- --exact`

Expected: FAIL before Task 4 pipeline integration, because `unused` is emitted.

Run: `cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_generic_specialization -- --exact`

Expected: FAIL before Task 4 pipeline integration if unused concrete functions/specializations are emitted. If it passes after Task 4, record that it is coverage of the already-integrated behavior.

- [ ] **Step 4: Run Task 15 regressions with new DCE active**

Run: `cargo test -p rock-lib --test integration test_generic_instance_call_edges_for_function_and_method -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_generic_function_argument_app_ir_does_not_define_stdlib_object_symbols -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/tests/integration.rs
git commit -m "add instance dce integration regressions"
```

---

### Task 6: Final Verification And Legacy DCE Cleanup

**Files:**
- Modify: `lib/src/dce.rs`
- Modify: Rust files touched by formatting, if any

- [ ] **Step 1: Update legacy HIR DCE documentation**

In `lib/src/dce.rs`, update the docs above `prune_dead_functions`:

```rust
/// Legacy HIR/name-based function pruning retained for focused compatibility tests.
/// The compiler pipeline uses `prune_unreachable_instances` after monomorphization.
```

Update the docs above `count_dead_functions`:

```rust
/// Legacy diagnostic helper for HIR/name-based pruning tests.
```

- [ ] **Step 2: Run formatting and focused DCE tests**

Run: `cargo fmt --all --check`

Expected: PASS. If this fails, run `cargo fmt --all`, then rerun `cargo fmt --all --check` and expect PASS.

Run: `cargo test -p rock-lib prune_unreachable_instances -- --nocapture`

Expected: PASS.

Run: `cargo test -p rock-lib dead_code_elimination_counts_alias_duplicate_once_by_def_id -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib resolved_var_name_does_not_mark_function_reachable -- --exact`

Expected: PASS.

- [ ] **Step 3: Run integration regressions**

Run: `cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_current_function -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_generic_specialization -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_generic_instance_call_edges_for_function_and_method -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact`

Expected: PASS.

- [ ] **Step 4: Run full verification**

Run: `cargo test -p rock-lib > /tmp/rock-lib-task16-final.log 2>&1`

Expected: PASS. The log should show `0 failed` for unit tests, integration tests, parser tests, and doc tests.

Run: `git diff --check`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/src/dce.rs lib/src/lib.rs lib/tests/integration.rs
git commit -m "verify instance dce completion"
```

---

## Final Verification

Run these commands after all task commits:

```bash
cargo fmt --all --check
cargo test -p rock-lib prune_unreachable_instances -- --nocapture
cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_current_function -- --exact
cargo test -p rock-lib --test integration test_instance_dce_does_not_emit_unused_generic_specialization -- --exact
cargo test -p rock-lib --test integration test_generic_instance_call_edges_for_function_and_method -- --exact
cargo test -p rock-lib --test integration test_generic_function_argument_in_monomorphized_method_call -- --exact
cargo test -p rock-lib > /tmp/rock-lib-task16-final.log 2>&1
```

Expected final state:
- Instance DCE prunes unreachable `InstanceRecord`s before codegen.
- Codegen and product link records consume the same pruned instance set.
- Reachable generic function/method call-edge regressions still pass.
- Product artifacts still do not serialize local `InstanceId` values.
- `/tmp/rock-lib-task16-final.log` shows `0 failed` for all `rock-lib` test phases.
