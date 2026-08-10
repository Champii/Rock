# Higher-Kinded Types And Eager Functional Abstractions Design

## Status

Completed and implemented on branch `feature/higher-kinded-types` on 2026-08-08.
All 14 implementation tasks and the feature acceptance audit are complete.
The repository-wide strict Clippy baseline cleanup is explicitly deferred to
priority-1 task `new_lang2-n2e`; no lint suppression was added.

Specification work is tracked by `new_lang2-ds4`. Implementation is tracked by
epic `new_lang2-gdt` and its dependency-ordered child tasks.

This document defines the language and compiler contract. The companion
implementation plan is
`docs/superpowers/plans/2026-08-05-higher-kinded-types-functional-abstractions.md`.

## Executive Decision

Rock will support statically dispatched, predicative higher-kinded types. A
generic parameter may denote either a runtime type or a type constructor with a
declared kind. Type constructors may be passed to generic functions and traits,
partially applied, composed, and implemented by constructor-level traits.

Rock will not use runtime type-class dictionaries, erased containers, dynamic
casts, compiler-recognized `Functor` names, or unrestricted higher-order
unification. Generic constructor code is selected by canonical IDs and
monomorphized before MIR, exactly like existing generic value-type code.

The central surface forms are:

```rock
// F has kind Type -> Type.
trait Functor for F _
    fmap: M -> F A -> F B where M: FnMut A, B

// A type section. This is sugar for: \T -> Result T, E
impl Functor for Result _, E
```

Type sections are not a `Result` special case. They lower to ordinary type
lambdas, so the same mechanism supports constructor composition, repeated
parameters, aliases, and higher-kinded library APIs.

Functional collection APIs will be eager. `Functor`, `Foldable`, and
`Traversable` operate directly on finite containers. Rock will not make lazy,
stateful Rust-style iterator adapters the foundation of this work.

## Motivation

Rock already has:

- generic nominal types such as `Option T`, `Result T, E`, and `Vec T`;
- generic functions and trait bounds;
- associated types and canonical trait/member IDs;
- static trait selection and monomorphization;
- first-class callable metadata through `Fn`, `FnMut`, and `FnOnce`;
- consuming `map` operations on `Option` and `Result`;
- strict accepted-HIR and MIR type barriers.

Rock cannot currently abstract over the shared constructor in `Option A`,
`Vec A`, or `Result A, E`. Every `GenericParamId` denotes a complete type, and
`Type::Generic` is a leaf. Although the parser can represent a head with type
arguments, type lowering discards those arguments when the head is a generic
parameter. Unification only decomposes concrete structs and enums with equal
nominal IDs.

An associated-output encoding can imitate `fmap`, but it cannot express the
constructor relationship directly, compose constructors naturally, define
standard `Applicative` and `Monad` signatures, or support a reusable
`Traversable` abstraction. This specification adds the missing type-level
model rather than extending that imitation.

## Goals

1. Represent and check kinds such as `Type`, `Type -> Type`, and
   `(Type -> Type) -> Type`.
2. Allow kinded generic parameters in nominal declarations, functions, traits,
   impls, type aliases, associated types, and method-level generics.
3. Allow constructors to be passed, returned, partially applied, and composed
   in type expressions.
4. Provide explicit type lambdas and ergonomic type-section syntax.
5. Keep type equality, impl identity, artifact identity, and instance identity
   canonical under alpha, beta, and eta equivalence.
6. Extend inference with decidable, kind-aware constructor pattern matching.
7. Support constructor-level traits and static trait members.
8. Enforce declaration-time orphan and overlap rules across local and artifact
   impls.
9. Preserve ID-based selection and eliminate any need for name-based HKT
   dispatch.
10. Ensure every runtime type is fully applied before MIR and LLVM codegen.
11. Ship ordinary stdlib `Functor`, `Applicative`, `Monad`, `Foldable`, and
    `Traversable` traits without compiler special cases.
12. Provide eager, stack-safe transformation and traversal for finite
    containers.
13. Solve `Result T, E` with a general type-constructor abstraction.
14. Preserve Rock ownership: no implicit clone, sharing, allocation, or boxing.
15. Persist all public HKT contracts in portable product artifacts.

## Non-Goals

- No runtime type-class dictionaries or dynamic HKT dispatch.
- No type constructor values at runtime.
- No arbitrary type-level recursion or general type-level computation.
- No unrestricted Huet-style higher-order unification.
- No impredicative polymorphism or first-class `forall` values.
- No kind polymorphism in the first implementation. Generic kinds are explicit.
- No specialization, negative impls, or overlapping impl priority.
- No variance or subtyping redesign. Generic parameters remain invariant unless
  a later, separate variance specification changes that rule.
- No lifetime-kinded, const-kinded, or row-kinded parameters in this feature.
- No `Functor` implementation for references or slices until Rock has the
  lifetime model needed to state it soundly.
- No automatic `Applicative` or `Monad` implementation for every `Functor`.
- No `Applicative` or `Monad` implementation for `Vec` unless a later ownership
  design supplies a lawful, unconstrained meaning.
- No lazy iterator chains, hidden iterator state machines, or implicit infinite
  sequences.
- No compiler-owned knowledge of stdlib trait names or laws.
- No compatibility decoder for the previous artifact format.

## Terminology

### Type

A term of kind `Type` may describe a runtime value. Examples are `I64`,
`Option I64`, and `Result String, IoError`.

### Type Constructor

A term whose kind is an arrow and which must receive more type arguments before
it describes a runtime value. Examples are:

```text
Option             : Type -> Type
Result             : Type -> Type -> Type
Result String      : Type -> Type
Result _, IoError  : Type -> Type
```

### Kind

The type of a type-level term. Kinds are compile-time only.

### Type Lambda

A non-recursive type-level function, such as `\T -> Result T, E`.

### Type Section

An application containing `_` holes. It is surface sugar for a type lambda.
Holes bind left-to-right.

### Saturated Application

A constructor application whose resulting kind is `Type`. Known nominal
applications normalize to the existing concrete `Struct` or `Enum` type form.

## Kind System

The semantic kind representation is:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Type,
    Arrow(Box<Kind>, Box<Kind>),
}
```

Arrows associate to the right:

```text
Type -> Type -> Type == Type -> (Type -> Type)
```

Nominal declaration kinds are derived from their generic parameter kinds. If
all parameters are ordinary types:

```text
Option T      gives Option : Type -> Type
Result T, E   gives Result : Type -> Type -> Type
```

A constructor-parameterized declaration may itself consume constructors:

```rock
struct Compose (F _), (G _), A
    value: F (G A)
