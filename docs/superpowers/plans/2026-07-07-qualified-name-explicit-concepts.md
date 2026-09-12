# Qualified Name Explicit Concepts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete `HirFunction.qualified_name` and replace its backend/display/lookup responsibilities with explicit ID-keyed owners.

**Architecture:** `qualified_name` disappears from HIR and serialized product function rows. Type-variable/generalization bookkeeping becomes `DefId`-keyed, resolver/artifact canonical names remain display metadata, and mono creates deterministic backend symbols from `InstanceOrigin` plus substitutions instead of HIR strings.

**Tech Stack:** Rust 2021, `rock-lib`, serde/bincode product artifacts, Cargo tests.

---

## Context

The approved spec is `docs/superpowers/specs/2026-07-07-qualified-name-explicit-concepts-design.md`.

The worktree is `/root/new_lang2/.worktrees/hir-codegen-debt-removal`. The branch currently has an unrelated unstaged `MEMORY.md` change. Do not stage or modify `MEMORY.md` unless the user explicitly asks.

Use `/home/linuxbrew/.linuxbrew/bin/rg` for residue scans if the default `rg` is unreliable.

## File Structure

Modify these files:

- `lib/src/semantic_identity_audit.rs`: add durable source-level regression tests for Step 4.
- `lib/src/hir/mod.rs`: remove the field from `HirFunction` and update nearby tests/constructors.
- `lib/src/collect/context.rs`: change collection `function_type_vars` maps from `String` to `DefId`.
- `lib/src/collect/mod.rs`: change `Declarations.function_type_vars` type and related tests.
- `lib/src/collect/headers.rs`: store function type vars by `DefId`; remove impl backend-name formatter and rekeying.
- `lib/src/collect/collector.rs`: stop writing qualified function names into HIR; copy type-var sets by function ID when cloning module-local function headers.
- `lib/src/lower/mod.rs`: change lowerer `function_type_vars` type to `HashMap<DefId, HashSet<TypeVarId>>`.
- `lib/src/lower/function.rs`: store lowered function type vars by `DefId` and remove HIR field initialization.
- `lib/src/lower/bodies.rs`: use function IDs for body contexts and remove `qualified_name` type-var cleanup.
- `lib/src/lower/collect/traits.rs`: remove lower-side impl backend-name formatter and rekeying.
- `lib/src/lower/traits/conformance.rs`: stop generating a missing default method `qualified_name`.
- `lib/src/lower/traits/defaults.rs`, `lib/src/lower/session.rs`, and other HIR fixture files: remove field initializers.
- `lib/src/infer/mod.rs`: change `PartialHir.function_type_vars` to `HashMap<DefId, HashSet<TypeVarId>>`.
- `lib/src/infer/generalize.rs`: look up function header vars by `func.id` only.
- `lib/src/infer/finalize.rs`: identify own functions through IDs instead of string keys.
- `lib/src/mono/mod.rs`: remove `qualified_name` origin recovery and add ID-based local backend-symbol helpers.
- `lib/src/mono/methods.rs`: use ID-based instance symbols for method specializations and display names from explicit method names only.
- `lib/src/mono/specialize.rs`: use ID-based backend symbols for function specializations.
- `lib/src/mono/external.rs`: remove test/helper dependency on `method.qualified_name`.
- `lib/src/mono/process.rs`: keep resolver/display alias registration out of symbol identity; update fixtures.
- `lib/src/mono/registry.rs`: keep `InstanceSymbols` as source/debug plus backend-symbol data, with tests proving identity ignores symbols.
- `lib/src/products.rs`: remove `ProductFunctionInterface.qualified_name`; bump artifact version.
- `lib/src/products/type_table.rs`: remove serialized `qualified_name` fields and encode/decode entries.
- `rock-shared/src/sysroot.rs`: bump shared artifact version.
- `lib/src/crate_artifact/types.rs`: stop inserting/copying `qualified_name` through artifact interfaces.
- `lib/src/crate_artifact/load.rs`: use canonical/display names only; remove backend-shaped callable-name fallbacks.
- `CLEAN_SLATE_COMPILER_AUDIT.md`: mark Step 4 complete after verification.

Do not restructure modules. Keep changes local to the existing owners.

---

### Task 1: Add RED Step 4 Audit Tests

**Files:**
- Modify: `lib/src/semantic_identity_audit.rs`

- [ ] **Step 1: Add the failing audit tests**

Add these tests after `external_object_symbols_do_not_fall_back_to_source_names`:

```rust
#[test]
fn hir_functions_do_not_own_qualified_backend_names() {
    assert_production_file_absent(
        "hir/mod.rs",
        &[
            "pub qualified_name: Option<String>",
            "qualified_name:",
        ],
    );
    assert_production_file_absent(
        "products.rs",
        &[
            "pub qualified_name: Option<String>",
            "qualified_name: function.qualified_name.clone()",
        ],
    );
    assert_production_file_absent(
        "products/type_table.rs",
        &[
            "pub qualified_name: Option<String>",
            "qualified_name: value.qualified_name.clone()",
            "qualified_name: self.qualified_name",
        ],
    );
}

#[test]
fn collect_and_lower_do_not_construct_impl_backend_names() {
    for path in [
        "collect/headers.rs",
        "collect/collector.rs",
        "lower/collect/traits.rs",
        "lower/traits/conformance.rs",
        "lower/bodies.rs",
    ] {
        assert_production_file_absent(
            path,
            &[
                "format_impl_backend_name",
                "TypeName_methodName",
                ".qualified_name",
                "qualified_name = Some",
            ],
        );
    }
}

#[test]
fn mono_and_artifacts_do_not_use_qualified_name_for_identity_or_symbols() {
    for path in [
        "mono/mod.rs",
        "mono/methods.rs",
        "mono/specialize.rs",
        "mono/external.rs",
        "crate_artifact/types.rs",
        "crate_artifact/load.rs",
    ] {
        assert_production_file_absent(
            path,
            &[
                ".qualified_name",
                "qualified_name.clone()",
                "qualified_name.get_or_insert",
            ],
        );
    }

    assert_production_file_absent(
        "crate_artifact/load.rs",
        &[
            "format!(\"{}_{}\", imp.type_name, method_name)",
            "format!(\"{}_{}\", owner, method_name)",
            "format!(\"{}_{}\", short, method_name)",
        ],
    );
}
```

- [ ] **Step 2: Run the audit test and confirm RED**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::hir_functions_do_not_own_qualified_backend_names -- --exact --nocapture
```

Expected: FAIL because `HirFunction` and product rows still expose `qualified_name`.

- [ ] **Step 3: Run the collect/lower audit test and confirm RED**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::collect_and_lower_do_not_construct_impl_backend_names -- --exact --nocapture
```

Expected: FAIL because collect/lower still contain `format_impl_backend_name` and `.qualified_name` writes.

- [ ] **Step 4: Run the mono/artifact audit test and confirm RED**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit::mono_and_artifacts_do_not_use_qualified_name_for_identity_or_symbols -- --exact --nocapture
```

Expected: FAIL because mono and artifact loading still read `.qualified_name`.

- [ ] **Step 5: Commit the RED tests**

```bash
git add lib/src/semantic_identity_audit.rs
git commit -m "test: capture qualified name cleanup target"
```

---

### Task 2: Make Function Type-Variable Tracking DefId-Keyed

**Files:**
- Modify: `lib/src/collect/context.rs`
- Modify: `lib/src/collect/mod.rs`
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/lower/mod.rs`
- Modify: `lib/src/lower/function.rs`
- Modify: `lib/src/lower/pipeline.rs`
- Modify: `lib/src/infer/mod.rs`
- Modify: `lib/src/infer/generalize.rs`
- Modify: `lib/src/infer/finalize.rs`
- Modify: nearby tests in `lib/src/collect/headers.rs`, `lib/src/collect/mod.rs`, and `lib/src/infer/generalize.rs`

- [ ] **Step 1: Change map types from `String` to `DefId`**

In `lib/src/collect/mod.rs`, change `Declarations.function_type_vars`:

```rust
pub function_type_vars: HashMap<crate::ids::DefId, HashSet<crate::ids::TypeVarId>>,
```

In `lib/src/collect/context.rs`, change both `CollectContext` and `LocalCollection`:

```rust
pub(crate) function_type_vars: HashMap<DefId, HashSet<TypeVarId>>,
```

In `lib/src/lower/mod.rs`, change the lowerer field:

```rust
pub(crate) function_type_vars: HashMap<DefId, HashSet<TypeVarId>>,
```

In `lib/src/infer/mod.rs`, change `PartialHir.function_type_vars`:

```rust
pub function_type_vars: HashMap<DefId, HashSet<TypeVarId>>,
```

- [ ] **Step 2: Store collect-stage type vars by function ID**

In `lib/src/collect/headers.rs`, replace both string-key inserts with ID-key inserts:

```rust
context
    .function_type_vars
    .insert(function_id, func_type_vars);
```

Do this in `build_function_header_with_id` and `build_function_header_with_sig_and_id`.

- [ ] **Step 3: Remove impl-method type-var rekeying in collect headers**

