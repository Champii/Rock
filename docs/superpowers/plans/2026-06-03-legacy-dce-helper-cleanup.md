# Legacy DCE Helper Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the legacy non-pipeline HIR/name-based DCE helpers so `prune_unreachable_instances` is the only DCE implementation in `lib/src/dce.rs`.

**Architecture:** Delete the unused HIR/name pruning functions and tests that exist only for those functions. Keep the active instance reachability implementation and tests intact, then update audit/roadmap docs to mark only this cleanup complete.

**Tech Stack:** Rust 2021, `rock-lib`, `lib/src/dce.rs`, Cargo tests, `cargo fmt --all --check`, `git diff --check`.

---

## File Structure

- Modify `lib/src/dce.rs`
  - Remove module wording that says older HIR/name helpers remain.
  - Delete `prune_dead_functions`, `count_dead_functions`, and the legacy `collect_calls_*` traversal helpers.
  - Remove the two tests that only exercise those helpers.
  - Keep `prune_unreachable_instances` and its tests unchanged except for import cleanup.
- Modify `docs/superpowers/plans/master-audit-checklist.md`
  - Move the legacy non-pipeline HIR/name pruning helper item to Done.
  - Leave instance body representation cleanup open.
- Modify `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`
  - Record the legacy DCE helper cleanup without claiming full Monomorphization Instances completion.

## Task 1: Remove Legacy HIR/Name DCE Helpers

**Files:**
- Modify: `lib/src/dce.rs`

- [ ] **Step 1: Verify legacy helper references are local only**

Run:

```bash
rg "prune_dead_functions|count_dead_functions|collect_calls_block|collect_calls_stmt|collect_calls_expr" lib/src
```

Expected before cleanup: matches only in `lib/src/dce.rs`.

- [ ] **Step 2: Update the module comment**

In `lib/src/dce.rs`, replace:

```rust
//! The compiler DCE authority is instance reachability over monomorphized
//! callable records. The older HIR/name helpers remain for legacy coverage.
```

with:

```rust
//! The compiler DCE authority is instance reachability over monomorphized
//! callable records.
```

- [ ] **Step 3: Remove legacy helper imports from tests**

In `lib/src/dce.rs`, inside `#[cfg(test)] mod tests`, replace:

```rust
    use super::{count_dead_functions, prune_dead_functions, prune_unreachable_instances};
```

with:

```rust
    use super::prune_unreachable_instances;
```

- [ ] **Step 4: Delete legacy helper implementation**

In `lib/src/dce.rs`, delete the full contiguous legacy block starting at:

```rust
/// Legacy HIR/name-based function pruning retained for focused compatibility tests.
/// The compiler pipeline uses `prune_unreachable_instances` after monomorphization.
pub fn prune_dead_functions(program: &mut HirProgram) {
```

and ending after the closing brace of this function:

```rust
fn collect_calls_expr(expr: &HirExpr, all: &HashSet<String>, out: &mut Vec<String>) {
}
```

The deleted block must include:

- `prune_dead_functions`
- `count_dead_functions`
- `collect_calls_block`
- `collect_calls_stmt`
- `collect_calls_expr`

- [ ] **Step 5: Delete legacy helper-only tests**

In `lib/src/dce.rs`, delete these two complete tests near the end of the file:

```rust
    #[test]
    fn dead_code_elimination_counts_alias_duplicate_once_by_def_id() {
    }
```

and:

```rust
    #[test]
    fn resolved_var_name_does_not_mark_function_reachable() {
    }
```

- [ ] **Step 6: Verify no legacy helper references remain**

Run:

```bash
rg "prune_dead_functions|count_dead_functions|collect_calls_block|collect_calls_stmt|collect_calls_expr" lib/src
```

Expected after cleanup: no matches.

- [ ] **Step 7: Run focused DCE tests**

Run:

```bash
cargo test -p rock-lib dce::tests -- --nocapture
```

Expected: all remaining DCE tests pass.

- [ ] **Step 8: Run formatting and diff checks**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: both pass.

- [ ] **Step 9: Commit Task 1**

Run:

```bash
git add lib/src/dce.rs
git commit -m "remove legacy hir name dce helpers"
```

