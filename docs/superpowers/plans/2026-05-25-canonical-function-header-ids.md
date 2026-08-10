# Canonical Function Header IDs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the remaining function-header `DefId(0, 0)` placeholder by requiring callers to provide a concrete `DefId` when constructing `HirFunction` headers.

**Architecture:** `LocalCollector` is already the boundary that translates source names to `CollectedIdEnvironment` IDs through `item_id_for_name`. Thread that ID into `collect::headers` function-header builders so standalone and signature-backed top-level function declarations receive their canonical identity at construction time, and generic owners are remapped to that same ID immediately. Trait and impl method headers should also receive explicit IDs at construction time, using the existing provisional method ID path until the later canonical method assignment pass replaces those IDs.

**Tech Stack:** Rust 2021, `rock-lib`, collection headers, collection tests, `cargo test -p rock-lib ...`, `cargo fmt --all --check`, `git diff --check`.

---

## File Structure

- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
- Test: `lib/src/collect/headers.rs` unit tests
- Test: `lib/src/collect/mod.rs` collection tests

Do not commit these changes unless the user explicitly asks for a commit.

---

### Task 1: Add Red Tests For Function Header IDs

**Files:**
- Modify: `lib/src/collect/headers.rs`

- [x] **Step 1: Write failing unit tests**

Add these tests near the existing function-header tests in `lib/src/collect/headers.rs`:

```rust
    #[test]
    fn test_build_function_header_uses_provided_function_id() {
        let mut lowerer = CollectContext::new();
        let decl = function_decl("map", &["f", "x"], None, LambdaArrowKind::Curried);
        let function_id = def_id(44);

        let function = build_function_header_with_id(&mut lowerer, &decl, function_id);

        assert_eq!(function.id, function_id);
    }

    #[test]
    fn test_build_function_header_with_sig_uses_provided_function_id_for_generic_owner() {
        let mut lowerer = CollectContext::new();
        let decl = function_decl("id", &["value"], None, LambdaArrowKind::Normal);
        let signature_id = def_id(11);
        let function_id = def_id(45);
        let sig_generic = GenericParamId {
            owner: signature_id,
            index: 0,
        };
        let sig = HirFunctionSig {
            id: signature_id,
            name: "id".to_string(),
            generic_params: vec!["T".to_string()],
            generic_param_ids: vec![sig_generic],
            params: vec![Type::Generic(sig_generic)],
            ret: Type::Generic(sig_generic),
            generic_bounds: HashMap::new(),
            self_receiver: None,
        };

        let function = build_function_header_with_sig(&mut lowerer, &decl, &sig, function_id);

        assert_eq!(function.id, function_id);
        assert_eq!(
            function.generic_param_ids,
            vec![GenericParamId {
                owner: function_id,
                index: 0,
            }]
        );
        assert_eq!(
            function.params[0].ty,
            Type::Generic(GenericParamId {
                owner: function_id,
                index: 0,
            })
        );
        assert_eq!(
            function.ret_type,
            Type::Generic(GenericParamId {
                owner: function_id,
                index: 0,
            })
        );
    }
```

- [x] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p rock-lib test_build_function_header
```

Expected: FAIL because `build_function_header_with_id` does not exist and `build_function_header_with_sig` does not accept a function ID argument yet.

---

### Task 2: Thread Canonical Function IDs Through Header Builders

**Files:**
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/collector.rs`

- [x] **Step 1: Replace `build_function_header` with ID-taking builder**

In `lib/src/collect/headers.rs`, replace the current `build_function_header` signature with an ID-taking builder:

```rust
pub(crate) fn build_function_header_with_id(
    context: &mut CollectContext,
    fd: &ast::FunctionDecl,
    function_id: DefId,
) -> HirFunction {
```

Then in the returned `HirFunction`, replace:

```rust
        id: DefId::new(CrateId(0), LocalDefId(0)),
```

with:

```rust
        id: function_id,
```

Do not keep a no-argument wrapper that constructs `DefId::new(CrateId(0), LocalDefId(0))`.

- [x] **Step 2: Update `build_function_header_with_sig` API**