In `lib/src/collect/headers.rs`, remove the `format_impl_type_arg` and `format_impl_backend_name` helpers. In `build_impl_with_id`, replace the method setup block:

```rust
let func = build_function_header_with_id(context, fd, method_id);
let method_name = ident.name.clone();

context
    .methods
    .insert((type_name.clone(), method_name.clone()), func.clone());
methods.insert(method_name, func);
```

The `build_function_header_with_id` call already stores the method header variables by `method_id`.

- [ ] **Step 4: Copy cloned module function type vars by ID in collection**

In `lib/src/collect/collector.rs`, remove the `short_name_type_vars` logic and the name rekeying in `collect_function_header`. The body after header construction should not write `func.qualified_name`:

```rust
let mut func = if let Some(sig) = self.context.function_sigs.get(&name).cloned() {
    let mut func =
        headers::build_function_header_with_sig(&mut self.context, fd, &sig, function_id);
    self.context.function_sigs.remove(&name);
    if self
        .context
        .function_sig_unsafe
        .remove(&name)
        .unwrap_or(false)
    {
        func.is_unsafe = true;
    }
    func
} else {
    headers::build_function_header_with_id(&mut self.context, fd, function_id)
};
```

In `collect_inline_qualified_declarations`, when cloning an existing short-name function for a qualified module item, copy the type-var set by the original ID before changing the ID:

```rust
let original_id = func.id;
if let Some(type_vars) = self.context.function_type_vars.get(&original_id).cloned() {
    self.context.function_type_vars.insert(function_id, type_vars);
}
func.id = function_id;
```

Do not write `func.qualified_name`.

- [ ] **Step 5: Store lower-stage type vars by function ID**

In `lib/src/lower/function.rs`, change `lower_function_decl_header` so it computes the ID before storing type vars:

```rust
let function_id = self.def_id_for_name(&[&func_name]);
self.function_type_vars.insert(function_id, func_type_vars);
HirFunction {
    id: function_id,
    name: func_name,
    generic_params: vec![],
    generic_param_ids: Vec::new(),
    generic_bounds: HashMap::new(),
    params,
    ret_type,
    body: HirBlock {
        stmts: vec![],
        ty: Type::Unit,
    },
    is_curried,
    is_method: fd.self_receiver.is_some(),
    self_receiver: fd.self_receiver,
    is_unsafe: fd.is_unsafe,
}
```

In `lower_function_decl_header_with_sig_and_id`, store any collected type vars under `id` if the function collects header vars in that path.

- [ ] **Step 6: Simplify generalization lookup**

In `lib/src/infer/generalize.rs`, replace the lookup-key block with ID lookup only:

```rust
let func_header_vars = function_type_vars
    .get(&func.id)
    .cloned()
    .unwrap_or_default();
```

Remove the suffix-match fallback entirely.

- [ ] **Step 7: Update finalization own-function detection**

In `lib/src/infer/finalize.rs`, replace the string set with an ID set:

```rust
let own_functions: HashSet<DefId> = hir
    .functions
    .values()
    .filter(|function| {
        hir.function_type_vars.contains_key(&function.id)
            && hir.current_def_ids.contains(&function.id)
    })
    .map(|function| function.id)
    .collect();
```

Then use function IDs when finalizing:

```rust
let is_own = own_functions.contains(&func.id);
```

Add `DefId` to the imports if needed.

- [ ] **Step 8: Update tests that inspect `function_type_vars`**

Replace string-key assertions with ID-key assertions. For example, in `lib/src/collect/headers.rs`:

```rust
let function_id = DefId::new(CrateId(0), LocalDefId(0));
assert_eq!(lowerer.function_type_vars.get(&function_id), Some(&expected));
```

For impl method tests, assert the method ID is present and the short method name is not a possible key because the map type no longer accepts strings:

```rust
let method_id = methods["render"].id;
assert!(lowerer.function_type_vars.contains_key(&method_id));
```

- [ ] **Step 9: Run focused tests**

Run:

```bash
cargo test -p rock-lib collect::headers -- --nocapture
cargo test -p rock-lib infer::generalize -- --nocapture
cargo test -p rock-lib infer::finalize -- --nocapture
```

Expected: all compile and pass.

- [ ] **Step 10: Commit the ID-keyed type-var tracking change**

```bash
git add lib/src/collect/context.rs lib/src/collect/mod.rs lib/src/collect/headers.rs lib/src/collect/collector.rs lib/src/lower/mod.rs lib/src/lower/function.rs lib/src/lower/pipeline.rs lib/src/infer/mod.rs lib/src/infer/generalize.rs lib/src/infer/finalize.rs
git commit -m "refactor: key function type vars by def id"
```

---

### Task 3: Delete `HirFunction.qualified_name`

