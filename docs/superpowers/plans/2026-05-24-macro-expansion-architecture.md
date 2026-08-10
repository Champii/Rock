# Macro Expansion Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild Rock macro expansion around explicit declarative token-tree expansion, source maps, diagnostics, and process-isolated proc macros.

**Architecture:** Add a macro-expansion boundary in `lib/src/macro_expansion/` with context, source-map, token-tree, declarative matcher/template, registry, generated-parser, and proc-macro protocol modules. Preserve existing user-visible declarative macro behavior while replacing parser-internal `Config::default()` reparsing and making proc macros a first-class host-binary protocol.

**Tech Stack:** Rust 2021, `rock-lib`, existing parser/lexer/diagnostics, `serde`, `bincode`, `std::process`, focused unit tests, `cargo fmt --all --check`, `git diff --check`, `cargo test -p rock-lib`.

---

## Approved Spec

- `docs/superpowers/specs/2026-05-24-macro-expansion-architecture-design.md`

## Worktree Constraint

All work for this plan must happen in the isolated worktree:

- Path: `/root/new_lang2/.worktrees/macro-architecture-redesign`
- Branch: `macro-architecture-redesign`

Do not implement this plan in the primary checkout unless the user explicitly changes that constraint.

## File Structure

- Modify `lib/src/macro_expansion/mod.rs`: public expansion entrypoint, context wiring, registry use, generated parsing, and diagnostics.
- Create `lib/src/macro_expansion/context.rs`: `MacroExpansionContext`, expansion stack, generated-source ID allocation, depth limit, and helper diagnostics.
- Create `lib/src/macro_expansion/source_map.rs`: `ExpansionId`, `GeneratedSourceId`, `TokenOrigin`, `MacroSourceMap`, expansion records, and trace labels.
- Create `lib/src/macro_expansion/token_tree.rs`: `TokenTree`, `TokenStream`, token conversion helpers, and stable token-stream serialization helpers for proc macros.
- Create `lib/src/macro_expansion/declarative.rs`: `DeclarativeMacro`, `MacroMatcher`, `MacroTemplate`, `CaptureKind`, `CaptureSet`, matching, and template expansion.
- Create `lib/src/macro_expansion/registry.rs`: current-crate macro discovery and macro lookup by name/module metadata.
- Create `lib/src/macro_expansion/generated_parser.rs`: public generated-token/source parsing boundary with explicit `Config` and source-map remapping hooks.
- Create `lib/src/macro_expansion/proc_macro.rs`: proc-macro artifact metadata, protocol structs, request/response codec, and host runner.
- Modify `lib/src/lib.rs`: call macro expansion through explicit context.
- Modify `lib/src/parser/mod.rs`: expose generated token/source parsing helpers only through public parser functions.
- Modify `lib/src/diagnostic.rs`: add a small `Diagnostic::with_labels` helper so macro expansion can attach expansion traces without mutating label vectors at call sites.
- Modify `lib/src/products.rs`: add minimal proc-macro artifact metadata once protocol types exist.
- Modify `lib/tests/integration.rs`: user-visible macro/proc-macro regressions.

## Task 1: Add Macro Expansion Context And Structured Depth Diagnostics

**Files:**
- Create: `lib/src/macro_expansion/context.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Modify: `lib/src/macro_expansion/macro_arg_matcher.rs`
- Modify: `lib/src/lib.rs`
- Test: `lib/src/macro_expansion/mod.rs`

- [ ] **Step 1: Write failing context and depth tests**

Add these tests to the existing `#[cfg(test)] mod tests` in `lib/src/macro_expansion/mod.rs`:

```rust
#[test]
fn macro_expansion_uses_context_depth_limit_instead_of_panicking() {
    let input = r#"macro repeat
    =>
        %repeat
%repeat"#;
    let config = Config::default();
    let input_program = parse_string(input, &config).unwrap();
    let context = MacroExpansionContext::new(&config).with_max_depth(2);

    let result = expand_macros_with_context(input_program, &context);

    let diagnostics = result.expect_err("recursive macro should produce diagnostics");
    assert!(diagnostics
        .0
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Macro expansion depth exceeded")));
}

#[test]
fn macro_arg_matcher_uses_context_config_for_expr_parsing() {
    let input = r#"macro id
    $value:expr =>
        main = -> $value
%id 1 + 2"#;
    let config = Config {
        debug_print: vec![crate::DebugPrint::Tokens],
        ..Config::default()
    };
    let input_program = parse_string(input, &config).unwrap();
    let context = MacroExpansionContext::new(&config);

    let expanded = expand_macros_with_context(input_program, &context).unwrap();

    assert!(!expanded.module.has_macro_invoc());
}
```

- [ ] **Step 2: Run the red tests**

Run:

```bash
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: FAIL to compile because `MacroExpansionContext` and `expand_macros_with_context` do not exist.

- [ ] **Step 3: Add `MacroExpansionContext`**

Create `lib/src/macro_expansion/context.rs`:

```rust
use crate::{lexer::Span, Config};

#[derive(Debug)]
pub struct MacroExpansionContext<'a> {
    pub config: &'a Config,
    pub max_depth: usize,
}

impl<'a> MacroExpansionContext<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            max_depth: 100,
        }
    }

    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn depth_exceeded_diagnostic(&self, span: Span) -> crate::diagnostic::Diagnostic {
        crate::diagnostic::Diagnostic::new("Macro expansion depth exceeded".to_string(), span)
    }
}
```

- [ ] **Step 4: Wire context through macro expansion**

In `lib/src/macro_expansion/mod.rs`, register the module and expose the context:

```rust
mod context;
mod correspondances;
mod macro_arg_matcher;

