# Task 6 ID-Keyed Declarations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make declaration payloads ID-keyed from collection through partial HIR, delete lower-phase candidate-name identity recovery, and prove Step 6 complete with a blocking full-codebase closure analysis.

**Architecture:** Collection may use names while resolving source, but its `Declarations` output owns one `DeclarationItems` payload per `DefId` plus `ItemIndex` source correspondence and resolver/display tables. Lowering consumes and mutates those ID-keyed items, carries indexed owner IDs into body lowering, and emits ID-keyed `PartialHir`; inference preserves the keys when constructing final HIR.

**Tech Stack:** Rust 2021, `rock-lib`, co-located unit tests, integration tests, CodeGraph-assisted call-path analysis, Cargo test/Clippy/rustfmt gates.

**VCS Constraint:** Do not stage, commit, amend, push, or otherwise mutate Git state unless the user explicitly requests it. The usual per-task commit steps are intentionally replaced by diff checkpoints.

---

## File Structure

- Create `lib/src/collect/declarations.rs`: canonical ID-keyed declaration payload store and checked conversion from collection-local named maps.
- Modify `lib/src/collect/mod.rs`: export the store, change `Declarations`, construct the store in current-source and artifact collection, and test the phase boundary.
- Modify `lib/src/collect/context.rs`: keep collection-local name maps private to collection and remove `methods` from the extracted production boundary.
- Modify `lib/src/collect/collector.rs`: pass collection output through checked ID-keying and preserve indexed source-owner identity.
- Modify `lib/src/collect/item_index.rs`: expose exact module/item source correspondence needed by body lowering.
- Modify `lib/src/lower/items.rs`: make `LowerItems` a single ID-keyed mutable authority.
- Modify `lib/src/lower/services.rs`: expose ID-keyed item accessors through `LowerItemService`.
- Modify `lib/src/lower/mod.rs`: consume `DeclarationItems`, retain `ItemIndex`, build source scopes from resolver names plus IDs, and delete `def_id_for_name`.
- Modify `lib/src/lower/pipeline.rs`: emit ID-keyed `PartialHir` without index rebuilding.
- Modify `lib/src/lower/program.rs`: carry exact indexed owner IDs through root, inline, loaded, and dependency body traversal.
- Modify `lib/src/lower/function.rs`: lower and mutate function bodies by explicit `DefId`.
- Modify `lib/src/lower/collect/declarations.rs`: require explicit collected IDs for declaration/header operations.
- Modify `lib/src/lower/collect/types.rs`: require explicit collected IDs for struct and enum operations.
- Modify `lib/src/lower/collect/traits.rs`: require explicit trait/impl/member IDs and remove global method-name payload registration.
- Modify `lib/src/lower/traits/defaults.rs`: lower trait defaults by owner/member IDs.
- Modify `lib/src/lower/control_flow/secondary.rs`: remove the enum-name fallback to `def_id_for_name`.
- Modify `lib/src/lower/body_lowerer.rs`: preserve source module identity while dispatching body lowering.
- Modify `lib/src/lower/module_context.rs`: map loaded and inline modules to collection `ModuleId` records.
- Modify `lib/src/lower/prelude.rs`: resolve aliases to IDs before item access.
- Modify `lib/src/infer/mod.rs`: make `PartialHir` ID-keyed and construct final HIR without name-keyed deduplication.
- Modify `lib/src/infer/finalize.rs`: finalize ID-keyed values in place.
- Modify `lib/src/infer/generalize.rs`: generalize ID-keyed function values.
- Modify `lib/src/infer/authority.rs`: materialize authorities through ID-keyed items.
- Modify `lib/src/infer/solve.rs`: consume impl/nominal values from ID-keyed maps.
- Modify `lib/src/hir/mod.rs`: expose the existing ID-keyed constructor needed by inference and keep names in `HirNameTables`.
- Modify affected fixture sites under `lib/src/crate_artifact/`, `lib/src/crate_system/`, `lib/src/products.rs`, `lib/src/mono/`, `lib/src/mir/`, and `lib/tests/integration.rs` only where constructors or explicit phase-boundary fixtures change.
- Modify `CLEAN_SLATE_COMPILER_AUDIT.md`: update Step 6 only after all implementation, verification, and closure-analysis tasks pass.

## Task 1: Add The Checked ID-Keyed Collection Boundary

**Files:**
- Create: `lib/src/collect/declarations.rs`
- Modify: `lib/src/collect/mod.rs:10-75, 1371-1748`
- Modify: `lib/src/collect/context.rs:307-360`
- Modify: `lib/src/collect/collector.rs:1234-1265`
- Test: `lib/src/collect/mod.rs` co-located tests

- [ ] **Step 1: Write RED tests for canonical ID-keyed output**

Add tests that collect a root function and a qualified/aliased dependency declaration, then assert payload access is by `DefId` and duplicate aliases do not create duplicate payloads:

```rust
#[test]
fn collected_declaration_payloads_are_canonical_by_def_id() {
    let program = crate::parser::parse_string(
        "answer: I64\nanswer = -> 42\n",
        &crate::Config::default(),
    )
    .unwrap();
    let decls = collect(
        &program,
        &CrateContext::new(),
        false,
        Some("demo"),
    )
    .unwrap();
    let answer_id = decls.resolver.item_paths["demo::answer"];

    assert_eq!(decls.items.functions.len(), 1);
    assert_eq!(decls.items.functions[&answer_id].id, answer_id);
}

#[test]
fn declaration_items_reject_key_payload_id_mismatch() {
    let key = DefId::new(CrateId(0), LocalDefId(1));
    let payload_id = DefId::new(CrateId(0), LocalDefId(2));
    let errors = DeclarationItems::from_id_maps(
        HashMap::from([(key, test_function(payload_id, "answer"))]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    )
    .unwrap_err();

    assert!(errors[0].message.contains("function declaration key"));
}
```

- [ ] **Step 2: Run the tests and verify RED**

Run:

```bash
cargo test -p rock-lib collected_declaration_payloads_are_canonical_by_def_id -- --exact --nocapture
cargo test -p rock-lib declaration_items_reject_key_payload_id_mismatch -- --exact --nocapture
```

Expected: compilation fails because `Declarations::items`, `DeclarationItems`, and `from_id_maps` do not exist.

- [ ] **Step 3: Define `DeclarationItems` as the single collection output authority**

Create `lib/src/collect/declarations.rs` with the canonical store and checked constructor:

```rust
use std::collections::HashMap;

use crate::hir::{
    HirEnum, HirExtern, HirFunction, HirFunctionSig, HirImpl, HirStruct, HirTrait,
};
use crate::ids::DefId;
use crate::lower::ResolveError;

#[derive(Debug, Default)]
pub struct DeclarationItems {
    pub functions: HashMap<DefId, HirFunction>,
    pub function_sigs: HashMap<DefId, HirFunctionSig>,
    pub structs: HashMap<DefId, HirStruct>,
    pub enums: HashMap<DefId, HirEnum>,
    pub traits: HashMap<DefId, HirTrait>,
    pub impls: HashMap<DefId, HirImpl>,
    pub externs: HashMap<DefId, HirExtern>,
}

impl DeclarationItems {
    pub fn from_id_maps(
        functions: HashMap<DefId, HirFunction>,
        function_sigs: HashMap<DefId, HirFunctionSig>,
        structs: HashMap<DefId, HirStruct>,
        enums: HashMap<DefId, HirEnum>,
        traits: HashMap<DefId, HirTrait>,
        impls: HashMap<DefId, HirImpl>,
        externs: HashMap<DefId, HirExtern>,
    ) -> Result<Self, Vec<ResolveError>> {
        let items = Self {
            functions,
            function_sigs,
            structs,
            enums,
            traits,
            impls,
            externs,
        };
        items.validate_key_ids()?;
        Ok(items)
    }
}
```

Implement `validate_key_ids` with this shared check so every category reports all mismatches instead of panicking:

```rust
fn validate_ids<T>(
    kind: &str,
    entries: &HashMap<DefId, T>,
    id_of: impl Fn(&T) -> DefId,
    errors: &mut Vec<ResolveError>,
) {
    for (key, item) in entries {
        let payload_id = id_of(item);
        if *key != payload_id {
            errors.push(ResolveError::new(format!(
                "{kind} declaration key {key:?} does not match payload id {payload_id:?}",
            )));
        }
    }
}

pub fn validate_key_ids(&self) -> Result<(), Vec<ResolveError>> {
    let mut errors = Vec::new();
    validate_ids("function", &self.functions, |item| item.id, &mut errors);
    validate_ids("function signature", &self.function_sigs, |item| item.id, &mut errors);
    validate_ids("struct", &self.structs, |item| item.id, &mut errors);
    validate_ids("enum", &self.enums, |item| item.id, &mut errors);
    validate_ids("trait", &self.traits, |item| item.id, &mut errors);
    validate_ids("impl", &self.impls, |item| item.id, &mut errors);
    validate_ids("extern", &self.externs, |item| item.id, &mut errors);
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

- [ ] **Step 4: Move name-map collapse from inference to collection**

Move and generalize the existing `move_id_keyed_named_declarations` logic from `lib/src/infer/mod.rs:223-295` into collection-boundary helpers. The collection helper must:

```rust
fn collapse_named_declarations<T>(
    entries: HashMap<String, T>,
    canonical_names_by_id: &HashMap<DefId, String>,
    kind: &str,
    id_of: impl Fn(&T) -> DefId,
    same_definition: impl Fn(&T, &T) -> bool,
) -> Result<HashMap<DefId, T>, Vec<ResolveError>>
where
    T: Clone + std::fmt::Debug,
