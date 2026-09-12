# Monomorphization Instance Registry Design

**Date:** 2026-04-29
**Status:** Draft for review
**Scope:** `lib/src/mono` plus the smallest codegen contract change needed to consume instance records without reconstructing semantic identity from strings.

## Purpose

The monomorphizer still uses string keys for concrete functions, generic functions, specialization caches, and backend symbol selection. That works mechanically, but it keeps instance identity tied to display names instead of compiler-owned IDs.

This slice finishes the identity area by making monomorphization own a real `InstanceId` registry keyed by canonical definition identity and substitution. Codegen should consume those instance records, not rediscover them from names.

## Goals

- Make `InstanceId` the canonical identity for monomorphized bodies.
- Key specialization by `DefId + substitution`, not by function-name strings.
- Deduplicate repeated instantiations through one registry.
- Keep backend symbol names as output metadata, not semantic identity.
- Preserve current object-backed skip behavior and external generic specialization behavior.

## Non-Goals

- Do not rewrite the HIR type system or the `Type` model.
- Do not replace every string name in HIR with IDs.
- Do not redesign MIR in this slice.
- Do not change parser, lower, or collect semantics beyond the identity tables mono needs as input.
- Do not introduce a new backend symbol scheme unless it falls out naturally from the instance record.

## Current Shape

Today `lib/src/mono` still tracks:

- `concrete_functions: HashMap<String, HirFunction>`
- `generic_functions: HashMap<String, HirFunction>`
- `external_generic_functions: HashMap<String, HirFunction>`
- `specializations: HashMap<(String, Vec<Type>), String>`
- `new_functions: HashMap<String, HirFunction>`

Method dispatch and object-backed impl checks also rely on type-name strings and backend-name strings. That means mono is still deciding instance uniqueness from names instead of canonical definition identity.

## Design

### Identity Model

Introduce a mono-owned instance registry with these concepts:

- `InstanceId` is the stable identity for one concrete instantiated body.
- `InstanceKey` is the semantic deduplication key, built from a canonical `DefId` and the substitution used for instantiation.
- `InstanceRecord` stores the resolved body, backend symbol, origin metadata, and whether the body is provided externally.

The registry is the only place that decides whether two requests are the same instance.

### Data Flow

1. Mono receives the HIR program plus canonical resolver data from the preceding phase.
2. When mono needs a concrete specialization, it resolves the source definition to `DefId` and builds an `InstanceKey` from that `DefId` plus the substitution.
3. The registry returns an existing `InstanceId` if the key was already seen, or allocates a new one and records the specialized body.
4. Call sites may still carry backend symbol strings as emitted targets, but those strings are produced from instance metadata, not used to decide identity.
5. Codegen consumes the monomorphized output and the instance table. It may keep internal string maps for LLVM objects, but it should not infer semantic instance existence from strings.

### Output Shape

The mono boundary should become a monomorphized wrapper rather than a raw `HirProgram` alone.

Recommended shape:

```text
MonomorphizedProgram {
    program: HirProgram,
    instances: BTreeMap<InstanceId, InstanceRecord>,
}
```

This keeps HIR intact while giving backend consumers a first-class instance table.

### Instance Sources

The registry should handle at least these origins:

- local generic functions
- local generic impl methods
- external generic functions
- trait default bodies that need downstream instantiation
- object-backed bodies that should be recorded but not re-emitted locally

Object-backed bodies remain skipped for local emission, but they still need instance metadata so downstream consumers can see that the instance exists and where it comes from.

### Backend Symbols

Backend symbols remain a derived concern.

- They can still be strings.
- They can still be generated from the canonical definition and substitution.
- They should no longer be the primary key used to determine whether an instance already exists.

## Testing

Add focused coverage for:

- same `DefId + substitution` returning the same `InstanceId`
- different substitutions returning distinct `InstanceId`s
- external generic functions being registered through the same registry
- object-backed impls still being skipped for local emission while remaining visible to the instance table
- codegen accepting the monomorphized output without consulting string identity as its source of truth

## Risks

- If mono cannot reliably recover canonical `DefId`s for a specialization origin, the registry would need a fallback and the slice would lose its value.
- If codegen keeps branching on string names instead of instance records, this becomes a naming cleanup instead of an identity fix.
- If the output shape is widened too far, this slice will drift into type-context or MIR work.

## Follow-Up

- Migrate the remaining type-name-driven specialization helpers onto the instance registry.
- Use the same canonical identity model in type context and semantic type lowering.
- Then continue shrinking string-keyed identity in backend paths.