pub use context::MacroExpansionContext;
```

Replace the public entrypoint with an explicit-context entrypoint:

```rust
pub fn expand_macros_with_context(
    mut program: Program,
    context: &MacroExpansionContext<'_>,
) -> Result<Program, Diagnostics> {
    let mut depth = 0;
    let mut module = program.module;

    while module.has_macro_invoc() {
        let mut decls = HashMap::new();

        for (i, top_level) in module.top_levels.iter().enumerate() {
            if let TopLevel::MacroInvoc(invocation) = &top_level {
                let TopLevel::MacroDecl(ref decl) =
                    module.top_level_from_ident(&invocation.name.name).unwrap()
                else {
                    continue;
                };

                decls.insert(
                    i,
                    (
                        decl.clone(),
                        invocation.args.clone(),
                        invocation.name.span.clone(),
                    ),
                );
            }
        }
        module = expand_macros_once(module, &decls, context)?;

        depth += 1;

        if depth > context.max_depth {
            let mut diagnostics = Diagnostics::default();
            diagnostics.push(context.depth_exceeded_diagnostic(Span::default()));
            return Err(diagnostics);
        }
    }

    program.module = module;

    Ok(program)
}
```

Update `expand_macros_once`, `expand_top_level`, and `MacroArgMatcher::new` calls to accept `context` and pass `context.config` into `ParseCtx::from` instead of `&Config::default()`.

Update existing macro expansion tests in `lib/src/macro_expansion/mod.rs` so each test constructs `let context = MacroExpansionContext::new(&config);` and calls `expand_macros_with_context(input_program, &context)`.

- [ ] **Step 5: Update compiler pipeline call site**

In `lib/src/lib.rs`, replace:

```rust
let ast = match macro_expansion::expand_macros(ast) {
```

with:

```rust
let macro_context = macro_expansion::MacroExpansionContext::new(config);
let ast = match macro_expansion::expand_macros_with_context(ast, &macro_context) {
```

- [ ] **Step 6: Verify focused tests**

Run:

```bash
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/context.rs lib/src/macro_expansion/mod.rs lib/src/macro_expansion/macro_arg_matcher.rs lib/src/lib.rs
git commit -m "add macro expansion context"
```

## Task 2: Add Token Origins And Expansion Source Map

**Files:**
- Create: `lib/src/macro_expansion/source_map.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/source_map.rs`

- [ ] **Step 1: Write failing source-map tests**

Create `lib/src/macro_expansion/source_map.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::lexer::Span;

    fn span(start: usize, end: usize) -> Span {
        Span {
            file_path: "macro_test.rk".into(),
            start,
            end,
        }
    }

    #[test]
    fn expansion_source_map_records_invocation_definition_and_parent() {
        let mut map = MacroSourceMap::new();
        let parent = map.record_expansion(MacroExpansionRecord {
            macro_name: "outer".to_string(),
            invocation_span: span(1, 7),
            definition_span: span(10, 20),
            parent: None,
        });
        let child = map.record_expansion(MacroExpansionRecord {
            macro_name: "inner".to_string(),
            invocation_span: span(30, 36),
            definition_span: span(40, 50),
            parent: Some(parent),
        });

        let trace = map.trace(child);

        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].macro_name, "inner");
        assert_eq!(trace[1].macro_name, "outer");
    }

    #[test]
    fn token_origin_distinguishes_capture_and_generated_tokens() {
        let expansion = ExpansionId(3);
        let captured = TokenOrigin::Captured {
            expansion,
            invocation_span: span(1, 5),
            capture_span: span(6, 9),
        };
        let generated = TokenOrigin::Generated {
            expansion,
            definition_span: span(10, 12),
        };

        assert_eq!(captured.expansion_id(), expansion);
        assert_eq!(generated.expansion_id(), expansion);
    }
}
```

- [ ] **Step 2: Run the red tests**

Run:

```bash
cargo test -p rock-lib macro_expansion::source_map -- --nocapture
```

Expected: FAIL to compile because the source-map types do not exist and the module is not registered.

- [ ] **Step 3: Implement source-map types**

Add this implementation above the tests in `lib/src/macro_expansion/source_map.rs`:

```rust
use crate::lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpansionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeneratedSourceId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenOrigin {
    Source(Span),
    Captured {
        expansion: ExpansionId,
        invocation_span: Span,
        capture_span: Span,
    },
    Generated {
        expansion: ExpansionId,
        definition_span: Span,
    },
}

