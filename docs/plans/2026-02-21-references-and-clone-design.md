# Reference Parameters and Clone Trait Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create the implementation plan.

**Goal:** Implement explicit reference parameters and Clone trait to fix the 11 failing integration tests that have use-after-move errors.

**Architecture:** Three-layer implementation with Copy/Clone traits, reference parameter handling, and full borrow checker integration.

**Tech Stack:** Rust, LLVM (inkwell), existing Rock compiler infrastructure

---

## Overview

This design adds:
1. **Copy trait** - Marker trait for types that can be implicitly copied (primitives)
2. **Clone trait** - Explicit `.clone()` method for heap-allocated types
3. **Reference parameters** - `&T` and `&mut T` with explicit borrow syntax `&var`
4. **Auto-deref** - Automatic dereferencing for field access on references
5. **Borrow checker integration** - Track borrows, prevent use-after-move

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  1. Type System Layer                                    │
│  - Copy trait (marker for implicit copy)                │
│  - Clone trait (explicit .clone() method)               │
│  - Reference types &T, &mut T (already exists)          │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  2. MIR/Borrow Checker Layer                            │
│  - Track Copy vs Move types                             │
│  - Borrow checking for reference parameters             │
│  - Loan tracking (already partially exists)             │
└─────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────┐
│  3. Codegen Layer                                        │
│  - Reference passing (pointers in LLVM)                 │
│  - Clone implementations for built-in types             │
│  - Copy semantics (implicit memcpy for Copy types)      │
└─────────────────────────────────────────────────────────┘
```

## Copy and Clone Traits

### Trait Definitions (in stdlib)

```rock
// stdlib/traits.rk

trait Clone
    @clone = -> Self

trait Copy
    // Marker trait - no methods
```

### Type Classification

| Type | Copy | Clone | Notes |
|------|------|-------|-------|
| I8, I16, I32, I64 | Yes | Yes | Primitives |
| U8, U16, U32, U64 | Yes | Yes | Primitives |
| F32, F64 | Yes | Yes | Primitives |
| Bool, Char | Yes | Yes | Primitives |
| Unit | Yes | Yes | Empty type |
| `str` | Yes | Yes | Primitive fat pointer |
| String | No | Yes | Stdlid struct, heap allocated |
| Array<T> | No | Yes (if T: Clone) | Heap allocated |
| Tuple<T...> | Yes (if all Copy) | Yes (if all Clone) | Depends on elements |
| Struct | No | Yes (if impl) | User must impl Clone |
| Enum | No | Yes (if impl) | User must impl Clone |
| `&T` (reference) | Yes | Yes | Just a pointer |

### Impl Syntax

```rock
// No type signatures in impl - already in trait definition
impl Clone for String
    @clone = ->
        new_data = malloc self.len
        memcpy new_data, self.data, self.len
        String
            data: new_data
            len: self.len
            cap: self.len
```

## String Types

### Two string types (like Rust):

**`str` - Primitive string slice**
- Compiler intrinsic type
- Fat pointer (ptr + len)
- Implements Copy and Clone
- String literals have type `&str`

**`String` - Heap-allocated string**
- Defined in stdlib as a struct
- Implements Clone only (no Copy)
- Created via `.to_string()` on `&str`

```rock
// stdlib/string.rk

extern malloc : U64 -> *U8
extern memcpy : *U8, *U8, U64 -> *U8
extern free : *U8 -> ()

struct String
    data: *U8
    len: U64
    cap: U64

impl Clone for String
    @clone = ->
        new_data = malloc self.len
        memcpy new_data, self.data, self.len
        String
            data: new_data
            len: self.len
            cap: self.len
```

### String Literals

```rock
s = "hello"        // s: &str (already a reference)
heap_s = "hello".to_string()  // heap_s: String
```

## Reference Parameters

### Syntax

```rock
// Type signature with reference parameter
string_len : &str -> I64

// Implementation with reference binding
string_len = &s ->
    s.len

// Multiple parameters use -> not commas
char_at : &str -> I64 -> I64
char_at = &s, idx ->
    // body
```

### Explicit Borrow

```rock
// Variable is not a reference
my_struct = MyStruct
    x: 42
    y: 100

// Must explicitly borrow with &
process : &MyStruct -> I64
process &my_struct   // Creates &MyStruct

// Can use my_struct again - borrow ended
process &my_struct   // OK, borrow again
```

### Auto-deref

Like Rust, references auto-dereference for field access:

```rock
s: &str
s.len       // Auto-derefs to (*s).len
s.data      // Auto-derefs to (*s).data

// Works for nested references too
s: &&str
s.len       // Auto-derefs multiple levels
```

### Deref Coercion

```rock
// &String can coerce to &str
s = "hello".to_string()  // s: String
string_len &s            // &String -> &str via deref coercion
```

## Borrow Checker Integration

### When calling a function with reference parameters:

```rock
string_len : &str -> I64

my_str = "hello"
result = string_len my_str  // my_str is already &str, no borrow needed
```

### For non-reference types:

```rock
process : &MyStruct -> I64

my_struct = MyStruct
    x: 42

result = process &my_struct  // Explicit borrow creates reference
// my_struct still valid here
```

### MIR Generation

```
// At call site with explicit borrow:
my_struct = MyStruct { ... }     // my_struct: MyStruct, Init

// Create reference (borrow)
tmp_ref = &my_struct
result = Call process, [tmp_ref]

// After call:
my_struct still Init             // Borrow ended, can use again
```

### Borrow Tracking

- Track active borrows at each point
- Allow multiple immutable borrows: `&T`
- Allow single mutable borrow: `&mut T`
- Prevent use-after-move when value is borrowed
- Borrow ends when reference goes out of scope

## Fixing the Failing Tests

### Update stdlib function signatures:

```rock
// Before (moves the argument):
string_len : String -> I64
char_at : String -> I64 -> I64
array_len : Array I64 -> I64

// After (borrows the argument):
string_len : &str -> I64
char_at : &str -> I64 -> I64
array_len : &Array I64 -> I64
```

### String literals already work:

```rock
s = "Hello"
string_len s       // s is already &str
char_at s, 0       // s is still valid
```

### For heap values, explicit borrow:

```rock
arr = [1, 2, 3]
array_len &arr .println!
array_index &arr, 1 .println!
```

### For cases needing ownership, use clone:

```rock
result = some_function my_string.clone()
```

## Implementation Order

### Phase 1: Foundation (Copy/Clone traits)
1. Add Copy and Clone trait definitions to stdlib
2. Implement Copy for primitive types (I8-I64, U8-U64, F32-F64, Bool, Char)
3. Implement Clone for primitive types (same as Copy, just return self)
4. Update type system to recognize Copy types (implicit copy instead of move)

### Phase 2: Reference Parameters
1. Parser: Add `&pattern` syntax for reference binding
2. HIR: Handle reference parameters in function signatures
3. MIR: Generate reference creation at call sites
4. Borrow checker: Track borrows and validate reference lifetimes
5. Codegen: Pass references as pointers in LLVM

### Phase 3: String/Array in stdlib
1. Define `str` primitive type in compiler (fat pointer)
2. Define `String` struct in stdlib
3. Implement Clone for String using libc malloc/memcpy
4. Implement Clone for Array
5. Update string literals to have type `&str`

### Phase 4: Auto-deref
1. Implement auto-deref for field access on references
2. Implement deref coercion (`&String` -> `&str`)

### Phase 5: Fix failing tests
1. Update stdlib function signatures to use reference parameters
2. Update test code where needed (add `&` or `.clone()`)
3. Verify all 88 tests pass