```

Select the resolver canonical entry when aliases share an ID, reject conflicting payload clones with a structured error, and return exactly one value per ID. Add equivalent direct conversions for impl and extern vectors, rejecting duplicate IDs.

- [ ] **Step 5: Change `Declarations` to contain `items: DeclarationItems`**

Replace the seven payload fields and the standalone `methods` map with:

```rust
pub struct Declarations {
    pub indexing_ids: IndexingIds,
    pub item_index: ItemIndex,
    pub resolver: ResolverTables,
    pub current_def_ids: BTreeSet<DefId>,
    pub items: DeclarationItems,
    pub function_type_vars: HashMap<DefId, HashSet<TypeVarId>>,
    pub infix_precedence: HashMap<String, u8>,
    pub loaded_module_paths: Vec<(String, PathBuf)>,
    pub type_vars: DeclarationTypeVars,
    pub inject_prelude: bool,
    pub loaded_prelude_export_ids: HashMap<String, ArtifactExport>,
    pub module_file_cache: HashMap<PathBuf, ast::Module>,
    pub source_modules: SourceModuleSet,
    pub dependency_root_export_ids: HashMap<String, HashMap<String, ArtifactExport>>,
}
```

Construct `DeclarationItems` after canonical ID remapping in both `collect_impl` and `collect_artifact_declarations`. Do not expose the collection-local named maps through `LocalCollection` after this boundary.

- [ ] **Step 6: Delete the standalone method payload map from the production boundary**

Remove `Declarations.methods` and `LowerItems.methods` inputs. Standalone impl methods remain inside `HirImpl.methods`; trait methods and signatures remain inside `HirTrait`. Keep collection-local method maps only while existing collection algorithms require source-name resolution, then discard them before constructing `Declarations`.

- [ ] **Step 7: Run focused collection tests**

Run:

```bash
cargo test -p rock-lib collect:: -- --nocapture
```

Expected: all collection tests pass, including new canonical-ID and mismatch tests.

- [ ] **Step 8: Record a diff checkpoint**

Run `git diff --check`. Inspect `git diff -- lib/src/collect` and confirm no non-collection consumer has been given access to collection-local name-keyed payload maps.

## Task 2: Preserve Exact Source Owner Identity In `ItemIndex`

**Files:**
- Modify: `lib/src/collect/item_index.rs:35-210`
- Modify: `lib/src/collect/mod.rs:351-470`
- Modify: `lib/src/lower/module_context.rs`
- Test: `lib/src/collect/item_index.rs` co-located tests

- [ ] **Step 1: Write RED source-correspondence tests**

Add tests proving that two same-named declarations in different modules and an impl item are recoverable by module ID plus source ordinal, without probing names:

```rust
#[test]
fn item_index_resolves_body_owner_by_module_and_source_ordinal() {
    let mut ids = IndexingIds::new_root();
    let module = module_with(vec![function_item("answer"), struct_item("Box")]);
    let index = index_root_module_items(&mut ids, &module);

    let answer = index
        .item_at_source(ids.root_module_id(), 0)
        .expect("first top-level item should be indexed");
    let structure = index
        .item_at_source(ids.root_module_id(), 1)
        .expect("second top-level item should be indexed");

    assert_eq!(answer.name, "answer");
    assert_eq!(structure.name, "Box");
    assert_ne!(answer.def_id, structure.def_id);
}
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test -p rock-lib collect::item_index::tests::item_index_resolves_body_owner_by_module_and_source_ordinal -- --exact --nocapture
```

Expected: compilation fails because `ItemRecord` has no source ordinal and `item_at_source` does not exist.

- [ ] **Step 3: Add structural source identity to item records**

Extend `ItemRecord` and `ItemIndex`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemSourceId {
    pub module_id: ModuleId,
    pub top_level_index: u32,
}

pub struct ItemRecord {
    pub def_id: DefId,
    pub module_id: ModuleId,
    pub source: ItemSourceId,
    pub name: String,
    pub kind: ItemKind,
}

pub fn item_at_source(
    &self,
    module_id: ModuleId,
    top_level_index: usize,
) -> Option<&ItemRecord>;

impl ItemKind {
    pub(crate) fn matches_top_level(self, top_level: &ast::TopLevel) -> bool {
        item_name_and_kind(top_level).is_some_and(|(_, kind)| kind == self)
    }
}
```

Populate `top_level_index` from `module.top_levels.iter().enumerate()` in root, inline, and source-backed indexing. Keep index records for non-declaration syntax absent; `item_at_source` must therefore key the original AST ordinal rather than the compact item vector position.

- [ ] **Step 4: Expose module identity without semantic name fallback**

Add this `ItemIndex` API, implemented by walking `ModuleRecord.parent` and exact path segments:

```rust
pub fn module_id_by_path(&self, path: &[String]) -> Option<ModuleId>;
```

An empty path returns the root module. `LowerModuleService` stores the resulting `crate::ids::ModuleId` beside each loaded AST. Failure to map one exact module path produces a lowering invariant diagnostic; it must not retry short or crate-qualified alternatives.

- [ ] **Step 5: Preserve member IDs under canonical owner IDs**

Keep `CollectedIdEnvironment::trait_member_ids_by_owner` and `impl_method_ids_by_owner` collection-private. Ensure the IDs are embedded into `HirTrait` and `HirImpl` members before `DeclarationItems` is constructed. Lowering may then select an owner by `ItemSourceId` and a member inside that canonical owner; it must not receive the collection name maps.

- [ ] **Step 6: Run item-index and collection tests**

Run:

```bash
cargo test -p rock-lib collect::item_index -- --nocapture
cargo test -p rock-lib collect:: -- --nocapture
```

Expected: all tests pass.

## Task 3: Replace `LowerItems` With One ID-Keyed Mutable Store