```

Its constructor kind is:

```text
(Type -> Type) -> (Type -> Type) -> Type -> Type
```

Every generic declaration stores a canonical descriptor:

```rust
pub struct GenericParamDecl {
    pub id: GenericParamId,
    pub name: String,
    pub kind: Kind,
}
```

`GenericParamId` remains owner-and-index based. Names remain display and source
metadata. Kind lookup is by the canonical parameter ID.

The owner is the declaration that introduces the binder. Trait target and trait
generic binders are owned by the trait `DefId`; function and trait-member
binders are owned by that function/member `DefId`; impl binders are owned by the
impl `DefId`; alias binders are owned by the alias `DefId`. A member may refer to
its enclosing trait target binder, but its own `A`, `B`, and callable binders
have member-owned IDs. Type-lambda binders use lexical de Bruijn identity and do
not receive `GenericParamId`s.

Ordinary generic binders default to `Type`. Higher-kinded binders are explicit;
the compiler must not guess a higher kind from an otherwise invalid use.

## Generic Binder Syntax

Simple constructor kinds use the same space-separated, comma-delimited
application syntax as ordinary Rock types:

```rock
F _       // Type -> Type
F _, _    // Type -> Type -> Type
```

`F _` is legal only where a higher-kinded generic parameter is declared,
including a trait target binder, a nominal declaration header, a type-lambda
binder, or a kind-declaring where-clause subject. Application remains
left-associative exactly as for ordinary types, so `F _, _` declares one binary
constructor parameter. Parentheses delimit adjacent declaration parameters
when needed; they do not change constructor arity:

```rock
struct Compose (F _), (G _), A
```

The holes appear only at the declaration site. Once a constructor parameter is
declared, later bounds and references use its name normally, as in `F: Functor`
and `F A`.

Ordinary function, method, and impl generics remain implicit, preserving
existing Rock syntax and behavior:

```rock
identity: T -> T

impl Functor for Result _, E

trait Functor for F _
    fmap: M -> F A -> F B where M: FnMut A, B
```

As today, unresolved uppercase type names in declarations introduce ordinary
generic parameters of kind `Type`. Higher kinds are never guessed from use:
applying an implicitly introduced ordinary generic as a constructor is a kind
error with a diagnostic explaining how to declare its constructor kind.

A constructor generic on a function or method is declared by annotating its
where-clause subject. The same form may carry a trait bound or stand alone when
the constructor is unbounded:

```rock
apply_f: F A -> A where F _: Functor
constructor_identity: F A -> F A where F _
```

Generic discovery order remains the existing source-defined semantic ordering
for `GenericParamId` allocation, artifacts, and monomorphization. Explicit
constructor binders participate in that ordering at their declaration site.

Higher-order kinds use an explicit parenthesized kind annotation:

```rock
(H: (Type -> Type) -> Type)
```

Examples:

```rock
struct Compose (F _), (G _), A

apply_f: F A -> A where F _: Functor

trait Natural G _ for F _
```

A constructor-level trait has exactly one distinguished implementation target,
introduced after `for`. Additional ordinary or constructor parameters remain
trait generic parameters before `for`, as `G _` is above. Multi-target trait
headers are not part of the language.

The formatter uses `F _` and `F _, _` for constructors whose inputs and result
are all `Type`. It preserves parentheses that delimit adjacent declaration
parameters, as in `(F _), (G _), A`, and uses the explicit annotation for
higher-order kinds. `Type` is reserved only in kind grammar.

## Type Expression Syntax

### Application

Existing Rock application remains valid:

```rock
Option I64
Result I64, IoError
F A
F (G A)
```

Partial applications are legal when the context expects a constructor:

```rock
Result          // Type -> Type -> Type
Result I64      // Type -> Type
```

An unsaturated constructor is rejected in a runtime value position.

### Explicit Type Lambdas

Rock adds type lambdas in type-expression contexts:

```rock
\T -> Result T, E
\T -> Pair T, T
\A, B -> Either A, B
\(F _), T -> F T
```

Type lambdas are compile-time terms and cannot be recursive. Their binders may
have ordinary or constructor kinds. Parentheses are required when a lambda is
immediately applied:

```rock
(\T -> Result T, E) I64
```

### Type Sections

An underscore inside a type application introduces a type-lambda binder:

```rock
Result _, E       == \T -> Result T, E
Pair _, _         == \A, B -> Pair A, B
Compose F, G, _   == \T -> Compose F, G, T
```

Each `_` is fresh. Repeated use of one parameter requires an explicit lambda:

```rock
\T -> Pair T, T
```

Sections are accepted anywhere a constructor term is expected, including impl
targets, constructor arguments, aliases, and explicit type arguments. A section
whose resulting kind is not expected produces a kind diagnostic rather than an
ordinary type mismatch.

### Transparent Constructor Aliases

Type aliases may name constructor terms:

```rock
type ResultWith E = \T -> Result T, E
type Composed (F _), (G _) = \T -> F (G T)
```

Aliases are transparent for kind checking, type equality, coherence, and
monomorphization. Alias expansion is ID-based, cycle-checked, and normalized.
Aliases never create a distinct runtime representation.

Every alias is a first-class declaration with a `DefId`, ordered
`GenericParamDecl`s, a parsed/lowered body, visibility, module ownership, and an
interface record. Add `HirTypeAlias` and `ProductTypeAliasInterface` rather than
continuing to parse `TopLevel::NewType` and then dropping it during collection.
Alias exports, imports, dependency loading, ID remapping, and diagnostics follow
the same canonical declaration path as other type-level items.

### Precedence

From tightest to loosest:

1. Type atoms and parenthesized types.
2. Type application.
3. Function type `->`.
4. Type-lambda body after `\... ->`.

Commas delimit sibling type arguments at the current application level. The
parser and formatter must roundtrip all ambiguous-looking combinations.

## Parsed And Semantic Representation

The AST must distinguish declarations from applications. `ParseTypeInner` must
not continue serving simultaneously as a nominal reference, generic binder
list, and application node.

The parsed model gains dedicated forms equivalent to:

```rust
pub struct ParsedGenericParam {
    pub name: Ident,
    pub kind: ParsedKind,
}

