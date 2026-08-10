# Borrowed Slices And Str Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reject standalone unsized source types (`[T]`, `Str`) and separate text strings (`&Str`) from byte slices (`&[U8]`).

**Architecture:** Keep the parser shape and enforce source validity in lowering/collection. Keep `Type::Slice` for `[T]`, `Type::Array` for `[T; N]`, and make `Type::Str` the unsized string referent used through `&Str`; update lookup, intrinsics, codegen, and stdlib to treat `&Str` as text and `&[U8]` as bytes.

**Tech Stack:** Rust 2021, `rock-lib`, Rock stdlib source files, LLVM 18 through `inkwell`, `cargo test -p rock-lib`.

---

## Commit Policy

Commit steps are included as task boundaries. During execution, run commit steps only if the user has explicitly requested commits in that execution session.

## File Structure

- `lib/src/types/mod.rs`: Owns builtin indexing result types and `Type` display/copy behavior.
- `lib/src/lower/types.rs`: Owns normal lowering from `ast::ParseType` to `Type`; add the `Str` unsized-source rule here.
- `lib/src/collect/context.rs`: Owns collection-time duplicate type lowering; mirror the `Str` unsized-source rule here.
- `lib/src/lower/expression.rs`: Owns literal lowering; string literals become `&Str` here.
- `lib/src/lower/types_helpers/helpers.rs`: Owns lowerer method lookup type-name candidates and `type_from_name`; remove `Str -> [U8]` and `[U8]` text fallback behavior here.
- `lib/src/lower/control_flow/secondary.rs`: Owns calls, method resolution, intrinsic call typing, and index operator diagnostics; add `&Str` intrinsic acceptance and reject `Str` indexing here.
- `lib/src/lower/intrinsics.rs`: Owns intrinsic registration and fallback return/argument inference; add `BorrowStr` and `ArrPtr(&Str) -> *U8` here.
- `lib/src/codegen/types.rs`: Owns LLVM type lowering; make references/pointers to `Str` fat.
- `lib/src/codegen/expr/access.rs`: Owns slice/fat-pointer part extraction; teach it to extract parts from `Str` and `&Str`.
- `lib/src/codegen/intrinsics.rs`: Owns intrinsic codegen; add `ArrayLen`, `ArrPtr`, and `BorrowStr` support for `&Str`.
- `lib/src/codegen/mod.rs`, `lib/src/codegen/types.rs`, `lib/src/mono/mod.rs`, `lib/src/infer/solve.rs`: Own type-name helpers used after lowering; remove `[U8]` as string identity while preserving existing generic slice fallback until canonical identity work replaces it.
- `stdlib/show.rk`: Move string `Show` impl from `&[U8]` to `&Str`.
- `stdlib/eq.rk`: Move string `Eq` impl from `&[U8]` to `&Str`.
- `stdlib/string_type.rk`: Change `String::from_str` to accept `&Str`.
- `stdlib/string.rk`: Change text APIs to `&Str`; keep byte APIs byte-oriented.
- `stdlib/convert.rk`: Return `&Str` from numeric-to-string conversion using the new unsafe `~BorrowStr` intrinsic.
- `lib/tests/integration.rs`: Update stale string/byte-slice tests and add user-visible regressions.

---

### Task 1: Byte Slice Indexing Is Byte-Oriented

**Files:**
- Modify: `lib/src/types/mod.rs:199-217`
- Test: `lib/src/types/mod.rs:587-613`

- [ ] **Step 1: Add failing type tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `lib/src/types/mod.rs`:

```rust
#[test]
fn test_u8_slice_and_array_index_output_is_u8() {
    let idx = Type::I64;

    assert_eq!(
        Type::Slice(Box::new(Type::U8)).builtin_index_output(&idx),
        Some(Type::U8)
    );
    assert_eq!(
        Type::Array(Box::new(Type::U8), 4).builtin_index_output(&idx),
        Some(Type::U8)
    );
}

#[test]
fn test_str_has_no_builtin_index_output() {
    assert_eq!(Type::Str.builtin_index_output(&Type::I64), None);
}

#[test]
fn test_display_formats_str_with_rock_spelling() {
    assert_eq!(Type::Str.to_string(), "Str");
    assert_eq!(
        Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        }
        .to_string(),
        "&Str"
    );
}
```

- [ ] **Step 2: Run the new tests and verify failure**

Run: `cargo test -p rock-lib test_u8_slice_and_array_index_output_is_u8 -- --exact`

Expected: FAIL because `Type::Slice(U8)` and `Type::Array(U8, _)` currently return `Some(Type::Char)`.

- [ ] **Step 3: Make builtin indexing structural**

Replace `Type::builtin_index_output` in `lib/src/types/mod.rs` with:

```rust
pub fn builtin_index_output(&self, idx: &Type) -> Option<Type> {
    match (self, idx) {
        (Type::Slice(inner), Type::I64) | (Type::Array(inner, _), Type::I64) => {
            Some((**inner).clone())
        }
        (Type::Pointer(inner), Type::I64) => match inner.as_ref() {
            Type::Slice(elem) => Some((**elem).clone()),
            _ => Some((**inner).clone()),
        },
        _ => None,
    }
}
```

In the `fmt::Display for Type` implementation, change the `Type::Str` arm to preserve Rock's uppercase source spelling:

```rust
Type::Str => write!(f, "Str"),
```

- [ ] **Step 4: Run the type tests**

Run: `cargo test -p rock-lib test_u8_slice_and_array_index_output_is_u8 -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_str_has_no_builtin_index_output -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_display_formats_str_with_rock_spelling -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit task boundary if commits are approved**

```bash
git add lib/src/types/mod.rs
git commit -m "types: separate byte slice indexing from strings"
```

---

### Task 2: Lower `Str` As An Unsized Referent

**Files:**
- Modify: `lib/src/lower/types.rs:13-141`
- Test: `lib/src/lower/types.rs:144-201`

- [ ] **Step 1: Add failing lowerer tests**

Add these tests to `lib/src/lower/types.rs` in the existing `tests` module:

```rust
#[test]
fn test_lower_parse_bare_str_reports_error() {
    let mut lowerer = Lowerer::new();

    let ty = lowerer.lower_parse_type(&named_type("Str"));

    assert_eq!(ty, Type::Error);
    assert!(lowerer.errors.iter().any(|err| err
        .message
        .contains("bare string slice type Str must be written behind a reference")));
}

