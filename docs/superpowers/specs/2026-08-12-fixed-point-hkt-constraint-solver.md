# Fixed-Point HKT Constraint Solver

## Status

Implemented on `experiment/signatureless-inference` on 2026-08-14.

The fixed-point constructor, authority, `Try`, SCC worklist, and annotation-free
higher-order callable inference are implemented. Callable instances preserve
per-call type-variable relations, selected deferred methods reconnect their
provisional callable constraints, and array-reference-to-slice coercions remain
deferred until a unique expected slice type is known. This specification does
not change Rock syntax, add compiler-owned functional operators, or relax the
strict accepted-HIR and MIR type barriers.

## Problem

Inference currently runs several ordered passes:

1. solve stored equality and trait constraints;
2. materialize authorities whose receiver or constructor heads are known;
3. propagate inferred function call instances;
4. solve constraints again;
5. materialize deferred `Try` authorities;
6. generalize and finalize.

This succeeds when information happens to become available before the pass
that consumes it. It fails when obligations form a cycle. The motivating
example is:

```rock
accepted_client = listener.accept! <&> start_client state.clone!
handler = accepted_client?
```

The `<&>` operator relates an input carrier constructor, a callback output,
and a mapped result carrier. The following `?` requires the result carrier's
rigid head before it can select `Try::branch` and `FromResidual`. Callable
constraints can discover the callback output after equality constraints have
already run, but the equality is not retried. The program therefore needs a
local annotation even though the full constraint graph has a unique solution.

The same ordering problem affects deferred methods, method values, fields,
associated projections, numeric defaults, and inferred function schemes.

## Decision

Inference will use one monotonic, kind-aware worklist for each function call
graph SCC. Obligations are retried when the inference substitution generation
changes. Solving stops only at quiescence, failure, or a deterministic
ambiguity boundary.

The solver remains predicative and decidable. It will not implement
unrestricted higher-order unification. Constructor inference is allowed only
when rigid-spine matching or canonical trait/impl evidence yields one unique
solution.

## Goals

1. Infer annotation-free HKT operator pipelines when their complete constraint
   graph has one solution.
2. Make constraint solving independent of source order and pass order.
3. Solve recursive and mutually recursive function SCCs before generalization.
4. Preserve kind checking, occurs checks, rigid constructor heads, and
   constructor-section semantics.
5. Preserve canonical `DefId` authority for methods, operators, traits,
   associated types, `Try`, and `FromResidual`.
6. Preserve ambiguity diagnostics instead of selecting the first candidate.
7. Preserve receiver adjustment, ownership, bounds, and unsafe context.
8. Leave accepted HIR and artifacts free of `TypeVar` and `Type::Error`.
9. Leave runtime MIR types fully applied and constructor-free.
10. Remove executable local type annotations from
    `test_projects/new_new/main.rk`; nominal struct field types remain required
    because they define layout.

## Non-Goals

- Unrestricted higher-order or impredicative inference.
- Guessing a constructor when multiple canonical candidates remain viable.
- Compiler recognition of `Result`, `Option`, `<&>`, `>>=`, or stdlib names.
- Runtime dictionaries, type erasure, dynamic dispatch, or runtime HKT values.
- Backward-compatibility paths for unresolved accepted HIR.
- Inferring nominal struct field layout from constructor use.

## Constraint Model

Every obligation has stable identity, source context, and one of four states:

- `Pending`: insufficient information; retry after relevant progress.
- `Solved`: all equalities and canonical authorities are committed.
- `Ambiguous`: quiescence left more than one viable canonical solution.
- `Failed`: rigid evidence proves the obligation impossible.

The worklist covers:

- type equality;
- constructor application equality;
- callable argument and return relationships;
- callable ownership, reference mutability, callable kind, and ABI pass mode;
- explicit coercion obligations, including fixed-array reference to slice
  reference coercion;