Expected: commit succeeds.

## Task 2: Update Audit And Roadmap Docs

**Files:**
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [ ] **Step 1: Update master audit checklist Done list**

In `docs/superpowers/plans/master-audit-checklist.md`, under `## 7. Monomorphization Instances`, add this Done item after the string-keyed mono semantic lookup item:

```markdown
- [x] Removed the legacy non-pipeline HIR/name DCE helpers (`prune_dead_functions`, `count_dead_functions`, and their name-based call traversal); instance reachability remains the active DCE authority.
```

- [ ] **Step 2: Remove the matching Still to do item**

In the same section, remove this unchecked item from `Still to do`:

```markdown
- [ ] Retire legacy non-pipeline HIR/name pruning helpers once their compatibility tests are no longer needed.
```

Keep this item unchecked:

```markdown
- [ ] Carry instance bodies toward the eventual MIR/codegen boundary instead of storing instance bodies as `HirFunction` records long term.
```

- [ ] **Step 3: Update the audit summary row**

In the top summary table row for `Monomorphization Instances`, replace the remaining-work wording so it no longer says legacy non-pipeline HIR/name pruning helpers remain. Use wording like:

Use this remaining-work wording inside the row:

```markdown
string-keyed mono semantic lookup cleanup is complete where canonical IDs are available; legacy non-pipeline HIR/name DCE helpers are removed; remaining work is instance body representation and MIR/codegen metadata cleanup
```

- [ ] **Step 4: Update ordered roadmap Task 16 row**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update the Task 16 table row from wording like:

```markdown
Legacy HIR/name pruning helpers remain only as non-pipeline compatibility tests;
```

to wording like:

```markdown
Legacy HIR/name pruning helpers have been removed; direct trait/member backend metadata cleanup is complete, while broader HIR-body instance payload and MIR/codegen metadata cleanup remains future work
```

- [ ] **Step 5: Add a Task 16 verification note**

In the Task 16 section after the existing `2026-05-25 verification note`, add:

```markdown
**2026-06-03 verification note:** Removed the legacy non-pipeline HIR/name DCE helpers and helper-only tests. `prune_unreachable_instances` remains the DCE authority for the compile pipeline, and instance body representation cleanup remains future work.
```

- [ ] **Step 6: Verify docs do not overclaim completion**

Run:

```bash
rg "legacy non-pipeline HIR/name pruning helpers remain|Retire legacy non-pipeline|instance body representation cleanup is complete|Monomorphization Instances.*Complete" docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
```

Expected: no stale claim that legacy DCE helpers remain, no claim that instance body representation cleanup is complete, and no broad Monomorphization Instances completion claim.

- [ ] **Step 7: Verify documentation diff**

Run:

```bash
git diff -- docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git diff --check
```

Expected: docs accurately mark only legacy DCE helper cleanup complete.

- [ ] **Step 8: Commit Task 2**

Run:

```bash
git add docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git commit -m "update audit for legacy dce cleanup"
```

Expected: commit succeeds.

## Task 3: Final Verification

**Files:**
- All modified files.

- [ ] **Step 1: Run focused DCE suite**

Run:

```bash
cargo test -p rock-lib dce::tests -- --nocapture
```

Expected: all remaining DCE tests pass.

- [ ] **Step 2: Verify legacy helpers are gone**

Run:

```bash
rg "prune_dead_functions|count_dead_functions|collect_calls_block|collect_calls_stmt|collect_calls_expr" lib/src
```

Expected: no matches.

- [ ] **Step 3: Run formatting check**

Run:

```bash
cargo fmt --all --check
```

Expected: pass.

- [ ] **Step 4: Run full library tests**

Run:

```bash
cargo test -p rock-lib > /tmp/rock-lib-legacy-dce-helper-cleanup.log 2>&1
```

Expected: exit code 0. If it fails, inspect `/tmp/rock-lib-legacy-dce-helper-cleanup.log`, fix the smallest failing case first, then rerun the full command.

- [ ] **Step 5: Run final diff checks**

Run:

```bash
git diff --check
git status --short --branch
```

Expected: no whitespace errors; only unrelated user files may remain outside this task.