#[test]
fn test_lower_parse_borrowed_str_lowers_to_reference_to_str_type() {
    let mut lowerer = Lowerer::new();

    let ty = lowerer.lower_parse_type(&ParseType::Reference {
        is_mut: false,
        pointee: Box::new(named_type("Str")),
    });

    assert_eq!(
        ty,
        Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        }
    );
    assert!(lowerer.errors.is_empty());
}

#[test]
fn test_lower_parse_mut_borrowed_str_lowers_to_mut_reference_to_str_type() {
    let mut lowerer = Lowerer::new();

    let ty = lowerer.lower_parse_type(&ParseType::Reference {
        is_mut: true,
        pointee: Box::new(named_type("Str")),
    });

    assert_eq!(
        ty,
        Type::Reference {
            mutable: true,
            inner: Box::new(Type::Str),
        }
    );
    assert!(lowerer.errors.is_empty());
}
```

- [ ] **Step 2: Run one failing lowerer test**

Run: `cargo test -p rock-lib test_lower_parse_borrowed_str_lowers_to_reference_to_str_type -- --exact`

Expected: FAIL because `Str` currently lowers to a borrowed `[U8]` alias.

- [ ] **Step 3: Add context-aware `Str` lowering**

In `lib/src/lower/types.rs`, change the `ParseType::Type` arm and add a context-aware helper:

```rust
ast::ParseType::Type(inner) => {
    self.lower_parse_type_inner_with_slice_context(inner, allow_bare_slice)
}
```

Replace `lower_parse_type_inner` with this wrapper plus helper:

```rust
pub(crate) fn lower_parse_type_inner(&mut self, inner: &ast::ParseTypeInner) -> Type {
    self.lower_parse_type_inner_with_slice_context(inner, false)
}

fn lower_parse_type_inner_with_slice_context(
    &mut self,
    inner: &ast::ParseTypeInner,
    allow_bare_slice: bool,
) -> Type {
    let generics: Vec<Type> = inner
        .generics
        .iter()
        .map(|g| self.lower_parse_type(g))
        .collect();

    match inner.name.as_str() {
        "I8" | "i8" => Type::I8,
        "I16" | "i16" => Type::I16,
        "I32" | "i32" | "Int" => Type::I32,
        "I64" | "i64" => Type::I64,
        "U8" | "u8" => Type::U8,
        "U16" | "u16" => Type::U16,
        "U32" | "u32" => Type::U32,
        "U64" | "u64" => Type::U64,
        "F32" | "f32" => Type::F32,
        "F64" | "f64" | "Float" => Type::F64,
        "Bool" | "bool" => Type::Bool,
        "Char" | "char" => Type::Char,
        "Str" => {
            if !allow_bare_slice {
                self.push_error(
                    "bare string slice type Str must be written behind a reference, such as &Str"
                        .to_string(),
                );
                return Type::Error;
            }
            Type::Str
        }
        name => {
            if self.structs.contains_key(name) {
                Type::Struct(name.to_string(), generics)
            } else if self.enums.contains_key(name) {
                Type::Enum(name.to_string(), generics)
            } else {
                Type::Generic(name.to_string())
            }
        }
    }
}
```

- [ ] **Step 4: Run the lowerer tests**

Run: `cargo test -p rock-lib test_lower_parse_bare_str_reports_error -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_lower_parse_borrowed_str_lowers_to_reference_to_str_type -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_lower_parse_mut_borrowed_str_lowers_to_mut_reference_to_str_type -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit task boundary if commits are approved**

```bash
git add lib/src/lower/types.rs
git commit -m "lower: require borrowed Str source types"
```

---

### Task 3: Mirror `Str` Lowering In Collection

**Files:**
- Modify: `lib/src/collect/context.rs:626-704`
- Test: `lib/src/collect/headers.rs:597-739`

- [ ] **Step 1: Add collection-time tests through struct header building**

Add these tests to the existing `tests` module in `lib/src/collect/headers.rs`:

```rust
#[test]
fn test_build_struct_rejects_bare_str_field_type() {
    let mut context = CollectContext::new();
    let decl = StructDecl {
        name: ParseTypeInner {
            name: "TextHolder".to_string(),
            generics: vec![],
            span: Span::default(),
        },
        fields: vec![field("text", named_type("Str"), true)],
        exported: false,
    };

    let hir_struct = build_struct(&mut context, &decl);

    assert_eq!(hir_struct.fields[0].ty, Type::Error);
    assert!(context.errors.iter().any(|err| err
        .message
        .contains("bare string slice type Str must be written behind a reference")));
}

#[test]
fn test_build_struct_accepts_borrowed_str_field_type() {
    let mut context = CollectContext::new();
    let decl = StructDecl {
        name: ParseTypeInner {
            name: "TextHolder".to_string(),
            generics: vec![],
            span: Span::default(),
        },
        fields: vec![field(
            "text",
            ParseType::Reference {
                is_mut: false,
                pointee: Box::new(named_type("Str")),
            },
            true,
        )],
        exported: false,
    };

    let hir_struct = build_struct(&mut context, &decl);

    assert_eq!(
        hir_struct.fields[0].ty,
        Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        }
    );
    assert!(context.errors.is_empty());
}
```

- [ ] **Step 2: Run one failing collection test**

Run: `cargo test -p rock-lib test_build_struct_accepts_borrowed_str_field_type -- --exact`

Expected: FAIL because collection-time lowering still maps `Str` to a borrowed `[U8]` alias.

- [ ] **Step 3: Mirror the lowerer helper in `CollectContext`**

In `lib/src/collect/context.rs`, change the `ParseType::Type` arm to pass the context flag:

```rust
ast::ParseType::Type(inner) => {
    self.lower_parse_type_inner_with_slice_context(inner, allow_bare_slice)
}
```

Replace `lower_parse_type_inner` with this wrapper plus helper:

```rust
pub(crate) fn lower_parse_type_inner(&mut self, inner: &ast::ParseTypeInner) -> Type {
    self.lower_parse_type_inner_with_slice_context(inner, false)
}

fn lower_parse_type_inner_with_slice_context(
    &mut self,
    inner: &ast::ParseTypeInner,
    allow_bare_slice: bool,
) -> Type {
    let generics: Vec<Type> = inner
        .generics
        .iter()
        .map(|generic| self.lower_parse_type(generic))
        .collect();

    match inner.name.as_str() {
        "I8" | "i8" => Type::I8,
        "I16" | "i16" => Type::I16,
        "I32" | "i32" | "Int" => Type::I32,
        "I64" | "i64" => Type::I64,
        "U8" | "u8" => Type::U8,
        "U16" | "u16" => Type::U16,
        "U32" | "u32" => Type::U32,
        "U64" | "u64" => Type::U64,
        "F32" | "f32" => Type::F32,
        "F64" | "f64" | "Float" => Type::F64,
        "Bool" | "bool" => Type::Bool,
        "Char" | "char" => Type::Char,
        "Str" => {
            if !allow_bare_slice {
                self.push_error(
                    "bare string slice type Str must be written behind a reference, such as &Str"
                        .to_string(),
                );
                return Type::Error;
            }
            Type::Str
        }
        name => {
            if self.structs.contains_key(name) {
                Type::Struct(name.to_string(), generics)
            } else if self.enums.contains_key(name) {
                Type::Enum(name.to_string(), generics)
            } else {
                Type::Generic(name.to_string())
            }
        }
    }
}
```

- [ ] **Step 4: Run collection tests**

Run: `cargo test -p rock-lib test_build_struct_rejects_bare_str_field_type -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_build_struct_accepts_borrowed_str_field_type -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit task boundary if commits are approved**

```bash
git add lib/src/collect/context.rs lib/src/collect/headers.rs
git commit -m "collect: require borrowed Str source types"
```

---

### Task 4: String Literals Lower To `&Str`

**Files:**
- Modify: `lib/src/lower/expression.rs:805-812`
- Test: `lib/src/lower/expression.rs`

- [ ] **Step 1: Add a focused literal-lowering test**

Add this test module to the bottom of `lib/src/lower/expression.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::ast::{Literal, LiteralKind};
    use crate::lexer::Span;
    use crate::lower::Lowerer;
    use crate::types::Type;

    #[test]
    fn test_string_literal_lowers_to_borrowed_str() {
        let mut lowerer = Lowerer::new();
        let hir = lowerer.lower_literal(&Literal {
            kind: LiteralKind::String("hello".to_string()),
            span: Span::default(),
        });

        assert_eq!(
            hir.ty,
            Type::Reference {
                mutable: false,
                inner: Box::new(Type::Str),
            }
        );
    }
}
```

- [ ] **Step 2: Run the failing literal test**

Run: `cargo test -p rock-lib test_string_literal_lowers_to_borrowed_str -- --exact`

Expected: FAIL because string literals currently lower to `&[U8]`.

- [ ] **Step 3: Change string literal lowering**

In `lib/src/lower/expression.rs`, replace the string literal type with:

```rust
ast::LiteralKind::String(s) => HirExpr {
    ty: Type::Reference {
        mutable: false,
        inner: Box::new(Type::Str),
    },
    kind: HirExprKind::StringLiteral(s.clone()),
    span,
},
```

- [ ] **Step 4: Run the literal test**

Run: `cargo test -p rock-lib test_string_literal_lowers_to_borrowed_str -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit task boundary if commits are approved**

```bash
git add lib/src/lower/expression.rs
git commit -m "lower: give string literals borrowed Str type"
```

---

### Task 5: Add `&Str` Fat-Pointer ABI And `BorrowStr`

**Files:**
- Modify: `lib/src/lower/intrinsics.rs:63-148`
- Modify: `lib/src/lower/control_flow/secondary.rs:429-476`
- Modify: `lib/src/codegen/types.rs:26-37`
- Modify: `lib/src/codegen/expr/access.rs:148-249`
- Modify: `lib/src/codegen/intrinsics.rs:437-616`
- Test: `lib/src/codegen/types.rs:468-511`

- [ ] **Step 1: Add a codegen ABI test for `&Str`**

Add this test to the existing `tests` module in `lib/src/codegen/types.rs`:

```rust
#[test]
fn test_llvm_type_for_str_reference_is_fat() {
    let context = inkwell::context::Context::create();
    let codegen = CodeGen::new(&context, "test");
    let ty = Type::Reference {
        mutable: false,
        inner: Box::new(Type::Str),
    };

    assert!(codegen.llvm_type(&ty).is_struct_type());
}
```

- [ ] **Step 2: Run the failing ABI test**

Run: `cargo test -p rock-lib test_llvm_type_for_str_reference_is_fat -- --exact`

Expected: FAIL because `&Str` currently lowers as a thin pointer.

- [ ] **Step 3: Make references and pointers to `Str` fat**

In `lib/src/codegen/types.rs`, update `is_fat_pointer_type`:

```rust
pub(crate) fn is_fat_pointer_type(&self, ty: &Type) -> bool {
    match self.resolve_projection_type(ty) {
        Type::Reference { inner, .. } | Type::Pointer(inner) => {
            matches!(inner.as_ref(), Type::Slice(_) | Type::Str)
        }
        _ => false,
    }
}
```

- [ ] **Step 4: Add `BorrowStr` to intrinsic metadata**

In `lib/src/lower/intrinsics.rs`, add `BorrowStr` to `is_intrinsic_name` next to `BorrowSlice`:

```rust
| "PtrOffset" | "MakeArr" | "BorrowSlice" | "BorrowStr" | "ArrPtr" | "SizeOf"
```

Add this match arm to `infer_intrinsic_return_type`:

```rust
"BorrowStr" => {
    return Type::Reference {
        mutable: false,
        inner: Box::new(Type::Str),
    };
}
```

Add this match arm to `infer_intrinsic_arg_types`:

```rust
"BorrowStr" => return vec![Type::Pointer(Box::new(Type::U8)), Type::I64],
```

Extend the existing `ArrPtr` return inference so `&Str` returns `*U8`:

```rust
Type::Reference { inner, .. } => match inner.as_ref() {
    Type::Slice(elem_ty) => return Type::Pointer(elem_ty.clone()),
    Type::Str => return Type::Pointer(Box::new(Type::U8)),
    _ => {}
},
```

