# Macro Architecture Spec Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the final macro architecture spec gaps left after the 14-task implementation plan.

**Architecture:** Keep the existing macro expansion phase and Rock syntax, but make the compiled declarative model the active matcher/template boundary. Preserve generated token origin metadata through template expansion, enrich the process-isolated proc-macro protocol metadata, and make diagnostics/verification stable enough for final branch review.

**Tech Stack:** Rust 2021, `rock-lib`, existing parser/lexer/diagnostics, `serde`, `bincode`, focused unit tests, integration tests, `cargo fmt --all --check`, `git diff --check`, `cargo test -p rock-lib`.

---

## Approved Delta Spec

- `docs/superpowers/specs/2026-05-24-macro-architecture-spec-completion-design.md`

## Worktree Constraint

All work for this plan must happen in the isolated worktree:

- Path: `/root/new_lang2/.worktrees/macro-architecture-redesign`
- Branch: `macro-architecture-redesign`

Do not touch `/root/new_lang2/.sisyphus/`. Do not push.

## File Structure

- Modify `lib/src/macro_expansion/mod.rs`: expansion orchestration, depth diagnostics, proc-macro test fixture, origin-aware generated parsing calls.
- Modify `lib/src/macro_expansion/context.rs`: latest expansion tracking and generated source lookup helpers.
- Modify `lib/src/macro_expansion/macro_arg_matcher.rs`: consume compiled `MatcherFragment` instead of AST `MacroFragment`.
- Modify `lib/src/macro_expansion/declarative.rs`: origin-aware capture/template expansion helpers.
- Modify `lib/src/macro_expansion/token_tree.rs`: constructors/helpers for captured/generated token origins.
- Modify `lib/src/macro_expansion/generated_parser.rs`: accept origin-carrying token streams without rebuilding all origins from source spans.
- Modify `lib/src/macro_expansion/proc_macro.rs`: richer artifact/export/request metadata and protocol tests.
- Modify `lib/src/products.rs` and `rock-shared/src/sysroot.rs`: product artifact format version bump for serialized proc-macro metadata.
- Modify `lib/tests/integration.rs`: public integration-style macro/proc-macro regression coverage.

## Task 1: Stabilize Proc-Macro Fixture And Add Depth Trace Diagnostics

**Files:**
- Modify: `lib/src/macro_expansion/context.rs`
- Modify: `lib/src/macro_expansion/mod.rs`

- [ ] **Step 1: Write failing depth trace assertion**

Update `macro_expansion_uses_context_depth_limit_instead_of_panicking` in `lib/src/macro_expansion/mod.rs` so it also checks trace labels:

```rust
let diagnostic = diagnostics
    .0
    .iter()
    .find(|diagnostic| diagnostic.message.contains("Macro expansion depth exceeded"))
    .expect("expected depth diagnostic");
assert!(diagnostic
    .labels
    .iter()
    .any(|(label, _)| label.contains("expanded from macro 'repeat'")));
```

- [ ] **Step 2: Run the red depth test**

Run:

```bash
cargo test -p rock-lib macro_expansion_uses_context_depth_limit_instead_of_panicking -- --nocapture
```

Expected: FAIL because the depth diagnostic has no expansion trace label.

- [ ] **Step 3: Track latest expansion in context**

In `lib/src/macro_expansion/context.rs`, add a `latest_expansion: RefCell<Option<ExpansionId>>` field, initialize it to `None`, set it in `record_expansion`, and expose:

```rust
pub fn latest_expansion_id(&self) -> Option<ExpansionId> {
    *self.latest_expansion.borrow()
}
```

- [ ] **Step 4: Attach trace labels to depth overflow**

In `expand_macros_with_context`, replace the default-only diagnostic construction with:

```rust
let span = first_macro_invocation_span(&module).unwrap_or_default();
let mut diagnostic = context.depth_exceeded_diagnostic(span);
if let Some(expansion_id) = context.latest_expansion_id() {
    diagnostic = diagnostic.with_labels(context.trace_labels(expansion_id));
}
diagnostics.push(diagnostic);
```

Add this helper near `proc_macro_error`:

