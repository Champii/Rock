# Monomorphization Instance Registry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the registry-backed monomorphization boundary so specialization is keyed by canonical `DefId + substitution` and codegen consumes instance records directly.

**Architecture:** Mono stays responsible for specialization, but `InstanceRegistry` becomes the single source of truth for instance identity and emitted specialized bodies. Codegen keeps LLVM declaration/body lowering, but it should compile `MonomorphizedProgram.instances` directly instead of relying on specialized functions being reinserted into `program.functions`.

**Tech Stack:** Rust 2021, `cargo test -p rock-lib`, existing HIR/mono/codegen types, LLVM 18 via `inkwell`.

**Critical lessons from the failed attempt on `mono-instance-registry`:**
- Object-backed instances need LLVM declarations even though their bodies come from linked objects. The current `register_instances` skips them entirely, which is fine while `append_specialized_functions` re-emits them into `program.functions` — but breaks when that re-emission is removed.
- Object-backed instances must carry the original method signature (including `self_receiver`, `is_method`, param types) in a `declared` field on `InstanceRecord` so codegen can build the correct LLVM function type.
- Object-backed **functions** (not just impl methods) also need instance registration.

---

### Task 1: Lock in the registry and wrapper contract

**Files:**
- Modify: `lib/src/mono/registry.rs`
- Modify: `lib/src/codegen/mod.rs`
- Modify: `lib/src/mono/external.rs`
- Test: `lib/src/mono/registry.rs`, `lib/src/codegen/mod.rs`, `lib/src/mono/external.rs`

- [ ] **Step 1: Add the `declared` field to `InstanceRecord`**

The `InstanceRecord` needs a new field `declared: Option<HirFunction>` that holds the original method/function signature for object-backed instances. This is used by codegen to create LLVM function declarations when the body is `None`.

```rust
// lib/src/mono/registry.rs
#[derive(Debug, Clone)]
pub struct InstanceRecord {
    pub id: InstanceId,
    pub origin: InstanceOrigin,
    pub substitution: Vec<Type>,
    pub source_name: String,
    pub backend_symbol: String,
    pub declared: Option<HirFunction>,  // NEW: original signature for object-backed instances
    pub body: Option<HirFunction>,
    pub provided_by_object: bool,
}
```

- [ ] **Step 2: Write the failing tests**

```rust
// lib/src/mono/registry.rs
#[test]
fn instance_registry_distinguishes_substitutions() {
    let mut registry = InstanceRegistry::new();
    let def_id = DefId::new(CrateId(0), LocalDefId(1));

    let first_key = InstanceKey::new(InstanceOrigin::Function(def_id), vec![Type::I64]);
    let second_key = InstanceKey::new(InstanceOrigin::Function(def_id), vec![Type::U8]);

    let first = registry.intern(first_key.clone(), |id| InstanceRecord {
        id,
        origin: first_key.origin.clone(),
        substitution: first_key.substitution.clone(),
        source_name: "identity".to_string(),
        backend_symbol: "identity_mono_0".to_string(),
        declared: None,
        body: None,
        provided_by_object: false,
    });

    let second = registry.intern(second_key, |id| InstanceRecord {
        id,
        origin: InstanceOrigin::Function(def_id),
        substitution: vec![Type::U8],
        source_name: "identity".to_string(),
        backend_symbol: "identity_mono_1".to_string(),
        declared: None,
        body: None,
        provided_by_object: false,
    });

    assert_ne!(first, second);
    assert_eq!(registry.len(), 2);
}

// lib/src/codegen/mod.rs
#[test]
fn compile_program_uses_instance_records_directly() {
    let context = Context::create();
    let mut codegen = CodeGen::new(&context, "mono_signature");

    let body_func = HirFunction {
        name: "identity_mono_0".to_string(),
        qualified_name: None,
        generic_params: vec![],
        generic_bounds: HashMap::new(),
        params: vec![HirParam {
            name: "value".to_string(),
            ty: Type::I64,
            mutable: false,
            is_ref: false,
        }],
        ret_type: Type::I64,
        body: HirBlock { stmts: vec![], ty: Type::I64 },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };

    let mut program = MonomorphizedProgram::new(HirProgram {
        functions: HashMap::new(),
        structs: HashMap::new(),
        enums: HashMap::new(),
        traits: HashMap::new(),
        impls: vec![],
        externs: vec![],
    });

    program.instances.insert(
        InstanceId(0),
        InstanceRecord {
            id: InstanceId(0),
            origin: InstanceOrigin::Function(DefId::new(CrateId(0), LocalDefId(1))),
            substitution: vec![Type::I64],
            source_name: "identity".to_string(),
            backend_symbol: "identity_mono_0".to_string(),
            declared: None,
            body: Some(body_func),
            provided_by_object: false,
        },
    );

    codegen.compile_program(&program).unwrap();
    assert!(
        codegen
            .module
            .get_function("identity_mono_0")
            .unwrap()
            .count_basic_blocks()
            > 0
    );
}
```