impl TokenOrigin {
    pub fn expansion_id(&self) -> ExpansionId {
        match self {
            TokenOrigin::Source(_) => ExpansionId(0),
            TokenOrigin::Captured { expansion, .. }
            | TokenOrigin::Generated { expansion, .. } => *expansion,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroExpansionRecord {
    pub macro_name: String,
    pub invocation_span: Span,
    pub definition_span: Span,
    pub parent: Option<ExpansionId>,
}

#[derive(Debug, Clone, Default)]
pub struct MacroSourceMap {
    records: Vec<MacroExpansionRecord>,
}

impl MacroSourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_expansion(&mut self, record: MacroExpansionRecord) -> ExpansionId {
        let id = ExpansionId(self.records.len() as u32 + 1);
        self.records.push(record);
        id
    }

    pub fn record(&self, id: ExpansionId) -> Option<&MacroExpansionRecord> {
        if id.0 == 0 {
            return None;
        }
        self.records.get(id.0 as usize - 1)
    }

    pub fn trace(&self, id: ExpansionId) -> Vec<MacroExpansionRecord> {
        let mut trace = Vec::new();
        let mut current = Some(id);
        while let Some(id) = current {
            let Some(record) = self.record(id) else {
                break;
            };
            trace.push(record.clone());
            current = record.parent;
        }
        trace
    }
}
```

- [ ] **Step 4: Register and re-export source-map types**

In `lib/src/macro_expansion/mod.rs`, add:

```rust
pub mod source_map;

pub use source_map::{ExpansionId, GeneratedSourceId, MacroSourceMap, TokenOrigin};
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion::source_map -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/mod.rs lib/src/macro_expansion/source_map.rs
git commit -m "track macro expansion source origins"
```

## Task 3: Add Compiler-Owned Token Trees

**Files:**
- Create: `lib/src/macro_expansion/token_tree.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/token_tree.rs`

- [ ] **Step 1: Write failing token-tree tests**

Create `lib/src/macro_expansion/token_tree.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::lexer::{Span, Token, TokenType};
    use crate::macro_expansion::TokenOrigin;

    #[test]
    fn token_stream_preserves_leaf_tokens_and_origins() {
        let span = Span {
            file_path: "tokens.rk".into(),
            start: 0,
            end: 1,
        };
        let token = Token {
            token_type: TokenType::Ident("x".to_string()),
            span: span.clone(),
        };

        let stream = TokenStream::from_tokens(vec![token.clone()]);

        assert_eq!(stream.to_tokens(), vec![token]);
        assert_eq!(stream.trees[0].origin(), Some(&TokenOrigin::Source(span)));
    }

    #[test]
    fn delimited_token_tree_flattens_with_delimiters() {
        let stream = TokenStream {
            trees: vec![TokenTree::Delimited {
                delimiter: Delimiter::Paren,
                open_origin: TokenOrigin::Source(Span::default()),
                inner: TokenStream::from_tokens(vec![Token::from(TokenType::Ident("x".to_string()))]),
                close_origin: TokenOrigin::Source(Span::default()),
            }],
        };

        let tokens = stream.to_tokens();

        assert!(matches!(tokens[0].token_type, TokenType::OpenParen));
        assert!(matches!(tokens[1].token_type, TokenType::Ident(_)));
        assert!(matches!(tokens[2].token_type, TokenType::CloseParen));
    }
}
```

- [ ] **Step 2: Run the red tests**

Run:

```bash
cargo test -p rock-lib macro_expansion::token_tree -- --nocapture
```

Expected: FAIL to compile because token-tree types do not exist and the module is not registered.

- [ ] **Step 3: Implement token-tree types**

Add this implementation above the tests:

```rust
use crate::lexer::{Token, TokenType};
use crate::macro_expansion::TokenOrigin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delimiter {
    Paren,
    Bracket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenTree {
    Leaf {
        token: Token,
        origin: TokenOrigin,
    },
    Delimited {
        delimiter: Delimiter,
        open_origin: TokenOrigin,
        inner: TokenStream,
        close_origin: TokenOrigin,
    },
}

impl TokenTree {
    pub fn origin(&self) -> Option<&TokenOrigin> {
        match self {
            TokenTree::Leaf { origin, .. } => Some(origin),
            TokenTree::Delimited { open_origin, .. } => Some(open_origin),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenStream {
    pub trees: Vec<TokenTree>,
}

impl TokenStream {
    pub fn from_tokens(tokens: Vec<Token>) -> Self {
        Self {
            trees: tokens
                .into_iter()
                .map(|token| TokenTree::Leaf {
                    origin: TokenOrigin::Source(token.span.clone()),
                    token,
                })
                .collect(),
        }
    }

    pub fn to_tokens(&self) -> Vec<Token> {
        let mut tokens = Vec::new();
        for tree in &self.trees {
            match tree {
                TokenTree::Leaf { token, .. } => tokens.push(token.clone()),
                TokenTree::Delimited {
                    delimiter, inner, ..
                } => {
                    match delimiter {
                        Delimiter::Paren => tokens.push(Token::from(TokenType::OpenParen)),
                        Delimiter::Bracket => tokens.push(Token::from(TokenType::OpenBracket)),
                    }
                    tokens.extend(inner.to_tokens());
                    match delimiter {
                        Delimiter::Paren => tokens.push(Token::from(TokenType::CloseParen)),
                        Delimiter::Bracket => tokens.push(Token::from(TokenType::CloseBracket)),
                    }
                }
            }
        }
        tokens
    }
}
```

- [ ] **Step 4: Register and re-export token-tree types**

In `lib/src/macro_expansion/mod.rs`, add:

```rust
pub mod token_tree;

pub use token_tree::{Delimiter, TokenStream, TokenTree};
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion::token_tree -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/mod.rs lib/src/macro_expansion/token_tree.rs
git commit -m "add macro token tree model"
```

## Task 4: Compile Declarative Macros Into Matcher And Template Forms

**Files:**
- Create: `lib/src/macro_expansion/declarative.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/declarative.rs`

- [ ] **Step 1: Write failing declarative model tests**

Create `lib/src/macro_expansion/declarative.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Ident, MacroDecl, MacroEntry, MacroFragment};
    use crate::lexer::{Span, Token, TokenType};

    fn ident(name: &str) -> Ident {
        Ident {
            name: name.to_string(),
            span: Span::default(),
        }
    }

    #[test]
    fn declarative_macro_compiles_capture_kinds_and_template_tokens() {
        let decl = MacroDecl {
            name: ident("make"),
            entries: vec![MacroEntry {
                defs: vec![MacroFragment::Ident(ident("name"))],
                body: vec![
                    MacroFragment::Ident(ident("name")),
                    MacroFragment::Token(Token::from(TokenType::Equal)),
                ],
            }],
        };

        let compiled = DeclarativeMacro::from_ast(&decl);

        assert_eq!(compiled.name, "make");
        assert_eq!(compiled.arms.len(), 1);
        assert!(matches!(
            compiled.arms[0].matcher.fragments[0],
            MatcherFragment::Capture {
                kind: CaptureKind::Ident,
                ..
            }
        ));
        assert!(matches!(
            compiled.arms[0].template.fragments[0],
            TemplateFragment::Capture { .. }
        ));
    }
}
```

- [ ] **Step 2: Run the red tests**

Run:

```bash
cargo test -p rock-lib macro_expansion::declarative -- --nocapture
```

Expected: FAIL to compile because `DeclarativeMacro` and related types do not exist.

- [ ] **Step 3: Implement declarative model types**

Add this implementation above the tests:

```rust
use crate::ast::{MacroDecl, MacroFragment};
use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMacro {
    pub name: String,
    pub arms: Vec<DeclarativeMacroArm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMacroArm {
    pub matcher: MacroMatcher,
    pub template: MacroTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroMatcher {
    pub fragments: Vec<MatcherFragment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroTemplate {
    pub fragments: Vec<TemplateFragment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Ident,
    Expr,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatcherFragment {
    Capture { name: String, kind: CaptureKind },
    Token(Token),
    Repetition(Vec<MatcherFragment>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateFragment {
    Capture { name: String },
    Token(Token),
    Repetition(Vec<TemplateFragment>),
}

impl DeclarativeMacro {
    pub fn from_ast(decl: &MacroDecl) -> Self {
        Self {
            name: decl.name.name.clone(),
            arms: decl
                .entries
                .iter()
                .map(|entry| DeclarativeMacroArm {
                    matcher: MacroMatcher {
                        fragments: compile_matcher_fragments(&entry.defs),
                    },
                    template: MacroTemplate {
                        fragments: compile_template_fragments(&entry.body),
                    },
                })
                .collect(),
        }
    }
}

fn compile_matcher_fragments(fragments: &[MacroFragment]) -> Vec<MatcherFragment> {
    fragments
        .iter()
        .map(|fragment| match fragment {
            MacroFragment::Ident(name) => MatcherFragment::Capture {
                name: name.name.clone(),
                kind: CaptureKind::Ident,
            },
            MacroFragment::Expr(name) => MatcherFragment::Capture {
                name: name.name.clone(),
                kind: CaptureKind::Expr,
            },
            MacroFragment::Type(name) => MatcherFragment::Capture {
                name: name.name.clone(),
                kind: CaptureKind::Type,
            },
            MacroFragment::Token(token) => MatcherFragment::Token(token.clone()),
            MacroFragment::Repetition(inner) => {
                MatcherFragment::Repetition(compile_matcher_fragments(inner))
            }
        })
        .collect()
}

fn compile_template_fragments(fragments: &[MacroFragment]) -> Vec<TemplateFragment> {
    fragments
        .iter()
        .map(|fragment| match fragment {
            MacroFragment::Ident(name)
            | MacroFragment::Expr(name)
            | MacroFragment::Type(name) => TemplateFragment::Capture {
                name: name.name.clone(),
            },
            MacroFragment::Token(token) => TemplateFragment::Token(token.clone()),
            MacroFragment::Repetition(inner) => {
                TemplateFragment::Repetition(compile_template_fragments(inner))
            }
        })
        .collect()
}
```

- [ ] **Step 4: Register declarative module**

In `lib/src/macro_expansion/mod.rs`, add:

```rust
pub mod declarative;
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion::declarative -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/mod.rs lib/src/macro_expansion/declarative.rs
git commit -m "compile declarative macros into matchers"
```

## Task 5: Expand Declarative Templates Through The New Model

**Files:**
- Modify: `lib/src/macro_expansion/declarative.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/declarative.rs`
- Test: `lib/src/macro_expansion/mod.rs`

- [ ] **Step 1: Write failing template expansion tests**

Add this test to `lib/src/macro_expansion/declarative.rs`:

```rust
#[test]
fn declarative_template_expands_direct_capture_tokens() {
    let mut captures = CaptureSet::new();
    captures.insert_direct(
        "name".to_string(),
        TokenStream::from_tokens(vec![Token::from(TokenType::Ident("main".to_string()))]),
    );
    let template = MacroTemplate {
        fragments: vec![
            TemplateFragment::Capture {
                name: "name".to_string(),
            },
            TemplateFragment::Token(Token::from(TokenType::Equal)),
        ],
    };

    let expanded = template.expand(&captures).unwrap();

    assert!(matches!(expanded.to_tokens()[0].token_type, TokenType::Ident(_)));
    assert!(matches!(expanded.to_tokens()[1].token_type, TokenType::Equal));
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib declarative_template_expands_direct_capture_tokens -- --nocapture
```

Expected: FAIL because `CaptureSet` and `MacroTemplate::expand` do not exist.

- [ ] **Step 3: Add `CaptureSet` and template expansion**

In `lib/src/macro_expansion/declarative.rs`, add:

```rust
use std::collections::HashMap;

use crate::macro_expansion::TokenStream;

#[derive(Debug, Clone, Default)]
pub struct CaptureSet {
    direct: HashMap<String, TokenStream>,
}

impl CaptureSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_direct(&mut self, name: String, stream: TokenStream) {
        self.direct.insert(name, stream);
    }

    pub fn get_direct(&self, name: &str) -> Option<&TokenStream> {
        self.direct.get(name)
    }
}

impl MacroTemplate {
    pub fn expand(&self, captures: &CaptureSet) -> Result<TokenStream, String> {
        let mut trees = Vec::new();
        for fragment in &self.fragments {
            match fragment {
                TemplateFragment::Capture { name } => {
                    let Some(stream) = captures.get_direct(name) else {
                        return Err(format!("missing macro capture '{}'", name));
                    };
                    trees.extend(stream.trees.clone());
                }
                TemplateFragment::Token(token) => {
                    trees.extend(TokenStream::from_tokens(vec![token.clone()]).trees);
                }
                TemplateFragment::Repetition(inner) => {
                    let nested = MacroTemplate {
                        fragments: inner.clone(),
                    };
                    trees.extend(nested.expand(captures)?.trees);
                }
            }
        }
        Ok(TokenStream { trees })
    }
}
```

- [ ] **Step 4: Migrate `expand_top_level` to compiled templates**

In `lib/src/macro_expansion/mod.rs`, keep the existing matcher while replacing only template replacement with `MacroTemplate::expand` for direct captures. Convert the existing `Correspondance` result into `CaptureSet` with a helper:

```rust
fn capture_set_from_correspondance(correspondances: &Correspondance) -> declarative::CaptureSet {
    let mut captures = declarative::CaptureSet::new();
    for (name, streams) in &correspondances.entries {
        if let Some(tokens) = streams.first() {
            captures.insert_direct(name.clone(), TokenStream::from_tokens(tokens.clone()));
        }
    }
    captures
}
```

Use `DeclarativeMacro::from_ast(macro_decl)` to select the matching arm and expand its template. Leave nested repetition on the existing `replace_body_variables` path until Task 6 migrates repetitions.

- [ ] **Step 5: Verify existing macro behavior**

Run:

```bash
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
cargo fmt --all --check
git diff --check
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/declarative.rs lib/src/macro_expansion/mod.rs
git commit -m "expand declarative macro templates"
```

## Task 6: Replace Repetition Correspondence With Structured Capture Sets

**Files:**
- Modify: `lib/src/macro_expansion/declarative.rs`
- Modify: `lib/src/macro_expansion/macro_arg_matcher.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/declarative.rs`
- Test: `lib/src/macro_expansion/mod.rs`

- [ ] **Step 1: Write failing nested repetition tests**

Add this test to `lib/src/macro_expansion/mod.rs`:

```rust
#[test]
fn declarative_repetition_expands_each_capture_group() {
    let input = r#"macro defs
    $( $name:ident ) =>
        $( $name = -> 1 )
%defs one two three"#;
    let expected = r#"macro defs
    $( $name:ident ) =>
        $( $name = -> 1 )
one = -> 1
two = -> 1
three = -> 1"#;
    let config = Config::default();
    let context = MacroExpansionContext::new(&config);

    let expanded = expand_macros_with_context(parse_string(input, &config).unwrap(), &context).unwrap();
    let expected = parse_string(expected, &config).unwrap();

    assert_eq!(expanded, expected);
}
```

- [ ] **Step 2: Run the red or regression test**

Run:

```bash
cargo test -p rock-lib declarative_repetition_expands_each_capture_group -- --nocapture
```

Expected: FAIL if Task 5 only expanded direct captures, or PASS if the old compatibility path still handles the case. If it passes through compatibility, continue and use this test as the guard while replacing the old path.

- [ ] **Step 3: Extend `CaptureSet` for repetitions**

In `lib/src/macro_expansion/declarative.rs`, replace `CaptureSet` with:

```rust
#[derive(Debug, Clone, Default)]
pub struct CaptureSet {
    direct: HashMap<String, TokenStream>,
    repeated: HashMap<String, Vec<TokenStream>>,
}

impl CaptureSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_direct(&mut self, name: String, stream: TokenStream) {
        self.direct.insert(name, stream);
    }

    pub fn insert_repeated(&mut self, name: String, streams: Vec<TokenStream>) {
        self.repeated.insert(name, streams);
    }

    pub fn get_direct(&self, name: &str) -> Option<&TokenStream> {
        self.direct.get(name)
    }

    pub fn get_repeated(&self, name: &str) -> Option<&[TokenStream]> {
        self.repeated.get(name).map(Vec::as_slice)
    }
}
```

Update repetition expansion to compute the repetition length from captures used inside the repetition and expand each group.

- [ ] **Step 4: Convert old nested correspondence data into `CaptureSet`**

In `lib/src/macro_expansion/mod.rs`, update `capture_set_from_correspondance` to include nested entries:

```rust
for nested in &correspondances.nested_corresp {
    for (name, streams) in &nested.entries {
        captures.insert_repeated(
            name.clone(),
            streams
                .iter()
                .map(|tokens| TokenStream::from_tokens(tokens.clone()))
                .collect(),
        );
    }
}
```

Remove the direct call to `replace_body_variables` after all existing macro expansion tests pass with `MacroTemplate::expand`.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/declarative.rs lib/src/macro_expansion/macro_arg_matcher.rs lib/src/macro_expansion/mod.rs
git commit -m "structure declarative macro repetitions"
```

## Task 7: Add Macro Registry And Current-Crate Discovery

**Files:**
- Create: `lib/src/macro_expansion/registry.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/registry.rs`

- [ ] **Step 1: Write failing registry tests**

Create `lib/src/macro_expansion/registry.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Ident, MacroDecl, Module, TopLevel};
    use crate::lexer::Span;

    fn macro_decl(name: &str) -> MacroDecl {
        MacroDecl {
            name: Ident {
                name: name.to_string(),
                span: Span::default(),
            },
            entries: Vec::new(),
        }
    }

    #[test]
    fn registry_discovers_current_module_declarative_macros() {
        let module = Module {
            name: None,
            top_levels: vec![TopLevel::MacroDecl(macro_decl("make"))],
            is_inline: true,
            filepath: None,
        };

        let registry = MacroRegistry::from_module(&module);

        assert!(registry.declarative("make").is_some());
    }
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib macro_expansion::registry -- --nocapture
```

Expected: FAIL because `MacroRegistry` does not exist.

- [ ] **Step 3: Implement registry**

Add this implementation above the tests:

```rust
use std::collections::HashMap;

use crate::ast::{Module, TopLevel};
use crate::macro_expansion::declarative::DeclarativeMacro;

#[derive(Debug, Clone, Default)]
pub struct MacroRegistry {
    declarative: HashMap<String, DeclarativeMacro>,
}

impl MacroRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_module(module: &Module) -> Self {
        let mut registry = Self::new();
        for top_level in &module.top_levels {
            if let TopLevel::MacroDecl(decl) = top_level {
                registry
                    .declarative
                    .insert(decl.name.name.clone(), DeclarativeMacro::from_ast(decl));
            }
        }
        registry
    }

    pub fn declarative(&self, name: &str) -> Option<&DeclarativeMacro> {
        self.declarative.get(name)
    }
}
```

- [ ] **Step 4: Use registry during expansion**

In `expand_macros_with_context`, build the registry once per expansion iteration:

```rust
let registry = registry::MacroRegistry::from_module(&module);
```

Use `registry.declarative(&invocation.name.name)` instead of `module.top_level_from_ident(...).unwrap()` when selecting macro definitions. If a macro invocation has no declaration in the registry, leave it unchanged as current behavior does for later definitions.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/mod.rs lib/src/macro_expansion/registry.rs
git commit -m "discover macros through registry"
```

## Task 8: Add Generated Parser Boundary

**Files:**
- Create: `lib/src/macro_expansion/generated_parser.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Modify: `lib/src/parser/mod.rs`
- Test: `lib/src/macro_expansion/generated_parser.rs`

- [ ] **Step 1: Write failing generated parser tests**

Create `lib/src/macro_expansion/generated_parser.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    use crate::lexer::{Token, TokenType};
    use crate::macro_expansion::TokenStream;
    use crate::Config;

    #[test]
    fn generated_parser_parses_top_level_tokens_with_explicit_config() {
        let tokens = vec![
            Token::from(TokenType::Ident("main".to_string())),
            Token::from(TokenType::Equal),
            Token::from(TokenType::Arrow),
            Token::from(TokenType::Number("0".to_string())),
            Token::from(TokenType::Eof),
        ];
        let stream = TokenStream::from_tokens(tokens);
        let config = Config::default();

        let module = parse_generated_module(&stream, &config).unwrap();

        assert_eq!(module.top_levels.len(), 1);
    }
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib macro_expansion::generated_parser -- --nocapture
```

Expected: FAIL because `generated_parser` and `parse_generated_module` do not exist.

- [ ] **Step 3: Add public parser helper**

In `lib/src/parser/mod.rs`, add:

```rust
pub fn parse_module_tokens(tokens: &[crate::lexer::Token], config: &Config) -> Result<Module, ParseError> {
    engine::reset_best_error();
    let result = module_inline.process(ParseCtx::from(tokens, config));
    match result {
        Ok((_, module)) => Ok(module),
        Err(e) => Err(engine::get_best_error(e)),
    }
}
```

- [ ] **Step 4: Add generated parser boundary**

Implement `lib/src/macro_expansion/generated_parser.rs`:

```rust
use crate::ast::Module;
use crate::diagnostic::Diagnostics;
use crate::macro_expansion::TokenStream;
use crate::{parser, Config};

pub fn parse_generated_module(
    stream: &TokenStream,
    config: &Config,
) -> Result<Module, Diagnostics> {
    parser::parse_module_tokens(&stream.to_tokens(), config).map_err(Diagnostics::from)
}
```

In `lib/src/macro_expansion/mod.rs`, add:

```rust
pub mod generated_parser;
```

- [ ] **Step 5: Use generated parser in macro expansion**

Replace direct `module_inline.process(ParseCtx::from(...))` in `expand_top_level` with:

```rust
let stream = TokenStream::from_tokens(body);
let module = generated_parser::parse_generated_module(&stream, context.config)?;
```

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/parser/mod.rs lib/src/macro_expansion/generated_parser.rs lib/src/macro_expansion/mod.rs
git commit -m "parse generated macro tokens explicitly"
```

## Task 9: Add Proc-Macro Protocol Types

**Files:**
- Create: `lib/src/macro_expansion/proc_macro.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Test: `lib/src/macro_expansion/proc_macro.rs`

- [ ] **Step 1: Write failing protocol tests**

Create `lib/src/macro_expansion/proc_macro.rs` with this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_macro_protocol_roundtrips_request_and_response() {
        let request = ProcMacroRequest {
            protocol_version: PROC_MACRO_PROTOCOL_VERSION,
            macro_name: "make_main".to_string(),
            input: Vec::new(),
        };
        let encoded = encode_request(&request).unwrap();
        let decoded = decode_request(&encoded).unwrap();

        assert_eq!(decoded.macro_name, "make_main");

        let response = ProcMacroResponse::Expand { output: Vec::new() };
        let encoded = encode_response(&response).unwrap();
        let decoded = decode_response(&encoded).unwrap();

        assert!(matches!(decoded, ProcMacroResponse::Expand { .. }));
    }

    #[test]
    fn proc_macro_artifact_records_host_executable_and_exports() {
        let artifact = ProcMacroArtifact {
            protocol_version: PROC_MACRO_PROTOCOL_VERSION,
            host_triple: "x86_64-unknown-linux-gnu".to_string(),
            executable: "target/proc-macros/make_main".into(),
            exports: vec![ProcMacroExport {
                name: "make_main".to_string(),
                kind: ProcMacroKind::FunctionLike,
            }],
        };

        assert_eq!(artifact.exports[0].kind, ProcMacroKind::FunctionLike);
    }
}
```

- [ ] **Step 2: Run the red tests**

Run:

```bash
cargo test -p rock-lib macro_expansion::proc_macro -- --nocapture
```

Expected: FAIL because proc-macro protocol types do not exist.

- [ ] **Step 3: Implement protocol data and codecs**

Add this implementation above the tests:

```rust
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const PROC_MACRO_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcMacroKind {
    FunctionLike,
    AttributeLike,
    DeriveLike,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcMacroExport {
    pub name: String,
    pub kind: ProcMacroKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcMacroArtifact {
    pub protocol_version: u32,
    pub host_triple: String,
    pub executable: PathBuf,
    pub exports: Vec<ProcMacroExport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcMacroRequest {
    pub protocol_version: u32,
    pub macro_name: String,
    pub input: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcMacroResponse {
    Expand { output: Vec<u8> },
    Diagnostics { messages: Vec<String> },
}

pub fn encode_request(request: &ProcMacroRequest) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(request)
}

pub fn decode_request(bytes: &[u8]) -> Result<ProcMacroRequest, bincode::Error> {
    bincode::deserialize(bytes)
}

pub fn encode_response(response: &ProcMacroResponse) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(response)
}

pub fn decode_response(bytes: &[u8]) -> Result<ProcMacroResponse, bincode::Error> {
    bincode::deserialize(bytes)
}
```

In `lib/src/macro_expansion/mod.rs`, add:

```rust
pub mod proc_macro;
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion::proc_macro -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/mod.rs lib/src/macro_expansion/proc_macro.rs
git commit -m "define proc macro protocol"
```

## Task 10: Add Process-Isolated Proc-Macro Runner

**Files:**
- Modify: `lib/src/macro_expansion/proc_macro.rs`
- Test: `lib/src/macro_expansion/proc_macro.rs`
- Test fixture: `lib/tests/fixtures/proc_macro_echo.rs` if a compiled fixture is needed by the test harness

- [ ] **Step 1: Write failing runner validation tests**

Add tests to `lib/src/macro_expansion/proc_macro.rs`:

```rust
#[test]
fn proc_macro_runner_rejects_protocol_version_mismatch() {
    let response = ProcMacroResponse::Diagnostics {
        messages: vec!["version mismatch".to_string()],
    };
    let diagnostics = response.into_diagnostics("make_main");

    assert!(diagnostics
        .0
        .iter()
        .any(|diagnostic| diagnostic.message.contains("make_main")));
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib proc_macro_runner_rejects_protocol_version_mismatch -- --nocapture
```

Expected: FAIL because `ProcMacroResponse::into_diagnostics` does not exist.

- [ ] **Step 3: Add runner error and diagnostic conversion**

In `lib/src/macro_expansion/proc_macro.rs`, add:

```rust
use crate::diagnostic::{Diagnostic, Diagnostics};
use crate::lexer::Span;

impl ProcMacroResponse {
    pub fn into_diagnostics(self, macro_name: &str) -> Diagnostics {
        let mut diagnostics = Diagnostics::default();
        match self {
            ProcMacroResponse::Expand { .. } => {}
            ProcMacroResponse::Diagnostics { messages } => {
                for message in messages {
                    diagnostics.push(Diagnostic::new(
                        format!("Proc macro '{}' failed: {}", macro_name, message),
                        Span::default(),
                    ));
                }
            }
        }
        diagnostics
    }
}
```

- [ ] **Step 4: Add host process runner API**

Add this runner skeleton:

```rust
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn run_proc_macro_process(
    artifact: &ProcMacroArtifact,
    request: &ProcMacroRequest,
    timeout: Duration,
) -> Result<ProcMacroResponse, String> {
    if artifact.protocol_version != request.protocol_version {
        return Ok(ProcMacroResponse::Diagnostics {
            messages: vec![format!(
                "protocol version mismatch: artifact={}, request={}",
                artifact.protocol_version, request.protocol_version
            )],
        });
    }

    let mut child = Command::new(&artifact.executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to start proc macro: {}", err))?;

    let request_bytes = encode_request(request).map_err(|err| err.to_string())?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "proc macro stdin unavailable".to_string())?
        .write_all(&request_bytes)
        .map_err(|err| err.to_string())?;
    drop(child.stdin.take());

    let started = Instant::now();
    loop {
        if started.elapsed() > timeout {
            let _ = child.kill();
            return Ok(ProcMacroResponse::Diagnostics {
                messages: vec!["proc macro timed out".to_string()],
            });
        }

        if let Some(status) = child.try_wait().map_err(|err| err.to_string())? {
            let mut stdout = Vec::new();
            if let Some(mut child_stdout) = child.stdout.take() {
                child_stdout
                    .read_to_end(&mut stdout)
                    .map_err(|err| err.to_string())?;
            }

            if !status.success() {
                return Ok(ProcMacroResponse::Diagnostics {
                    messages: vec![format!("proc macro exited with {}", status)],
                });
            }

            return decode_response(&stdout).map_err(|err| err.to_string());
        }

        std::thread::sleep(Duration::from_millis(5));
    }
}
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion::proc_macro -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/proc_macro.rs
git commit -m "run proc macros out of process"
```

## Task 11: Integrate Function-Like Proc Macros With Expansion

**Files:**
- Modify: `lib/src/macro_expansion/registry.rs`
- Modify: `lib/src/macro_expansion/mod.rs`
- Modify: `lib/src/macro_expansion/proc_macro.rs`
- Modify: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing registry test for proc macro exports**

Add this test to `lib/src/macro_expansion/registry.rs`:

```rust
#[test]
fn registry_records_function_like_proc_macro_exports() {
    let artifact = crate::macro_expansion::proc_macro::ProcMacroArtifact {
        protocol_version: crate::macro_expansion::proc_macro::PROC_MACRO_PROTOCOL_VERSION,
        host_triple: "x86_64-unknown-linux-gnu".to_string(),
        executable: "macro-host".into(),
        exports: vec![crate::macro_expansion::proc_macro::ProcMacroExport {
            name: "make_main".to_string(),
            kind: crate::macro_expansion::proc_macro::ProcMacroKind::FunctionLike,
        }],
    };
    let mut registry = MacroRegistry::new();

    registry.add_proc_macro_artifact(artifact);

    assert!(registry.proc_macro("make_main").is_some());
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib registry_records_function_like_proc_macro_exports -- --nocapture
```

Expected: FAIL because proc-macro registry methods do not exist.

- [ ] **Step 3: Add proc-macro registry entries**

In `lib/src/macro_expansion/registry.rs`, add a `proc_macros` map and methods:

```rust
use crate::macro_expansion::proc_macro::{ProcMacroArtifact, ProcMacroExport};

#[derive(Debug, Clone)]
pub struct ProcMacroRegistryEntry {
    pub artifact: ProcMacroArtifact,
    pub export: ProcMacroExport,
}
```

Extend `MacroRegistry`:

```rust
proc_macros: HashMap<String, ProcMacroRegistryEntry>,
```

Add methods:

```rust
pub fn add_proc_macro_artifact(&mut self, artifact: ProcMacroArtifact) {
    for export in &artifact.exports {
        self.proc_macros.insert(
            export.name.clone(),
            ProcMacroRegistryEntry {
                artifact: artifact.clone(),
                export: export.clone(),
            },
        );
    }
}

pub fn proc_macro(&self, name: &str) -> Option<&ProcMacroRegistryEntry> {
    self.proc_macros.get(name)
}
```

- [ ] **Step 4: Add expansion hook for function-like proc macros**

In `lib/src/macro_expansion/mod.rs`, when a macro invocation does not match a declarative macro, check `registry.proc_macro(&invocation.name.name)`. For function-like proc macros, encode invocation tokens with the proc-macro protocol, run the host process, decode output tokens, parse them through `generated_parser::parse_generated_module`, and replace the invocation top level with the parsed top levels.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/registry.rs lib/src/macro_expansion/mod.rs lib/src/macro_expansion/proc_macro.rs lib/tests/integration.rs
git commit -m "integrate function-like proc macros"
```

## Task 12: Add Proc-Macro Artifact Metadata To Products

**Files:**
- Modify: `lib/src/products.rs`
- Modify: `lib/src/crate_system/context.rs`
- Modify: `lib/src/macro_expansion/registry.rs`
- Test: `lib/src/products.rs`

- [ ] **Step 1: Write failing product metadata test**

Add a product test in `lib/src/products.rs` near existing product serialization tests:

```rust
#[test]
fn compiler_products_preserve_proc_macro_exports() {
    use std::collections::BTreeMap;

    let mut products = CompilerProducts {
        crate_identity: ProductCrateIdentity::local("macros".to_string()),
        identity_table: ProductIdentityTable::default(),
        metadata: ProductMetadata::default(),
        bodies: ProductBodies::default(),
        link: ProductLinkData::default(),
        dependencies: Vec::new(),
        source_fingerprint: ProductSourceFingerprint::default(),
        prelude_exports: BTreeMap::new(),
        infix_precedence: BTreeMap::new(),
        proc_macros: Vec::new(),
    };
    products.proc_macros.push(crate::macro_expansion::proc_macro::ProcMacroArtifact {
        protocol_version: crate::macro_expansion::proc_macro::PROC_MACRO_PROTOCOL_VERSION,
        host_triple: "x86_64-unknown-linux-gnu".to_string(),
        executable: "macro-host".into(),
        exports: vec![crate::macro_expansion::proc_macro::ProcMacroExport {
            name: "make_main".to_string(),
            kind: crate::macro_expansion::proc_macro::ProcMacroKind::FunctionLike,
        }],
    });

    let bytes = products.to_artifact_bytes().unwrap();
    let roundtrip = CompilerProducts::from_artifact_bytes(&bytes).unwrap();

    assert_eq!(roundtrip.proc_macros[0].exports[0].name, "make_main");
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib compiler_products_preserve_proc_macro_exports -- --nocapture
```

Expected: FAIL because `CompilerProducts::proc_macros` does not exist.

- [ ] **Step 3: Add product metadata field**

In `lib/src/products.rs`, add to `CompilerProducts`:

```rust
pub proc_macros: Vec<crate::macro_expansion::proc_macro::ProcMacroArtifact>,
```

Initialize it to `Vec::new()` in constructors and product builders. Because this changes the binary product artifact schema, increment `PRODUCT_ARTIFACT_FORMAT_VERSION` and update the existing artifact format version test expectation in `lib/src/products.rs`.

- [ ] **Step 4: Load proc-macro artifacts into registry**

When dependency products are loaded in crate-system context, expose their `proc_macros` through a provider method or direct accessor matching existing product metadata style. During macro registry construction, add those proc-macro artifacts with `registry.add_proc_macro_artifact`.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib compiler_products_preserve_proc_macro_exports -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/products.rs lib/src/crate_system/context.rs lib/src/macro_expansion/registry.rs
git commit -m "preserve proc macro artifact metadata"
```

## Task 13: Add Expansion Trace Diagnostics

**Files:**
- Modify: `lib/src/macro_expansion/source_map.rs`
- Modify: `lib/src/macro_expansion/context.rs`
- Modify: `lib/src/diagnostic.rs`
- Test: `lib/src/macro_expansion/source_map.rs`
- Test: `lib/tests/integration.rs`

- [ ] **Step 1: Write failing diagnostic trace test**

Add this test to `lib/src/macro_expansion/source_map.rs`:

```rust
#[test]
fn expansion_trace_converts_to_diagnostic_labels() {
    let mut map = MacroSourceMap::new();
    let id = map.record_expansion(MacroExpansionRecord {
        macro_name: "make".to_string(),
        invocation_span: Span::default(),
        definition_span: Span::default(),
        parent: None,
    });

    let labels = map.trace_labels(id);

    assert!(labels.iter().any(|(label, _)| label.contains("expanded from macro 'make'")));
}
```

- [ ] **Step 2: Run the red test**

Run:

```bash
cargo test -p rock-lib expansion_trace_converts_to_diagnostic_labels -- --nocapture
```

Expected: FAIL because `trace_labels` does not exist.

- [ ] **Step 3: Add trace label helper**

In `lib/src/macro_expansion/source_map.rs`, add:

```rust
pub fn trace_labels(&self, id: ExpansionId) -> Vec<(String, Span)> {
    self.trace(id)
        .into_iter()
        .flat_map(|record| {
            [
                (
                    format!("expanded from macro '{}'", record.macro_name),
                    record.invocation_span,
                ),
                ("macro definition here".to_string(), record.definition_span),
            ]
        })
        .collect()
}
```

- [ ] **Step 4: Add diagnostic label helper and use trace labels in macro diagnostics**

In `lib/src/diagnostic.rs`, add this method to `impl Diagnostic`:

```rust
pub fn with_labels(mut self, labels: Vec<(String, Span)>) -> Self {
    self.labels.extend(labels);
    self
}
```

When macro matching, generated parsing, or proc-macro execution reports diagnostics for an expansion with an `ExpansionId`, append `MacroSourceMap::trace_labels(expansion_id)` to each diagnostic's labels.

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

Commit:

```bash
git add lib/src/macro_expansion/source_map.rs lib/src/macro_expansion/context.rs lib/src/diagnostic.rs lib/tests/integration.rs
git commit -m "report macro expansion traces"
```

## Task 14: Final Cleanup And Full Verification

**Files:**
- Modify: `lib/src/macro_expansion/mod.rs`
- Modify: `lib/src/macro_expansion/macro_arg_matcher.rs`
- Modify: `docs/superpowers/plans/master-audit-checklist.md`
- Modify: `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`

- [ ] **Step 1: Remove compatibility `Config::default()` macro reparse paths**

Search within source files:

```bash
rg "Config::default\(\)|ParseCtx::from" lib/src/macro_expansion lib/src/parser
```

Expected after cleanup:

- No `Config::default()` in `lib/src/macro_expansion/**`.
- No direct `module_inline.process(ParseCtx::from(...))` in `lib/src/macro_expansion/mod.rs`.
- `ParseCtx::from` remains allowed inside public parser APIs and parser tests.

- [ ] **Step 2: Update roadmap and audit status**

In `docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md`, update Task 22 status to describe the completed macro architecture slice. In `docs/superpowers/plans/master-audit-checklist.md`, update the Parser, Macro, And Module Loader Cleanup row and checklist entries for macro expansion only. Do not mark parser IO or formatter trivia complete unless separate implementation landed.

- [ ] **Step 3: Run focused macro tests**

Run:

```bash
cargo test -p rock-lib macro_expansion -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

Expected: PASS.

- [ ] **Step 5: Commit final cleanup**

Commit:

```bash
git add lib/src/macro_expansion docs/superpowers/plans/master-audit-checklist.md docs/superpowers/plans/2026-05-17-compiler-architecture-ordered-roadmap.md
git commit -m "finish macro expansion architecture"
```

## Final Review

After Task 14, request two reviews before considering the branch complete:

- Spec compliance review against `docs/superpowers/specs/2026-05-24-macro-expansion-architecture-design.md`.
- Code quality review focused on macro parser boundaries, source-map correctness, proc-macro process isolation, diagnostics, and product metadata compatibility.

Final branch completion requires:

```bash
cargo fmt --all --check
git diff --check
cargo test -p rock-lib
```

All commands must pass in `/root/new_lang2/.worktrees/macro-architecture-redesign` before merge or PR work.