pub enum ParseType {
    // Existing structural forms.
    Name(ParsedPath),
    Apply {
        constructor: Box<ParseType>,
        args: Vec<ParseType>,
    },
    Lambda {
        params: Vec<ParsedGenericParam>,
        body: Box<ParseType>,
    },
    Hole(Span),
    // Function, tuple, reference, pointer, slice, array, projection, unit...
}
```

Exact Rust names may differ, but the distinctions are mandatory. Holes may
exist only in parsed types. They must be desugared before semantic HIR.

The semantic type term adds forms equivalent to:

```rust
pub enum Type {
    // Existing runtime and inference forms.
    Constructor {
        id: DefId,
        flavor: NominalTypeKind,
    },
    Apply {
        constructor: Box<Type>,
        args: Vec<Type>,
    },
    Lambda {
        params: Vec<Kind>,
        body: Box<Type>,
    },
    BoundVar {
        depth: u32,
        index: u32,
        kind: Kind,
    },
}
```

Bound variables use de Bruijn identity so alpha-equivalent lambdas are
structurally equal. Source names are retained separately for diagnostics.

Known saturated nominal applications normalize to the existing forms:

```rust
Type::Struct { id, args }
Type::Enum { id, args }
```

This preserves the current layout, drop, MIR, and codegen model. Generic or
unsaturated applications remain `Apply` until substitution makes them concrete.

Associated type declarations also receive a kind. `type Output` defaults to
`Type`; `type Family _` has kind `Type -> Type`. A projection may therefore be
a constructor and may be applied later. This is higher-kinded associated-type
support, not lifetime- or const-generic GAT support.

An abstract projection remains a `Projection` term with its declared kind and
may be the head of `Apply`. Once exact impl authority and substitutions are
known, projection normalization selects the associated definition by trait,
impl, and `AssocTypeId`, substitutes its generic descriptors, and normalizes the
result. Constructor projections are not reduced by display name. Coherence impl
heads may not contain unresolved projections. Before MIR, every projection used
inside an executable runtime type must have resolved to a concrete kind-`Type`
term; an unresolved applied projection is a specialization error.

## Type Traversal Infrastructure

Before adding semantic variants, introduce central type traversal and folding
facilities under `lib/src/type_services/`. They must cover:

- immutable visiting;
- mutable visiting;
- capture-avoiding substitution;
- generic and inference-variable collection;
- DefId remapping;
- kind queries;
- normalization.

Semantic matches that genuinely depend on a concrete shape remain explicit.
Mechanical recursive walkers must use the shared traversal facilities. This is
required because the current compiler has many independent recursive matches
over `Type`; adding HKT variants without central traversal would make future
omissions likely.

## Normalization And Definitional Equality

Type normalization is an owned type service, not ad hoc logic in inference,
selection, mono, or artifacts.

The normalizer performs:

1. Transparent alias expansion.
2. Flattening nested applications.
3. Beta reduction of applied type lambdas.
4. Capture-avoiding substitution using de Bruijn shifts.
5. Eta reduction where `\T -> F T` is equivalent to `F` and `T` is not free in
   `F`.
6. Saturation of known nominal constructors into `Struct` or `Enum` forms.
7. Recursive normalization of projection inputs and trait arguments.

Applying a lambda with `k` binders to `n` arguments is defined as follows:

- when `n < k`, substitute the first `n` binders, shift bound indices
  capture-safely, and return a lambda over the remaining `k - n` binders;
- when `n == k`, substitute all binders and normalize the body;
- when `n > k`, substitute the first `k`, normalize the body, then apply the
  remaining arguments to that result;
- every substitution checks the argument kind before rewriting the body.

Eta reduction runs inside nested lambdas after beta normalization. It removes
only a trailing application of the lambda's own distinct binders when those
binders are not free in the constructor being applied. This keeps reduction
terminating and avoids changing repeated-parameter semantics.

Normalization has cycle detection and a deterministic expansion-depth budget.
Recursive aliases produce a source diagnostic; malformed artifact cycles
produce an artifact-validation error.

Exceeding a cycle/depth/size limit is a hard normalization error. The compiler
must not retain the partially normalized term in an equality, interning,
coherence, selection, artifact, or instance-key store.

Canonicalization boundaries are:

- after type lowering;
- after inference substitution;
- before type interning;
- before impl overlap comparison;
- before instance-key construction;
- after artifact decode and DefId remapping.

`Type` equality and hashing must never depend on opportunistically normalizing
one side inside an arbitrary caller. Values admitted to canonical stores are
already normalized. Debug-only validation may assert this invariant.

Examples of required equality:

```text
Result _, E
\T -> Result T, E
\U -> Result U, E

Option
\T -> Option T
```

Each group has one canonical semantic identity.

## Kind Checking

Kind checking precedes ordinary value-type constraint solving for declaration
headers and explicit type annotations.

Rules include:

- Primitive and fully applied runtime types have kind `Type`.
- A generic parameter has its declared kind.
- A nominal constructor has the kind derived from its declaration.
- Applying `C` of kind `K1 -> K2` to `A` requires `A: K1` and yields `K2`.
- Applying a non-arrow kind is an error.
- A type lambda has the arrow kind formed from its binders and body.
- A function parameter, local, field, variant payload, expression result,
  extern ABI type, or final return type must have kind `Type`.
- A trait or impl target must have the self kind declared by that trait.
- Trait arguments must match their declared generic kinds.
- Associated projections have the kind stored on their associated declaration.
- No unresolved kind variable may survive collection/lowering.

Kind errors are emitted before secondary type mismatch errors whenever the kind
failure is the root cause.

Every runtime type classifier (`TypeFacts::is_concrete`, `is_copy`, `Sized`,
layoutability, ABI classification, drop requirements, and similar helpers) must
match HKT variants explicitly. No wildcard branch may classify a newly added
constructor/application/lambda term as a concrete runtime type.

## Kind-Aware Inference

`InferenceEngine` continues to distinguish rigid declaration generics from
fresh call-site inference variables. Every inference variable now has a kind:

```rust
pub struct InferenceVarData {
    pub kind: Kind,
    pub span: Option<Span>,
}
```

Existing `fresh_type_var()` remains a convenience for kind `Type`. New APIs
create constructor variables with an explicit kind.

Unification first normalizes both sides and checks equal kinds. It then applies
first-order structural rules plus a restricted constructor-pattern rule.

Supported constructor inference includes:

```text
?F ?A  ~ Option I64
=> ?F = Option, ?A = I64

?F (?G A) ~ Vec (Option I64)
=> ?F = Vec, ?G = Option, A = I64