In `lib/src/collect/headers.rs`, change the signature from:

```rust
pub(crate) fn build_function_header_with_sig(
    context: &mut CollectContext,
    fd: &ast::FunctionDecl,
    sig: &HirFunctionSig,
) -> HirFunction {
```

to:

```rust
pub(crate) fn build_function_header_with_sig(
    context: &mut CollectContext,
    fd: &ast::FunctionDecl,
    sig: &HirFunctionSig,
    function_id: DefId,
) -> HirFunction {
```

Inside that function, remove:

```rust
    let function_id = DefId::new(CrateId(0), LocalDefId(0));
```

Keep the existing `remap_generic_param_ids_in_type`, `remap_generic_bounds_owner`, and `id: function_id` logic so signature-owned generic IDs move to the canonical function owner.

- [x] **Step 3: Pass canonical IDs from `LocalCollector`**

In `lib/src/collect/collector.rs`, replace `collect_function_header` with:

```rust
    fn collect_function_header(&mut self, fd: &ast::FunctionDecl, name: String) {
        let function_id = self.item_id_for_name(&name);
        let func = if let Some(sig) = self.context.function_sigs.get(&name).cloned() {
            let func =
                headers::build_function_header_with_sig(&mut self.context, fd, &sig, function_id);
            self.context.function_sigs.remove(&name);
            func
        } else {
            headers::build_function_header_with_id(&mut self.context, fd, function_id)
        };

        let param_types: Vec<Type> = func.params.iter().map(|p| p.ty.clone()).collect();
        let func_type = Type::Function(param_types, Box::new(func.ret_type.clone()));
        self.context.scope.define(name.clone(), func_type, false);
        self.context.functions.insert(name, func);
    }
```

- [x] **Step 4: Update existing tests and header-local call sites**

In `lib/src/collect/headers.rs`, update existing direct calls to the changed signature:

```rust
let function = build_function_header_with_id(&mut lowerer, &decl, def_id(42));
let function = build_function_header_with_sig(&mut lowerer, &decl, &sig, def_id(43));
```

Use a distinct `def_id(...)` value in each test. For trait and impl method header construction inside `build_trait_with_id` and `build_impl_with_id`, pass `context.fresh_provisional_def_id()` so method headers no longer fabricate `DefId(0, 0)` and the existing `assign_canonical_method_ids` pass can continue replacing provisional method IDs with canonical method IDs.

---

### Task 3: Verify Collection Uses Canonical Function IDs

**Files:**
- Modify: `lib/src/collect/mod.rs`

- [x] **Step 1: Add collection regression test**

Add this test near existing collection tests in `lib/src/collect/mod.rs`:

```rust
    #[test]
    fn collect_assigns_canonical_id_to_signature_backed_function_header() {
        let program = Program {
            module: Module {
                name: None,
                top_levels: vec![
                    TopLevel::FunctionSig(crate::ast::FunctionSig {
                        name: ident("identity"),
                        sig: crate::ast::ParseType::Function(vec![
                            crate::ast::ParseType::Type(crate::ast::ParseTypeInner {
                                name: "T".to_string(),
                                generics: vec![],
                                span: crate::lexer::Span::default(),
                            }),
                            crate::ast::ParseType::Type(crate::ast::ParseTypeInner {
                                name: "T".to_string(),
                                generics: vec![],
                                span: crate::lexer::Span::default(),
                            }),
                        ]),
                        where_clauses: vec![],
                        self_receiver: None,
                        is_unsafe: false,
                        exported: false,
                    }),
                    TopLevel::FunctionDecl(crate::ast::FunctionDecl {
                        name: ident("identity"),
                        lambda: crate::ast::LambdaDecl {
                            parameters: vec![crate::ast::Pattern {
                                binding: None,
                                kind: crate::ast::PatternKind::Ident(crate::ast::IdentPattern {
                                    name: ident("value"),
                                    mut_: false,
                                }),
                            }],
                            body: crate::ast::Block { statements: vec![] },
                            arrow_kind: crate::ast::LambdaArrowKind::Normal,
                        },
                        self_receiver: None,
                        is_unsafe: false,
                        exported: false,
                    }),
                ],
                is_inline: false,
                filepath: None,
            },
        };

        let decls = collect(&program, &CrateContext::new(), false, Some("test"))
            .expect("collect should assign canonical IDs");
        let function = decls
            .functions
            .get("identity")
            .expect("identity function should be collected");

        assert_ne!(function.id, DefId::new(CrateId(0), LocalDefId(0)));
        assert!(decls.current_def_ids.contains(&function.id));
        assert!(
            function
                .generic_param_ids
                .iter()
                .all(|generic_id| generic_id.owner == function.id),
            "signature-backed generic owners should use the canonical function ID"
        );
    }
```

