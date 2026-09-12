# Borrow Checker Remaining Work Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the remaining borrow-check redesign work by moving provenance ownership into `mir::borrowck`, removing codegen's duplicate closure-capture authority, eliminating borrow-sensitive MIR lowering fallbacks, and explicitly resolving the lambda-MIR scope for the first milestone.

**Architecture:** Keep the current passing borrow-check semantics, but move the remaining borrow-sensitive responsibilities to the layers described in the redesign. Treat provenance and closure capture facts as borrow-check-owned data, keep codegen as a consumer rather than a second authority, and only expand MIR lowering where borrow semantics require precise place structure.

**Tech Stack:** Rust 2021, `rock-lib`, MIR builder and borrow checker under `lib/src/mir`, HIR lowering under `lib/src/lower`, closure codegen under `lib/src/codegen`, integration tests in `lib/tests/integration.rs`.

---

## File Map

- `lib/src/mir/borrowck/provenance.rs`: provenance normalization and reborrow resolution
- `lib/src/mir/borrowck/mod.rs`: consume provenance helpers during validation
- `lib/src/mir/dataflow/analyses/loans.rs`: keep generic loan-state mechanics only
- `lib/src/mir/borrowck/conflicts.rs`: overlap logic used after provenance resolution
- `lib/src/codegen/mod.rs`: remove independent closure-capture authority
- `lib/src/codegen/stmt.rs`: stop rebinding captures by side effect during lambda compilation
- `lib/src/mir/builder/mod.rs`: preserve precise place lowering for borrow-sensitive expressions
- `lib/src/mir/builder/expr.rs`: remove borrow-sensitive `Unit` fallbacks where place/cast/index structure matters
- `lib/tests/integration.rs`: parity and behavior verification
- `docs/superpowers/specs/2026-04-13-borrow-checker-remaining-work-design.md`: update if the lambda-MIR decision becomes a deliberate deferral

### Task 1: Move provenance and reborrow ownership into `mir::borrowck`