?F I64 ~ Result Bool, I64
=> ?F = Result Bool
```

Inference does not synthesize a type lambda to solve equations such as:

```text
?F I64 ~ Result I64, Bool
```

That equation has no rigid-prefix constructor solution. It could be solved by
inventing `\T -> Result T, Bool`, but Rock reports that lambda synthesis is not
performed and requires the explicit constructor `Result _, Bool`.

The permitted solver is deterministic constructor-spine pattern matching:

- for `?F P1 ... Pn ~ C A1 ... Am`, `C` must be a rigid nominal, rigid generic,
  or selected projection head and `m >= n`;
- the last `n` actual arguments are unified with `P1 ... Pn` in order;
- `?F` binds to the canonical rigid prefix `C A1 ... A(m-n)` when those
  argument unifications succeed;
- no constant-function or other lambda alternative is considered, making the
  selected solution principal within this restricted language;
- application heads and arguments are recursively decomposed by the same rule;
- no metavariable may escape a lambda binder;
- occurs checks traverse applications, lambdas, projections, functions, and
  captures;
- rigid `GenericParamId`s never become mutable unknowns;
- unresolved constructor variables may be generalized only at the same
  declaration boundary as ordinary inferred generics;
- strict finalization rejects every remaining unresolved variable regardless
  of kind.

Numeric defaulting remains restricted to kind `Type` variables with literal
evidence. Constructor variables are never defaulted.

## Constructor-Level Traits

An ordinary trait keeps an implicit implementation target of kind `Type`:

```rock
trait Show
```

A constructor-level trait declares and names a target constructor:

```rock
trait Functor for F _
```

The target binder `F` is available in member signatures. Because a constructor
is not a runtime value, constructor-level traits declare static members. The
container value is explicit:

```rock
trait Functor for F _
    fmap: M -> F A -> F B where M: FnMut A, B
```

Constructor impls use the existing `for` concept with a target of the required
kind:

```rock
impl Functor for Option

impl Functor for Result _, E
```

Inside the second impl, `E` is an impl generic of kind `Type`, and the target is
the canonical lambda `\T -> Result T, E`.

Static member lookup is available through the constructor:

```rock
Option::pure 42
F::pure value
```

The qualifier must have the trait's exact target kind. `Result` itself has kind
`Type -> Type -> Type`, so it cannot select unary `Applicative` members.
Partially applied and section constructors use a parenthesized owner:

```rock
(Result _, IoError)::pure 42
(Result _, E)::Applicative::pure value
```

A constructor alias may provide a shorter owner when used repeatedly.

If more than one trait contributes the same static member, qualification is:

```rock
Option::Applicative::pure 42
F::Functor::fmap mapper, value
```

Inherent static members take precedence only in the short form. Qualified
lookup always selects the named trait by canonical ID. Lowered HIR records the
exact trait/member identity and substitutions. A concrete constructor call also
records the exact impl; a constructor-generic bound call records deferred
trait-bound authority until monomorphization supplies the constructor.

Constructor-level traits are ordinary source traits. They are not language
items and are never discovered by spelling.

## Generalized Predicates And Supertraits

The current string-subject where-clause representation is insufficient. Replace
it with typed predicates whose subject may have any kind:

```rust
pub enum Predicate {
    Trait {
        subject: Type,
        trait_id: DefId,
        args: Vec<Type>,
    },
}
```

Kind constraints belong to generic declarations and are not encoded as fake
traits.

Trait declarations may have predicates. This supplies ordinary supertrait
semantics:

```rock
trait Applicative for F _ where F: Functor

