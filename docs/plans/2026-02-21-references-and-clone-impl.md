# Reference Parameters and Clone Trait Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement explicit reference parameters (`&T`), Copy trait (implicit copy for primitives), and Clone trait (explicit `.clone()`) to fix 11 failing integration tests.

**Architecture:** Three-layer implementation: (1) Type system with Copy/Clone traits, (2) MIR with enhanced borrow checking, (3) Codegen with reference passing. String literals become `&str`, String is a stdlib struct.

**Tech Stack:** Rust, LLVM (inkwell), existing Rock compiler infrastructure

---

## Phase 1: Copy Trait for Primitive Types

### Task 1.1: Add Copy trait recognition to type system

**Files:**
- Modify: `lib/src/types/mod.rs`

**Step 1: Add `is_copy` method to Type enum**

In `lib/src/types/mod.rs`, add a method to check if a type implements Copy:

```rust
impl Type {
    /// Check if this type implements Copy (can be implicitly copied)
    pub fn is_copy(&self) -> bool {
        match self {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 => true,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 => true,
            Type::F32 | Type::F64 => true,
            Type::Bool | Type::Char | Type::Unit => true,
            Type::Reference { .. } => true,  // References are Copy
            Type::Pointer(_) => true,
            Type::Function(_, _) => true,
            // Check if all elements are Copy
            Type::Tuple elems => elems.iter().all(|e| e.is_copy()),
            // These are NOT Copy by default
            Type::String | Type::Array(_) => false,
            Type::Struct(_, generics) => generics.iter().all(|g| g.is_copy()),
            Type::Enum(_, generics) => generics.iter().all(|g| g.is_copy()),
            Type::TypeVar(_) | Type::Generic(_) | Type::Error | Type::Never => false,
        }
    }
}
```

**Step 2: Run tests to verify compilation**

Run: `cargo build -p rock-lib`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add lib/src/types/mod.rs
git commit -m "feat: add is_copy method to Type enum"
```

### Task 1.2: Update MIR builder to use Copy semantics for Copy types

**Files:**
- Modify: `lib/src/mir/builder.rs`

**Step 1: Update `needs_move` function to use `is_copy`**

In `lib/src/mir/builder.rs`, find the `needs_move` function and update it to use `Type::is_copy()`:

```rust
fn needs_move(ty: &Type) -> bool {
    !ty.is_copy()
}
```

**Step 2: Run tests to verify no regressions**

Run: `cargo test -p rock-lib --test integration -- --test-threads=1 2>&1 | tail -20`
Expected: Same 77 pass / 11 fail result

**Step 3: Commit**

```bash
git add lib/src/mir/builder.rs
git commit -m "refactor: use Type::is_copy in needs_move function"
```

---

## Phase 2: Reference Pattern Syntax

### Task 2.1: Add reference pattern to AST

**Files:**
- Modify: `lib/src/ast/tree.rs`

**Step 1: Add Reference variant to PatternKind enum**

Find the `PatternKind` enum and add:

```rust
#[derive(Debug, Clone)]
pub enum PatternKind {
    IdentPattern { name: String, mut_: bool },
    ReferencePattern { pattern: Box<Pattern>, mutable: bool },  // NEW
    // ... existing variants
}
```

**Step 2: Run tests to verify compilation**

Run: `cargo build -p rock-lib`
Expected: Compiles (may have unused warnings)

**Step 3: Commit**

```bash
git add lib/src/ast/tree.rs
git commit -m "feat: add ReferencePattern variant to AST"
```

### Task 2.2: Parse reference patterns (`&name` and `&mut name`)

**Files:**
- Modify: `lib/src/new_parser/items/pattern.rs`

**Step 1: Update pattern parser to handle `&` prefix**

```rust
pub fn pattern(stream: Input) -> IResult<Pattern> {
    // Try to parse & or &mut prefix
    let (stream, ref_prefix) = reference_prefix().opt().parse(stream)?;

    let (stream, (binding, kind)) = (followed(ident, TokenType::Arobase).opt(), pattern_kind)
        .map(|(binding, kind)| (binding, kind))
        .parse(stream)?;

    let pattern = Pattern { binding, kind };

    // Wrap in ReferencePattern if we parsed a & prefix
    let final_pattern = match ref_prefix {
        Some(mutable) => Pattern {
            binding: None,
            kind: PatternKind::ReferencePattern {
                pattern: Box::new(pattern),
                mutable,
            },
        },
        None => pattern,
    };

    Ok((stream, final_pattern))
}

