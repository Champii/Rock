# Synthetic Array Owner Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `HirImpl.owner_path` with a structural owner enum that covers named impls and builtin slice-backed impls.

**Architecture:** Keep the change local to HIR ownership plumbing. Represent real item owners with `Named(String)` and slice-backed impls with `BuiltinSlice`, then thread that value through collection, lowering, artifact construction, and mono lookup code that currently reads or writes `owner_path`.

**Tech Stack:** Rust, serde, existing compiler pipeline.

---

### Task 1: Replace the HIR field and update constructors

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/collect/context.rs`

- [ ] **Step 1: Replace `owner_path` with `owner`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HirImplOwner {
    Named(String),
    BuiltinSlice,
}

pub struct HirImpl {
    pub owner: HirImplOwner,
    // ...
}
```

- [ ] **Step 2: Populate the new owner in HIR builders**

```rust
let owner = if matches!(imp.for_.as_ref(), Some(ast::ParseType::Slice(_) | ast::ParseType::Array { .. })) {
    HirImplOwner::BuiltinSlice
} else {
    HirImplOwner::Named(type_name.clone())
};
```

- [ ] **Step 3: Run a focused compile/test pass**

Run: `cargo test -p rock-lib collect::headers::tests -- --exact`
Expected: passes once direct `HirImpl` construction is updated.

### Task 2: Thread the new owner through lowering and artifact construction

**Files:**
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/crate_artifact/build.rs`

- [ ] **Step 1: Emit the same owner shape from lowering**

```rust
owner: if matches!(imp.for_.as_ref(), Some(ast::ParseType::Slice(_) | ast::ParseType::Array { .. })) {
    HirImplOwner::BuiltinSlice
} else {
    HirImplOwner::Named(self.canonical_owner_path(&type_name))
},
```

- [ ] **Step 2: Update copied impls in artifact construction**

```rust
imp.owner = lowered.owner.clone();
```

- [ ] **Step 3: Run focused mono-related tests**

Run: `cargo test -p rock-lib mono::external::tests::test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls -- --exact`
Expected: passes after constructor updates.

### Task 3: Remove leftover `owner_path` reads

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/registry.rs`

- [ ] **Step 1: Switch lookups to the enum**

```rust
match &imp.owner {
    HirImplOwner::Named(owner_path) => candidates.push(owner_path.clone()),
    HirImplOwner::BuiltinSlice => {}
}
```

- [ ] **Step 2: Re-run the focused tests and then `cargo test -p rock-lib`**

Run: `cargo test -p rock-lib`
Expected: green.