- [ ] **Step 5: Allow `ArrayLen` and `ArrPtr` on `&Str` during lowering**

In `lib/src/lower/control_flow/secondary.rs`, include `BorrowStr` in the unsafe intrinsic list:

```rust
let unsafe_intrinsics = ["PtrOffset", "MakeArr", "BorrowSlice", "BorrowStr"];
```

Replace the `ArrPtr`/`ArrayLen` argument handling block with:

```rust
if (name == "ArrPtr" || name == "ArrayLen") && !hir_args.is_empty() {
    let resolved_arg_ty = self.resolve_projection_type(&self.engine.resolve(&hir_args[0].ty));

    match &resolved_arg_ty {
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Str) => {
            if name == "ArrPtr" {
                arrptr_elem_ty = Some(Type::U8);
            }
        }
        Type::Reference { inner, .. }
            if matches!(inner.as_ref(), Type::Slice(_) | Type::Array(_, _)) =>
        {
        }
        Type::Array(_, _) if name == "ArrPtr" => {
            self.push_error_with_span(
                format!("ArrPtr expected slice, got {}", resolved_arg_ty),
                hir_args[0].span.clone(),
            );
            intrinsic_ty_error = true;
        }
        _ => {
            let elem_ty = self.engine.fresh_type_var();
            let array_ty = Type::Slice(Box::new(elem_ty.clone()));
            let _ = self.engine.unify(&hir_args[0].ty, &array_ty);
            if name == "ArrPtr" {
                arrptr_elem_ty = Some(elem_ty);
            }
        }
    }
}
```

- [ ] **Step 6: Extract fat parts from `Str` and `&Str`**

In `lib/src/codegen/expr/access.rs`, extend `slice_parts_from_value` with `Type::Str` and `&Str` cases:

```rust
Type::Slice(_) | Type::Str => {
    let slice = value.into_struct_value();
    let data_ptr = self
        .builder
        .build_extract_value(slice, 0, "slice_ptr")
        .map_err(|e| CodegenError::from(format!("Failed to extract slice ptr: {}", e)))?
        .into_pointer_value();
    let len = self
        .builder
        .build_extract_value(slice, 1, "slice_len")
        .map_err(|e| CodegenError::from(format!("Failed to extract slice len: {}", e)))?
        .into_int_value();
    Ok((data_ptr, len))
}
Type::Pointer(inner) if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
    let slice = value.into_struct_value();
    let data_ptr = self
        .builder
        .build_extract_value(slice, 0, "slice_ptr")
        .map_err(|e| CodegenError::from(format!("Failed to extract slice ptr: {}", e)))?
        .into_pointer_value();
    let len = self
        .builder
        .build_extract_value(slice, 1, "slice_len")
        .map_err(|e| CodegenError::from(format!("Failed to extract slice len: {}", e)))?
        .into_int_value();
    Ok((data_ptr, len))
}
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
    let slice = value.into_struct_value();
    let data_ptr = self
        .builder
        .build_extract_value(slice, 0, "slice_ref_ptr")
        .map_err(|e| CodegenError::from(format!("Failed to extract slice ref ptr: {}", e)))?
        .into_pointer_value();
    let len = self
        .builder
        .build_extract_value(slice, 1, "slice_ref_len")
        .map_err(|e| CodegenError::from(format!("Failed to extract slice ref len: {}", e)))?
        .into_int_value();
    Ok((data_ptr, len))
}
```

- [ ] **Step 7: Add codegen for `BorrowStr`, `ArrayLen(&Str)`, and `ArrPtr(&Str)`**

In `lib/src/codegen/intrinsics.rs`, extend `ArrayLen` reference matching:

```rust
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
    let arr = match compiled_args[0] {
        BasicValueEnum::PointerValue(ptr) => self
            .builder
            .build_load(self.slice_layout_type(), ptr, "slice_ref_load")
            .map_err(|e| CodegenError::from(format!("Failed to load slice ref: {}", e)))?
            .into_struct_value(),
        value => value.into_struct_value(),
    };
    let len = self
        .builder
        .build_extract_value(arr, 1, "arr_len")
        .map_err(|e| CodegenError::from(format!("Failed to extract array length: {}", e)))?;
    Ok(Some(len))
}
```

Add `BorrowStr` next to `BorrowSlice`:

```rust
"BorrowStr" => {
    let ptr = compiled_args[0].into_pointer_value();
    let len = compiled_args[1].into_int_value();
    let str_ref_ty = self.slice_layout_type();
    let mut val = str_ref_ty.get_undef();
    val = self
        .builder
        .build_insert_value(val, ptr, 0, "borrow_str_ptr")
        .map_err(|e| CodegenError::from(format!("Failed BorrowStr ptr: {}", e)))?
        .into_struct_value();
    val = self
        .builder
        .build_insert_value(val, len, 1, "borrow_str_len")
        .map_err(|e| CodegenError::from(format!("Failed BorrowStr len: {}", e)))?
        .into_struct_value();
    Ok(Some(val.into()))
}
```

Extend `ArrPtr` reference matching:

```rust
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
    let arr = match compiled_args[0] {
        BasicValueEnum::PointerValue(ptr) => self
            .builder
            .build_load(self.slice_layout_type(), ptr, "slice_ref_load")
            .map_err(|e| CodegenError::from(format!("Failed to load slice ref: {}", e)))?
            .into_struct_value(),
        value => value.into_struct_value(),
    };
    let ptr = self
        .builder
        .build_extract_value(arr, 0, "arr_raw_ptr")
        .map_err(|e| CodegenError::from(format!("Failed ArrPtr: {}", e)))?;
    Ok(Some(ptr))
}
```

- [ ] **Step 8: Run ABI and existing intrinsic tests**

Run: `cargo test -p rock-lib test_llvm_type_for_str_reference_is_fat -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_arr_ptr_rejects_fixed_array_values -- --exact`

Expected: PASS.

- [ ] **Step 9: Commit task boundary if commits are approved**

```bash
git add lib/src/lower/intrinsics.rs lib/src/lower/control_flow/secondary.rs lib/src/codegen/types.rs lib/src/codegen/expr/access.rs lib/src/codegen/intrinsics.rs
git commit -m "codegen: support borrowed Str fat pointers"
```