trait Monad for F _ where F: Applicative
```

Implementing `Applicative` requires a selected `Functor` impl for the same
canonical constructor. Implementing `Monad` similarly requires `Applicative`.
Supertrait obligations are stored by ID, serialized, checked at declaration and
artifact load boundaries, and made available to generic selection.

The supertrait graph must be acyclic. Collection rejects a cycle with the full
trait-ID/source path. Artifact validation repeats that check after remapping.
The obligation solver also keeps a canonical obligation stack and memo table so
malformed or mutually recursive dependency input cannot recurse indefinitely.

## Trait Selection And Conformance

`HirTrait` gains its target binder and target kind. `HirImpl` gains a semantic
impl target rather than encoding every owner as a runtime receiver pattern.
The representation distinguishes:

```rust
pub enum HirImplTarget {
    Value(HirImplReceiverPattern),
    Constructor(Type),
}
```

Exact names may differ, but constructor impls must not be represented as fake
value receivers or display-name owners.

Constructor selection:

1. Normalize the requested constructor.
2. Resolve the trait by `DefId`.
3. Verify the requested constructor kind equals the trait target kind.
4. Match canonical impl targets and infer impl generic substitutions.
5. Prove impl predicates and supertrait obligations.
6. Require exactly one candidate for a concrete constructor.
7. Record exact impl authority for concrete calls.

For a generic constructor declared as `F _` and subsequently constrained by
`F: Applicative`, accepted HIR records deferred exact trait/member authority
plus the constructor term and substitutions. It does not invent an impl ID.
After monomorphization substitutes a concrete constructor, the same selection
service resolves one impl and rewrites the call to exact impl/member instance
authority before MIR. Missing or ambiguous concrete impls are mono diagnostics
and cannot reach MIR.

HIR represents that distinction explicitly, equivalent to:

```rust
pub enum HirConstructorCallAuthority {
    ConcreteImpl {
        trait_id: DefId,
        member_id: DefId,
        impl_id: DefId,
        target: Type,
        substitutions: Vec<HirTypeBinding>,
    },
    DeferredTraitBound {
        trait_id: DefId,
        member_id: DefId,
        target: Type,
        substitutions: Vec<HirTypeBinding>,
    },
}
```

Accepted generic HIR permits either valid form. Monomorphized HIR and MIR permit
only `ConcreteImpl`/resolved instance authority.

Static trait functions monomorphize through the same `InstanceRegistry` as
other generic functions. No dictionary is built or passed.

Conformance substitutes the constructor target into the trait signatures, then
checks kind, generic binder order, parameter types, return type, safety, and
callable bounds. Alpha-renamed type lambdas must conform identically.

## Coherence And Orphan Rules

Production HKT requires declaration-time coherence. Selection-time ambiguity is
not an acceptable primary overlap policy.

Constructor impl headers use a deliberately restricted, decidable grammar
after alias expansion and normalization:

- the target is a constructor or an outer type lambda;
- the lambda body has a concrete nominal constructor head;
- arguments may contain impl generics as atomic first-order variables,
  lambda-bound variables, concrete types, and applications whose heads are
  concrete nominal constructors;
- unresolved inference variables and associated projections are forbidden;
- a generic constructor may not be the outer head;
- a generic constructor may not be used as an application head inside the impl
  target; it may occur atomically as a constructor-kind nominal argument;
- matching is first-order unification of the normalized lambda bodies.

This grammar covers `Option`, `Result _, E`, nested nominal composition, and
aliases while avoiding higher-order overlap solving.

For overlap checking, corresponding lambda-bound variables from both headers
are replaced by the same rigid skolems by binder position and kind. Impl generic
parameters are fresh flexible unification variables scoped to their own impl.
Concrete nominal constructors and ordinary concrete types are rigid symbols.
Repeated use of one impl variable imposes equality through ordinary first-order
unification. For example, `\T -> Wrap E, T` overlaps
`\T -> Wrap I64, T` with witness `E = I64`. An attempted target containing
`F T` for impl-generic constructor `F` is rejected by the header grammar rather
than sent to higher-order matching.

The following rules apply to all trait impls after this work, including existing
kind-`Type` impls:

1. An impl is legal when the trait is local or the normalized impl target's
   outermost nominal constructor is local.
2. Transparent aliases and type lambdas are expanded before locality is tested.
3. Generic parameters and references do not count as local outer constructors.
4. A foreign trait for a foreign constructor is rejected.
5. Two impls whose normalized headers can match one common substitution are
   rejected as overlapping.
6. Predicates do not make overlapping heads disjoint in the first version.
7. No specialization, priority, or negative reasoning is used.
8. Constructor-variable blanket targets are rejected in the first version,
   even for local traits. Concrete nominal heads or sections over concrete
   nominal heads are required.
9. Coherence includes current-source impls and every impl interface loaded from
   the complete transitive set of explicit dependency artifacts.
10. Diagnostics identify both impl IDs, source spans when available, normalized
    targets, and the witness substitution demonstrating overlap.

For ordinary kind-`Type` impls, a structural target such as a tuple, function,
array, slice, pointer, or projection has no local nominal constructor. Such an
impl is legal only when the trait is local. Existing stdlib slice/pointer impls
remain legal because their traits are local to that crate.

Examples:

```text
Result _, E
\T -> Result T, E
```

are the same impl target and conflict.

The overlap checker belongs before accepted HIR and must be shared by current
source and artifact validation. Mono and selection retain defensive ambiguity
checks but must not normally discover a new overlap.

## Phase Invariants

### Parser And AST

- Names, holes, explicit lambdas, binder kinds, and source spans are preserved.
- No canonical IDs or inferred kinds are fabricated.

### Collection

- Every declaration and child has its existing canonical ID.
- Every generic parameter has one `GenericParamDecl` with a canonical owner,
  index, name, and kind.
- Nominal, trait, associated-type, alias, and impl target kinds are known.
- Constructor impl headers and typed predicates are collected by ID.
- Orphan and overlap validation can run over complete local plus dependency
  interfaces.

### Lowered Partial HIR

- All type names and aliases are resolved by canonical ID.
- Parsed holes are gone.
- Type lambdas use bound-variable identity.
- Type terms are normalized and kind-correct.
- Body inference may still contain kinded `TypeVar`s.

### Accepted HIR

- No `TypeVar` or `Error` exists.
- Every type term is normalized.
- Every value-bearing type location has kind `Type`.
- Generic declarations may retain `Generic`, `Apply`, and `Lambda` terms.
- Constructor trait calls carry concrete impl authority or explicit deferred
  trait-bound authority according to whether their target is concrete.
- Every impl passed coherence and conformance checks.

### Monomorphized HIR

- Every reachable generic parameter, including constructor parameters, has a
  canonical `TypeId` substitution.
- Beta reduction and nominal saturation have run after substitution.
- Runtime expression types are concrete.
- A closed runtime nominal may retain constructor-kind generic arguments in its
  canonical instance identity, because those arguments determine its resolved
  fields, variants, drop glue, and impl selection. Every field, variant, ABI,
  place, and expression type derived from that nominal has kind `Type`.
- Constructor terms otherwise remain only in compile-time instance metadata.

### MIR Barrier

The current pipeline order remains:

```text
accepted HIR -> monomorphized HIR -> MIR -> borrow checking -> agreement -> LLVM
```

Borrow checking therefore consumes monomorphized MIR, not generic HIR. This
feature must not reorder the pipeline.

MIR rejects as an executable value/place/ABI type:

- `Generic`;
- unsaturated `Constructor`;
- `Apply` not normalized to a runtime type;
- `Lambda`;
- `BoundVar`;
- `TypeVar` and `Error`.

Closed constructor-kind arguments nested in a saturated nominal instance are
allowed only as compile-time identity metadata. The MIR backend contract must
already contain the fully resolved kind-`Type` field/variant/layout/drop facts
for that instance. MIR operations never project, apply, normalize, or request a
layout for those nested constructor arguments.

Nominal generic layout templates remain declaration metadata only. MIR and
codegen never invent layouts for constructor terms.

### Codegen

LLVM codegen continues to consume concrete MIR plus the backend contract. It
does not normalize HKT terms, select constructor impls, inspect `Functor`, or
carry runtime constructor evidence. It may use the opaque `TypeId` of a closed
nominal instance to look up already-resolved backend-contract facts; it must not
inspect constructor-kind arguments to derive those facts.

## Type Context And Instance Identity

`TypeContext` interns canonical type terms of any kind. Runtime layout APIs must
explicitly require kind `Type`; a `TypeId` alone does not imply layoutability.

`InstanceKey.substitution` continues to use `Vec<TypeId>`. Constructor
arguments therefore participate in instance identity without a parallel key
system. Before key construction:

- every substitution term is normalized;
- each argument kind is checked against its `GenericParamDecl`;
- alpha/beta/eta-equivalent constructors intern to the same `TypeId`.

`TypeId` is session-local and is never a persisted or backend-symbol identity.
Backend symbol suffixes use a stable `CanonicalTypeFingerprint` computed from
the normalized term, declaration/product crate identities, local definition
IDs, kinds, and binder indices. The fingerprint must be independent of source
aliases, pretty-printing, interning order, dependency load order, and producer
or consumer `TypeId` allocation.

`compile_impl` constructs one `CompilationIdentityContext` before product
emission and monomorphization. It contains the current crate's canonical product
identity and every dependency product identity. The current source fingerprint
is computed once even when product emission is disabled. The same identity
context is passed to product construction and `Monomorphizer`; optional artifact
emission must not be the phase that creates backend identity.

Monomorphization extraction and substitution must recurse through constructor
applications and projections. A generic function specialized with `F = Option`
and one specialized with `F = \T -> Option T` are the same instance.

## Products And Artifacts

This is an incompatible product artifact schema change. Increment
`PRODUCT_ARTIFACT_FORMAT_VERSION` from `43` to `44` in both:

- `lib/src/products.rs`;
- `rock-shared/src/sysroot.rs`.

Format 43 is rejected. There is no compatibility decoder.

Format 44 uses an envelope whose identity/dependency header precedes the type
table and product payload:

```rust
pub struct ProductArtifactPreamble {
    pub magic: [u8; 8],
    pub format_version: u32,
    pub header_len: u64,
    pub payload_len: u64,
}
```

The bounded header contains crate identity, target/freshness metadata, and
dependency identities/paths needed to determine load order. It contains no
semantic type table. The payload contains portable IDs, types, interfaces, and
bodies. Preamble lengths are checked for overflow and exact file-size agreement
before either section is decoded. Dependency discovery reads only the preamble
and header.

Untrusted artifact limits are compiler constants in format 44:

```text
maximum artifact bytes              512 MiB
maximum header bytes                  4 MiB
maximum individual UTF-8 string       1 MiB
maximum total decoded string bytes    64 MiB
maximum type-table rows             1,000,000
maximum declarations/impls          1,000,000
maximum generic params per owner        1,024
maximum type/lambda nesting depth          256
maximum normalization output nodes   4,000,000
```

The decoder must reject a declared length/count before allocating from it.
Using a byte-limited bincode reader alone is insufficient if a collection length
can trigger allocation first; collection fields require bounded decode seeds,
manual length checks, or an equivalently safe codec. All limit failures are
structured artifact errors containing the limit category, declared value, and
allowed maximum.

Path-based loading checks filesystem metadata against the artifact-byte limit
before reading a file-sized buffer. Production APIs are split into bounded
header reading and bounded portable-payload reading. Dependency discovery,
freshness checks, and CLI dependency printing use only the header API; semantic
crate loading uses the payload API after the remap environment exists. The old
eager whole-file `read_artifact_from_path` behavior must have no production
callers.

Product interfaces replace `Vec<String>` generic metadata with serialized
generic descriptors containing product-local owner IDs and kinds. This applies
to functions, structs, enums, traits, signatures, aliases, impl-level generics,
method-level generics, and associated type constructors. Constructor-trait
members also persist whether they are static, and generic bound calls persist
their deferred trait/member authority without a fabricated impl ID.

`ProductTypeRow` gains portable rows for:

- nominal constructors;
- application;
- type lambda;
- bound variable;
- higher-kinded generic parameter references.

Constructor impl targets, trait target binders, typed predicates, supertraits,
and constructor substitutions are serialized explicitly. Every embedded
`DefId` uses product-local identity and participates in remapping.

Artifact validation rejects:

- malformed kinds or applications;
- out-of-scope bound variables;
- non-canonical or cyclic type lambdas/aliases;
- unknown constructor IDs;
- kind-mismatched generic arguments;
- unresolved inference/error types;
- illegal orphan impls;
- local/dependency overlap;
- malformed constructor trait method signatures;
- unsatisfied supertrait declarations.

Artifact loading becomes explicitly staged. Raw product-local terms must not be
eagerly converted into consumer `Type` values before ID remapping:

1. Read and reject unsupported versions before full deserialization.
2. Bounded-decode portable DTOs and validate table references, lambda binder
   scope, finite structure, and required identity records while IDs are still
   product-local.
3. Remap all product-local declaration identities into the consumer crate
   identity environment.
4. Resolve the remapped alias/declaration environment and normalize terms.
5. Validate kinds, canonical form, generic descriptors, predicates,
   supertraits, orphan rules, and coherence against all loaded interfaces.
6. Intern validated canonical terms into the consumer `TypeContext` and
   materialize `CompilerProducts`/extern stores.

`CompilerProducts::from_artifact_bytes` must be split internally so a portable
decoded artifact can survive until the loader has the remap environment.
Header/freshness inspection may read a bounded header without materializing
semantic types. Producer `TypeId`s are never serialized.

## Stdlib Type-Class Hierarchy

These are ordinary exported Rock traits in a new module such as
`stdlib/functor.rk`. The final module split may use one file per trait if that
keeps imports acyclic.

### Functor

```rock
trait Functor for F _
    fmap: M -> F A -> F B where M: FnMut A, B
