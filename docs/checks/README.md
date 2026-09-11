# Book Highlighting

The book requires Node.js 22, mdBook (CI pins 0.4.52), tree-sitter-cli 0.26.9,
and a C compiler for the Rock parser. No npm dependencies are needed.

From the repository root:

```sh
cargo install tree-sitter-cli --version 0.26.9 --locked
(cd tree-sitter-rock && tree-sitter generate)
node --test docs/checks/rock-highlight.test.cjs
mdbook build docs
node docs/checks/verify-book.cjs
```

`rock-highlight.cjs` implements the mdBook preprocessor protocol. It batches Rock
fences through the tree-sitter CLI using `docs/theme/highlights.scm` and
`docs/theme/locals.scm` against
the real Rock syntax tree. The generated parser C source is ignored, so generation is
required on a fresh checkout. Grammar and query errors fail the build.

The preprocessor retains tree-sitter capture classes such as `function`,
`operator assignment`, and `variable builtin`. Style these under
`code.language-rock` in `docs/theme/rock.css`. Rock token classification must stay
in the AST query, not a JavaScript tokenizer. The `nohighlight` class prevents
mdBook's Highlight.js pass from replacing the spans; the ordinary `pre > code`
structure retains mdBook's copy buttons. Source round-trip checks ensure copied
text has no table line numbers, injected markup, or changed whitespace.

The locals query propagates parameter colors through their lexical scopes,
including captured references and reassignment. Member names and unrelated
body bindings retain their own roles rather than inheriting colors by spelling.

The preprocessor resolves grammar and query paths relative to its script, not
the caller's working directory. mdBook runs the configured command from `docs`.
The verifier checks every chapter's rendered Rock blocks against its Markdown
source and checks individual AST captures in the Resource/Drop example.