```rust
fn first_macro_invocation_span(module: &Module) -> Option<Span> {
    module.top_levels.iter().find_map(|top_level| match top_level {
        TopLevel::MacroInvoc(invocation) => Some(invocation.name.span.clone()),
        _ => None,
    })
}
```

- [ ] **Step 5: Stabilize the proc-macro shell fixture**

In `proc_macro_artifact_with_response` in `lib/src/macro_expansion/mod.rs`, change the host script to consume stdin before responding:

```rust
fs::write(&host, "#!/bin/sh\ncat >/dev/null\ncat \"$0.response\"\n").unwrap();
```

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion_uses_context_depth_limit_instead_of_panicking -- --nocapture
cargo test -p rock-lib macro_expansion::tests::function_like_proc_macro_expands_top_level_invocation -- --exact --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/context.rs lib/src/macro_expansion/mod.rs
git commit -m "stabilize macro expansion diagnostics"
```

## Task 2: Use Compiled Declarative Matchers As Active Matcher Boundary

**Files:**
- Modify: `lib/src/macro_expansion/macro_arg_matcher.rs`
- Modify: `lib/src/macro_expansion/mod.rs`

- [ ] **Step 1: Write failing compiled matcher API test**

Add this test to `lib/src/macro_expansion/macro_arg_matcher.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::lexer::{Token, TokenType};
    use crate::macro_expansion::declarative::{CaptureKind, MatcherFragment};

    fn test_config() -> crate::Config {
        crate::Config {
            entry_file: PathBuf::new(),
            output_dir: PathBuf::new(),
            debug_print: Vec::new(),
            meta_files: Vec::new(),
            extern_artifacts: Vec::new(),
            current_crate_name: None,
            opt_level: 0,
            emit_llvm: false,
            no_link: false,
            emit_object: None,
            no_prelude: false,
            no_std: false,
            sysroot: None,
        }
    }

    #[test]
    fn macro_arg_matcher_accepts_compiled_matcher_fragments() {
        let config = test_config();
        let context = MacroExpansionContext::new(&config);
        let args = vec![Token::from(TokenType::Ident("main".to_string()))];
        let matcher = vec![MatcherFragment::Capture {
            name: "name".to_string(),
            kind: CaptureKind::Ident,
        }];

        let correspondance = MacroArgMatcher::new(
            &args,
            matcher,
            Span::default(),
            Span::default(),
            &context,
        )
        .run()
        .unwrap();

        assert!(correspondance.get("name", 0).is_some());
    }
}
```

- [ ] **Step 2: Run the red matcher test**

Run:

```bash
cargo test -p rock-lib macro_arg_matcher_accepts_compiled_matcher_fragments -- --nocapture
```

Expected: FAIL to compile because `MacroArgMatcher::new` expects `Vec<MacroFragment>`.

- [ ] **Step 3: Change matcher input type**

In `lib/src/macro_expansion/macro_arg_matcher.rs`, replace `MacroFragment` imports and fields with `MatcherFragment`:

```rust
use super::{
    context::MacroExpansionContext,
    correspondances::Correspondance,
    declarative::{CaptureKind, MatcherFragment},
};