```

Required laws:

```text
fmap identity == identity
fmap (compose f, g) == compose (fmap f, fmap g)
```

These are extensional laws over pure, total callbacks. `FnMut` is the operational
capability needed by an eager implementation; observable mutation or external
effects inside a callback are governed by the documented left-to-right runtime
order and are not claimed to be preserved by algebraic rewrites such as map
fusion.

`fmap` consumes `F A`. Borrowed mapping is a separate concrete operation and
never inserts implicit clones.

### Applicative

```rock
trait Applicative for F _ where F: Functor
    pure: A -> F A
    ap: F (A -> B) -> F A -> F B
```

The stdlib may provide derived `lift2`, `keep_left`, and `keep_right` helpers.
The primitive contract remains `pure` plus `ap`.

Required laws are identity, composition, homomorphism, and interchange.

Rock evaluates arguments left-to-right before the call. An applicative cannot
retroactively suppress effects that occurred while constructing an argument.
This strict evaluation rule is part of the operational contract.

### Monad

```rock
trait Monad for F _ where F: Applicative
    bind: F A -> M -> F B where M: FnMut A, F B
```

The stdlib provides `join` and `flatten` in terms of `bind` where useful.

Required laws are left identity, right identity, and associativity. `bind`
defines sequencing and may short-circuit later callback invocation.

### Foldable

```rock
trait Foldable for F _
    foldl: S -> B -> F A -> B where S: FnMut (B, A), B
```

The contract is strict, consuming, left-to-right, and stack-safe for finite
stdlib collections. Derived operations include `for_each`, `length`, `any`,
`all`, and `find` where their ownership can be stated without cloning.

### Traversable

```rock
trait Traversable for T _ where T: Functor, T: Foldable
    traverse: V -> T A -> G (T B)
        where G _: Applicative, V: FnMut A, G B