- [ ] **Step 3: Run the tests to verify the current gap**

Run:
`cargo test -p rock-lib mono::registry::tests::instance_registry_reuses_same_function_instance -- --exact`
`cargo test -p rock-lib mono::registry::tests::instance_registry_distinguishes_substitutions -- --exact`
`cargo test -p rock-lib codegen::tests::compile_program_uses_instance_records_directly -- --exact`

Expected: at least the new codegen assertion should fail until the body-compilation pass is added.

- [ ] **Step 4: Commit this slice**

```bash
git add lib/src/mono/registry.rs
git commit -m "mono: add declared field to InstanceRecord"
```

### Task 2: Make mono specialize through the registry only

**Files:**
- Modify: `lib/src/mono/specialize.rs`
- Modify: `lib/src/mono/methods.rs`
- Modify: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/specialize.rs`, `lib/src/mono/methods.rs`

- [ ] **Step 1: Write the failing specialization reuse test**

```rust
#[test]
fn monomorphizer_reuses_specialization_for_same_type_args() {
    let mut mono = Monomorphizer::new();

    let generic_func = HirFunction {
        name: "identity".to_string(),
        qualified_name: Some("stdlib::identity".to_string()),
        generic_params: vec!["T".to_string()],
        generic_bounds: HashMap::new(),
        params: vec![HirParam {
            name: "value".to_string(),
            ty: Type::Generic("T".to_string()),
            mutable: false,
            is_ref: false,
        }],
        ret_type: Type::Generic("T".to_string()),
        body: HirBlock { stmts: vec![], ty: Type::Generic("T".to_string()) },
        is_curried: false,
        is_method: false,
        self_receiver: None,
        is_unsafe: false,
    };

    let args = vec![HirExpr { kind: HirExprKind::Var("x".to_string()), ty: Type::I64, span: Span::default() }];

    let first = mono.monomorphize_call("identity", &generic_func, &args).unwrap();
    let second = mono.monomorphize_call("identity", &generic_func, &args).unwrap();

    assert_eq!(first.0, second.0);
    assert_eq!(mono.instances.len(), 1);
}
```

- [ ] **Step 2: Run the test to verify the current string-keyed path still leaks through**

Run:
`cargo test -p rock-lib mono::specialize::tests::monomorphizer_reuses_specialization_for_same_type_args -- --exact`

Expected: fail until specialization keys are backed by `InstanceKey` only.

- [ ] **Step 3: Replace the fallback naming path with registry lookups**

The current `monomorphize_call` already uses the registry partially (it checks `self.instances.get(&instance_key)` then falls back to `self.counter` naming). The fix is to eliminate the fallback and always go through the registry. Also update `monomorphize_standalone_method_call` similarly.

- [ ] **Step 4: Re-run the specialization test**

Run:
`cargo test -p rock-lib mono::specialize::tests::monomorphizer_reuses_specialization_for_same_type_args -- --exact`

Expected: pass.

- [ ] **Step 5: Commit this slice**

```bash
git add lib/src/mono/specialize.rs lib/src/mono/methods.rs lib/src/mono/mod.rs
git commit -m "mono: intern specializations through the registry"
```

### Task 3: Register object-backed instances with declared signatures

**Files:**
- Modify: `lib/src/mono/external.rs`
- Test: `lib/src/mono/external.rs`

This task prepares the ground before removing `append_specialized_functions`. Codegen currently relies on `program.functions` for declarations of all functions, including object-backed ones. When we stop re-emitting specialized bodies, object-backed instances will only exist in the instance table — so their declarations must be creatable from instance records.

- [ ] **Step 1: Add `register_object_backed_functions`**

Register non-method functions from object-backed crate interfaces into the instance registry:

```rust
fn register_object_backed_functions(&mut self, crate_ctx: &CrateContext) {
    for loaded_crate in crate_ctx.crates.values() {
        if !loaded_crate.is_object_backed() {
            continue;
        }
        let Some(interface) = &loaded_crate.interface else {
            continue;
        };
        for (name, func) in &interface.functions {
            let origin = self.function_instance_origin(name, func);
            let instance_key = InstanceKey::new(origin.clone(), Vec::new());
            let mut declared = func.clone();
            declared.name = name.clone();
            self.instances.intern(instance_key, |id| InstanceRecord {
                id,
                origin: origin.clone(),
                substitution: Vec::new(),
                source_name: name.clone(),
                backend_symbol: name.clone(),
                declared: Some(declared.clone()),
                body: None,
                provided_by_object: true,
            });
        }
    }
}
```

- [ ] **Step 2: Add `declared` field to object-backed method registrations**

Update `record_object_backed_impl` to populate the `declared` field with the method's original signature (including `self_receiver`):

```rust
let mut declared = method.clone();
declared.name = format!("{}::{}", crate_name, method.name);
// ... in the InstanceRecord ...
declared: Some(declared),
```

- [ ] **Step 3: Commit this slice**

```bash
git add lib/src/mono/external.rs
git commit -m "mono: register object-backed instances with declared signatures"
```

### Task 4: Stop re-emitting specialized bodies into `program.functions`

**Files:**
- Modify: `lib/src/mono/process.rs`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/mono/mod.rs`
- Test: `lib/src/mono/external.rs`, `lib/src/mono/process.rs`

