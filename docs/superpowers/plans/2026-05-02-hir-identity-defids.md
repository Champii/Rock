# HIR Identity — Add DefId to HIR Entity Structs

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every HIR entity (function, struct, enum, trait, impl) a stable `DefId` so later phases can use IDs for identity instead of strings.

**Architecture:** Add `id: DefId` fields to `HirFunction`, `HirStruct`, `HirEnum`, `HirTrait`, and `HirImpl`. Wire them through the construction pipeline (collect → lower → HIR). This is the prerequisite for replacing string-keyed maps with ID-keyed tables in future slices. No `Type` model changes in this slice — that's a separate, much larger refactor.

**Tech Stack:** Rust 2021, `cargo test -p rock-lib`, existing HIR/mono/codegen types.

**What this slice enables:**
- Codegen can use `func.id` for identity instead of `func.name`
- Mono already uses DefId; this makes it directly available on HIR types
- Future slices can replace `HashMap<String, HirFunction>` with `HashMap<DefId, HirFunction>`
- Future slices can use DefId for cross-references in method dispatch and type lookup

**What this slice does NOT do:**
- Does not change `Type::Struct(String, …)` — that touches ~166 sites across ~30 files
- Does not replace string-keyed maps with ID-keyed maps — that's the follow-up
- Does not change codegen's `self.functions: HashMap<String, FunctionValue>` yet

---