**Files:**
- Modify: `lib/src/mir/borrowck/provenance.rs`
- Modify: `lib/src/mir/borrowck/mod.rs`
- Modify: `lib/src/mir/dataflow/analyses/loans.rs`
- Test: `lib/src/mir/borrowck/provenance.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write a failing provenance unit test for resolving a deref-owned reborrow root**

```rust
#[test]
fn test_resolve_place_reborrow_uses_owner_loan() {
    let active_loans = HashMap::from([(
        LoanId(0),
        Loan {
            place: Place {
                local: Local(1),
                projection: vec![],
            },
            owners: HashSet::from([Local(2)]),
            kind: LoanKind::Shared,
        },
    )]);

    let place = Place {
        local: Local(2),
        projection: vec![Projection::Deref],
    };

    assert_eq!(
        resolve_place(&place, &active_loans),
        Place {
            local: Local(1),
            projection: vec![],
        }
    );
}
```

- [ ] **Step 2: Run the provenance unit test to verify it fails before the move**

Run: `cargo test -p rock-lib --lib mir::borrowck::provenance::tests::test_resolve_place_reborrow_uses_owner_loan -- --exact`
Expected: FAIL because `resolve_place` does not exist yet in `mir::borrowck::provenance`.

- [ ] **Step 3: Implement provenance-owned place resolution in `mir::borrowck/provenance.rs`**

```rust
pub fn resolve_place(place: &Place, active_loans: &HashMap<LoanId, Loan>) -> Place {
    let mut resolved = place.clone();
    let mut seen = HashSet::new();

    loop {
        let Some(Projection::Deref) = resolved.projection.first() else {
            break;
        };

        if !seen.insert(resolved.local) {
            break;
        }

        let Some(loan) = active_loans
            .values()
            .find(|loan| loan.owners.contains(&resolved.local))
        else {
            break;
        };

        let mut next = loan.place.clone();
        next.projection
            .extend(resolved.projection.iter().skip(1).cloned());
        resolved = next;
    }

    resolved
}
```

- [ ] **Step 4: Make borrow checking use the new provenance helper**

```rust
let access_place = provenance::resolve_place(place, active_loans);
let borrowed_place = provenance::resolve_place(&loan.place, active_loans);
if conflicts::places_conflict(&access_place, &borrowed_place) {
    // existing conflict handling
}
```

- [ ] **Step 5: Remove the duplicated resolution helper from `loans.rs`**

```rust
let place = crate::mir::borrowck::provenance::resolve_place(place, active_loans);
let loan_place = crate::mir::borrowck::provenance::resolve_place(&loan.place, active_loans);
```

- [ ] **Step 6: Run focused provenance and reborrow verification**

Run: `cargo test -p rock-lib --lib mir::borrowck::provenance::tests::test_resolve_place_reborrow_uses_owner_loan -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_deref_mut_borrow_conflicts -- --exact`
Expected: PASS

### Task 2: Remove codegen's independent closure-capture authority

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/stmt.rs`
- Modify: `lib/src/codegen/closures.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write a focused regression test that ensures closure capture behavior survives without codegen rebinding tricks**

```rust
#[test]
fn test_closure_capture_still_works_without_codegen_rebinding() {
    compile_should_pass(
        r#"
main = ->
    x = 1
    f = -> x.println!
    f!
    0
"#,
    );
}
```

- [ ] **Step 2: Run the closure capture regression and verify current behavior is covered**

Run: `cargo test -p rock-lib --test integration test_closure_capture_still_works_without_codegen_rebinding -- --exact`
Expected: PASS

- [ ] **Step 3: Replace codegen-side capture rebinding with direct consumption of earlier capture facts**

```rust
// remove the "captures_before" / diffing logic
let val = self.compile_expr(value)?;
...
self.set_variable(name.clone(), alloca, ty.clone());
```

```rust
/// Remove closure_captures as a semantic authority field if it is no longer needed.
/// If a map is still required for codegen layout, populate it only from earlier capture facts.
```

- [ ] **Step 4: Run closure behavior verification**

Run: `cargo test -p rock-lib --test integration test_closure_capture -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`
Expected: PASS

### Task 3: Eliminate borrow-sensitive MIR lowering fallbacks

**Files:**
- Modify: `lib/src/mir/builder/mod.rs`
- Modify: `lib/src/mir/builder/expr.rs`
- Test: `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Write a failing MIR-builder test for index lowering with a computed index local**

```rust
#[test]
fn test_lower_expr_index_emits_place_use_instead_of_unit_fallback() {
    let program = HirProgram {
        functions: std::collections::HashMap::new(),
        structs: std::collections::HashMap::new(),
        enums: std::collections::HashMap::new(),
        traits: std::collections::HashMap::new(),
        impls: vec![],
        externs: vec![],
    };
    let mut builder = MirBuilder::new(&program);
    builder.blocks.push(BasicBlock {
        statements: vec![],
        terminator: None,
    });
    builder.current_block = Some(BasicBlockId(0));
    builder.locals.push(LocalDecl {
        ty: Type::Array(Box::new(Type::I64)),
        mutability: Mutability::Not,
        name: Some("arr".to_string()),
        span: None,
    });
    builder.var_map.insert("arr".to_string(), Local(0));
    builder.locals.push(LocalDecl {
        ty: Type::I64,
        mutability: Mutability::Not,
        name: Some("i".to_string()),
        span: None,
    });
    builder.var_map.insert("i".to_string(), Local(1));

    let expr = HirExpr {
        kind: HirExprKind::Index(
            Box::new(HirExpr {
                kind: HirExprKind::Var("arr".to_string()),
                ty: Type::Array(Box::new(Type::I64)),
                span: Span::default(),
            }),
            Box::new(HirExpr {
                kind: HirExprKind::Var("i".to_string()),
                ty: Type::I64,
                span: Span::default(),
            }),
        ),
        ty: Type::I64,
        span: Span::default(),
    };

    builder.lower_expr(
        &expr,
        Place {
            local: Local(1),
            projection: vec![],
        },
    );

    assert!(builder.blocks[0].statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Assign(_, Rvalue::Use(Operand::Copy(Place { projection, .. })))
            if matches!(projection.as_slice(), [Projection::Index(Local(1))])
    )));
}
```