**Files:**
- Modify: `lib/src/hir/mod.rs`
- Modify: all HIR construction sites reported by the compiler

- [ ] **Step 1: Remove the field from HIR**

In `lib/src/hir/mod.rs`, change `HirFunction` to remove `qualified_name`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirFunction {
    pub id: DefId,
    pub name: String,
    pub generic_params: Vec<String>,
    pub generic_param_ids: Vec<GenericParamId>,
    pub generic_bounds: HirGenericBounds,
    pub params: Vec<HirParam>,
    pub ret_type: Type,
    pub body: HirBlock,
    pub is_curried: bool,
    pub is_method: bool,
    pub self_receiver: Option<SelfReceiverMode>,
    pub is_unsafe: bool,
}
```

- [ ] **Step 2: Run check to collect constructor errors**

Run:

```bash
cargo check -p rock-lib
```

Expected: FAIL with `struct HirFunction has no field named qualified_name` and field-access errors.

- [ ] **Step 3: Remove field initializers from production constructors**

Remove `qualified_name: ...` from `HirFunction` literals in production files. The known files include:

```text
lib/src/collect/headers.rs
lib/src/lower/function.rs
lib/src/lower/traits/defaults.rs
lib/src/lower/traits/conformance.rs
lib/src/lower/bodies.rs
lib/src/lower/session.rs
lib/src/mono/specialize.rs
lib/src/mono/registry.rs
lib/src/mono/external.rs
lib/src/products/type_table.rs
lib/src/crate_artifact/types.rs
lib/src/lib.rs
```

Each literal should keep the remaining fields in the existing order. Example:

```rust
HirFunction {
    id,
    name: name.to_string(),
    generic_params: Vec::new(),
    generic_param_ids: Vec::new(),
    generic_bounds: HashMap::new(),
    params: Vec::new(),
    ret_type: Type::Unit,
    body,
    is_curried: false,
    is_method: false,
    self_receiver: None,
    is_unsafe: false,
}
```

- [ ] **Step 4: Remove direct field reads that are now compile errors**

Temporarily replace reads with explicit source names so the project compiles before deeper mono/artifact cleanup. Use these local replacements only where the value is display/debug metadata:

```rust
method.name.clone()
```

or, for maps that already carry the canonical function key:

```rust
func_name.to_string()
```

Do not use these replacements for backend symbols; those are handled in Task 5.

- [ ] **Step 5: Run check again**

Run:

```bash
cargo check -p rock-lib
```

Expected: PASS or fail only in product/artifact/mono files that later tasks intentionally fix.

- [ ] **Step 6: Commit the HIR field deletion**

```bash
git add lib/src/hir/mod.rs lib/src/collect lib/src/lower lib/src/mono lib/src/products lib/src/crate_artifact lib/src/lib.rs
git commit -m "refactor: remove hir function qualified names"
```

---

### Task 4: Remove Collect/Lower Backend-Name Construction

**Files:**
- Modify: `lib/src/collect/headers.rs`
- Modify: `lib/src/collect/collector.rs`
- Modify: `lib/src/lower/collect/traits.rs`
- Modify: `lib/src/lower/traits/conformance.rs`
- Modify: `lib/src/lower/bodies.rs`

- [ ] **Step 1: Remove lower-side backend-name helpers**

In `lib/src/lower/collect/traits.rs`, delete `format_impl_type_arg` and `format_impl_backend_name`. Replace the impl method collection loop with:

```rust
let mut methods = HashMap::new();
for (ident, fd) in &imp.methods {
    let func = this.lower_function_decl_header(fd);
    let method_name = ident.name.clone();

    if trait_name.is_none() {
        this.items
            .methods
            .insert((type_name.clone(), method_name.clone()), func.clone());
    }
    methods.insert(method_name, func);
}
```

- [ ] **Step 2: Stop generated default methods from creating backend names**

In `lib/src/lower/traits/conformance.rs`, remove this block from `prepare_missing_default_method`:

```rust
if func.qualified_name.is_none() {
    func.qualified_name = Some(Lowerer::format_impl_backend_name(
        &imp.type_name,
        &imp.receiver_arg_types,
        imp.trait_name.as_deref(),
        &imp.trait_arg_types,
        method_name,
        imp.id,
        &imp.type_generics,
    ));
}
```

No replacement is needed because `func.id` and `imp.id` carry identity.

- [ ] **Step 3: Update body-lowering contexts to use display name plus IDs**

In `lib/src/lower/bodies.rs`, replace body-context names that used `func.qualified_name` with `func.name.clone()` or `method_name.clone()`.

For explicit-signature cleanup, remove only the ID-keyed entry:

```rust
if explicit_sig.is_some() {
    self.function_type_vars.remove(&func.id);
}
```

Create contexts like:

```rust
let local_id_context = BodyLoweringContext::new(
    func.name.clone(),
    func.id,
    Some(func.id),
    func.generic_params.clone(),
    HirGenericBounds::new(),
    fd.is_unsafe || explicit_sig_is_unsafe,
);
```

and:

```rust
let mut body_context = BodyLoweringContext::new(
    method_name.clone(),
    func.id,
    Some(impl_id),
    type_generics.clone(),
    impl_bounds,
    fd.is_unsafe || explicit_sig_is_unsafe,
);
```

- [ ] **Step 4: Register static impl resolver paths explicitly**

In `lib/src/lower/bodies.rs`, keep the static impl method resolver registration but remove the deleted field branch:

```rust
let mangled = format!("{}_{}", type_name, method_name);
self.current_def_ids.insert(func.id);
self.resolver.item_paths.insert(mangled.clone(), func.id);
self.resolver
    .item_names_by_id
    .entry(func.id)
    .or_insert_with(|| mangled.clone());