### Task 1: Add `id: DefId` to HirFunction

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs` (construction sites)
- Modify: `lib/src/collect/mod.rs` (construction sites)
- Modify: `lib/src/lower/function.rs` (construction sites)
- Modify: `lib/src/lower/control_flow/secondary.rs` (construction sites)
- Modify: `lib/src/lower/traits/defaults.rs` (construction sites)
- Modify: `lib/src/mono/external.rs` (create_stub_generic_function + test sites)
- Modify: `lib/src/mono/specialize.rs` (create_specialization test)
- Modify: `lib/src/mono/process.rs` (1 test site)
- Modify: `lib/src/mir/builder/mod.rs` (multiple test sites)
- Test: all touched files

- [ ] **Step 1: Add the field**

```rust
// lib/src/hir/mod.rs — add to HirFunction struct (after qualified_name line)
pub struct HirFunction {
    pub id: DefId,                                     // NEW: canonical definition identity
    pub name: String,
    pub qualified_name: Option<String>,
    pub generic_params: Vec<String>,
    pub generic_bounds: HashMap<String, Vec<String>>,
    pub params: Vec<HirParam>,
    pub ret_type: Type,
    pub body: HirBlock,
    pub is_curried: bool,
    pub is_method: bool,
    pub self_receiver: Option<SelfReceiverMode>,
    pub is_unsafe: bool,
}
```

Add the import at the top of `lib/src/hir/mod.rs`:
```rust
use crate::ids::DefId;
```

- [ ] **Step 2: Update the two production construction sites in `lower/function.rs`**

`lib/src/lower/function.rs` around line 269 — add `id: DefId` field. Find the construction and add it:

```rust
// Look for the create_hir_function or similar method that builds HirFunction.
// Add: id: self.allocate_func_def_id() or equivalent
```

Read the file at the construction site to find the exact context. The resolver should provide the DefId for this function. Check if the lowerer already has access to resolver tables or if we need to thread it through.

- [ ] **Step 3: Update construction sites in `collect/headers.rs`**

`lib/src/collect/headers.rs` lines 305, 392 — these build `HirFunction` structs. Add `id: DefId` to each construction. The DefId comes from the resolver tables populated during collection.

- [ ] **Step 4: Update construction sites in `collect/mod.rs`**

`lib/src/collect/mod.rs` lines 653, 1075 — same pattern.

- [ ] **Step 5: Update construction sites in `lower/control_flow/secondary.rs`**

Lines 286, 613 — lambdas and helper functions built during lowering.

- [ ] **Step 6: Update construction in `lower/traits/defaults.rs`**

Line 62 — trait default method cloning.

- [ ] **Step 7: Update construction in `mono/external.rs` — `create_stub_generic_function`**

Line 271 — stub generic functions for external crates.

- [ ] **Step 8: Update test construction sites**

Search for all `HirFunction {` in test code and add `id: DefId::new(CrateId(0), LocalDefId(0))`:
- `lib/src/mono/external.rs` around lines 306, 340
- `lib/src/mono/methods.rs` around lines 583, 757, 789, 810, 831
- `lib/src/mono/process.rs` line 467
- `lib/src/mono/specialize.rs` line 320
- `lib/src/mir/builder/mod.rs` around lines 1051, 1083
- `lib/src/codegen/mod.rs` around line 519
- `lib/src/mono/registry.rs` test sites

- [ ] **Step 9: Build and run mono tests**

Run: `cargo test -p rock-lib mono::`
Expected: compile and all 11 tests pass.

- [ ] **Step 10: Run full test suite**

Run: `cargo test -p rock-lib`
Expected: all 800+ tests pass.

- [ ] **Step 11: Commit**

```bash
git add lib/src/hir/mod.rs lib/src/collect/headers.rs lib/src/collect/mod.rs \
        lib/src/lower/function.rs lib/src/lower/control_flow/secondary.rs \
        lib/src/lower/traits/defaults.rs lib/src/mono/external.rs \
        lib/src/mono/specialize.rs lib/src/mono/process.rs lib/src/mir/builder/mod.rs \
        lib/src/codegen/mod.rs lib/src/mono/registry.rs lib/src/mono/methods.rs
git commit -m "hir: add DefId field to HirFunction"
```

---

### Task 2: Add `id: DefId` to HirStruct

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/lower/collect/types.rs`
- Test: `lib/src/hir/mod.rs`, `lib/src/mir/builder/mod.rs`

- [ ] **Step 1: Add the field**

```rust
// lib/src/hir/mod.rs
pub struct HirStruct {
    pub id: DefId,              // NEW
    pub name: String,
    pub generic_params: Vec<String>,
    pub fields: Vec<HirField>,
}
```

- [ ] **Step 2: Update 11 construction sites**

Find each `HirStruct {` in:
- `lib/src/collect/headers.rs` line 142
- `lib/src/collect/mod.rs` lines 627, 962, 1094
- `lib/src/lower/collect/types.rs` lines 36, 125
- `lib/src/mir/builder/mod.rs` lines 361, 613 (test)

Add `id: DefId::new(CrateId(0), LocalDefId(0))` for tests, or the real DefId for production code (resolved from resolver tables).

- [ ] **Step 3: Build and test**

Run: `cargo test -p rock-lib`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/hir/mod.rs lib/src/collect/headers.rs lib/src/collect/mod.rs \
        lib/src/lower/collect/types.rs lib/src/mir/builder/mod.rs
git commit -m "hir: add DefId field to HirStruct"
```

---

### Task 3: Add `id: DefId` to HirEnum

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/collect/types.rs`

- [ ] **Step 1: Add the field**

```rust
// lib/src/hir/mod.rs
pub struct HirEnum {
    pub id: DefId,              // NEW
    pub name: String,
    pub generic_params: Vec<String>,
    pub variants: Vec<HirVariant>,
}
```

- [ ] **Step 2: Update 6 construction sites**

Find each `HirEnum {` in:
- `lib/src/collect/headers.rs` line 195
- `lib/src/lower/collect/types.rs` lines 92, 179

Add `id: DefId::new(...)` from resolver tables.

- [ ] **Step 3: Build and test**

Run: `cargo test -p rock-lib`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/hir/mod.rs lib/src/collect/headers.rs lib/src/lower/collect/types.rs
git commit -m "hir: add DefId field to HirEnum"
```

---

### Task 4: Add `id: DefId` to HirTrait

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/collect/traits.rs`

- [ ] **Step 1: Add the field**

```rust
// lib/src/hir/mod.rs
pub struct HirTrait {
    pub id: DefId,                            // NEW
    pub name: String,
    pub generic_params: Vec<String>,
    pub associated_types: Vec<HirAssociatedTypeDecl>,
    pub methods: HashMap<String, HirFunction>,
    pub signatures: HashMap<String, HirFunctionSig>,
}
```

- [ ] **Step 2: Update 4 construction sites**

Find each `HirTrait {` in:
- `lib/src/collect/headers.rs` line 452
- `lib/src/lower/collect/traits.rs` line 154

Add `id: DefId::new(...)` from resolver tables.

- [ ] **Step 3: Build and test**

Run: `cargo test -p rock-lib`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/hir/mod.rs lib/src/collect/headers.rs lib/src/lower/collect/traits.rs
git commit -m "hir: add DefId field to HirTrait"
```

---

### Task 5: Add `id: DefId` to HirImpl

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Test: mono test files

- [ ] **Step 1: Add the field**

```rust
// lib/src/hir/mod.rs
pub struct HirImpl {
    pub id: DefId,                         // NEW
    pub owner: HirImplOwner,
    pub type_name: String,
    pub type_generics: Vec<String>,
    pub receiver_arg_types: Vec<Type>,
    pub trait_name: Option<String>,
    pub trait_generics: Vec<String>,
    pub trait_arg_types: Vec<Type>,
    pub associated_types: Vec<HirAssociatedTypeDef>,
    pub bounds: Vec<TraitBound>,
    pub methods: HashMap<String, HirFunction>,
}
```

- [ ] **Step 2: Update 17 construction sites**

Find each `HirImpl {` in the 4 production files plus test files and add `id: DefId::new(...)`.

Production:
- `lib/src/collect/headers.rs` line 570
- `lib/src/lower/traits/conformance.rs` line 29
- `lib/src/lower/collect/traits.rs` line 257

Test (add synthetic `id: DefId::new(CrateId(0), LocalDefId(0))`):
- `lib/src/mono/mod.rs` lines 505, 517
- `lib/src/mono/methods.rs` lines 881, 928, 973, 985, 1020
- `lib/src/mono/external.rs` lines 432, 447, 511
- `lib/src/mono/process.rs` lines 489, 501

- [ ] **Step 3: Build and test**

Run: `cargo test -p rock-lib`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add lib/src/hir/mod.rs lib/src/collect/headers.rs lib/src/lower/traits/conformance.rs \
        lib/src/lower/collect/traits.rs lib/src/mono/mod.rs lib/src/mono/methods.rs \
        lib/src/mono/external.rs lib/src/mono/process.rs
git commit -m "hir: add DefId field to HirImpl"
```

---

### Task 6: Wire DefIds from the resolver through the pipeline

**Files:**
- Modify: `lib/src/collect/headers.rs` (allocate and assign DefIds)
- Modify: `lib/src/collect/mod.rs` (allocate and assign DefIds)
- Modify: `lib/src/lower/function.rs` (propagate DefId to HirFunction)
- Modify: `lib/src/lower/control_flow/secondary.rs` (propagate DefId)
- Modify: `lib/src/lower/collect/types.rs` (propagate DefId to HirStruct/HirEnum)
- Modify: `lib/src/lower/collect/traits.rs` (propagate DefId to HirTrait/HirImpl)
- Modify: `lib/src/lower/traits/conformance.rs` (propagate DefId to HirImpl)
- Modify: `lib/src/lower/traits/defaults.rs` (propagate DefId)
- Test: full suite

After adding `id` fields in tasks 1-5, most sites got a placeholder `DefId::new(CrateId(0), LocalDefId(0))`. This task replaces those with real DefIds from the resolver tables, which already map names → DefId.

- [ ] **Step 1: Allocate DefIds during collection**

In `collect/headers.rs`, the header collections already create resolver entries (in `ResolverTables.item_paths`). Read the DefId from the resolver and pass it when constructing each HIR entity:

```rust
// Example pattern — exact code depends on site context
let def_id = self.resolver.item_paths.get(&canonical_name).copied()
    .unwrap_or_else(|| self.def_id_gen.fresh_def_id());
self.resolver.item_paths.insert(canonical_name, def_id);
// Then when building HirStruct:
HirStruct {
    id: def_id,
    name: ...,
    ...
}
```

- [ ] **Step 2: Build and test**

Run: `cargo test -p rock-lib`
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add lib/src/collect/ lib/src/lower/
git commit -m "hir: wire real DefIds from resolver through the pipeline"
```

---

### Task 7: Full verification pass

**Files:**
- None; verification only.

- [ ] **Step 1: Run rock-lib tests**

Run: `cargo test -p rock-lib`
Expected: all tests pass.

- [ ] **Step 2: Run full workspace tests**

Run: `cargo test`
Expected: all crates pass (rock, rockc, rockup).

- [ ] **Step 3: Format**

Run: `cargo fmt --all`

- [ ] **Step 4: Commit**

```bash
git commit -m "hir: verify DefId wiring passes full test suite" --allow-empty
```