fn reference_prefix() -> impl Parser<Output = bool> {
    // Parse &mut or just &
    or(
        followed(token(TokenType::Ampersand), token(TokenType::Mut)).map(|_| true),
        token(TokenType::Ampersand).map(|_| false),
    )
}
```

**Step 2: Add TokenType::Ampersand if not present**

Check `lib/src/lexer/token.rs` for `Ampersand` token type. Add if missing.

**Step 3: Run parser tests**

Run: `cargo test -p rock-lib parser`
Expected: All parser tests pass

**Step 4: Commit**

```bash
git add lib/src/new_parser/items/pattern.rs lib/src/lexer/token.rs
git commit -m "feat: parse reference patterns (&name, &mut name)"
```

### Task 2.3: Add HirParam reference flag

**Files:**
- Modify: `lib/src/hir/mod.rs`

**Step 1: Add `is_ref` field to HirParam**

```rust
#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub is_ref: bool,  // NEW: parameter is passed by reference
}
```

**Step 2: Update all HirParam constructors**

Find all places that create `HirParam` and add `is_ref: false` as default.

**Step 3: Commit**

```bash
git add lib/src/hir/mod.rs
git commit -m "feat: add is_ref field to HirParam"
```

---

## Phase 3: Clone Trait Definition

### Task 3.1: Create Clone trait in stdlib

**Files:**
- Create: `stdlib/clone.rk`

**Step 1: Create Clone trait file**

```rock
// stdlib/clone.rk

trait Clone
    @clone = -> Self
```

**Step 2: Add Clone implementations for primitives**

```rock
// Clone for I64 (Copy types just return self)
impl Clone for I64
    @clone = -> self

impl Clone for I32
    @clone = -> self

impl Clone for F64
    @clone = -> self

impl Clone for Bool
    @clone = -> self

impl Clone for Char
    @clone = -> self