- [ ] **Step 1: Write the failing no-reemission test**

```rust
#[test]
fn process_with_crates_keeps_specialized_bodies_out_of_program_functions() {
    let output = wrapped_process_with_crates(&mut mono, program, &ctx);

    assert!(output.instances.values().any(|record| record.provided_by_object));
    assert!(!output.program.functions.keys().any(|name| name.contains("_mono_")));
}
```

- [ ] **Step 2: Run the test to verify the current append step is still active**

Run:
`cargo test -p rock-lib mono::external::tests::process_with_crates_keeps_specialized_bodies_out_of_program_functions -- --exact`

Expected: fail until `append_specialized_functions` is removed.

- [ ] **Step 3: Delete the append path**

In `process_with_crates_impl` (`lib/src/mono/external.rs`):
- Remove the call to `self.append_specialized_functions()` (line 86)
- Call `self.register_object_backed_functions(crate_ctx)` before `self.collect_impls`

In `lib/src/mono/mod.rs`:
- Remove `append_specialized_functions` method
- Remove `lookup_specialized_function` method if unused

- [ ] **Step 4: Re-run the no-reemission test**

Run:
`cargo test -p rock-lib mono::external::tests::process_with_crates_keeps_specialized_bodies_out_of_program_functions -- --exact`

Expected: pass.

- [ ] **Step 5: Commit this slice**

```bash
git add lib/src/mono/process.rs lib/src/mono/external.rs lib/src/mono/mod.rs
git commit -m "mono: keep specialized bodies in the instance table"
```

### Task 5: Teach codegen to declare and compile from instance records

**Files:**
- Modify: `lib/src/codegen/mod.rs`
- Test: `lib/src/codegen/mod.rs`

- [ ] **Step 1: Fix `register_instances` to declare object-backed instances**

Currently `register_instances` skips `provided_by_object` records. Change it so that:
- Object-backed instances get their LLVM declaration from the `declared` field (not `body`)
- Non-object-backed instances get their declaration from `body`

```rust
fn register_instances(&mut self, instances: &BTreeMap<InstanceId, InstanceRecord>) {
    for record in instances.values() {
        let declare_from = if record.provided_by_object {
            record.declared.as_ref()
        } else {
            record.body.as_ref().or(record.declared.as_ref())
        };

        if let Some(func) = declare_from {
            self.declare_function(&record.backend_symbol, func);

            // Register method metadata if applicable
            if func.is_method {
                self.method_functions.insert(record.backend_symbol.clone());
                self.method_receiver_modes
                    .insert(record.backend_symbol.clone(), func.self_receiver);
            }
        }
    }
}
```

- [ ] **Step 2: Add instance body compilation pass**

After the existing `program.functions` compilation loop and before impl method compilation, add a loop over `instances`:

```rust
// Compile instance bodies (specialized generic functions)
for record in instances.values() {
    if record.provided_by_object {
        continue;
    }
    if let Some(body) = &record.body {
        self.compile_function(&record.backend_symbol, body)?;
    }
}
```

- [ ] **Step 3: Re-run the codegen test**

Run:
`cargo test -p rock-lib codegen::tests::compile_program_uses_instance_records_directly -- --exact`

Expected: pass.

- [ ] **Step 4: Commit this slice**

```bash
git add lib/src/codegen/mod.rs
git commit -m "codegen: declare and compile from instance records"
```

### Task 6: Run the full verification pass

**Files:**
- None; verification only.

- [ ] **Step 1: Run the focused regression set**

Run:
`cargo test -p rock-lib mono::registry::tests::instance_registry_reuses_same_function_instance -- --exact`
`cargo test -p rock-lib mono::registry::tests::instance_registry_distinguishes_substitutions -- --exact`
`cargo test -p rock-lib mono::specialize::tests::monomorphizer_reuses_specialization_for_same_type_args -- --exact`
`cargo test -p rock-lib mono::external::tests::process_with_crates_keeps_specialized_bodies_out_of_program_functions -- --exact`
`cargo test -p rock-lib codegen::tests::compile_program_uses_instance_records_directly -- --exact`

- [ ] **Step 2: Run the full library test suite**

Run:
`cargo test -p rock-lib`

Expected: pass without regressions in array/slice dispatch, object-backed impl handling, or mono/codegen symbol lookup.

- [ ] **Step 3: Format if needed**

Run:
`cargo fmt --all`

- [ ] **Step 4: Commit the completed slice**

```bash
git add lib/src/mono lib/src/codegen docs/superpowers/plans/2026-05-01-monomorphization-instance-registry.md
git commit -m "finish monomorphization instance registry wiring"
```