---

### Task 6: Make Method Lookup See `&Str` And Stop Using `[U8]` As Text

**Files:**
- Modify: `lib/src/lower/types_helpers/helpers.rs:11-31,448-523`
- Modify: `lib/src/lower/control_flow/secondary.rs:22-34,80-180`
- Modify: `lib/src/codegen/mod.rs:263-281`
- Modify: `lib/src/codegen/types.rs:254-281`
- Modify: `lib/src/mono/mod.rs:378-416`
- Modify: `lib/src/infer/solve.rs:158-180`

- [ ] **Step 1: Add focused lookup-helper tests**

Add this test module to the bottom of `lib/src/lower/types_helpers/helpers.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::lower::Lowerer;
    use crate::types::Type;

    #[test]
    fn test_method_lookup_names_for_borrowed_str_include_reference_spelling() {
        let ty = Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        };

        assert_eq!(
            Lowerer::get_type_names_for_method_lookup(&ty),
            vec!["&Str".to_string(), "Str".to_string()]
        );
    }

    #[test]
    fn test_method_lookup_names_for_borrowed_u8_slice_do_not_include_u8_text_alias() {
        let ty = Type::Reference {
            mutable: false,
            inner: Box::new(Type::Slice(Box::new(Type::U8))),
        };
        let names = Lowerer::get_type_names_for_method_lookup(&ty);

        assert!(names.contains(&"&[U8]".to_string()));
        assert!(names.contains(&"Array".to_string()));
        assert!(!names.contains(&"[U8]".to_string()));
    }
}
```

- [ ] **Step 2: Run the new lookup-helper test and verify failure**

Run: `cargo test -p rock-lib test_method_lookup_names_for_borrowed_str_include_reference_spelling -- --exact`

Expected: FAIL because `&Str` is not currently included as its own method lookup spelling.

- [ ] **Step 3: Update lowerer method lookup names**

In `lib/src/lower/types_helpers/helpers.rs`, replace `get_type_names_for_method_lookup` with:

```rust
pub(crate) fn get_type_names_for_method_lookup(ty: &Type) -> Vec<String> {
    match ty {
        Type::Slice(_) => vec!["Array".to_string()],
        Type::Array(_, _) => vec![ty.to_string(), "Array".to_string()],
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
            let mut names = vec![ty.to_string()];
            names.extend(Self::get_type_names_for_method_lookup(inner));
            names
        }
        Type::Reference { inner, .. } => Self::get_type_names_for_method_lookup(inner),
        _ => Self::get_type_name_for_method_lookup(ty).into_iter().collect(),
    }
}
```

In the same file, update `get_type_name_for_method_lookup` so `Str` is distinct and byte slices are not string names:

```rust
Type::Str => Some("Str".to_string()),
Type::Slice(_) => Some("Array".to_string()),
Type::Array(_, _) => Some(ty.to_string()),
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
    Some(ty.to_string())
}
Type::Reference { inner, .. } => Self::get_type_name_for_method_lookup(inner),
```

Update `type_from_name` so `Str` is not `[U8]`:

```rust
"Str" => Type::Str,
"[U8]" => Type::Slice(Box::new(Type::U8)),
```

- [ ] **Step 4: Match generic borrowed-slice impl names by shape**

In `lib/src/lower/control_flow/secondary.rs`, replace `receiver_type_name_matches` with:

```rust
fn receiver_type_name_matches(candidate: &str, lookup: &str) -> bool {
    if candidate == lookup {
        return true;
    }

    let borrowed_slice = |name: &str| {
        name.starts_with("&[") && name.ends_with(']') && !name.contains(';')
    };
    let mut_borrowed_slice = |name: &str| {
        name.starts_with("&mut [") && name.ends_with(']') && !name.contains(';')
    };

    if (borrowed_slice(candidate) && borrowed_slice(lookup))
        || (mut_borrowed_slice(candidate) && mut_borrowed_slice(lookup))
    {
        return true;
    }

    match (
        Self::fixed_array_len_from_type_name(candidate),
        Self::fixed_array_len_from_type_name(lookup),
    ) {
        (Some(candidate_len), Some(lookup_len)) => candidate_len == lookup_len,
        _ => false,
    }
}
```

In `concrete_method_candidate`, include `Str` receiver argument extraction for references:

```rust
let receiver_arg_types = match &candidate_ty {
    Type::Struct(_, args) | Type::Enum(_, args) => args.clone(),
    Type::Slice(inner) => vec![inner.as_ref().clone()],
    Type::Array(inner, _) => vec![inner.as_ref().clone()],
    Type::Reference { inner, .. } => match inner.as_ref() {
        Type::Slice(elem) | Type::Array(elem, _) => vec![elem.as_ref().clone()],
        Type::Str => vec![],
        _ => vec![],
    },
    _ => vec![],
};
```

- [ ] **Step 5: Update codegen and mono type-name helpers**

In `lib/src/codegen/mod.rs`, replace `get_type_names_for_method` with the same shape as the lowerer helper:

```rust
fn get_type_names_for_method(recv_ty: &Type) -> Vec<String> {
    match recv_ty {
        Type::Slice(_) => vec!["Array".to_string()],
        Type::Array(_, _) => vec![recv_ty.to_string(), "Array".to_string()],
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
            let mut names = vec![recv_ty.to_string()];
            names.extend(Self::get_type_names_for_method(inner));
            names
        }
        Type::Reference { inner, .. } => Self::get_type_names_for_method(inner),
        _ => vec![Self::get_type_name_for_method(recv_ty)],
    }
}
```

In `lib/src/codegen/types.rs`, change `get_type_name_for_method` arms:

```rust
Type::Str => "Str".to_string(),
Type::Slice(_) => "Array".to_string(),
Type::Array(_, _) => ty.to_string(),
Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Slice(_) | Type::Str) => {
    ty.to_string()
}
Type::Reference { inner, .. } => Self::get_type_name_for_method(inner),
```

In `lib/src/mono/mod.rs`, update `lookup_type_names_for_receiver` and `get_type_name_for_method` with the same rules as lower/codegen.

In `lib/src/infer/solve.rs`, update `type_base_name` so `Type::Slice(_)` returns `"Array"` and never returns `"[U8]"` as a text identity:

```rust
Type::Str => "Str".into(),
Type::Slice(_) => "Array".into(),
Type::Array(_, _) => ty.to_string(),
```

- [ ] **Step 6: Run lookup-helper tests**

Run: `cargo test -p rock-lib test_method_lookup_names_for_borrowed_str_include_reference_spelling -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_method_lookup_names_for_borrowed_u8_slice_do_not_include_u8_text_alias -- --exact`

Expected: PASS.

- [ ] **Step 7: Commit task boundary if commits are approved**

```bash
git add lib/src/lower/types_helpers/helpers.rs lib/src/lower/control_flow/secondary.rs lib/src/codegen/mod.rs lib/src/codegen/types.rs lib/src/mono/mod.rs lib/src/infer/solve.rs
git commit -m "lower: separate Str lookup from byte slices"
```

---

### Task 7: Move Stdlib Text APIs To `&Str`

**Files:**
- Modify: `stdlib/show.rk`
- Modify: `stdlib/eq.rk`
- Modify: `stdlib/string_type.rk`
- Modify: `stdlib/string.rk`
- Modify: `stdlib/convert.rk`

- [ ] **Step 1: Update `String::from_str`**

In `stdlib/string_type.rk`, change the signature:

```rock
impl String
    from_str: &Str -> String
    from_str = s ->
        slice_len = ~ArrayLen s
        buf = malloc (slice_len + 1)
        slice_ptr = ~ArrPtr s
        unsafe
            memcpy buf, slice_ptr, slice_len
            *(buf + slice_len) = 0
        String
            raw_ptr: buf
            raw_len: slice_len
            raw_cap: (slice_len + 1)
```

- [ ] **Step 2: Update text conversion functions**

In `stdlib/convert.rk`, change `int_to_string` and `float_to_string` to return `&Str` through `~BorrowStr`:

```rock
// Convert integer to string, returns &Str slice
int_to_string: I64 -> &Str
< int_to_string = x ->
    buf = malloc 21
    unsafe *(buf + 20) = 0
    is_neg = x < 0
    n = if is_neg
        0 - x
    else
        x
    pos = 19
    if n == 0
        unsafe *(buf + pos) = 48
        pos = pos - 1
    while n > 0
        digit = n % 10
        unsafe *(buf + pos) = (digit + 48) as U8
        pos = pos - 1
        n = n / 10
    if is_neg
        unsafe *(buf + pos) = 45
        pos = pos - 1
    start = pos + 1
    len = 19 - pos
    unsafe ~BorrowStr (buf + start), len

// Convert float to string, returns &Str slice
float_to_string: F64 -> &Str
< float_to_string = x ->
    buf = malloc 32
    gcvt x, 6, buf
    unsafe ~BorrowStr buf, (strlen buf)

string_to_int: &Str -> I64
< string_to_int = s -> atol (~ArrPtr s)

string_to_float: &Str -> F64
< string_to_float = s -> atof (~ArrPtr s)
```

- [ ] **Step 3: Update string helpers**

In `stdlib/string.rk`, make text APIs accept `&Str`. Keep substring helpers byte-oriented and introduce no unchecked `&Str` substring return.

```rock
// Extract raw pointer from &Str fat pointer
str_raw_ptr: &Str -> *U8
< str_raw_ptr = s -> ~ArrPtr s

// Get the byte length of a UTF-8 string slice
string_len: &Str -> I64
< string_len = s -> ~ArrayLen s

// Get the byte at index idx in a byte slice
byte_at: &[U8] -> I64 -> U8
< byte_at = s, idx ->
    p = ~ArrPtr s
    unsafe *(p + idx)

// Concatenate two string slices. Concatenating valid UTF-8 preserves UTF-8 validity.
string_concat: &Str -> &Str -> &Str
< string_concat = a, b ->
    len_a = string_len a
    len_b = string_len b
    total = len_a + len_b
    buf = malloc (total + 1)
    memcpy buf, (~ArrPtr a), len_a
    unsafe
        memcpy (buf + len_a), (~ArrPtr b), len_b
        *(buf + total) = 0
        ~BorrowStr buf, total

// Extract byte substring starting at `start` with length `len`
byte_substr: &[U8] -> I64 -> I64 -> &[U8]
< byte_substr = s, start, len ->
    buf = malloc (len + 1)
    unsafe
        memcpy buf, ((~ArrPtr s) + start), len
        *(buf + len) = 0
        &(~MakeArr buf, len)

// Find needle in haystack; returns byte offset or -1 if not found
string_find: &Str -> &Str -> I64
< string_find = haystack, needle ->
    h_ptr = ~ArrPtr haystack
    result = strstr h_ptr, (~ArrPtr needle)
    result_int = result as I64
    h_int = h_ptr as I64
    if result_int == 0
        0 - 1
    else
        result_int - h_int

// Return 1 if haystack contains needle, 0 otherwise
string_contains: &Str -> &Str -> I64
< string_contains = haystack, needle ->
    idx = string_find haystack, needle
    if idx >= 0
        1
    else
        0
```

- [ ] **Step 4: Update `Show` impls**

In `stdlib/show.rk`, update `Char` and move string display to `&Str`:

```rock
impl Show for Char
    @show = ->
        buf = malloc 1
        unsafe
            *buf = self as U8
            String::from_str (~BorrowStr buf, 1)

impl Show for &[T] where T: Show
    @show = ->
        String::from_str "[]"

impl Show for &Str
    @show = ->
        String::from_str self

    @println = ->
        puts (~ArrPtr self)
```

Remove the old `impl Show for &[U8]` block.

- [ ] **Step 5: Update string equality**

In `stdlib/eq.rk`, replace the `[U8]` impl with:

```rock
// &Str implementation for UTF-8 string slices / static string literals
impl Eq for &Str
    @== = other ->
        result = strcmp (~ArrPtr self), (~ArrPtr other)
        ~I32Eq result, 0
    @!= = other ->
        result = strcmp (~ArrPtr self), (~ArrPtr other)
        ~I32Ne result, 0
```

- [ ] **Step 6: Run a stdlib-backed smoke test**

Run: `cargo test -p rock-lib --test integration test_stdlib_math -- --exact`

Expected: PASS after Tasks 5 and 6 are complete.