- trait and impl bounds;
- associated type projections;
- function call-site instantiation;
- method calls and method values;
- field access;
- operator functions and operator methods;
- `Try::branch` and `FromResidual::from_residual`;
- literal defaulting after structural solving.

Owned `T`, `&T`, `&mut T`, `[T; N]`, `[T]`, `&[T; N]`, and `&[T]` are distinct
semantic shapes. Structural equality must not silently identify them. A legal
coercion is a first-class obligation with an expected destination type; it is
not an inference fallback and cannot alter the declared or inferred ABI of a
named callable.

## Progress Model

`InferenceEngine` owns a monotonically increasing substitution generation.
Binding a previously free representative or changing its canonical resolved
form increments the generation. Probe engines do not mutate the production
generation.

An obligation records the generation at which it last ran. A pending
obligation is retried only after the generation changes or after another
obligation explicitly wakes it. This provides deterministic termination
without fixed retry counts.

Progress is monotonic:

- free variables may become aliases, concrete types, applications, or rigid
  constructors;
- solved canonical authority is never replaced;
- no pass resets a production substitution;
- generalization occurs only after the SCC reaches quiescence.

## HKT Equality

Ordinary structural equality continues to decompose equal function, nominal,
reference, tuple, application, and lambda shapes.

Rigid-spine constructor matching additionally supports equations such as:

```text
F<A> = Result<T, IoError>
```

when `F` has kind `Type -> Type` and the right side has one uniquely aligned
constructor section. The solution is equivalent to:

```text
F = Result<_, IoError>
A = T
```

This is constructor pattern matching, not general higher-order unification.
The solver rejects or leaves pending:

- two unknown heads with no rigid evidence;
- repeated constructor holes with inconsistent arguments;
- escaping `BoundVar`s;
- kind or application-arity mismatches;
- multiple canonical impl/constructor candidates;
- occurs-check cycles.

Normalization applies beta reduction and canonical constructor sections before
candidate comparison. Candidate identity uses `DefId`, kind, and normalized
arguments, never display names.

## Callable And Operator Propagation

Callable obligations relate the callable value, complete argument tuple,
return type, safety, callable kind, ownership/reference modes, and ABI pass
modes before HKT equality is validated. When callback inference changes any of
those facts, dependent callable, constructor equality, operator result, and
coercion obligations are woken.

Propagation is bidirectional for a signatureless named function used as a
callback. Callback use sites constrain the named function header before its SCC
is generalized. A call through the callback constrains the callback parameter
and return shapes, while the named function body and all other call sites
constrain the same monomorphic SCC header.

Callable compatibility is not return-type-only. Parameter types are checked
with function variance and exact ownership/reference shape. Fixed-array
reference to slice-reference conversion is allowed only as an explicit
argument coercion against an already known `&[T]` or `&mut [T]` expectation; it
must not infer the callback itself as taking `&[T; N]` when another use requires
`&[T]`.

Every failed callable unification is a failed obligation with a source span.
No propagation path may discard a unification error. At quiescence, accepted
HIR must contain one semantic callable type that is compatible with the exact
selected named function declaration and its MIR callable signature. If owned
and borrowed solutions remain viable, inference reports ambiguity rather than
choosing one or relying on a codegen wrapper.

Functional operators remain ordinary Rock functions. Lowering records their
function type equality and exact resolved variable target. The solver does not
recognize operator spelling or stdlib ownership.

## Deferred Authority

Deferred method, method-value, field, operator, and static-member selection
participates in the same worklist.

Selection may commit only when one canonical candidate remains after applying:

- resolved receiver and argument types;
- expected result type;
- receiver adjustment candidates;
- trait and function bounds;
- mutability and ownership rules;
- unsafe context;
- associated type normalization.

Accepted HIR stores the exact selected `DefId` authority, substitutions, and
receiver adjustment. Name-based rediscovery remains forbidden after lowering.

## Try And Residual Propagation

`?` creates linked obligations for:

- carrier implementation of marker-bound `Try`;
- `Try::Output`;
- `Try::Residual`;
- enclosing return implementation of marker-bound `FromResidual<Residual>`;
- exact branch and conversion method authorities.

These obligations remain pending while the carrier head is unknown. Once HKT
or callable propagation makes the head rigid, the solver retries selection.
At quiescence, a genuinely generic or ambiguous carrier receives a precise
diagnostic. The solver never assumes that the operand carrier and enclosing
return carrier are the same constructor.

## SCC And Generalization

Function headers remain monomorphic variables while their call-graph SCC is
solved. Recursive edges share one scheme environment. Calls outside the SCC
instantiate solved inferred schemes independently.

After quiescence:

1. report failed and ambiguous obligations;
2. apply literal defaults with evidence;
3. generalize free signature variables not captured by the environment;
4. strictly finalize every remaining runtime type;
5. validate and convert to accepted HIR.

No arbitrary three-pass propagation loop remains.

## Phase Ordering

The target pipeline is:

```text
collect headers and constraints
-> build call-graph SCCs
-> solve each SCC worklist to quiescence
-> validate remaining obligations
-> apply evidence-backed defaults
-> generalize
-> strict finalization
-> accepted HIR validation
-> monomorphization
-> MIR
```

## Diagnostics

Pending obligations retain their source spans. At quiescence diagnostics must
identify the unresolved relationship, for example:

- ambiguous constructor section with candidate canonical types;
- callable return does not determine mapped carrier;
- callable parameter ownership or reference mode is ambiguous;
- named callback parameter or return type is incompatible with its expected
  callback type;
- fixed-array reference requires a slice expectation that was never proven;
- `?` carrier head remains unresolved;
- multiple method implementations remain after expected-result filtering;
- kind, arity, occurs-check, bound, or callable ABI failure.

The solver must not emit cascades of anonymous finalization errors after one
root constructor failure. It must also reject a callable mismatch before MIR
or LLVM; codegen is a defensive verifier, not a callable-type repair phase.

## Implementation Phases

### Phase 1: Progress-Aware Structural Worklist

Status: complete.

- Add substitution generation tracking to `InferenceEngine`.
- Run equality and callable-derived equality obligations until no generation
  changes.
- Preserve current behavior and diagnostics.
- Add ordering regression tests where callable propagation unlocks an earlier
  equality.

### Phase 2: First-Class Obligation States

Status: complete.

- Assign stable IDs and states to constraints.
- Record dependencies from type-variable representatives to obligations.
- Remove full rescans when only a subset needs waking.

### Phase 3: Rigid-Spine Constructor Matching

Status: complete.

- Extract constructor-section matching from ad hoc callable handling.
- Normalize and solve unique aligned HKT applications.
- Add ambiguity, kind, repeated-hole, and occurs-check tests.

### Phase 4: Deferred Authority Integration

Status: complete.

- Move call-instance, method, method-value, field, and operator propagation onto
  the worklist.
- Remove arbitrary retry loops and duplicate eager/deferred selection logic.

### Phase 5: Try Integration

Status: complete. `Try::branch` and `FromResidual::from_residual` are separate
authority obligations with independent dependencies and wakeups.

- Represent `Try` and `FromResidual` as linked obligations.
- Wake them when carrier, residual, or enclosing return types change.
- Preserve custom-carrier, cross-residual, and unsafe semantics.

### Phase 6: SCC Finalization

Status: complete. SCC component worklists are requeued only after substitution
progress, then evidence-backed defaults and strict obligation validation run
before the solved components are generalized.

- Solve SCCs against explicit environments.
- Generalize only after quiescence.
- Remove obsolete staged solve/materialize sequences.

### Phase 7: Annotation-Free Acceptance

Status: complete. Annotation-free `new_new/main.rk` infers borrowed callable
parameters, reconnects provisional deferred-call constraints to selected method
signatures, and runs the bounded two-client broadcast scenario against a fresh
stdlib artifact.