**Files:**
- Modify: `lib/src/lower/items.rs:1-132`
- Modify: `lib/src/lower/services.rs`
- Modify: `lib/src/lower/mod.rs:52-81, 505-640`
- Modify: `lib/src/lower/prelude.rs`
- Test: `lib/src/lower/items.rs` co-located tests

- [ ] **Step 1: Replace the misleading existing test with RED authority tests**

Replace `lowerer_items_are_keyed_by_declaration_ids_not_map_names`, which currently passes despite name-keyed primary storage, with:

```rust
#[test]
fn lower_items_mutate_only_the_canonical_def_id_payload() {
    let id = DefId::new(CrateId(0), LocalDefId(1));
    let mut items = LowerItems::from_declarations(DeclarationItems::from_id_maps(
        HashMap::from([(id, test_function(id, "answer"))]),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
    ).unwrap());

    items.function_mut(id).unwrap().body.ty = Type::Bool;

    assert_eq!(items.function(id).unwrap().body.ty, Type::Bool);
    assert_eq!(items.functions().count(), 1);
}
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cargo test -p rock-lib lower::items::tests::lower_items_mutate_only_the_canonical_def_id_payload -- --exact --nocapture
```

Expected: compilation fails because `from_declarations`, `function_mut`, and iterator-based `functions` do not exist.

- [ ] **Step 3: Rewrite `LowerItems` around `DeclarationItems`**

Use one canonical copy of each payload:

```rust
#[derive(Debug, Default)]
pub(crate) struct LowerItems {
    declarations: DeclarationItems,
    pub(crate) function_sig_unsafe: HashMap<DefId, bool>,
}

impl LowerItems {
    pub(crate) fn from_declarations(declarations: DeclarationItems) -> Self;
    pub(crate) fn functions(&self) -> impl Iterator<Item = (&DefId, &HirFunction)>;
    pub(crate) fn impls(&self) -> impl Iterator<Item = (&DefId, &HirImpl)>;
    pub(crate) fn function(&self, id: DefId) -> Option<&HirFunction>;
    pub(crate) fn function_mut(&mut self, id: DefId) -> Option<&mut HirFunction>;
    pub(crate) fn structure(&self, id: DefId) -> Option<&HirStruct>;
    pub(crate) fn enumeration(&self, id: DefId) -> Option<&HirEnum>;
    pub(crate) fn trait_def(&self, id: DefId) -> Option<&HirTrait>;
    pub(crate) fn impl_def(&self, id: DefId) -> Option<&HirImpl>;
    pub(crate) fn impl_def_mut(&mut self, id: DefId) -> Option<&mut HirImpl>;
    pub(crate) fn extern_def(&self, id: DefId) -> Option<&HirExtern>;
    pub(crate) fn into_declarations(self) -> DeclarationItems;
}
```

Delete all name-keyed payload fields, `*_by_id` clones, fallback scans over values for an embedded ID, `from_maps`, and `rebuild_id_indexes`.

- [ ] **Step 4: Build source scopes by joining resolver names to IDs**

In `Lowerer::from_declarations`, iterate resolver source/canonical names, fetch the payload by ID, and define lexical aliases without using a payload name as the semantic key:

```rust
for (source_name, id) in &resolver.item_paths {
    if let Some(function) = declarations.functions.get(id) {
        let params = function.params.iter().map(|param| param.ty.clone()).collect();
        scope.define(
            source_name.clone(),
            Type::function_with_safety(
                params,
                function.ret_type.clone(),
                FunctionSafety::from_is_unsafe(function.is_unsafe),
            ),
            false,
        );
    } else if let Some(enumeration) = declarations.enums.get(id) {
        scope.define(
            source_name.clone(),
            Type::Enum { id: enumeration.id, args: Vec::new() },
            false,
        );
    } else if let Some(extern_def) = declarations.externs.get(id) {
        scope.define(
            source_name.clone(),
            Type::function_with_safety(
                extern_def.params.clone(),
                extern_def.ret.clone(),
                FunctionSafety::from_is_unsafe(extern_def.is_unsafe),
            ),
            false,
        );
    }
}
```

Keep names in `Scope` because they represent source lexical bindings. Payload access inside this loop is ID-keyed.

- [ ] **Step 5: Retain `ItemIndex` in `Lowerer`**

Add `pub(crate) item_index: ItemIndex` to `Lowerer` and initialize it from `Declarations.item_index`. This is the authority used by module/body traversal in Task 4; do not add a second wrapper or copied index.

- [ ] **Step 6: Convert prelude and import handling to resolve ID before payload access**

Any prelude/import code that indexes `LowerItems` by canonical string must instead use `ResolverTables`/prelude export IDs once, then call the ID accessor. Do not add a helper that scans payload values by name.

- [ ] **Step 7: Run lower item/service tests**

Run:

```bash
cargo test -p rock-lib lower::items -- --nocapture
cargo test -p rock-lib lower::services -- --nocapture
cargo test -p rock-lib lower::prelude -- --nocapture
```

Expected: all tests pass.

## Task 4: Carry Collected IDs Through Every Body-Lowering Path