```

**Step 3: Commit**

```bash
git add stdlib/clone.rk
git commit -m "feat: add Clone trait and primitive implementations"
```

### Task 3.2: Add Clone method call support in MIR

**Files:**
- Modify: `lib/src/mir/builder.rs`

**Step 1: Handle clone method calls**

In the MethodCall handling, check if method is "clone" and generate appropriate MIR:

```rust
HirExprKind::MethodCall(receiver, method_name, args) => {
    if method_name == "clone" && args.is_empty() {
        // Clone is just a copy for the receiver
        let recv_temp = self.new_local_from_expr(receiver.ty.clone(), receiver);
        self.emit_storage_live(recv_temp, Some(receiver.span.clone()));
        let recv_place = Place { local: recv_temp, projection: vec![] };
        // Clone borrows then copies
        self.lower_expr_with_context(receiver, recv_place.clone(), true);

        // Copy the temp to destination
        let rvalue = Rvalue::Use(Operand::Copy(recv_place));
        self.emit_assign(dest, rvalue, span);
        return;
    }
    // ... existing method call handling
}
```

**Step 2: Commit**

```bash
git add lib/src/mir/builder.rs
git commit -m "feat: handle .clone() method calls in MIR builder"
```

---

## Phase 4: String Type Refactoring

### Task 4.1: Add str primitive type

**Files:**
- Modify: `lib/src/types/mod.rs`

**Step 1: Add Str variant to Type enum**

```rust
pub enum Type {
    // ... existing variants
    Str,  // Primitive string slice (fat pointer: ptr + len)
    String,  // Heap-allocated string (keep for compatibility during transition)
    // ...
}
```

**Step 2: Update Type::is_copy to include Str**

```rust
Type::Str => true,  // str is Copy (just a fat pointer)
```

**Step 3: Commit**

```bash
git add lib/src/types/mod.rs
git commit -m "feat: add str primitive type"
```

### Task 4.2: Update string literal type to &str

**Files:**
- Modify: `lib/src/lower/mod.rs` or wherever string literals get their type

**Step 1: Find where StringLiteral gets typed**

Search for `StringLiteral` and update its type from `Type::String` to `Type::Reference { mutable: false, inner: Box::new(Type::Str) }`.

**Step 2: Verify string literals work**

Run: `cargo test -p rock-lib test_hello_world`
Expected: Test passes

**Step 3: Commit**

```bash
git add lib/src/lower/mod.rs
git commit -m "feat: string literals now have type &str"
```

---

## Phase 5: Reference Parameter Handling

### Task 5.1: Lower reference patterns to HIR

**Files:**
- Modify: `lib/src/lower/function.rs`

**Step 1: Handle ReferencePattern in lower_param_pattern**

```rust
fn lower_param_pattern(&mut self, pattern: &Pattern, ty: &Type) -> (String, bool, bool) {
    match &pattern.kind {
        PatternKind::IdentPattern { name, mut_ } => {
            (name.clone(), *mut_, false)
        }
        PatternKind::ReferencePattern { pattern, mutable } => {
            let (name, inner_mut, _) = self.lower_param_pattern(pattern, ty);
            (name, inner_mut || *mutable, true)  // is_ref = true
        }
        // ... other patterns
    }
}
```

**Step 2: Update lower_function_decl_header to use is_ref**

Ensure `HirParam { is_ref: true, ... }` is created for reference patterns.

**Step 3: Commit**

```bash
git add lib/src/lower/function.rs
git commit -m "feat: lower reference patterns to HIR params"
```

### Task 5.2: Generate reference creation at call sites

**Files:**
- Modify: `lib/src/mir/builder.rs`

**Step 1: In Call handling, check if param expects reference**

When generating function call arguments, check if the callee expects a reference and create one:

```rust
HirExprKind::Call(func, args) => {
    // Get function signature to check param types
    let func_sig = self.get_function_signature(func);

    let mut arg_operands = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let param_expects_ref = func_sig.as_ref()
            .map(|s| s.param_types.get(i)
                .map(|t| matches!(t, Type::Reference { .. }))
                .unwrap_or(false))
            .unwrap_or(false);

        let arg_temp = self.new_local_from_expr(arg.ty.clone(), arg);
        self.emit_storage_live(arg_temp, Some(arg.span.clone()));
        let arg_place = Place { local: arg_temp, projection: vec![] };

        if param_expects_ref {
            // Create a reference to the argument
            self.lower_expr_with_context(arg, arg_place.clone(), true);
            // The operand is a reference to the temp
            arg_operands.push(Operand::Copy(arg_place));
        } else {
            self.lower_expr(arg, arg_place.clone());
            arg_operands.push(Operand::Copy(arg_place));
        }
    }
    // ... rest of call handling
}
```

**Step 2: Commit**

```bash
git add lib/src/mir/builder.rs
git commit -m "feat: create references at call sites for ref params"
```

### Task 5.3: Codegen for reference parameters

**Files:**
- Modify: `lib/src/codegen/mod.rs`

**Step 1: Pass references as pointers in LLVM**

When a parameter is `is_ref`, pass it as a pointer:

```rust
fn compile_param(&mut self, param: &HirParam) -> BasicValueEnum<'ctx> {
    if param.is_ref {
        // Reference parameters are passed as pointers
        self.builder().build_alloca(self.llvm_type(&param.ty), &param.name)
    } else {
        // Regular parameters by value
        self.llvm_type(&param.ty).const_zero()
    }
}
```

**Step 2: Commit**

```bash
git add lib/src/codegen/mod.rs
git commit -m "feat: pass reference parameters as pointers in LLVM"
```

---

## Phase 6: Auto-deref

### Task 6.1: Implement auto-deref for field access

**Files:**
- Modify: `lib/src/mir/builder.rs`

**Step 1: Auto-deref in field access handling**

```rust
fn lower_place(&mut self, expr: &HirExpr) -> Option<Place> {
    match &expr.kind {
        HirExprKind::FieldAccess(base, field) => {
            let mut place = self.lower_place(base)?;
            // Auto-deref if base is a reference
            if let Some(Type::Reference { inner, .. }) = self.get_expr_type(base) {
                place.projection.push(Projection::Deref);
            }
            place.projection.push(Projection::Field(field.clone()));
            Some(place)
        }
        // ... other cases
    }
}
```

**Step 2: Commit**

```bash
git add lib/src/mir/builder.rs
git commit -m "feat: auto-deref for field access on references"
```

---

## Phase 7: Fix Failing Tests

### Task 7.1: Update stdlib function signatures

**Files:**
- Modify: `stdlib/string.rk` (or equivalent)
- Modify: `stdlib/array.rk` (or equivalent)

**Step 1: Change string functions to take &str**

```rock
// Before:
string_len : String -> I64
char_at : String -> I64 -> I64

