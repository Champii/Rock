# Artifact Collection Indexed IDs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move artifact declaration collection off the legacy provisional-ID `CollectContext::collect_crate_declarations` path and onto the indexed `LocalCollector` ID environment.

**Architecture:** Build the item index and resolver-derived ID environment before collecting source-crate artifact declarations. Reuse `LocalCollector` so all current-crate items get canonical IDs directly from the index; remove the legacy context collector that allocated `CrateId(u32::MAX)` provisional IDs for artifact collection.

**Tech Stack:** Rust 2021, `rock-lib`, declaration collection in `lib/src/collect/mod.rs`, collector logic in `lib/src/collect/collector.rs`, context cleanup in `lib/src/collect/context.rs`, focused Cargo tests, `cargo fmt --all --check`, `git diff --check`.

---

## Files

- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/collect/context.rs`
- Update: `docs/superpowers/plans/master-audit-checklist.md`
- Update: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

## Task 1: Add RED Coverage For Artifact Function Signatures

- [x] **Step 1: Add a failing artifact collection test**

Add `collect_artifact_declarations_preserves_function_signatures_with_indexed_ids` near the artifact declaration tests in `lib/src/collect/mod.rs`:

```rust
let decls = collect_artifact_declarations(&crate_ctx, false, "dep")
    .expect("artifact declaration collection should preserve signatures")
    .declarations;
let signature = decls
    .function_sigs
    .get("dep::declared_only")
    .expect("qualified signature should be collected");