- [ ] **Step 7: Commit task boundary if commits are approved**

```bash
git add stdlib/show.rk stdlib/eq.rk stdlib/string_type.rk stdlib/string.rk stdlib/convert.rk
git commit -m "stdlib: move text APIs to borrowed Str"
```

---

### Task 8: Reject `Str` Indexing And Update User-Visible Tests

**Files:**
- Modify: `lib/src/lower/control_flow/secondary.rs:826-918`
- Modify: `lib/tests/integration.rs:253-371`

- [ ] **Step 1: Replace stale string-indexing tests**

In `lib/tests/integration.rs`, replace `test_string_literal_still_behaves_like_slice` with:

```rust
#[test]
fn test_string_literal_indexing_is_rejected() {
    compile_should_fail(
        r#"
main = ->
    s = "abc"
    (s[1]).println!
    0
"#,
        "cannot index Str by integer",
    );
}
```

Replace `test_borrowed_slice_struct_field_passes_as_fat_ref` with a byte-slice-only version:

```rust
#[test]
fn test_borrowed_u8_slice_struct_field_indexes_as_byte() {
    let output = compile_and_run(
        r#"
struct Holder
    < data: [U8]

second: &[U8] -> U8
second = s -> s[1]

main = ->
    holder = Holder
        data: [97, 98, 99]
    slice = &holder.data
    (second slice as I64).println!
    0
"#,
    );

    assert_eq!(output.trim(), "98");
}
```

In tests that define `impl SliceLen for &[T]` and then call it on a string literal, change the receiver to a fixed array borrow:

```rock
main = ->
    arr = [1, 2, 3]
    (&arr).slice_len!.println!
    0
```

Replace `test_string_builtins` with a version that exercises UTF-8-safe string helpers only:

```rust
#[test]
fn test_string_builtins() {
    let output = compile_and_run(
        r#"
main = ->
    s = "Hello, World!"
    (string_len s).println!
    (string_find s, "World").println!
    (string_find s, "xyz").println!
    (string_contains s, "Hello").println!
    (string_contains s, "xyz").println!
    0
"#,
    );
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines[0], "13");
    assert_eq!(lines[1], "7");
    assert_eq!(lines[2], "-1");
    assert_eq!(lines[3], "1");
    assert_eq!(lines[4], "0");
}
```

- [ ] **Step 2: Add bare `Str` source rejection integration test**

Add this test near `test_bare_slice_impl_target_is_rejected`:

```rust
#[test]
fn test_bare_str_parameter_is_rejected() {
    compile_should_fail(
        r#"
len: Str -> I64
len = s -> ~ArrayLen s

main = -> 0
"#,
        "bare string slice type Str must be written behind a reference",
    );
}
```

- [ ] **Step 3: Add lookup integration tests for `&Str` vs `&[U8]`**

Add these tests near the existing slice/string tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_str_trait_impl_uses_borrowed_str_not_u8_slice() {
    let output = compile_and_run(
        r#"
trait Kind
    @kind = -> 0

impl Kind for &Str
    @kind = -> 7

impl Kind for &[U8]
    @kind = -> 3

main = ->
    s = "abc"
    s.kind!.println!
    0
"#,
    );

    assert_eq!(output.trim(), "7");
}

#[test]
fn test_u8_slice_trait_impl_does_not_apply_to_string_literal() {
    compile_should_fail(
        r#"
trait BytesOnly
    @bytes_only = -> 0

impl BytesOnly for &[U8]
    @bytes_only = -> 1

main = ->
    s = "abc"
    s.bytes_only!.println!
    0
"#,
        "bytes_only",
    );
}
```

- [ ] **Step 4: Reject `Str` indexing in lowering**

In `lib/src/lower/control_flow/secondary.rs`, before the builtin-index candidate search, add:

```rust
if matches!(resolved_expr_ty, Type::Str)
    || matches!(
        &resolved_expr_ty,
        Type::Reference { inner, .. } if matches!(inner.as_ref(), Type::Str)
    )
{
    self.push_error_with_span(
        "cannot index Str by integer; string slices are UTF-8 text, use an explicit string or byte API"
            .to_string(),
        span.clone(),
    );
    return HirExpr {
        ty: Type::Error,
        kind: HirExprKind::Index(Box::new(expr), Box::new(index)),
        span,
    };
}
```

Remove the old string-literal bounds check block that special-cases `Type::Slice(U8)` and `Type::Array(U8, _)` as strings.

- [ ] **Step 5: Run the updated integration tests**

Run: `cargo test -p rock-lib --test integration test_string_literal_indexing_is_rejected -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_borrowed_u8_slice_struct_field_indexes_as_byte -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_bare_str_parameter_is_rejected -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_str_trait_impl_uses_borrowed_str_not_u8_slice -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_u8_slice_trait_impl_does_not_apply_to_string_literal -- --exact`

Expected: PASS.

- [ ] **Step 6: Commit task boundary if commits are approved**

```bash
git add lib/src/lower/control_flow/secondary.rs lib/tests/integration.rs
git commit -m "lower: reject integer indexing on Str"
```

---

### Task 9: Refresh Mono Tests And Remaining `[U8]` String Assumptions

**Files:**
- Modify: `lib/src/mono/methods.rs:814-1049`
- Modify: `lib/src/mono/external.rs`
- Modify: `lib/src/products.rs:724-730`
- Modify: `lib/src/lower/mod.rs:650-688`

- [ ] **Step 1: Update artificial extern test types away from bare `Type::Str` parameters**

In `lib/src/lower/mod.rs`, change artificial `puts`-like extern params from `Type::Str` to `Type::Pointer(Box::new(Type::U8))`:

```rust
params: vec![Type::Pointer(Box::new(Type::U8))],
```

Make the same change in `lib/src/products.rs` for artificial extern test fixtures.

- [ ] **Step 2: Rename mono test helpers to byte-slice language**

In `lib/src/mono/methods.rs`, rename `slice_u8_println_method` to `byte_slice_println_method` and change its qualified name to a byte-oriented label:

```rust
fn byte_slice_println_method() -> HirFunction {
    HirFunction {
        id: DefId::new(CrateId(0), LocalDefId(0)),
        name: "println".to_string(),
        qualified_name: Some("stdlib::&[U8]_Bytes_println".to_string()),
        generic_params: vec![],
        generic_bounds: HashMap::new(),
        params: vec![HirParam {
            name: "self".to_string(),
            ty: Type::Reference {
                mutable: false,
                inner: Box::new(Type::Slice(Box::new(Type::U8))),
            },
            mutable: false,
            is_ref: false,
        }],
        ret_type: Type::I32,
        body: empty_body(Type::Unit),
        is_curried: false,
        is_method: true,
        self_receiver: Some(SelfReceiverMode::Move),
        is_unsafe: false,
    }
}
```

- [ ] **Step 3: Replace mono preference test with no-string-fallback assertion**

Replace `test_monomorphize_trait_method_call_prefers_concrete_slice_impl_over_generic_builtin_slice_impl` with:

```rust
#[test]
fn test_monomorphize_trait_method_call_does_not_treat_str_as_u8_slice() {
    let mut mono = Monomorphizer::new();
    seed_owner(&mut mono, "&[U8]");
    mono.trait_impls.insert(
        "Bytes".to_string(),
        vec![HirImpl {
            id: DefId::new(CrateId(0), LocalDefId(0)),
            owner: HirImplOwner::BuiltinSlice,
            type_name: "&[U8]".to_string(),
            type_generics: vec![],
            receiver_arg_types: vec![Type::U8],
            trait_name: Some("Bytes".to_string()),
            trait_generics: vec![],
            trait_arg_types: vec![],
            associated_types: vec![],
            bounds: vec![],
            methods: HashMap::from([("println".to_string(), byte_slice_println_method())]),
        }],
    );

    let recv = HirExpr {
        kind: HirExprKind::Var("text".to_string()),
        ty: Type::Reference {
            mutable: false,
            inner: Box::new(Type::Str),
        },
        span: Span::default(),
    };
    let mut expr = HirExpr {
        kind: HirExprKind::MethodCall(
            Box::new(recv.clone()),
            "println".to_string(),
            vec![],
            Some(SelfReceiverMode::Move),
        ),
        ty: Type::I32,
        span: Span::default(),
    };

    mono.monomorphize_trait_method_call("&Str", "println", &[recv], &mut expr);

    assert!(matches!(expr.kind, HirExprKind::MethodCall(..)));
}
```

- [ ] **Step 4: Update `mono/external.rs` tests that seed `[U8]` as text**

In `lib/src/mono/external.rs`, keep these tests byte-slice oriented by replacing concrete `[U8]` fixtures with borrowed byte-slice fixtures:

```rust
let byte_slice_ty = Type::Reference {
    mutable: false,
    inner: Box::new(Type::Slice(Box::new(Type::U8))),
};