self.items.functions.insert(mangled, func.clone());
```

This is a resolver compatibility alias for static associated functions, not a HIR backend symbol.

- [ ] **Step 5: Run collect/lower tests**

Run:

```bash
cargo test -p rock-lib collect::headers -- --nocapture
cargo test -p rock-lib lower -- --nocapture
cargo test -p rock-lib semantic_identity_audit::collect_and_lower_do_not_construct_impl_backend_names -- --exact --nocapture
```

Expected: all pass, including the Step 4 audit test for collect/lower.

- [ ] **Step 6: Commit collect/lower cleanup**

```bash
git add lib/src/collect lib/src/lower lib/src/semantic_identity_audit.rs
git commit -m "refactor: remove impl backend names from collect lower"
```

---

### Task 5: Generate Local Mono Backend Symbols From IDs

**Files:**
- Modify: `lib/src/mono/mod.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/specialize.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/registry.rs` tests if needed

- [ ] **Step 1: Add ID-based symbol helpers**

In `lib/src/mono/mod.rs`, add helpers inside `impl Monomorphizer` near the existing instance registration helpers:

```rust
fn def_symbol_fragment(def_id: DefId) -> String {
    format!("c{}_d{}", def_id.crate_id.0, def_id.local.0)
}

fn substitution_symbol_suffix(substitution: &[crate::ids::TypeId]) -> String {
    if substitution.is_empty() {
        return "none".to_string();
    }

    substitution
        .iter()
        .map(|ty| format!("t{}", ty.0))
        .collect::<Vec<_>>()
        .join("_")
}

