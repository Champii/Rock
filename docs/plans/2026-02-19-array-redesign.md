# Array Redesign for C Compatibility

**Date:** 2026-02-19
**Status:** Approved

## Motivation

The current `Type::Array` implementation is a heap-allocated structure that is not compatible with native C arrays, making FFI with libc difficult. We need a design that:

1. Provides C-compatible fixed-size arrays for FFI
2. Supports stack allocation for performance-critical code
3. Follows Rust's proven design pattern
4. Keeps heap-allocated growable collections in stdlib (not compiler)

## Design Overview

### Three-Tier Architecture

| Type | Syntax | Location | LLVM Representation | Purpose |
|------|--------|----------|---------------------|---------|
| **Fixed Array** | `[T; N]` | Compiler primitive | `[N x T]` inline | Stack, C FFI, compile-time size |
| **Slice** | `&[T]` | Compiler primitive (future) | `{ptr, i64}` struct | Borrowed view |
| **Vec** | `Vec<T>` | Stdlib only | `{ptr, i64, i64}` struct | Heap, growable |

### Key Principle

Only fixed-size arrays and slices are compiler primitives. `Vec<T>` is implemented entirely in stdlib using low-level memory primitives.

## Type System Changes

### Remove

```rust
// lib/src/types/mod.rs
Type::Array(Box<Type>)  // DELETE
```

### Add

```rust
// lib/src/types/mod.rs
pub enum Type {
    // ... existing variants ...

    /// Fixed-size array: [T; N]
    /// - Stack allocated (inline in LLVM)
    /// - Size known at compile time
    /// - C-compatible for FFI
    FixedSizeArray(Box<Type>, usize),  // (element_type, size)

    /// Slice: &[T] or &mut [T]
    /// - Fat pointer: {ptr, length}
    /// - Borrowed view into array or vec
    /// - Requires borrow checker for safety
    Slice(Box<Type>),  // (element_type) - future, add with borrow checker
}
```

## Syntax

### Fixed-Size Arrays

```rock
// Declaration
buffer: [I8; 256]
matrix: [[F64; 4]; 4]

// Literal
arr = [1, 2, 3, 4, 5]  // Type: [I64; 5]

// With explicit type
bytes: [U8; 4] = [0xDE, 0xAD, 0xBE, 0xEF]
```

### Slices (Future)

```rock
// Immutable slice
data: &[I8]

// Mutable slice
data: &mut [I8]

// Slicing syntax
slice = arr[0..4]  // &[T]
```

### Vec (Stdlib)

```rock
// Stdlib-defined
struct Vec<T>
    ptr: *T
    len: Usize
    cap: Usize

// Usage
names: Vec<String> = Vec.new()
names.push("hello")
```

## FFI Compatibility

Fixed-size arrays are C-compatible:

```rock
extern "C"
    read: I32, *void, Usize -> Isize
    write: I32, *void, Usize -> Isize
    memcpy: *void, *void, Usize -> ()

// Pass fixed array to C
buffer: [I8; 256]
bytes_read = read(fd, buffer.as_ptr(), 256)

// Fixed arrays are inline on stack (C-compatible ABI)
```

## Memory Primitives for Stdlib

The compiler provides minimal builtins for stdlib to build `Vec<T>`:

### Option A: Direct libc bindings (simpler)

```rock
extern "C"
    malloc: Usize -> *void
    free: *void -> ()
    realloc: *void, Usize -> *void
```

### Option B: Rust-style allocator API (more principled)

```rock
struct Layout
    size: Usize
    align: Usize

// Compiler intrinsics
intrinsic alloc: Layout -> *void
intrinsic dealloc: *void, Layout -> ()
intrinsic realloc: *void, Layout, Usize -> *void

// Type introspection
intrinsic size_of<T>: -> Usize
intrinsic align_of<T>: -> Usize
```

## Implicit Coercion (Future)

When slices are implemented, `[T; N]` and `Vec<T>` will implicitly coerce to `&[T]`:

```rock
print_slice = (s: &[I32]) ->
    // ...

arr: [I32; 4] = [1, 2, 3, 4]
vec: Vec<I32> = Vec.new()

// Both coerce to &[I32]
print_slice(arr)  // OK: [I32; 4] -> &[I32]
print_slice(vec)  // OK: Vec<I32> -> &[I32]
```

## Codegen Details

### Fixed-Size Array

```rust
// LLVM: inline array type
fn llvm_fixed_array_type(&self, elem_ty: &Type, size: usize) -> BasicTypeEnum<'ctx> {
    let elem = self.llvm_type(elem_ty);
    elem.array_type(size as u32).into()
}

// Array literal [a, b, c] generates:
// 1. Allocate inline array on stack
// 2. Store each element at index
// 3. Load and return the array value
```

### Slice (Future)

```rust
// LLVM: struct {ptr, i64}
fn llvm_slice_type(&self, elem_ty: &Type) -> StructType<'ctx> {
    self.context.struct_type(&[
        self.context.ptr_type(AddressSpace::default()).into(),
        self.context.i64_type().into(),
    ], false)
}
```

## Migration Plan

### Phase 1: Remove Old Array

1. Delete `Type::Array(Box<Type>)` from type system
2. Delete `array_len`, `array_push`, `array_concat` builtins from codegen
3. Remove array-related tests (to be replaced)

### Phase 2: Add Fixed-Size Array

1. Add `Type::FixedSizeArray(Box<Type>, usize)`
2. Add parser for `[T; N]` syntax (both types and literals)
3. Add codegen for inline LLVM arrays
4. Add array indexing `arr[i]`
5. Add `.len()` method or `len()` builtin for fixed arrays
6. Add tests for fixed-size arrays

### Phase 3: Add Slice (With Borrow Checker)

1. Add `Type::Slice(Box<Type>)`
2. Add parser for `&[T]` and `&mut [T]`
3. Add slicing syntax `arr[start..end]`
4. Add codegen for fat pointer struct
5. Implement implicit coercion from `[T; N]` to `&[T]`
6. Add tests for slices

### Phase 4: Stdlib Vec

1. Implement `Vec<T>` struct in stdlib
2. Implement `push`, `pop`, `len`, etc. using malloc/free
3. Implement `Deref` to `&[T]` for slice coercion
4. Add tests for Vec

## Open Questions

1. **Array initialization syntax**: Should we support `[T; N]` with a single value like Rust?
   ```rock
   zeros: [I32; 10] = [0; 10]  // Array of 10 zeros
   ```

2. **Bounds checking**: Runtime bounds check on indexing, or unchecked for performance?

3. **Memory primitives**: Option A (direct libc) or Option B (Rust-style allocator API)?

## References

- Rust's array design: https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html#the-slice-type
- LLVM array type: https://llvm.org/docs/LangRef.html#array-type