struct MacroThread<'a> {
    pub args: &'a [Token],
    pub tokens: Vec<MatcherFragment>,
    pub correspondances: Correspondance,
}
```

Update the match arms:

```rust
MatcherFragment::Capture { name, kind } => match kind {
    CaptureKind::Ident => { /* existing ident branch using name.clone() */ }
    CaptureKind::Expr => { /* existing expr branch using name.clone() */ }
    CaptureKind::Type => { /* existing type branch using name.clone() */ }
},
MatcherFragment::Token(t) => { /* existing token branch */ }
MatcherFragment::Repetition(repetition) => { /* existing repetition branch */ }
```

Remove AST-fragment-specific `trim_repetition_close` from `macro_arg_matcher.rs`; compiled matcher repetitions are already trimmed by `declarative::compile_matcher_fragments`.

- [ ] **Step 4: Use compiled matchers in expansion**

In `expand_top_level` in `lib/src/macro_expansion/mod.rs`, iterate only over `declarative_macro.arms` and pass `arm.matcher.fragments.clone()` into `MacroArgMatcher::new`:

```rust
for arm in &declarative_macro.arms {
    let mut macro_matcher = MacroArgMatcher::new(
        &args,
        arm.matcher.fragments.clone(),
        macro_decl.name.span.clone(),
        invoc_span.clone(),
        context,
    );
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_arg_matcher_accepts_compiled_matcher_fragments -- --nocapture
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/macro_arg_matcher.rs lib/src/macro_expansion/mod.rs
git commit -m "match declarative macros through compiled fragments"
```

## Task 3: Preserve Captured And Generated Token Origins During Template Expansion

**Files:**
- Modify: `lib/src/macro_expansion/token_tree.rs`
- Modify: `lib/src/macro_expansion/declarative.rs`
- Modify: `lib/src/macro_expansion/context.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Modify: `lib/src/macro_expansion/generated_parser.rs`

- [ ] **Step 1: Write failing origin-aware template test**

Add this test to `lib/src/macro_expansion/declarative.rs`:

```rust
#[test]
fn declarative_template_marks_captured_and_generated_origins() {
    let expansion = ExpansionId(9);
    let generated_source = GeneratedSourceId(4);
    let invocation_span = span(1, 7);
    let capture_span = span(8, 12);
    let template_span = span(20, 21);
    let mut captures = CaptureSet::new();
    captures.insert_direct(
        "name".to_string(),
        TokenStream::captured_from_tokens(
            vec![Token {
                token_type: TokenType::Ident("main".to_string()),
                span: capture_span.clone(),
            }],
            expansion,
            invocation_span.clone(),
        ),
    );
    let template = MacroTemplate {
        fragments: vec![
            TemplateFragment::Capture {
                name: "name".to_string(),
            },
            TemplateFragment::Token(Token {
                token_type: TokenType::Equal,
                span: template_span.clone(),
            }),
        ],
    };

    let expanded = template.expand_with_origins(&captures, expansion, generated_source).unwrap();
    let tokens = expanded.to_tokens_with_origins();

    match &tokens[0].1 {
        TokenOrigin::Captured {
            expansion: id,
            invocation_span: span,
            capture_span: captured,
        } => {
            assert_eq!(*id, expansion);
            assert_eq!((span.start, span.end), (invocation_span.start, invocation_span.end));
            assert_eq!((captured.start, captured.end), (capture_span.start, capture_span.end));
        }
        origin => panic!("expected captured origin, got {origin:?}"),
    }
    match &tokens[1].1 {
        TokenOrigin::Generated {
            expansion: id,
            generated_source: source,
            definition_span: span,
        } => {
            assert_eq!(*id, expansion);
            assert_eq!(*source, generated_source);
            assert_eq!((span.start, span.end), (template_span.start, template_span.end));
        }
        origin => panic!("expected generated origin, got {origin:?}"),
    }
}
```

Ensure the test module imports:

```rust
use crate::macro_expansion::{ExpansionId, GeneratedSourceId, TokenOrigin, TokenStream};
```

- [ ] **Step 2: Run the red origin test**

Run:

```bash
cargo test -p rock-lib declarative_template_marks_captured_and_generated_origins -- --nocapture
```

Expected: FAIL because `captured_from_tokens` and `expand_with_origins` do not exist.

- [ ] **Step 3: Add origin constructors to token streams**

In `lib/src/macro_expansion/token_tree.rs`, add:

```rust
pub fn captured_from_tokens(
    tokens: Vec<Token>,
    expansion: ExpansionId,
    invocation_span: Span,
) -> Self {
    Self {
        trees: tokens
            .into_iter()
            .map(|token| TokenTree::Leaf {
                origin: TokenOrigin::Captured {
                    expansion,
                    invocation_span: invocation_span.clone(),
                    capture_span: token.span.clone(),
                },
                token,
            })
            .collect(),
    }
}

pub fn generated_from_tokens(
    tokens: Vec<Token>,
    expansion: ExpansionId,
    generated_source: GeneratedSourceId,
) -> Self {
    Self {
        trees: tokens
            .into_iter()
            .map(|token| TokenTree::Leaf {
                origin: TokenOrigin::Generated {
                    expansion,
                    generated_source,
                    definition_span: token.span.clone(),
                },
                token,
            })
            .collect(),
    }
}
```

- [ ] **Step 4: Add origin-aware template expansion**

In `lib/src/macro_expansion/declarative.rs`, keep `expand` for existing tests and implement:

```rust
pub fn expand_with_origins(
    &self,
    captures: &CaptureSet,
    expansion: ExpansionId,
    generated_source: GeneratedSourceId,
) -> Result<TokenStream, String> {
    expand_template_fragments_with_origins(&self.fragments, captures, None, expansion, generated_source)
}
```

In `TemplateFragment::Token(token)`, append `TokenStream::generated_from_tokens(vec![token.clone()], expansion, generated_source).trees`. Captures continue to clone the captured streams already stored in `CaptureSet`.

- [ ] **Step 5: Use origin-aware captures and generated parsing**

In `lib/src/macro_expansion/context.rs`, expose:

```rust
pub fn generated_source_id(&self, id: ExpansionId) -> Option<GeneratedSourceId> {
    self.source_map.borrow().generated_source_id(id)
}
```

Change `capture_set_from_correspondance` in `lib/src/macro_expansion/mod.rs` to accept `expansion_id` and `invoc_span`, and build captures with `TokenStream::captured_from_tokens(tokens, expansion_id, invoc_span.clone())`.

In `expand_top_level`, call:

```rust
let generated_source = context.generated_source_id(expansion_id).unwrap_or(GeneratedSourceId(0));
let captures = capture_set_from_correspondance(&correspondances, expansion_id, &invoc_span);
let body = arm.template.expand_with_origins(&captures, expansion_id, generated_source)?;
top_levels.extend(parse_generated_top_levels(body, context, Some(expansion_id))?);
```

Change `parse_generated_top_levels` to take `TokenStream` instead of `Vec<Token>`. When appending generated EOL/Indent/EOF tokens, push `TokenTree::Leaf` values with `TokenOrigin::Generated` when `expansion_id` has a generated source ID, otherwise `TokenOrigin::Source(Span::default())`.

In `expand_proc_macro_top_level`, wrap decoded proc-macro output tokens before parsing them:

```rust
let generated_source = context.generated_source_id(expansion_id).unwrap_or(GeneratedSourceId(0));
let stream = TokenStream::generated_from_tokens(tokens, expansion_id, generated_source);
parse_generated_top_levels(stream, context, Some(expansion_id))
```

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib declarative_template_marks_captured_and_generated_origins -- --nocapture
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/token_tree.rs lib/src/macro_expansion/declarative.rs lib/src/macro_expansion/context.rs lib/src/macro_expansion/mod.rs lib/src/macro_expansion/generated_parser.rs
git commit -m "preserve macro token origins"
```

## Task 4: Enrich Proc-Macro Protocol And Artifact Metadata

**Files:**
- Modify: `lib/src/macro_expansion/proc_macro.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Modify: `lib/src/macro_expansion/registry.rs`
- Modify: `lib/src/products.rs`
- Modify: `rock-shared/src/sysroot.rs`
- Modify any test fixtures that construct `ProcMacroArtifact` or `ProcMacroExport`.

- [ ] **Step 1: Write failing protocol metadata roundtrip test**

Add this test to `lib/src/macro_expansion/proc_macro.rs`:

```rust
#[test]
fn proc_macro_protocol_preserves_identity_context_and_capabilities() {
    let request = ProcMacroRequest {
        protocol_version: PROC_MACRO_PROTOCOL_VERSION,
        compiler_version: env!("CARGO_PKG_VERSION").to_string(),
        macro_name: "make_main".to_string(),
        macro_identity: "macros::make_main".to_string(),
        source_module: Some("main".to_string()),
        expansion_id: Some(7),
        input: vec![1, 2, 3],
    };
    let decoded = decode_request(&encode_request(&request).unwrap()).unwrap();
    assert_eq!(decoded.macro_identity, "macros::make_main");
    assert_eq!(decoded.source_module.as_deref(), Some("main"));
    assert_eq!(decoded.expansion_id, Some(7));

    let artifact = ProcMacroArtifact {
        artifact_format_version: crate::products::PRODUCT_ARTIFACT_FORMAT_VERSION,
        crate_identity: "macros".to_string(),
        protocol_version: PROC_MACRO_PROTOCOL_VERSION,
        host_triple: "x86_64-unknown-linux-gnu".to_string(),
        executable: "macro-host".into(),
        capabilities: vec![ProcMacroCapability::Stdio],
        exports: vec![ProcMacroExport {
            name: "make_main".to_string(),
            identity: "macros::make_main".to_string(),
            kind: ProcMacroKind::FunctionLike,
            input_shape: ProcMacroInputShape::TokenStream,
        }],
    };
    assert_eq!(artifact.exports[0].input_shape, ProcMacroInputShape::TokenStream);
    assert_eq!(artifact.capabilities, vec![ProcMacroCapability::Stdio]);
}
```

- [ ] **Step 2: Run the red protocol test**

Run:

```bash
cargo test -p rock-lib proc_macro_protocol_preserves_identity_context_and_capabilities -- --nocapture
```

Expected: FAIL because the metadata fields and enums do not exist.

- [ ] **Step 3: Add protocol metadata types and fields**

In `lib/src/macro_expansion/proc_macro.rs`, add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcMacroInputShape {
    TokenStream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcMacroCapability {
    Stdio,
}
```

Extend `ProcMacroExport`, `ProcMacroArtifact`, and `ProcMacroRequest` with the fields used by the test. Update request creation in `expand_proc_macro_top_level` to fill:

```rust
compiler_version: env!("CARGO_PKG_VERSION").to_string(),
macro_identity: entry.export.identity.clone(),
source_module: None,
expansion_id: Some(expansion_id.0),
```

- [ ] **Step 4: Update fixtures and product version**

Update all `ProcMacroArtifact` and `ProcMacroExport` initializers in tests and product fixtures with the new fields. Increment `PRODUCT_ARTIFACT_FORMAT_VERSION` from `19` to `20` in:

- `lib/src/products.rs`
- `rock-shared/src/sysroot.rs`

Update the product format version assertion in `lib/src/products.rs` from `19` to `20`.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib proc_macro_protocol_preserves_identity_context_and_capabilities -- --nocapture
cargo test -p rock-lib compiler_products_preserve_proc_macro_exports -- --nocapture
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/proc_macro.rs lib/src/macro_expansion/mod.rs lib/src/macro_expansion/registry.rs lib/src/products.rs rock-shared/src/sysroot.rs
git commit -m "enrich proc macro protocol metadata"
```

## Task 5: Add Integration-Style Macro Regression Coverage

**Files:**
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing declarative macro integration test**

Add this test near the other integration tests in `lib/tests/integration.rs`:

```rust
#[test]
fn test_declarative_macro_expands_user_program() {
    let (_stdout, success) = compile_and_run_with_status(
        r#"macro make_main
    =>
        main = -> 0
%make_main"#,
    );

    assert!(success);
}
```

- [ ] **Step 2: Write proc-macro integration-style expansion test**

Add imports at the top of `lib/tests/integration.rs`:

```rust
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};
```

Add a helper and test at the bottom of the file:

```rust
fn proc_macro_host_with_response(response: rock_lib::macro_expansion::proc_macro::ProcMacroResponse) -> (rock_lib::macro_expansion::proc_macro::ProcMacroArtifact, PathBuf) {
    let response = rock_lib::macro_expansion::proc_macro::encode_response(&response).unwrap();
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("rock-integration-proc-macro-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();
    let host = temp_dir.join("proc_macro_host");
    fs::write(host.with_extension("response"), response).unwrap();
    fs::write(&host, "#!/bin/sh\ncat >/dev/null\ncat \"$0.response\"\n").unwrap();
    let mut permissions = fs::metadata(&host).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&host, permissions).unwrap();
    let artifact = rock_lib::macro_expansion::proc_macro::ProcMacroArtifact {
        artifact_format_version: rock_lib::products::PRODUCT_ARTIFACT_FORMAT_VERSION,
        crate_identity: "integration_macros".to_string(),
        protocol_version: rock_lib::macro_expansion::proc_macro::PROC_MACRO_PROTOCOL_VERSION,
        host_triple: "x86_64-unknown-linux-gnu".to_string(),
        executable: host,
        capabilities: vec![rock_lib::macro_expansion::proc_macro::ProcMacroCapability::Stdio],
        exports: vec![rock_lib::macro_expansion::proc_macro::ProcMacroExport {
            name: "make_main".to_string(),
            identity: "integration_macros::make_main".to_string(),
            kind: rock_lib::macro_expansion::proc_macro::ProcMacroKind::FunctionLike,
            input_shape: rock_lib::macro_expansion::proc_macro::ProcMacroInputShape::TokenStream,
        }],
    };
    (artifact, temp_dir)
}

#[test]
fn test_function_like_proc_macro_expands_through_public_macro_api() {
    let config = rock_lib::Config {
        entry_file: PathBuf::new(),
        output_dir: PathBuf::new(),
        debug_print: Vec::new(),
        meta_files: Vec::new(),
        extern_artifacts: Vec::new(),
        current_crate_name: None,
        opt_level: 0,
        emit_llvm: false,
        no_link: false,
        emit_object: None,
        no_prelude: false,
        no_std: false,
        sysroot: None,
    };
    let generated_tokens = vec![
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Indent(0)),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Ident("main".to_string())),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Equal),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Arrow),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Number("0".to_string())),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Eol),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Indent(0)),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Eol),
        rock_lib::lexer::Token::from(rock_lib::lexer::TokenType::Eof),
    ];
    let response = rock_lib::macro_expansion::proc_macro::ProcMacroResponse::Expand {
        output: rock_lib::macro_expansion::proc_macro::encode_tokens(&generated_tokens).unwrap(),
    };
    let (artifact, temp_dir) = proc_macro_host_with_response(response);
    let input_program = rock_lib::parser::parse_string("%make_main", &config).unwrap();
    let context = rock_lib::macro_expansion::MacroExpansionContext::new(&config)
        .with_proc_macro_artifact(artifact);

    let expanded = rock_lib::macro_expansion::expand_macros_with_context(input_program, &context).unwrap();

    assert!(expanded.module.top_level_from_ident("main").is_some());
    let _ = fs::remove_dir_all(temp_dir);
}
```

- [ ] **Step 3: Run the red integration tests**

Run:

```bash
cargo test -p rock-lib test_declarative_macro_expands_user_program --test integration -- --exact --nocapture
cargo test -p rock-lib test_function_like_proc_macro_expands_through_public_macro_api --test integration -- --exact --nocapture
```

Expected: PASS after Tasks 1-4. These tests are regression coverage for user-visible declarative expansion and public proc-macro expansion; if either fails, fix the implementation before continuing.

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib --test integration test_declarative_macro_expands_user_program -- --exact --nocapture
cargo test -p rock-lib --test integration test_function_like_proc_macro_expands_through_public_macro_api -- --exact --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/tests/integration.rs
git commit -m "cover macro expansion integration paths"
```

## Task 6: Final Cleanup, Verification, And Reviews

**Files:**
- No planned source edits. If verification fails, fix only the file named by the failing diagnostic or test output, then rerun this task from Step 1.

- [ ] **Step 1: Confirm no forbidden macro reparse paths**

Run:

```bash
rg "Config::default\(\)|module_inline\.process\(ParseCtx::from" lib/src/macro_expansion
```

Expected: no matches for `Config::default()` or direct `module_inline.process(ParseCtx::from(...))` in `lib/src/macro_expansion`. `ParseCtx::from` inside `macro_arg_matcher.rs` remains allowed only with `self.context.config`.

- [ ] **Step 2: Run final verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

Expected: PASS. If the proc-macro smoke test reports `Broken pipe`, treat it as a blocker because Task 1 should have fixed the shell fixture.

- [ ] **Step 3: Commit any final cleanup**

If Step 1 or Step 2 required fixes, commit them:

```bash
git add lib/src/macro_expansion lib/src/products.rs rock-shared/src/sysroot.rs lib/tests/integration.rs
git commit -m "complete macro architecture spec gaps"
```

If there are no changes, do not create an empty commit.

- [ ] **Step 4: Request final reviews**

Request two reviews before claiming completion:

- Spec compliance review against `docs/superpowers/specs/2026-05-24-macro-expansion-architecture-design.md` and `docs/superpowers/specs/2026-05-24-macro-architecture-spec-completion-design.md`.
- Code quality review focused on macro parser boundaries, source-map correctness, proc-macro process isolation, diagnostics, product metadata compatibility, and verification stability.

Expected: both reviews PASS or all Critical/Important findings are fixed and re-reviewed.
