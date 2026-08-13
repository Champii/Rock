# Fixed-Point HKT Constraint Solver

## Status

Implemented on `experiment/signatureless-inference` on 2026-08-13.

This specification extends the implemented higher-kinded type model. It does
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
- trait and impl bounds;
- associated type projections;
- function call-site instantiation;
- method calls and method values;
- field access;
- operator functions and operator methods;
- `Try::branch` and `FromResidual::from_residual`;
- literal defaulting after structural solving.

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

Callable obligations relate the callable value, argument tuple, and return
type before HKT equality is validated. When callback inference changes a
return type, dependent constructor equality and operator result obligations
are woken.

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
- `?` carrier head remains unresolved;
- multiple method implementations remain after expected-result filtering;
- kind, arity, occurs-check, or bound failure.

The solver must not emit cascades of anonymous finalization errors after one
root constructor failure.

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

Status: complete.

- Remove executable local annotations from `new_new/main.rk`.
- Build a fresh stdlib artifact.
- Compile and run representative listener/connect paths.

## Required Tests

- Equality recorded before callable evidence is retried after callable progress.
- `F<A> = Result<T, E>` infers the unique `Result<_, E>` section.
- Two unknown heads remain ambiguous.
- Multiple matching constructor impls remain ambiguous.
- Callback return inference propagates through `<&>` into a following `?`.
- Annotation-free `finish_connection`, `receive_with`, `broadcast`, and
  `run_listener` patterns compile.
- Same-name methods remain ambiguous without sufficient expected type.
- Deferred shared, mutable, and autoderef receivers preserve adjustments.
- Generic method and impl bounds are enforced.
- Deferred unsafe calls require unsafe context.
- Custom `Try` and cross-residual conversions retain exact authority.
- Direct and mutual recursion terminate and specialize once per instance key.
- Accepted HIR rejects every unresolved type or authority.
- Full `cargo test -p rock-lib` passes.
- Fresh stdlib artifact plus annotation-free `new_new/main.rk` passes.

## Implementation Evidence

- `InferenceEngine` tracks monotonic substitution generations and isolates
  probes from production progress.
- `ConstraintStore` assigns stable obligation IDs and records dependencies
  incrementally when obligations are created; changed representatives wake only
  their dependents.
- Lowering computes function call-graph SCCs before body inference and records
  dependency-first component order.
- Structural, callable, method, field, operator, `Try`, and residual authority
  progress is driven by the component worklist without fixed retry counts or
  whole-store dependency rebuilds.
- Rigid-spine matching infers unique constructor sections while preserving kind,
  arity, binder-escape, repeated-hole, occurs-check, and ambiguity failures.
- Accepted HIR validation rejects unresolved types, fields, methods, `Try`
  branch authorities, and residual conversion authorities.
- Instance interning reuses one canonical specialization for each
  `InstanceKey`.
- `test_projects/new_new/main.rk` contains no executable local type annotations;
  its remaining annotated fields define nominal layout.
- The fresh-stdlib integration gate compiles and runs the real annotation-free
  `new_new` project, and the localhost TCP roundtrip test covers bounded network
  execution.

## Rollback Gates

Each phase must preserve the full test suite. If a phase introduces source-order
dependence, first-candidate selection, unresolved accepted HIR, recursive
specialization re-entry, or stdlib-name special cases, revert that phase rather
than adding compatibility fallbacks.