- [ ] **Step 2: Run the MIR-builder test to verify the fallback is real**

Run: `cargo test -p rock-lib test_lower_expr_index_emits_place_use_instead_of_unit_fallback -- --exact`
Expected: FAIL if index lowering still falls back to `Unit` in this borrow-sensitive case.

- [ ] **Step 3: Make borrow-sensitive index lowering preserve explicit place structure**

```rust
HirExprKind::Index(base, index) => {
    let base_place = self.lower_place(base)?;
    let index_local = self.lower_index_operand_local(index)?;
    let mut place = base_place;
    place.projection.push(Projection::Index(index_local));
    let operand = self.operand_for_place(&expr.ty, place, false);
    self.emit_assign(dest, Rvalue::Use(operand), span);
}
```

- [ ] **Step 4: Keep tuple and field borrow-sensitive lowering on the same explicit-place path**

```rust
if let Some(place) = self.lower_place(expr) {
    let operand = self.operand_for_place(&expr.ty, place, false);
    self.emit_assign(dest, Rvalue::Use(operand), span);
}
```

- [ ] **Step 5: Run MIR-builder verification**

Run: `cargo test -p rock-lib test_lower_place_field_projection -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib test_lower_place_tuple_index_projection -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib test_lower_place_index_projection -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib test_lower_expr_index_emits_place_use_instead_of_unit_fallback -- --exact`
Expected: PASS

### Task 4: Resolve lambda MIR scope for the first milestone

**Files:**
- Modify: `docs/superpowers/specs/2026-04-13-borrow-checker-remaining-work-design.md`
- Modify: `docs/superpowers/plans/2026-04-13-borrow-checker-remaining-work.md`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Re-run the closure parity set to verify the supported surface is satisfied without synthetic lambda MIR**

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_closure_capture -- --exact`
Expected: PASS

- [ ] **Step 2: If those tests pass and no remaining supported case requires lambda-body MIR, document a deliberate first-milestone deferral**

```markdown
### Lambda MIR Scope Decision

For the first milestone, explicit closure capture facts plus closure-value liveness are sufficient to enforce the currently supported closure borrow semantics. Synthetic MIR lambda-body lowering is deferred to a later milestone unless a supported borrow rule requires borrowck to inspect closure body MIR directly.
```

- [ ] **Step 3: If a supported case does require lambda-body MIR during this task, stop and replace this task with a dedicated synthetic-MIR implementation plan**

Expected: not needed if all current supported closure tests remain green.

- [x] **Decision:** current supported closure behavior remains green without synthetic lambda-body MIR, so the first milestone explicitly defers that work.

### Task 5: Run the targeted set and full suite serially

**Files:**
- No code changes required unless verification exposes a regression

- [ ] **Step 1: Run the targeted borrow regression set serially**

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_assignment_rust_parity -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_shared_ref_blocks_move_rust_parity -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_deref_mut_borrow_conflicts -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_mut_ref_branch_merge_blocks_later_use -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_loop -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_closure_shared_capture_blocks_mutation -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_closure_move_capture_moves_value -- --exact`
Expected: PASS

Run: `cargo test -p rock-lib --test integration test_borrow_raw_pointer_from_borrow_preserves_reference_rules -- --exact`
Expected: PASS

- [ ] **Step 2: Run the full library suite once after the follow-up settles**

Run: `cargo test -p rock-lib`
Expected: PASS