fn backend_symbol_for_origin(origin: &InstanceOrigin, substitution: &[crate::ids::TypeId]) -> String {
    let origin = match origin {
        InstanceOrigin::Function(def_id) => {
            format!("fn_{}", Self::def_symbol_fragment(*def_id))
        }
        InstanceOrigin::ImplMethod { owner, method } => {
            let owner = match owner {
                crate::mono::InstanceImplOwner::Named(def_id) => {
                    Self::def_symbol_fragment(*def_id)
                }
                crate::mono::InstanceImplOwner::BuiltinSlice => "builtin_slice".to_string(),
            };
            format!("impl_{}_method_{}", owner, Self::def_symbol_fragment(*method))
        }
        InstanceOrigin::TraitDefault { trait_id, method } => format!(
            "trait_{}_default_{}",
            Self::def_symbol_fragment(*trait_id),
            Self::def_symbol_fragment(*method)
        ),
    };

    format!(
        "__rock_{}_{}",
        origin,
        Self::substitution_symbol_suffix(substitution)
    )
}
```

- [ ] **Step 2: Preserve the executable entry symbol explicitly**

Still in `lib/src/mono/mod.rs`, add:

```rust
fn backend_symbol_for_function(name: &str, origin: &InstanceOrigin, substitution: &[crate::ids::TypeId]) -> String {
    if name == "main" && substitution.is_empty() {
        "main".to_string()
    } else {
        Self::backend_symbol_for_origin(origin, substitution)
    }
}
```

This keeps the platform entry point explicit without depending on `qualified_name`.

- [ ] **Step 3: Remove function origin recovery from names**

In `lib/src/mono/mod.rs`, replace `function_instance_origin` with:

```rust
fn function_instance_origin(&self, _func_name: &str, func: &HirFunction) -> InstanceOrigin {
    InstanceOrigin::Function(func.id)
}
```

Then delete `try_resolve_def_id` and `resolve_def_id` if no production code uses them.

- [ ] **Step 4: Use ID-based symbols for concrete local functions**

In `register_function_instance`, stop accepting a caller-provided backend symbol. Change the signature to:

```rust
fn register_function_instance(&mut self, source_name: &str, func: &HirFunction) -> InstanceId
```

Build the symbol from the origin:

```rust
let origin = InstanceOrigin::Function(func.id);
let key = InstanceKey::new(origin.clone(), Vec::new());
let backend_symbol = Self::backend_symbol_for_function(source_name, &origin, &[]);
```

Update the call in `process`:

```rust
self.register_function_instance(&name, &func);
```

- [ ] **Step 5: Use ID-based symbols for impl methods and trait defaults**

Delete `impl_method_backend_symbol` and `trait_default_backend_symbol` from `lib/src/mono/mod.rs`.

In `register_impl_method_instance`, replace backend-symbol construction with:

```rust
let backend_symbol = Self::backend_symbol_for_origin(&origin, &[]);
```

In `register_trait_default_instance`, replace backend-symbol construction with:

```rust
let backend_symbol = Self::backend_symbol_for_origin(&origin, &[]);
```

Keep `InstanceSymbols::new(format!("{}::{}", imp.type_name, method_name), backend_symbol)` for source/debug display only.

- [ ] **Step 6: Use ID-based symbols for specializations**

In `lib/src/mono/specialize.rs`, replace `InstanceSymbols::new(func_name, specialized_name.clone())` with:

```rust
let backend_symbol = Monomorphizer::backend_symbol_for_function(
    func_name,
    &origin,
    &substitution,
);
```

and:

```rust
symbols: crate::mono::InstanceSymbols::new(func_name, backend_symbol),
```

In `lib/src/mono/methods.rs`, replace each `InstanceSymbols::new(..., specialized_name.clone())` for local impl/trait method specializations with:

```rust
let backend_symbol = Self::backend_symbol_for_origin(&origin, &substitution);
```

Use method/source display names such as `format!("{}::{}", imp.type_name, method_name)` for the first argument only.

- [ ] **Step 7: Remove remaining `qualified_name` test fixture behavior in mono external helpers**

In `lib/src/mono/external.rs`, update `loaded_object_backed_crate_named_with_backend_symbols` so resolver entries for methods use explicit backend symbols only:

```rust
let Some(backend_name) = backend_symbols.get(&method.id).cloned() else {
    continue;
};
```

Change `simple_hir_function` to take only `(name: &str, id: DefId)` and remove the field initialization.

- [ ] **Step 8: Add or update mono tests for ID-based symbols**

Add a test in `lib/src/mono/process.rs` or update an existing semantic identity test:

```rust
#[test]
fn local_backend_symbols_use_instance_origin_not_display_name() {
    let function_id = DefId::new(crate::ids::CrateId(0), crate::ids::LocalDefId(42));
    let origin = InstanceOrigin::Function(function_id);

    assert_eq!(
        Monomorphizer::backend_symbol_for_origin(&origin, &[]),
        "__rock_fn_c0_d42_none"
    );
}
```

If the helper remains private, place the test in the same module file.

- [ ] **Step 9: Run mono tests**

Run:

```bash
cargo test -p rock-lib mono -- --nocapture
cargo test -p rock-lib semantic_identity_audit::mono_and_artifacts_do_not_use_qualified_name_for_identity_or_symbols -- --exact --nocapture
```

Expected: mono tests pass or fail only on artifact-side `qualified_name` residue, which Task 6 removes.

- [ ] **Step 10: Commit mono symbol cleanup**

```bash
git add lib/src/mono lib/src/semantic_identity_audit.rs
git commit -m "refactor: generate mono symbols from instance identity"
```

---

### Task 6: Remove Product And Artifact `qualified_name` Schema Data

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/products/type_table.rs`
- Modify: `rock-shared/src/sysroot.rs`
- Modify: `lib/src/crate_artifact/types.rs`
- Modify: `lib/src/crate_artifact/load.rs`
- Modify: artifact/product tests in the same files

- [ ] **Step 1: Remove product interface field**