```

The stdlib provides `sequence` from `traverse identity`.

`traverse` is eager and invokes the visitor exactly once for each visited
element in container order. With only an `Applicative G` bound, it does not
promise that a callback will be skipped after an already-produced value later
represents failure. A separate `traverse_m` helper declaring
`where G _: Monad` sequences through `bind` and inherits that monad's
callback-short-circuit behavior.

`sequence` has the same eager, non-short-circuiting callback-construction rule
as `traverse`. Users requiring fail-fast callback suppression use `traverse_m`
or concrete Result-aware collection operations.

Traversable laws are identity, composition, and naturality. Law tests use pure,
total callbacks and extensional value equality. They do not claim that two
law-equivalent formulations have identical externally visible callback effects;
the separate operational contract above fixes actual callback order.

## Initial Lawful Implementations

### Option

`Option` implements `Functor`, `Applicative`, `Monad`, `Foldable`, and
`Traversable`.

- `pure` is `Some`.
- `ap` propagates `None`.
- `bind` invokes its callback only for `Some`.
- fold/traverse visit zero or one element.

### Result

For each fixed error type `E`, the constructor `Result _, E` implements
`Functor`, `Applicative`, `Monad`, `Foldable`, and `Traversable`.

- `pure` is `Ok`.
- `fmap` transforms only `Ok`.
- `ap` preserves deterministic left-to-right error precedence.
- `bind` invokes its callback only for `Ok`.
- the error type remains unchanged.

There is no compiler branch for `Result`. Its implementation target is the
ordinary normalized type lambda:

```text
\T -> Result T, E
```

### Vec

`Vec` implements `Functor`, `Foldable`, and `Traversable`.

- `fmap` consumes the vector, moves each element once, preserves order, and
  returns a fully allocated result.
- `foldl` consumes elements in index order.
- `traverse` constructs the output eagerly and preserves order.

`Vec` does not initially implement `Applicative` or `Monad`. A Cartesian
applicative duplicates values under Rock ownership, while a zipped applicative
does not match list-monad semantics. Adding constraints such as `Clone` to the
impl would change the trait contract. Shipping no impl is more lawful than
shipping a surprising or internally inconsistent one.

## Eager Collection Operations

HKT abstractions do not replace useful concrete ownership-sensitive APIs.
`Vec` also gains:

```rock
~@map: M -> Vec U
@map_ref: M -> Vec U
@for_each: A -> Unit
~@for_each_owned: A -> Unit
^@retain: P -> Unit
~@filter: P -> Vec T
~@filter_map: M -> Vec U
~@try_map: M -> Result (Vec U), E
@try_for_each: A -> Result Unit, E
```

Exact callable bounds are chosen according to invocation and ownership:
multi-call operations use `FnMut`; consuming callbacks may accept owned values;
borrowed variants accept `&T`.

All operations are eager and stack-safe. `Vec::map` must transfer element
ownership without double-drop, preserve order, and leave the consumed source
with zero live elements before its storage is released. No operation silently
clones an element.

Arrays and slices receive concrete eager APIs only when their output and
borrowing rules can be represented soundly. Fixed-array `Functor` is deferred
because abstracting `[T; N]` as a unary constructor also requires const-kinded
parameters, which are outside this specification.

## `new_new` Transformation Scope

`test_projects/new_new/main.rk` is an integration proof, not the owner of
generic abstractions.

Expected transformations include:

```rock
@snapshot = ->
    self.clients.map_ref (connection ->
        clone_writer (&connection.writer))