**Files:**
- Modify: `lib/src/lower/body_lowerer.rs`
- Modify: `lib/src/lower/program.rs`
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/collect/declarations.rs`
- Modify: `lib/src/lower/collect/types.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/traits/defaults.rs`
- Modify: `lib/src/lower/control_flow/secondary.rs`
- Modify: `lib/src/lower/mod.rs:234-298`
- Test: co-located tests in the files above
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Add RED same-name and alias body-mutation coverage**

Add a source-backed module test with same-named functions in distinct modules. Assert each collected `DefId` receives its own body and that changing resolver/display aliases cannot redirect body attachment:

```rust
#[test]
fn body_lowering_attaches_same_named_module_functions_by_indexed_owner() {
    let temp_dir = std::env::temp_dir().join(format!(
        "rock_task6_body_owner_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let entry = temp_dir.join("main.rk");
    let left = temp_dir.join("left.rk");
    let right = temp_dir.join("right.rk");
    std::fs::write(
        &entry,
        "mod left\nmod right\nmain: I64\nmain = -> left::answer! + right::answer!\n",
    )
    .unwrap();
    std::fs::write(&left, "answer: I64\nanswer = -> 1\n< answer\n").unwrap();
    std::fs::write(&right, "answer: I64\nanswer = -> 2\n< answer\n").unwrap();

    let config = crate::Config {
        entry_file: entry,
        no_std: true,
        no_prelude: true,
        current_crate_name: Some("demo".to_string()),
        ..crate::Config::default()
    };
    let mut db = crate::source_loader::SourceDatabase::new();
    let graph = db.load_entry(config.entry_file.clone(), &config).unwrap();
    let program = crate::ast::Program {
        module: graph.root_module().clone(),
    };
    let declarations = crate::collect::collect_with_source_graph(
        &program,
        &graph,
        &CrateContext::new(),
        false,
        Some("demo"),
    )
    .unwrap();
    let partial = crate::lower::program::lower_from_declarations(
        &program,
        declarations,
        &CrateContext::new(),
        Some("demo"),
    )
    .unwrap();
    let left_id = partial.resolver.item_paths["demo::left::answer"];
    let right_id = partial.resolver.item_paths["demo::right::answer"];

    let left_body = format!("{:?}", partial.functions[&left_id].body);
    let right_body = format!("{:?}", partial.functions[&right_id].body);

    assert_ne!(left_id, right_id);
    assert!(left_body.contains('1'));
    assert!(right_body.contains('2'));
    assert_ne!(left_body, right_body);

    std::fs::remove_dir_all(temp_dir).unwrap();
}
```

Update `lower::pipeline::tests::lowering_pipeline_lowers_root_trait_defaults_with_glob_imports` to fetch the trait by resolver ID and assert its lowered method keeps the collected member ID:

```rust
let trait_id = lowered.resolver.item_paths["demo::HasAnswer"];
let trait_def = &lowered.traits[&trait_id];
let method = &trait_def.methods["value"];
assert!(lowered.current_def_ids.contains(&method.id));
assert!(!method.body.stmts.is_empty());
```

Replace `collect_trait_impl_does_not_register_method_in_name_map` with an ID-authority assertion after the method side map is deleted:

```rust
let (_, imp) = lowerer.items.impls().next().expect("impl should be collected");
let method = &imp.methods["show"];
assert!(lowerer.current_def_ids.contains(&method.id));
```

- [ ] **Step 2: Run the new tests and verify RED**

Run the fully qualified new test paths with `--exact --nocapture`.

Expected: compilation fails while `PartialHir` is not ID-keyed, or the test exposes current name-keyed body mutation.

- [ ] **Step 3: Make module traversal yield `(ItemSourceId, DefId, &TopLevel)`**

Change root, inline, and loaded-module traversal to enumerate original AST top levels and query `ItemIndex::item_at_source`. Pass the resulting record into the relevant lowering operation:

```rust
for (top_level_index, top_level) in module.top_levels.iter().enumerate() {
    let record = self
        .item_index
        .item_at_source(module_id, top_level_index)
        .filter(|record| record.kind.matches(top_level));
    self.lower_top_level_body(record, top_level);
}
```

Imports, exports, operators, and module declarations that have no declaration payload continue through syntax-specific handling without manufacturing a `DefId`.

- [ ] **Step 4: Add explicit-ID body APIs**

Replace declaration functions that recover IDs from names with signatures that require the owner:

```rust
fn lower_function_body(&mut self, id: DefId, declaration: &ast::FunctionDecl);
fn lower_struct_decl(&mut self, id: DefId, declaration: &ast::StructDecl);
fn lower_enum_decl(&mut self, id: DefId, declaration: &ast::EnumDecl);
fn lower_trait_decl(&mut self, id: DefId, declaration: &ast::TraitDecl);
fn lower_impl_decl(&mut self, id: DefId, declaration: &ast::Impl);
```

For trait and impl members, fetch the canonical owner by ID, read the member's embedded `DefId` under that owner-local source name, and pass that member ID into body lowering. Do not resolve a global candidate path.

- [ ] **Step 5: Convert dependency and artifact body paths**

When lowering dependency source bodies, derive the top-level owner from the dependency interface/resolver's exact canonical ID before dispatch. Do not probe unqualified, qualified, and crate-qualified alternatives. Object-only artifacts remain bodyless and continue using their interface IDs.

- [ ] **Step 6: Remove every production `def_id_for_name` call**

Replace the calls currently in:

```text
lib/src/lower/function.rs
lib/src/lower/control_flow/secondary.rs
lib/src/lower/collect/traits.rs
lib/src/lower/collect/declarations.rs
lib/src/lower/collect/types.rs
```

For enum construction in `secondary.rs`, require the `DefId` already returned by nominal resolution. Missing identity becomes a lowering diagnostic; it must not fall back to `def_id_for_name`.

- [ ] **Step 7: Delete `Lowerer::def_id_for_name` and its panic tests**

Delete the helper at `lib/src/lower/mod.rs:288-298`. Replace tests of fallback/panic behavior with tests of explicit source-owner diagnostics.

- [ ] **Step 8: Run focused body-lowering suites**

Run serially:

```bash
cargo test -p rock-lib lower::function -- --nocapture
cargo test -p rock-lib lower::body_lowerer -- --nocapture
cargo test -p rock-lib lower::collect -- --nocapture
cargo test -p rock-lib lower::traits -- --nocapture
cargo test -p rock-lib lower::control_flow::secondary -- --nocapture
```

Expected: all tests pass.

- [ ] **Step 9: Run focused integration coverage**

Run these existing integration tests serially and record every result:

```bash
cargo test -p rock-lib --test integration test_inline_program -- --exact --nocapture
cargo test -p rock-lib --test integration test_id_owned_hir_preserves_same_name_module_calls -- --exact --nocapture
cargo test -p rock-lib --test integration test_modules -- --exact --nocapture
cargo test -p rock-lib --test integration test_extern_functions -- --exact --nocapture
cargo test -p rock-lib --test integration test_trait_default_methods -- --exact --nocapture
cargo test -p rock-lib --test integration test_impl_methods -- --exact --nocapture
cargo test -p rock-lib --test integration test_static_method_value_preserves_selected_authority -- --exact --nocapture
```

## Task 5: Make `PartialHir` And Finalization ID-Keyed

**Files:**
- Modify: `lib/src/lower/pipeline.rs:71-108`
- Modify: `lib/src/infer/mod.rs:95-487`
- Modify: `lib/src/infer/finalize.rs`
- Modify: `lib/src/infer/generalize.rs`
- Modify: `lib/src/infer/authority.rs`
- Modify: `lib/src/infer/solve.rs`
- Modify: `lib/src/hir/mod.rs:295-380`
- Test: co-located infer and HIR tests

- [ ] **Step 1: Write RED tests for direct ID preservation**

Add an inference test that gives the resolver an alias different from the payload's `name`, finalizes the partial HIR, and proves identity and display data remain separate:

```rust
#[test]
fn finalize_preserves_id_keyed_payload_when_display_alias_differs() {
    let id = DefId::new(CrateId(0), LocalDefId(4));
    let mut partial = partial_hir();
    partial.functions.insert(id, function(id, "source_name"));
    partial
        .resolver
        .item_names_by_id
        .insert(id, "demo::source_name".to_string());
    partial
        .resolver
        .insert_import_alias_with_name(
            "alias".to_string(),
            "demo::source_name".to_string(),
            id,
        );

    let resolved = finalize(partial).unwrap();

    assert_eq!(resolved.program.function_by_id(id).unwrap().1.id, id);
    assert!(resolved.program.function_display_aliases(id).contains(&"alias"));
}
```

- [ ] **Step 2: Run the test and verify RED**

Run the fully qualified test with `--exact --nocapture`.

Expected: compilation fails because `PartialHir.functions` still expects `String` keys.

- [ ] **Step 3: Change every semantic `PartialHir` collection to `DefId` keys**

Use:

```rust
pub struct PartialHir {
    pub functions: HashMap<DefId, HirFunction>,
    pub structs: HashMap<DefId, HirStruct>,
    pub enums: HashMap<DefId, HirEnum>,
    pub traits: HashMap<DefId, HirTrait>,
    pub impls: HashMap<DefId, HirImpl>,
    pub externs: HashMap<DefId, HirExtern>,
    pub engine: InferenceEngine,
    pub function_type_vars: HashMap<DefId, HashSet<TypeVarId>>,
    pub import_aliases: HashMap<String, String>,
    pub loaded_module_paths: Vec<(String, PathBuf)>,
    pub constraint_store: ConstraintStore,
    pub resolver: ResolverTables,
    pub current_def_ids: BTreeSet<DefId>,
    pub root_crate_id: CrateId,
    pub local_def_ids: IdGen<LocalDefId>,
    pub imported_effective_trait_methods: HashMap<(DefId, DefId), DefId>,
}
```

`LoweringPipeline::finish` moves these maps directly from `LowerItems::into_declarations`; it does not call `rebuild_id_indexes`.

- [ ] **Step 4: Make inference traversals operate on map values**

Update finalization, generalization, authority materialization, and constraint solving to use `values()`/`values_mut()` and keyed access. Where deterministic diagnostics matter, collect IDs, sort them, and then index by ID; do not sort or select by display name.

- [ ] **Step 5: Delete inference-time identity reconstruction**

Delete:

```text
move_id_keyed_named_declarations
declarations_are_same_function
declarations_are_same_struct
declarations_are_same_enum
declarations_are_same_trait
```

Those checks now belong at the collection boundary. `hir_program_from_partial` must pass function, struct, enum, and trait maps directly to `HirProgram::from_id_parts_with_names_and_canonical_names`. Convert impl and extern maps to vectors in sorted `DefId` order only because the existing constructor records explicit iteration order; do not use display names for that ordering.

- [ ] **Step 6: Build `HirNameTables` only from resolver/display metadata**

Add a helper that filters resolver names and aliases by the IDs present in each category:

```rust
fn names_for_ids(
    resolver: &ResolverTables,
    ids: impl IntoIterator<Item = DefId>,
) -> HashMap<String, DefId> {
    let ids = ids.into_iter().collect::<HashSet<_>>();
    resolver
        .item_paths
        .iter()
        .chain(resolver.import_aliases.iter())
        .chain(resolver.export_aliases.iter())
        .filter(|(_, id)| ids.contains(id))
        .map(|(name, id)| (name.clone(), *id))
        .collect()
}
```

Use category ID sets to populate `HirNameTables`. Canonical display names continue to come from `resolver.item_names_by_id`.

- [ ] **Step 7: Validate key/embedded-ID agreement before solving**

Call the shared `DeclarationItems` validation when finishing lower and add a lightweight `PartialHir::validate_item_ids` barrier before inference. Return `ResolveError`s; do not panic.

- [ ] **Step 8: Run focused inference and HIR suites**

Run serially:

```bash
cargo test -p rock-lib infer -- --nocapture
cargo test -p rock-lib hir -- --nocapture
cargo test -p rock-lib lower::pipeline -- --nocapture
```

Expected: all tests pass.

## Task 6: Convert Fixtures And Boundary Consumers Without Compatibility Shims

**Files:**
- Modify: `lib/src/lower/mod.rs` declaration fixtures and destructuring
- Modify: `lib/src/lower/items.rs` declaration fixtures
- Modify: `lib/src/collect/mod.rs` production declaration constructors
- Modify: `lib/src/lower/pipeline.rs` production partial-HIR constructor
- Modify: `lib/src/infer/finalize.rs` partial-HIR fixture
- Modify: `lib/src/infer/mod.rs` partial-HIR destructuring and fixtures
- Modify: additional boundary consumers only when the compiler or final constructor inventory identifies a direct type mismatch
- Test: affected co-located suites

- [ ] **Step 1: Generate a complete constructor inventory**

Use CodeGraph first for `Declarations PartialHir LowerItems from_maps rebuild_id_indexes`, then use repository-wide exact searches to enumerate any unindexed fixture or documentation references. Save the inventory once under `/tmp/opencode/task6-constructor-inventory.txt`.

- [ ] **Step 2: Convert fixtures to canonical IDs**

Replace name-keyed fixture payloads with `DeclarationItems::from_id_maps`, explicitly passing functions, signatures, structs, enums, traits, impls, and externs, or use direct ID-keyed `PartialHir` maps. Keep names in fixture resolver/name tables. Do not add `from_legacy_maps`, `sync_*`, or test-only repair constructors.

- [ ] **Step 3: Adapt impl and extern consumers to ID maps**

Replace slice/vector iteration with `.values()` and direct `.get(&id)` where identity is known. If a consumer requires deterministic output, sort `DefId`s explicitly. Do not reconstruct a name-keyed view.

- [ ] **Step 4: Add artifact-boundary regression coverage**

Ensure existing artifact tests prove imported aliases and canonical names still load the same `DefId` payload, while object/link symbol behavior remains unchanged. Run:

```bash
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib crate_system -- --nocapture
cargo test -p rock-lib products -- --nocapture
```

Expected: all tests pass.

- [ ] **Step 5: Run semantic identity audits**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected: all behavioral audits pass. Add a new audit only if it verifies runtime/semantic behavior; source residue remains a manual closure check.

- [ ] **Step 6: Check formatting and the current diff**

Run:

```bash
cargo fmt --all
cargo fmt --all --check
git diff --check
```

Expected: all commands exit zero.

## Task 7: Run Normal Verification Gates

**Files:**
- No source edits unless a gate exposes a defect
- Logs: `/tmp/opencode/task6-*.log`

- [ ] **Step 1: Run the focused phase suites serially**

Run and save output once per suite:

```bash
cargo test -p rock-lib collect -- --nocapture
cargo test -p rock-lib lower -- --nocapture
cargo test -p rock-lib infer -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected: every suite passes with non-zero matching test counts.

- [ ] **Step 2: Run integration tests**

Run:

```bash
cargo test -p rock-lib --test integration > /tmp/opencode/task6-integration.log 2>&1
```

Expected: exit zero. Record the exact passed count from the log.

- [ ] **Step 3: Run the full library suite**

Run:

```bash
cargo test -p rock-lib > /tmp/opencode/task6-rock-lib.log 2>&1
```

Expected: exit zero. Record library, integration, auxiliary, and doc-test counts once.

- [ ] **Step 4: Run Clippy and formatting gates**

Run serially:

```bash
cargo clippy --workspace --all-targets > /tmp/opencode/task6-clippy.log 2>&1
cargo fmt --all --check
git diff --check
```

Expected: all commands exit zero. Record warnings separately from errors and identify any warning introduced in changed production code.

## Task 8: Perform The Blocking Full-Codebase Closure Analysis

**Files:**
- Inspect: all tracked source and test files, excluding `.sisyphus/` and generated/untracked tool state
- Modify: any file containing a confirmed Step 6 residual defect
- Evidence: `/tmp/opencode/task6-closure-ledger.md`

- [ ] **Step 1: Rebuild the production call-path map from final source**

Use CodeGraph on the final tree for this complete path:

```text
collect_impl / collect_artifact_declarations
  -> Declarations / DeclarationItems
  -> Lowerer::from_declarations / LowerItems
  -> root, inline, loaded, dependency, trait-default, impl-method body lowering
  -> LoweringPipeline::finish / PartialHir
  -> solve, authority, generalize, finalize
  -> HirProgram::from_id_parts_with_names_and_canonical_names
```

Record every production definition, constructor, mutation, and consumer in the closure ledger.

- [ ] **Step 2: Run repository-wide residue inventories**

Inventory, at minimum:

```text
Declarations {
PartialHir {
LowerItems
HashMap<String, Hir
HashMap<(String, String)
rebuild_id_indexes
from_maps
resolve_item_id
value scans comparing an embedded `id`
item_names_by_id
function_display_aliases
struct_display_aliases
enum_display_aliases
nominal_display_aliases
```

Search the full repository, not only the files changed during implementation. Classify every hit as source resolution, lexical scope, import/export metadata, diagnostics/display, test-only support, or prohibited semantic payload/identity use.

- [ ] **Step 3: Audit every required path explicitly**

Record a pass/fail finding for:

```text
current root crate
inline modules
source-backed modules
source dependency bodies
artifact dependency interfaces
prelude injection
standalone function signatures
extern declarations
trait declarations and defaults
standalone impl methods
trait impl methods
struct and enum declarations
inference finalization
accepted HIR construction
product/artifact consumption
```

- [ ] **Step 4: Search for renamed or indirect compatibility paths**

Inspect helpers that map names to IDs, scan values by embedded ID, synchronize duplicate maps, select one alias payload, or repair missing indexes. A renamed replacement for `def_id_for_name`, `rebuild_id_indexes`, or inference-time alias deduplication is a blocking finding.

- [ ] **Step 5: Resolve every finding before proceeding**

For each finding, add a focused RED test, run it and record the failure, implement the smallest correction, rerun it GREEN, and rerun any invalidated focused suite. Repeat Steps 1-4 against the final corrected source. Ambiguous findings remain blocking until classified with direct source and call-path evidence.

- [ ] **Step 6: Obtain independent compliance and code-quality reviews**

Use at most two review agents concurrently:

```text
Reviewer 1: compare final source against every design requirement and Step 6 audit criterion.
Reviewer 2: inspect changed code for semantic regressions, hidden duplicate authority, panic/fallback paths, and missing tests.
```

Neither reviewer may edit files, touch VCS state, or inspect `.sisyphus/`. Resolve every valid finding and repeat affected verification and closure checks.

- [ ] **Step 7: Re-run invalidated full gates after the last closure fix**

At minimum rerun `cargo test -p rock-lib`, `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, and `git diff --check` after the last source correction.

- [ ] **Step 8: Write the closure verdict**

The ledger must state either:

```text
PASS: no production Step 6 debt remains; all name-keyed uses are classified allowed boundaries.
```

or:

```text
BLOCKED: <specific remaining finding with file, symbol, and required correction>.
```

Do not update `CLEAN_SLATE_COMPILER_AUDIT.md` on a blocked verdict.

## Task 9: Update The Audit Only After Closure Passes

**Files:**
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md:1491-1509`
- Read: `/tmp/opencode/task6-closure-ledger.md`
- Read: verification logs from Task 7 and any reruns from Task 8

- [ ] **Step 1: Verify the closure verdict is PASS**

Confirm the final closure ledger has no unresolved or ambiguous finding and was produced after the last source edit.

- [ ] **Step 2: Update Step 6 status and evidence**

Add a factual status block describing:

```text
ID-keyed Declarations/LowerItems/PartialHir ownership
deleted candidate-string identity recovery
source/resolver/display boundaries intentionally retained
root/module/dependency/trait/impl/extern/signature/artifact coverage
exact focused and full test counts
Clippy, rustfmt, and diff-check results
full-codebase closure-analysis scope and PASS verdict
```

Do not claim completion of Steps 7-13.

- [ ] **Step 3: Re-read the changed audit section**

Read the complete Step 6 status, goal, tasks, and acceptance criteria together. Correct stale wording, unsupported claims, inconsistent dates, or missing evidence.

- [ ] **Step 4: Run final documentation and diff checks**

Run:

```bash
cargo fmt --all --check
git diff --check
git status --short
```

Expected: formatting and diff checks pass; status lists only intentional Task 6 work plus pre-existing untracked tool state.