In `lib/src/products.rs`, remove `qualified_name` from `ProductFunctionInterface`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFunctionInterface {
    pub id: DefId,
    pub name: String,
    pub generic_params: Vec<String>,
    pub generic_param_ids: Vec<crate::types::GenericParamId>,
    pub generic_bounds: HashMap<crate::types::GenericParamId, Vec<TraitBound>>,
    pub params: Vec<Type>,
    pub ret_type: Type,
    pub is_curried: bool,
    pub is_method: bool,
    pub self_receiver: Option<crate::ast::SelfReceiverMode>,
    pub is_unsafe: bool,
}
```

Update `impl From<&HirFunction>` by deleting:

```rust
qualified_name: function.qualified_name.clone(),
```

- [ ] **Step 2: Remove serialized fields and encode/decode entries**

In `lib/src/products/type_table.rs`, remove `qualified_name` from `SerializedHirFunction` and `SerializedProductFunctionInterface`.

In `SerializedHirFunction::encode`, delete:

```rust
qualified_name: value.qualified_name.clone(),
```

In `SerializedHirFunction::decode`, delete:

```rust
qualified_name: self.qualified_name,
```

In `SerializedProductFunctionInterface::encode`, delete:

```rust
qualified_name: value.qualified_name.clone(),
```

In `SerializedProductFunctionInterface::decode`, delete:

```rust
qualified_name: self.qualified_name,
```

- [ ] **Step 3: Bump artifact format version**

In `lib/src/products.rs`:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 33;
```

In `rock-shared/src/sysroot.rs`:

```rust
pub const PRODUCT_ARTIFACT_FORMAT_VERSION: u32 = 33;
```

Update the pinned version test in `lib/src/products.rs`:

```rust
assert_eq!(PRODUCT_ARTIFACT_FORMAT_VERSION, 33);
```

- [ ] **Step 4: Stop artifact interfaces from mutating HIR function names**

In `lib/src/crate_artifact/types.rs`, update `insert_function`:

```rust
#[cfg(test)]
pub(crate) fn insert_function(&mut self, canonical_name: String, function: HirFunction) {
    self.canonical_names.insert(function.id, canonical_name);
    self.functions
        .insert(function.id, ProductFunctionInterface::from(&function));
}
```

Update `hir_function_from_interface` by deleting the removed field and keeping `name: function.name.clone()`.

- [ ] **Step 5: Remove artifact-load canonical-name backfill**

In `lib/src/crate_artifact/load.rs`, replace the function-interface map around `function.qualified_name.get_or_insert(canonical_name)` with:

```rust
.map(|(id, _name, function)| {
    remap_function_interface_id(function, id, remap, &type_validator)
        .map(|function| (function.id, function))
})
```

Canonical names stay in `ArtifactCrateInterface.canonical_names` and product display metadata.

- [ ] **Step 6: Remove backend-shaped callable-name fallbacks**

In `lib/src/crate_artifact/load.rs`, replace `static_method_callable_names` with:

```rust
fn static_method_callable_names(
    _imp: &crate::hir::HirImpl,
    method_name: &str,
    method: &crate::hir::HirFunction,
    display_name: Option<&str>,
) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(display_name) = display_name {
        names.push(display_name.to_string());
    }
    names.push(method.name.clone());
    if method.name != method_name {
        names.push(method_name.to_string());
    }
    names
}
```

Replace `static_method_interface_callable_names` with:

```rust
fn static_method_interface_callable_names(
    _imp: &ProductImplInterface,
    method_name: &str,
    method: &ProductFunctionInterface,
    display_name: Option<&str>,
) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(display_name) = display_name {
        names.push(display_name.to_string());
    }
    names.push(method.name.clone());
    if method.name != method_name {
        names.push(method_name.to_string());
    }
    names
}
```

Do not add `Type_method`, owner `DefId`, or owner-short-name fallbacks.

- [ ] **Step 7: Update artifact/product tests**

Remove `qualified_name` setup from tests such as:

```rust
method.qualified_name = Some("Box_value".to_string());
```

When a test needs an artifact callable name, insert it explicitly through product display metadata, resolver canonical names, or link records. For object-backed symbol tests, use:

```rust
let backend_symbols = BTreeMap::from([(method.id, "artifact_Box_value".to_string())]);
```

- [ ] **Step 8: Run product and artifact tests**

Run:

```bash
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib semantic_identity_audit::hir_functions_do_not_own_qualified_backend_names -- --exact --nocapture
cargo test -p rock-lib semantic_identity_audit::mono_and_artifacts_do_not_use_qualified_name_for_identity_or_symbols -- --exact --nocapture
```

Expected: all pass.

- [ ] **Step 9: Commit product/artifact schema cleanup**

```bash
git add lib/src/products.rs lib/src/products/type_table.rs rock-shared/src/sysroot.rs lib/src/crate_artifact/types.rs lib/src/crate_artifact/load.rs lib/src/semantic_identity_audit.rs
git commit -m "refactor: remove qualified names from product artifacts"
```

---

### Task 7: Remove Remaining `qualified_name` Residue In Compiler Paths

**Files:**
- Modify files reported by residue scans, excluding source-loader module-name terminology.

- [ ] **Step 1: Scan for `qualified_name` residue**