assert_eq!(signature.id, decls.item_index.defs_named("declared_only")[0]);
assert!(decls.current_def_ids.contains(&signature.id));
```

- [x] **Step 2: Run the focused test and confirm RED**

Run: `cargo test -p rock-lib collect_artifact_declarations_preserves_function_signatures_with_indexed_ids`
Expected: FAIL because the legacy artifact collector currently skips standalone `FunctionSig` items.

Observed: FAIL with `qualified signature should be collected` before implementation.

## Task 2: Route Artifact Collection Through `LocalCollector`

- [x] **Step 1: Add a crate-qualified collection entry point to `LocalCollector`**

Add `collect_crate_declarations(&mut self, module: &ast::Module, crate_name: &str, is_stdlib: bool)` in `lib/src/collect/collector.rs`. It should mirror the old artifact-specific root behavior, but use `item_id_for_name` / `next_impl_id` instead of provisional IDs:

```rust
pub(crate) fn collect_crate_declarations(
    &mut self,
    module: &ast::Module,
    crate_name: &str,
    is_stdlib: bool,
) {
    let root_exports = collect_exports(module);
    for top_level in &module.top_levels {
        match top_level {
            ast::TopLevel::FunctionSig(sig) => {
                let qualified_name = format!("{}::{}", crate_name, sig.name.name);
                self.collect_function_signature(sig, qualified_name);
            }
            ast::TopLevel::FunctionDecl(fd) => {
                let qualified_name = format!("{}::{}", crate_name, fd.name.name);
                self.collect_function_header(fd, qualified_name);
            }
            ast::TopLevel::Extern(sig) => {
                let qualified_name = format!("{}::{}", crate_name, sig.name.name);
                let id = self.item_id_for_name(&qualified_name);
                let hir_extern = headers::build_extern_with_id(&mut self.context, sig, qualified_name, id);
                self.context.externs.push(hir_extern);
            }
            ast::TopLevel::StructDecl(sd) => {
                let qualified_name = format!("{}::{}", crate_name, sd.name.name);
                let id = self.item_id_for_name(&qualified_name);
                let strukt = headers::build_struct_with_id(&mut self.context, sd, id);
                self.context.structs.insert(sd.name.name.clone(), strukt.clone());
                self.context.structs.insert(qualified_name, strukt);
            }
            ast::TopLevel::EnumDecl(ed) => {
                let qualified_name = format!("{}::{}", crate_name, ed.name.name);
                let id = self.item_id_for_name(&qualified_name);
                let enum_ = headers::build_enum_with_id(&mut self.context, ed, id);
                self.context.enums.insert(ed.name.name.clone(), enum_.clone());
                self.context.enums.insert(qualified_name, enum_);
            }
            ast::TopLevel::TraitDecl(td) => {
                let qualified_name = format!("{}::{}", crate_name, td.name.name);
                let id = self.item_id_for_name(&qualified_name);
                let hir_trait = headers::build_trait_with_id(&mut self.context, td, id);
                self.context.traits.insert(td.name.name.clone(), hir_trait.clone());
                self.context.traits.insert(qualified_name, hir_trait);
            }
            ast::TopLevel::Impl(imp) => {
                let id = self.next_impl_id(Some(crate_name));
                let hir_impl = headers::build_impl_with_id(&mut self.context, imp, id);
                self.context.impls.push(hir_impl);
            }
            ast::TopLevel::Module(module_decl) => { /* delegate to collect_source_backed_qualified_declarations with crate-qualified prefix */ }
            ast::TopLevel::Mod(ident, _) => { /* preserve export/prelude filtering and delegate to handle_source_backed_mod_decl_in_context */ }
            ast::TopLevel::InfixOperator(precedence, name) => { self.context.infix_precedence.insert(name.clone(), *precedence); }
            _ => {}
        }
    }
}
```

- [x] **Step 2: Build the item index before artifact collection**

In `collect_artifact_declarations`, build `IndexingIds`, `source_modules`, `item_index`, preliminary resolver, and `CollectedIdEnvironment` before collecting the current source crate.

- [x] **Step 3: Replace legacy context collection**

Replace `context.collect_crate_declarations(...)` with:

```rust
let mut collector = collector::LocalCollector::new_with_id_environment(context, id_environment);
collector.collect_crate_declarations(module, current_crate_name, current_crate_name == "stdlib");
let context::LocalCollection { ... } = collector.finish();
```

- [x] **Step 4: Remove fresh repair IDs from artifact current IDs**

Remove the `assign_fresh_ids_to_unresolved_named_items` call from `collect_artifact_declarations`; current IDs should be item-index IDs plus generated method IDs only.

- [x] **Step 5: Delete the legacy context collector**

Remove `CollectContext::collect_crate_declarations` and `CollectContext::collect_crate_declarations_qualified_with_references` from `lib/src/collect/context.rs`, and remove any now-unused `headers` import.

## Task 3: Verify Behavior And Docs

- [x] **Step 1: Run focused artifact tests**

Run: `cargo test -p rock-lib collect_artifact_declarations`
Expected: PASS.

Observed: PASS, 7 artifact declaration tests, including the inline-module glob export and resolver-alias regression added from review feedback.

- [x] **Step 2: Run provisional evidence search**

Run: use `Grep` for `fresh_provisional_def_id` in `lib/src/collect`.
Expected: no `CollectContext::collect_crate_declarations` provisional allocation calls remain; remaining uses are header/collector temporary-ID fallback paths.

Observed: no legacy context collector or unresolved fresh-ID repair remains; remaining `fresh_provisional_def_id` matches are `CollectContext::fresh_provisional_def_id`, `LocalCollector` missing-ID fallbacks, and header builder method fallback paths.

- [x] **Step 3: Update roadmap and master checklist**

Update `docs/superpowers/plans/master-audit-checklist.md` and `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md` to record that artifact declaration collection now consumes indexed IDs directly and no longer uses the context provisional-ID collector.

- [x] **Step 4: Run formatting and diff checks**

Run: `cargo fmt --all --check`
Expected: exit 0.

Run: `git diff --check`
Expected: exit 0.

Observed: `cargo fmt --all --check` and `git diff --check` both exited 0 after formatting.

- [x] **Step 5: Request focused code review**

Review the diff for artifact declaration regressions, missing root qualified names, export/prelude behavior, and remaining provisional-ID leaks before commit.

Observed: initial review found inline-module glob export and resolver-alias gaps; both were fixed with RED/GREEN coverage. Final focused review reported no findings.

- [ ] **Step 6: Commit verified slice**

Commit message: `index artifact declaration collection`.