- Remove executable local annotations from `new_new/main.rk` after its inferred
  callable signatures are ABI-compatible with the supplied named functions.
- Build a fresh stdlib artifact.
- Run a bounded listener with two connect clients and assert actual payload
  delivery in both clients.
- Reject incompatible named-callable propagation before accepted HIR and retain
  the selected callable signature through MIR lowering.

## Required Tests

- Equality recorded before callable evidence is retried after callable progress.
- `F<A> = Result<T, E>` infers the unique `Result<_, E>` section.
- Two unknown heads remain ambiguous.
- Multiple matching constructor impls remain ambiguous.
- Callback return inference propagates through `<&>` into a following `?`.
- Signatureless `broadcast` infers exactly
  `&mut SharedServerState -> &[U8] -> I64 -> Result I64, IoError`.
- Signatureless `receive_with` infers its callback as
  `&mut T -> &[U8] -> I64 -> Result I64, E`, not `&[U8; N]`; each fixture call
  specializes `E` to `IoError`.
- Explicitly typed and signatureless variants produce ABI-compatible accepted-HIR
  callable signatures and identical MIR pass modes; the signatureless variant
  may validly generalize variables that its body does not constrain.
- A named callback semantic-signature mismatch fails before LLVM generation.
- Array-reference to slice-reference coercion occurs only against a known slice
  expectation.
- Ambiguous owned-versus-borrowed callback inference is diagnosed.
- Same-name methods remain ambiguous without sufficient expected type.
- Deferred shared, mutable, and autoderef receivers preserve adjustments.
- Generic method and impl bounds are enforced.
- Deferred unsafe calls require unsafe context.
- Custom `Try` and cross-residual conversions retain exact authority.
- Direct and mutual recursion terminate and specialize once per instance key.
- Accepted HIR rejects every unresolved type, authority, or incompatible named
  callable signature.
- Full `cargo test -p rock-lib` passes.
- A fresh stdlib artifact plus annotation-free `new_new/main.rk` runs a bounded
  listener and two clients and verifies payload delivery.

## Implementation Evidence

Confirmed implementation evidence:

- `InferenceEngine` tracks monotonic substitution generations and isolates
  probes from production progress.
- `ConstraintStore` assigns stable obligation IDs and records dependencies
  incrementally when obligations are created; changed representatives wake only
  their dependents.
- Lowering computes function call-graph SCCs before body inference and records
  dependency-first component order.
- Structural, constructor, method, field, operator, `Try`, and residual
  authority progress is driven by the component worklist without fixed retry
  counts or whole-store dependency rebuilds.
- Rigid-spine matching infers unique constructor sections while preserving kind,
  arity, binder-escape, repeated-hole, occurs-check, and ambiguity failures.
- Accepted HIR validation rejects unresolved types, fields, methods, `Try`
  branch authorities, and residual conversion authorities.
- Instance interning reuses one canonical specialization for each
  `InstanceKey`.
- Deferred callable coercion obligations wake when selected method signatures
  reconnect the provisional callee type, so `&mut [U8; N]` coerces to
  `&mut [U8]` without guessing while the callee is unknown.
- Per-call relation maps preserve repeated inferred variables across target and
  callback parameters without globally monomorphizing polymorphic helpers.
- Variables needed for authority inside a function body still propagate from a
  call instance into the source signature.
- Known borrowed field receivers materialize an explicit dereference before MIR
  field projection.
- Function-value and callable-argument propagation report concrete unification
  failures instead of discarding them.
- The fresh-stdlib integration gate compiles annotation-free `new_new/main.rk`,
  starts its listener on a bounded dynamic port, connects two clients, and
  verifies that both receive the same payload.

## Rollback Gates

Each phase must preserve the full test suite. If a phase introduces source-order
dependence, first-candidate selection, unresolved accepted HIR, recursive
specialization re-entry, or stdlib-name special cases, revert that phase rather
than adding compatibility fallbacks.
