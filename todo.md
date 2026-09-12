## stdlib

implement Vec and other heap data structs
network and fs

## traits

Trait resolution

## Rock

Implement rock binary to mimic Cargo

## Misc

Better name resolution

## Artifacts

phase 2: split artifact handling into explicit subsystems for interface registration, generic cross-crate template transport, and concrete body ownership
teach rockc to resolve transitive product artifact dependencies from artifact metadata, so rock can pass only direct dependency artifacts while final linking still includes the full object closure

## Codegen

remove array and string builtin. might need proper pointer pointer arithmetic handling
remove builtins from closure code
remove array len and builtin tostring conversion from intrinsic, also remove string intrinsic
check why we dont have primitive specialisation for intrinsic operation (all int types are treated the same with to_int_value())
In operators, we still have BinOp operations when it should be handled in stdlib with the use of builtin llvm intrinsic. we need to remove hir BinOp altogether and have a operator precedence desugaring before lowering to hir
remove I64 fallback when generic/inference resolution fails in codegen type lowering and match codegen; unresolved generic shapes should not silently collapse to i64
fix enum/result LLVM shape handling so payload layout is variant-correct instead of assuming the first type argument is the payload type
fix generic enum method monomorphization so specialized impl bodies preserve enum self types everywhere, not only in the parameter list
fix higher-order generic method return typing for methods like Result.map_err and Result.fold so callable return types are concretized instead of leaking callable structs into final codegen
audit nested enum payload extraction in match codegen to ensure payload binding uses the actual variant field type, not a lossy generic fallback
fix custom operator lowering/inference for heterogeneous higher-order operators like >>= and reverse fmap so operator methods do not collapse to fresh scalars before chained method lookup
support explicit generic method signatures strongly enough that operator methods like @>>= and @<&> can describe their higher-order return types without relying on fragile inference
fix glob import/export of operator-named functions so re-exported operators like |> resolve through the same exact import path as ordinary function names
fix same-module function resolution so local helper calls in stdlib methods resolve lexically at the call site without requiring fully qualified module paths


## Loops

make loops as expressions that return new array

## Arrays

have a builtin immutable array type like slices in rust and a dynamic Vec type

## Mono

split into submodules
Investigate why we dont monomorphise method calls
capture and keep extending this list with newly discovered compiler and stdlib bugs during implementation sessions

## Resolve

Split into submodules
Why is the hir lowerer in resolve ??? also split
remove stdprelude from lowerer, use one stdlib prelude module and inject that
remove binop and useless intrinsic from infer