// After:
string_len : &str -> I64
char_at : &str -> I64 -> I64
string_substr : &str -> I64 -> I64 -> String
string_find : &str -> &str -> I64
string_contains : &str -> &str -> Bool
```

**Step 2: Change array functions to take references**

```rock
// Before:
array_len : Array I64 -> I64
array_index : Array I64 -> I64 -> I64

// After:
array_len : &Array I64 -> I64
array_index : &Array I64 -> I64 -> I64
```

**Step 3: Commit**

```bash
git add stdlib/*.rk
git commit -m "feat: update stdlib to use reference parameters"
```

### Task 7.2: Update test code where needed

**Files:**
- Modify: `lib/tests/integration.rs` (test source strings)

**Step 1: Add explicit borrows where needed**

For tests using heap-allocated values that are now moved:

```rock
// Before:
arr = [1, 2, 3]
array_len arr .println!
array_index arr, 1 .println!

// After:
arr = [1, 2, 3]
array_len &arr .println!
array_index &arr, 1 .println!
```

**Step 2: Run all tests**

Run: `cargo test -p rock-lib --test integration 2>&1 | tail -30`
Expected: More tests passing

**Step 3: Commit**

```bash
git add lib/tests/integration.rs
git commit -m "test: update tests for reference parameters"
```

### Task 7.3: Verify all tests pass

**Step 1: Run full test suite**

Run: `cargo test -p rock-lib --test integration`
Expected: 88 tests pass

**Step 2: If tests fail, debug and fix**

Run individual failing tests with:
```bash
cargo test -p rock-lib test_name -- --nocapture
```

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: complete reference parameters and Clone trait implementation"
```

---

## Summary

**Total Tasks: 15**

1. Task 1.1: Add Copy trait recognition to type system
2. Task 1.2: Update MIR builder to use Copy semantics
3. Task 2.1: Add reference pattern to AST
4. Task 2.2: Parse reference patterns
5. Task 2.3: Add HirParam reference flag
6. Task 3.1: Create Clone trait in stdlib
7. Task 3.2: Add Clone method call support in MIR
8. Task 4.1: Add str primitive type
9. Task 4.2: Update string literal type to &str
10. Task 5.1: Lower reference patterns to HIR
11. Task 5.2: Generate reference creation at call sites
12. Task 5.3: Codegen for reference parameters
13. Task 6.1: Implement auto-deref for field access
14. Task 7.1: Update stdlib function signatures
15. Task 7.2: Update test code where needed
16. Task 7.3: Verify all tests pass

**Estimated time:** This is a significant feature requiring careful implementation. Each task should be implemented and tested before moving to the next.