```

Broadcast over an owned snapshot uses eager `for_each`; removal uses `retain`.
Argument dispatch may use existing `Option`/`Result` combinators where that
improves clarity.

Socket read loops are open-ended effectful stream processing, not finite
functor mapping. They remain loops until a separate eager stream-fold API has a
sound cancellation, error, and ownership contract. This feature must not hide
those loops behind a lazy iterator facade.

## Ownership And Effects

- HKT changes type abstraction, not value ownership.
- `Functor::fmap` consumes the container in the common trait contract.
- Implementations move each contained value according to ordinary Rock rules.
- Borrowed variants are explicit concrete APIs.
- No trait operation inserts `Clone`, `Arc`, boxing, or reference counting.
- `FnMut` is used when a callback may be invoked multiple times.
- `FnOnce` is sufficient only when the abstraction guarantees at most one call.
- Evaluation order for stdlib functional operations is deterministic and
  left-to-right unless a type's public contract explicitly says otherwise.
- User callback side effects are observable; law documentation assumes
  extensional equality but runtime tests also verify evaluation order.

## Diagnostics

Required diagnostics include:

- `expected a type of kind Type, found Type -> Type`;
- `F is a type, not a type constructor`;
- `constructor Result expects 2 arguments but received 3`;
- `unsaturated constructor Result cannot be used as a runtime value type`;
- `type hole is only valid inside a constructor section`;
- `cannot infer constructor F; add an annotation or explicit constructor`;
- `constructor inference would require an ambiguous type lambda`;
- `type lambda parameter escapes its scope`;
- `constructor impl target has kind ..., but trait ... requires ...`;
- `constructor trait ... may only declare static members`;
- orphan and overlap diagnostics with both impl locations;
- malformed artifact kind/application/lambda diagnostics;
- MIR barrier diagnostics naming the phase that failed to specialize a
  constructor term.

Diagnostics use source names for readability and canonical IDs for notes when
identity is relevant. They never recover by treating an unknown constructor as
kind `Type` or by defaulting it to a stdlib type.

## Testing Strategy

### Syntax And Formatting

- `F _`, `F _, _`, adjacent parenthesized constructor parameters, and explicit
  higher-order kind annotations;
- explicit type lambdas and multi-binder lambdas;
- type sections in every argument position;
- application/lambda/function precedence;
- trait and impl constructor headers;
- method-level constructor bounds;
- higher-kinded associated type declarations;
- formatter idempotence;
- tree-sitter parse corpus and generated grammar.

### Kinds And Normalization

- nominal kind derivation;
- kind-correct and kind-invalid applications;
- unsaturated runtime rejection;
- beta, eta, and alpha equivalence;
- nested constructor composition;
- alias expansion and cycle rejection;
- section canonicalization;
- projection kinds;
- normalization idempotence;
- equal canonical terms intern to one `TypeId`.

### Inference

- infer `F = Option` and nested constructor heads;
- explicit `Result _, E` constructor arguments;
- reject ambiguous lambda synthesis;
- kind-aware occurs checks;
- generalize constructor variables with stable IDs and kinds;
- reject unresolved constructor variables during strict finalization;
- preserve ordinary numeric defaulting and generic inference.

### Traits And Coherence

- constructor trait conformance;
- static constructor member selection by exact IDs;
- constructor-qualified generic calls;
- supertrait obligations;
- local-trait and local-constructor orphan cases;
- foreign/foreign rejection;
- alpha/beta/eta-equivalent overlap rejection;
- cross-crate overlap rejection;
- no selection by display spelling.

### Mono, MIR, And Codegen

- constructor substitutions participate in instance reuse;
- equivalent lambdas reuse one instance;
- nested HKT generic functions specialize correctly;
- dependency generic bodies specialize from artifacts;
- no constructor operation or unspecialized term crosses the MIR barrier;
- generated LLVM contains ordinary concrete Option/Result/Vec layouts and calls;
- no dictionary parameter or HKT runtime object is emitted.

### Artifacts

- format 44 contract in both crates;
- format 43 rejection before full decode;
- kind and generic descriptor roundtrips;
- constructor/application/lambda/bound-variable type-table roundtrips;
- constructor impl and supertrait roundtrips;
- DefId remapping inside constructor terms;
- malformed bound scope, kind, cycle, and overlap rejection;
- envelope length overflow/mismatch and every format-44 resource-limit
  rejection before unbounded allocation;
- cross-crate Functor/Monad selection and specialization.

### Laws And Runtime Behavior

- Functor identity and composition for every impl;
- Applicative identity, composition, homomorphism, and interchange;
- Monad left identity, right identity, and associativity;
- Foldable order and strictness;
- Traversable identity, composition, naturality, and order;
- Option/Result short-circuit callback behavior for `bind` and `traverse_m`;
- Result left-to-right error precedence;
- Vec map order, empty input, move-only values, captured mutable callbacks, and
  no double-drop;
- large Vec operations are stack-safe;
- `new_new` compiles and preserves observable behavior.

Law tests are executable examples over representative finite values. The
compiler does not attempt to prove user impl laws.

## Security And Robustness

Artifacts are untrusted input. Kind checking, bound-variable scope checking,
normalization limits, alias-cycle checks, and impl coherence run on decoded
interfaces before they enter semantic stores.

Normalization must not recurse without a depth guard. Diagnostic rendering of
malformed deeply nested types must also use bounded traversal. Hashing and
instance symbol generation operate on canonical bounded structures.

No malformed constructor term may reach layout computation, MIR agreement, or
LLVM. Those phases retain explicit rejection rather than relying only on earlier
validation.

## Compatibility Policy

This repository is in a prototyping phase. The implementation should migrate
the compiler and stdlib directly to the new canonical model:

- replace generic name vectors with descriptors rather than keeping parallel
  old/new metadata;
- do not preserve a leaf-only HKT compatibility representation;
- do not retain name-based impl fallback;
- do not decode artifact format 43;
- do not add deprecated `Fmap A, B` aliases as a transition layer.

Existing ordinary source syntax remains valid where it is unambiguous, but
internal deprecated representations are removed rather than adapted forever.

## Acceptance Criteria

The feature is complete only when all of the following are true:

1. Generic parameters carry canonical kinds in every compiler phase.
2. `F _` declarations and bounds parse, format, lower, and roundtrip with
   ordinary type-application associativity.
3. Explicit type lambdas and type sections normalize canonically.
4. `Result _, E` is implemented without a Result-specific compiler rule.
5. Alpha/beta/eta-equivalent constructors compare, intern, select, and
   monomorphize identically.
6. Kind-aware inference handles aligned constructor patterns and rejects
   ambiguous higher-order equations.
7. Constructor-level traits and static members select by canonical IDs.
8. Supertrait obligations work for constructor traits.
9. Orphan and overlap rules run before accepted HIR and across artifacts.
10. Associated type constructors carry and enforce kinds.
11. Constructor substitutions are part of canonical instance identity.
12. Product artifact format is 44 in `rock-lib` and `rock-shared`.
13. Format 43 artifacts are rejected without compatibility code.
14. Malformed artifact kinds, lambdas, and impls are rejected safely.
15. No HKT operation, unspecialized term, or runtime dictionary reaches
    executable MIR/codegen; closed constructor arguments are opaque nominal
    identity metadata only.
16. Stdlib Functor, Applicative, Monad, Foldable, and Traversable are ordinary
    source traits with no language markers.
17. Option and fixed-error Result have lawful initial implementations.
18. Vec has lawful eager Functor, Foldable, and Traversable implementations and
    intentionally lacks Applicative/Monad.
19. Concrete Vec operations cover borrowed and consuming traversal without
    implicit cloning or double-drop.
20. `new_new` uses the eager abstractions where semantically appropriate.
21. All focused, integration, artifact, full-suite, ordinary Clippy, rustfmt,
    tree-sitter, and diff gates pass serially. The pre-existing strict
    `-D warnings` baseline is tracked independently by `new_lang2-n2e`.
22. A production-source audit finds no name-based constructor selection,
    Result-specific HKT fallback, old generic-name-only semantic metadata, or
    codegen handling of unspecialized constructor terms.
23. Implicit ordinary function, member, and impl type variables retain existing
    source behavior, while all ordinary and explicit constructor variables have
    stable owner-scoped IDs and higher kinds are never inferred from use.
24. Generic constructor-bound calls carry deferred trait/member authority and
    acquire an exact impl only after constructor substitution.
25. Constructor aliases are collected, exported, serialized, remapped, cycle
    checked, and normalized as ID-owned declarations.
26. Supertrait cycles are rejected and obligation solving is cycle guarded.
27. Backend symbols use stable canonical type fingerprints, never session-local
    TypeIds or pretty-printed constructor terms.
28. Closed constructor arguments nested in runtime nominal identity never
    require layouts and all backend-contract field/variant facts are concrete.
29. Format-44 headers can be read without decoding type payloads, and every
    declared resource limit is enforced before untrusted allocation.

## Final Architectural Invariant

Kinds and type constructors are compile-time semantic data owned by the type
system. Collection assigns canonical generic and declaration identities.
Lowering resolves source type terms and normalizes type lambdas. Inference solves
only kind-correct, decidable constructor patterns. Coherence admits one legal
impl for each constructor-trait obligation. Selection records exact IDs.
Monomorphization substitutes constructors and reduces every runtime application
to an ordinary concrete type. Executable MIR and LLVM semantics never depend on
`Functor`, `Applicative`, `Monad`, or type-lambda operations; closed constructor
arguments may survive only as opaque nominal-instance identity metadata. The stdlib builds eager
functional abstractions entirely from those general language facilities.
