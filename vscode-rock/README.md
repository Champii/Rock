# Rock for VS Code

This extension connects the existing Rock tools to VS Code:

- `rock-lsp` provides compiler diagnostics, hover types, and signature help.
- `tree-sitter-rock` provides highlighting through VS Code's semantic token API, using the repository's existing grammar and `queries/highlights.scm`.
- `.rk` files are recognized as Rock, with comment toggling and bracket pairing.

Highlighting is syntax-based, not inferred type classification. Completion, go-to-definition, references, and rename are not yet provided by `rock-lsp`.

## Build and Install

From the repository root, build the language server; this contributor workflow requires the compiler's LLVM 18 dependencies:

```sh
cargo build -p rock-lsp
cd vscode-rock
npm ci
npm run check
npm run build
npm test
npm run package
code --install-extension rock-language-0.1.0.vsix
```

Use Node.js 22 or newer for the extension build. The pinned Tree-sitter CLI generates the existing grammar and builds a WASM parser, including its external scanner. On its first build it downloads the WASI SDK and Binaryen, so network access is required. The VSIX bundles both WASM modules and the shared highlight query; these build tools are not needed at runtime.

Set `rock.server.path` to your built `rock-lsp` executable in VS Code settings. This repository's workspace settings use `./target/debug/rock-lsp`; other workspaces default to `rock-lsp` on PATH. Relative executable paths resolve against the first workspace folder. Without an open folder, use an absolute executable path or a command on PATH.

Open a `.rk` file. If it was already open during installation, run **Developer: Reload Window**. Hover an identifier for its inferred type, use **Trigger Parameter Hints** for a call signature, and view compiler errors in the Problems panel. The **Rock** output channel contains server startup errors and LSP logs.

## Project Dependencies

The server resolves each source file's Rock project itself. For ordinary applications, use the existing `rock` CLI to configure and build the project's dependencies; the extension does not inject a stdlib or download a compiler.

For standalone files or explicit artifact overrides, configure server arguments:

```json
{
  "rock.server.path": "/absolute/path/to/rock-lsp",
  "rock.server.args": [
    "--extern-artifact",
    "stdlib=/absolute/path/to/stdlib.rkca"
  ]
}
```

These arguments are passed directly to the server without a shell. They apply to the whole window; reload the window after changing server settings. The extension only starts in trusted, filesystem workspaces because project analysis can build dependencies.

## Highlighting

Semantic highlighting is enabled by default for Rock and uses standard token types with TextMate scope mappings for theme support. An explicit user override disabling semantic highlighting will disable Rock colors as well. If another syntax-highlighting extension handles `.rk` files, disable its Rock support to avoid competing colors.

After editing `tree-sitter-rock/grammar.js` or its highlight queries, rebuild and reinstall the VSIX to update its bundled assets. There is no separate VS Code grammar to maintain.