Run:

```bash
/home/linuxbrew/.linuxbrew/bin/rg -n "qualified_name|format_impl_backend_name" lib/src --glob '*.rs'
```

Expected: matches may remain in source-loader/module path code and tests, but not in HIR function/product/mono/artifact identity paths.

- [ ] **Step 2: Remove production HIR/product/mono/artifact residue**

For each production match in these paths, delete or replace it:

```text
lib/src/hir
lib/src/collect
lib/src/lower
lib/src/infer
lib/src/mono
lib/src/products.rs
lib/src/products
lib/src/crate_artifact
lib/src/crate_system
```

Allowed matches:

```text
lib/src/source_loader/mod.rs
lib/src/lower/services.rs source module lookup names
tests that assert source-loader qualified module behavior
```

Do not allow `.qualified_name` field access in production compiler code.

- [ ] **Step 3: Run audit tests**

Run:

```bash
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected: all semantic identity audit tests pass.

- [ ] **Step 4: Commit residue cleanup**

```bash
git add lib/src
git commit -m "refactor: remove qualified name residue"
```

---

### Task 8: Full Verification And Audit Doc Update

**Files:**
- Modify: `CLEAN_SLATE_COMPILER_AUDIT.md`

- [ ] **Step 1: Run focused verification**

Run these commands serially:

```bash
cargo test -p rock-lib products -- --nocapture
cargo test -p rock-lib crate_artifact -- --nocapture
cargo test -p rock-lib mono -- --nocapture
cargo test -p rock-lib semantic_identity_audit -- --nocapture
```

Expected: all pass.

- [ ] **Step 2: Run integration verification**

Run:

```bash
cargo test -p rock-lib --test integration > /tmp/rock-lib-integration-step4.log 2>&1
```

Expected: command exits with status 0. Inspect `/tmp/rock-lib-integration-step4.log` only if it fails.

- [ ] **Step 3: Run formatting and whitespace checks**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: both pass.

- [ ] **Step 4: Run final residue scans**

Run:

```bash
if /home/linuxbrew/.linuxbrew/bin/rg -n "HirFunction \{[^}]*qualified_name|pub qualified_name: Option<String>|\.qualified_name|format_impl_backend_name" lib/src/hir lib/src/collect lib/src/lower lib/src/infer lib/src/mono lib/src/products.rs lib/src/products lib/src/crate_artifact lib/src/crate_system --glob '*.rs'; then exit 1; else test "$?" -eq 1; fi
```

Expected: no matches and exit status 0.

Run the allowed source-loader scan separately for awareness:

```bash
/home/linuxbrew/.linuxbrew/bin/rg -n "qualified_name" lib/src/source_loader lib/src/lower/services.rs --glob '*.rs'
```

Expected: source/module path matches are acceptable.

- [ ] **Step 5: Update Step 4 status in the audit doc**

In `CLEAN_SLATE_COMPILER_AUDIT.md`, replace the Step 4 section status with:

```markdown
Status: Complete as of 2026-07-07. `HirFunction.qualified_name` and serialized
product function `qualified_name` rows were removed. Collect/lower no longer
construct impl backend names, function type-variable tracking is keyed by
`DefId`, mono derives local backend symbols from `InstanceOrigin` and
substitution data, and artifact loading keeps canonical names as display/interface
metadata instead of backend or callable-identity fallbacks. The product artifact
format version was bumped for the schema change.

Validation evidence:

- `cargo test -p rock-lib products -- --nocapture`
- `cargo test -p rock-lib crate_artifact -- --nocapture`
- `cargo test -p rock-lib mono -- --nocapture`
- `cargo test -p rock-lib semantic_identity_audit -- --nocapture`
- `cargo test -p rock-lib --test integration`
- `cargo fmt --all --check`
- `git diff --check`
- Source residue scan over HIR/products/collect/lower/infer/mono/artifact/crate-system paths found no production `HirFunction.qualified_name`, product function `qualified_name`, `.qualified_name`, or `format_impl_backend_name` matches.
```

Keep the existing Goal/Tasks/Acceptance criteria below the status for historical context, matching the style used by Steps 1-3.

- [ ] **Step 6: Commit audit doc and final changes**

```bash
git add CLEAN_SLATE_COMPILER_AUDIT.md lib/src rock-shared/src/sysroot.rs
git commit -m "docs: mark qualified name cleanup complete"
```

---

## Final Handoff Checks

- [ ] Run `git status --short --branch` and confirm only intended files are changed.
- [ ] If the user explicitly requested pushing in the implementation session, run `git push` after inspecting status, diff, and recent log.
- [ ] Report the exact verification commands and outcomes.