- [x] **Step 2: Run focused collection tests**

Run:

```bash
cargo test -p rock-lib test_build_function_header
cargo test -p rock-lib collect_assigns_canonical_id_to_signature_backed_function_header
```

Expected: PASS.

---

### Task 4: Update Roadmap Documentation

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [x] **Step 1: Update `master-audit-checklist.md` evidence and done list**

In Identity And Arenas evidence, add:

```markdown
- Function HIR headers built by collection now receive the collector-provided canonical `DefId` instead of constructing the placeholder `DefId(0, 0)` and repairing it later.
```

In the Done list, add:

```markdown
- [x] Removed function-header `DefId(0, 0)` construction from the collection-authoritative path by threading canonical IDs into function header builders.
```

- [x] **Step 2: Update ordered roadmap reconciliation**

In Task 1's code evidence column, add `collection-built function headers use collector-provided canonical IDs instead of `DefId(0, 0)``.

In Task 1's remaining work column, remove function-header placeholder wording if present. Keep broader remaining work for provisional generic owners, auto `Sized` impl provenance, inference placeholder repairs, and legacy signature fallback IDs.

---

### Task 5: Final Verification

**Files:**
- Verify only; do not commit unless explicitly requested.

- [x] **Step 1: Run focused tests**

Run:

```bash
cargo test -p rock-lib test_build_function_header
cargo test -p rock-lib collect_assigns_canonical_id_to_signature_backed_function_header
```

Expected: PASS.

- [x] **Step 2: Run broader collection tests touched by this change**

Run:

```bash
cargo test -p rock-lib collect_preserves_import_aliases_infix_precedence_and_function_signatures
```

Expected: PASS.

- [x] **Step 3: Run hygiene checks**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: both commands exit successfully.

- [x] **Step 4: Record verification result**

Update this plan with the exact commands run and whether they passed. Do not state the task is complete unless these checks passed in this session.

Verification recorded on 2026-05-25:

- RED confirmed: `cargo test -p rock-lib test_build_function_header_uses_provided_function_id -- --exact` failed to compile because `build_function_header_with_id` was missing and `build_function_header_with_sig` did not accept a function ID.
- PASS: `cargo test -p rock-lib test_build_function_header` ran 6 matching header tests successfully.
- PASS: `cargo test -p rock-lib collect_assigns_canonical_id_to_signature_backed_function_header` ran the collection regression successfully.
- PASS: `cargo test -p rock-lib collect_preserves_import_aliases_infix_precedence_and_function_signatures` ran the broader collection bookkeeping regression successfully.
- PASS: `cargo test -p rock-lib` completed with unit `1258 passed; 0 failed; 1 ignored`, integration `277 passed; 0 failed`, parser integration `1 passed`, and doctests `1 passed; 1 ignored`.
- PASS: `cargo fmt --all --check`.
- PASS: `git diff --check`.
- PASS: `grep` for `DefId::new(CrateId(0), LocalDefId(0))` in `lib/src/collect/headers.rs` returned no files.

---

## Self-Review

- Spec coverage: This plan covers the approved canonical function-header ID cleanup and leaves broader provisional generic owner, auto-impl provenance, and inference fallback work out of scope.
- Placeholder scan: No unresolved placeholder markers or unspecified implementation steps remain.
- Type consistency: The planned signatures consistently use `DefId`, `GenericParamId`, `HirFunctionSig`, and existing `CollectContext`/`LocalCollector` APIs.
