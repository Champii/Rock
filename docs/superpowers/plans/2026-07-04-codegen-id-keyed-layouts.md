# Codegen ID-Keyed Layouts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete `CLEAN_SLATE_COMPILER_AUDIT.md` Step 2 by removing production codegen name-keyed layout side tables and leaving semantic layout/generic lookup ID-keyed and contract-owned.

**Architecture:** `MirBackendContract.nominal_layouts` remains the only source used to populate codegen nominal layout state. `CodeGen` keeps ID-keyed struct/enum layout and generic-parameter tables, while dead name-keyed struct maps and the name-based `struct_substitution` helper are deleted. Validation uses focused compile tests plus manual `rg` residue scans, not permanent absence-only audit tests.

**Tech Stack:** Rust 2021, `rock-lib`, MIR backend contract, LLVM codegen, `DefId`, `TypeId`, `cargo test -p rock-lib`, `cargo fmt --all --check`, `git diff --check`.

---

## Scope

This plan implements only Step 2. It does not change `qualified_name`, product link records, method selection, language items, or builtin index semantics.

## File Map

- Modify `lib/src/codegen/mod.rs`: delete dead name-keyed `CodeGen` fields and constructor entries.
- Modify `lib/src/codegen/types.rs`: delete the name-based `struct_substitution` helper; keep `struct_substitution_by_id`; replace the old name-keyed unit test with a contract-loaded ID-keyed test; remove stale direct `struct_info` fixture writes.
- Modify `CLEAN_SLATE_COMPILER_AUDIT.md`: mark Step 2 complete with exact validation evidence after implementation.
- No durable source-content absence tests are added; use manual residue scans for deleted symbol names.

## Task 1: Establish Red Residue Checks

**Files:**
- Inspect: `lib/src/codegen/mod.rs`
- Inspect: `lib/src/codegen/types.rs`

- [x] **Step 1: Run the Step 2 residue check and confirm it fails before implementation**

Run:

```bash
if /home/linuxbrew/.linuxbrew/bin/rg -n "struct_info|enum_info|struct_generic_owners|enum_generic_owners|struct_generic_param_ids\b|enum_generic_param_ids\b|struct_substitution\(" lib/src/codegen; then exit 1; else test "$?" -eq 1; fi
```

Expected before implementation: FAIL because `CodeGen` still contains those name-keyed residues.

- [x] **Step 2: Run the focused existing codegen type tests as baseline**

Run:

```bash
cargo test -p rock-lib codegen::types -- --nocapture
```

Expected before implementation: PASS, establishing behavioral baseline before deleting the dead side tables.

## Task 2: Remove Name-Keyed Struct Layout State

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/codegen/types.rs`

- [x] **Step 1: Delete name-keyed fields from `CodeGen`**

Remove these fields from `CodeGen` in `lib/src/codegen/mod.rs`:

```rust
struct_info: HashMap<String, Vec<(String, TypeId)>>,
struct_generic_param_ids: HashMap<String, Vec<GenericParamId>>,
struct_generic_owners: HashMap<String, DefId>,
```

Remove the matching `HashMap::new()` constructor initializers.

- [x] **Step 2: Delete the name-based `struct_substitution` helper**

Remove the complete function named `struct_substitution` from
`lib/src/codegen/types.rs`. The function starts with this signature:

```rust
pub(crate) fn struct_substitution(
    &self,
    name: &str,
    type_args: &[Type],
) -> HashMap<GenericParamId, Type>
```

Keep `struct_substitution_by_id` unchanged as the semantic replacement.

- [x] **Step 3: Remove stale name-keyed tests and direct side-table writes**

Delete the test that seeds `struct_generic_owners`/`struct_generic_param_ids` and calls `struct_substitution` with the string name `"Pair"`.

Replace the `test_unknown_nominal_struct_id_fails_explicitly` setup so it no longer writes `codegen.struct_info`; the test should still call `llvm_type(&Type::Struct { id: unknown_def_id, args: Vec::new() })` and panic with `unknown nominal struct DefId`.

## Task 3: Add Contract-Loaded ID-Keyed Coverage

**Files:**
- Modify: `lib/src/codegen/types.rs`

- [x] **Step 1: Add or keep a test proving struct generic substitution is ID-keyed**

Use `MirBackendContract.nominal_layouts` to load a generic struct layout into `CodeGen`, then call `struct_substitution_by_id`:

```rust
#[test]
fn struct_substitution_by_id_uses_contract_generic_param_ids() {
    let context = inkwell::context::Context::create();
    let mut type_context = crate::type_context::TypeContext::new();
    let mut codegen = CodeGen::new(&context, "struct_generic_id_table_test");
    let owner = DefId::new(CrateId(1), LocalDefId(7));
    let first = GenericParamId { owner, index: 0 };
    let second = GenericParamId { owner, index: 1 };
    let i64_id = type_context.intern_type(&Type::I64);
    let bool_id = type_context.intern_type(&Type::Bool);
    let mut backend_contract = MirBackendContract::default();
    backend_contract.nominal_layouts.insert(
        owner,
        MirNominalLayout::Struct {
            id: owner,
            fields: vec![
                ("left".to_string(), i64_id),
                ("right".to_string(), bool_id),
            ],
            generic_params: vec![first, second],
        },
    );
    let mir = MirProgram {
        functions: BTreeMap::new(),
        type_context,
        backend_contract,
    };
    codegen.compile_program_from_mir(&mir).unwrap();

    let subst = codegen.struct_substitution_by_id(owner, &[Type::I64, Type::Bool]);

    assert_eq!(subst.get(&first), Some(&Type::I64));
    assert_eq!(subst.get(&second), Some(&Type::Bool));
}
```

- [x] **Step 2: Run the focused test**

Run:

```bash
cargo test -p rock-lib codegen::types::tests::struct_substitution_by_id_uses_contract_generic_param_ids -- --exact --nocapture
```

Expected after implementation: PASS.

## Task 4: Verify Step 2 And Update Audit

**Files:**
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md`

- [x] **Step 1: Run final focused validation**

Run:

```bash
cargo test -p rock-lib codegen::types -- --nocapture
cargo test -p rock-lib codegen -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
cargo test -p rock-lib --test integration
```

Expected: all tests pass.

- [x] **Step 2: Run formatting and residue checks**

Run:

```bash
cargo fmt --all --check
git diff --check
if /home/linuxbrew/.linuxbrew/bin/rg -n "struct_info|enum_info|struct_generic_owners|enum_generic_owners|struct_generic_param_ids\b|enum_generic_param_ids\b|struct_substitution\(" lib/src/codegen; then exit 1; else test "$?" -eq 1; fi
```

Expected: formatting and diff checks produce no output; residue check produces no output and exits successfully.

- [x] **Step 3: Update `CLEAN_SLATE_COMPILER_AUDIT.md` Step 2**

Add a `Status: Complete as of 2026-07-04` note under Step 2 with the commands that passed.

- [x] **Step 4: Request review**

Ask for review focused on Step 2 only: no remaining production name-keyed layout side tables, no accidental removal of ID-keyed layout state, and no test fixture bypass of `MirBackendContract` for semantic layout data.