let concrete_impl = HirImpl {
    id: DefId::new(CrateId(0), LocalDefId(0)),
    owner: HirImplOwner::BuiltinSlice,
    type_name: "&[U8]".to_string(),
    type_generics: vec![],
    receiver_arg_types: vec![Type::U8],
    trait_name: Some("Show".to_string()),
    trait_generics: vec![],
    trait_arg_types: vec![],
    associated_types: vec![],
    bounds: vec![],
    methods: HashMap::from([("println".to_string(), println_method(byte_slice_ty.clone()))]),
};
```

In `test_load_external_generic_functions_keeps_concrete_object_backed_trait_impls`, change the assertion and receiver to borrowed byte-slice spelling:

```rust
assert!(show_impls.iter().any(|imp| imp.type_name == "&[U8]"));

let recv = HirExpr {
    kind: HirExprKind::Var("bytes".to_string()),
    ty: byte_slice_ty,
    span: Span::default(),
};
```

Change the monomorphization call in that test to:

```rust
mono.monomorphize_trait_method_call("&[U8]", "println", &[recv], &mut expr);
```

- [ ] **Step 5: Run mono and fixture tests**

Run: `cargo test -p rock-lib test_monomorphize_trait_method_call_does_not_treat_str_as_u8_slice -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib mono::external -- --nocapture`

Expected: PASS.

- [ ] **Step 6: Commit task boundary if commits are approved**

```bash
git add lib/src/mono/methods.rs lib/src/mono/external.rs lib/src/products.rs lib/src/lower/mod.rs
git commit -m "test: remove byte-slice string assumptions"
```

---

### Task 10: Final Verification

**Files:**
- Verify all modified files.

- [ ] **Step 1: Format Rust code**

Run: `cargo fmt --all`

Expected: command exits successfully.

- [ ] **Step 2: Run focused regressions**

Run: `cargo test -p rock-lib test_lower_parse_bare_str_reports_error -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib test_string_literal_lowers_to_borrowed_str -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_string_literal_indexing_is_rejected -- --exact`

Expected: PASS.

Run: `cargo test -p rock-lib --test integration test_stdlib_math -- --exact`

Expected: PASS.

- [ ] **Step 3: Run the main documented library test suite**

Run: `cargo test -p rock-lib`

Expected: PASS.

- [ ] **Step 4: Inspect remaining production `[U8]` and `Str` references**

Run: `rg '"\[U8\]"|Type::Str|"Str"' lib/src stdlib`

Expected: remaining matches are legitimate `Type::Str` handling, byte-slice spelling, tests, or `&Str` stdlib APIs; no match should map `Str` to `Type::Slice(Box::new(Type::U8))`.

- [ ] **Step 5: Commit final verification if commits are approved**

```bash
git add lib/src stdlib lib/tests/integration.rs
git commit -m "test: verify borrowed slices and Str separation"
```

---

## Self-Review

- Spec coverage: Tasks 1-4 cover source type rules and literal lowering; Tasks 5-6 cover ABI, intrinsics, and lookup; Task 7 covers stdlib surface; Task 8 covers diagnostics and integration regressions; Task 9 covers stale mono/artifact test assumptions; Task 10 covers final verification.
- Placeholder scan: The plan contains concrete files, commands, expected outcomes, and code snippets for each implementation step.
- Type consistency: `Str` is always the unsized referent and `&Str` is always represented as `Type::Reference { inner: Type::Str }`; byte slices remain `Type::Slice(Type::U8)` and `&[U8]` remains `Type::Reference { inner: Type::Slice(Type::U8) }`.
