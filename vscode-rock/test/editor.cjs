const assert = require('node:assert/strict');
const { mkdtemp, writeFile, rm } = require('node:fs/promises');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const vscode = require('vscode');
const { tokenTypes } = require('../dist/highlights.cjs');

async function waitFor(action, description) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const result = await action();
    if (result) return result;
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  throw new Error(`Timed out waiting for ${description}`);
}

exports.run = async () => {
  const directory = await mkdtemp(join(tmpdir(), 'rock-vscode-'));
  try {
    const path = join(directory, 'main.rk');
    await writeFile(path, 'main = ->\n    value = 42\n    value\n');
    const document = await vscode.workspace.openTextDocument(path);
    await vscode.window.showTextDocument(document);
    assert.equal(document.languageId, 'rock');
    const extension = vscode.extensions.getExtension('champii.rock-language');
    assert.ok(extension);
    await extension.activate();
    const tokens = await waitFor(async () => {
      const result = await vscode.commands.executeCommand('vscode.provideDocumentSemanticTokens', document.uri);
      return result?.data?.length ? result : undefined;
    }, 'Tree-sitter semantic tokens');
    assert.ok(tokens.data.length >= 5);
    const semanticDocument = await vscode.workspace.openTextDocument({
      language: 'rock',
      content: 'enum Event\n    Empty\n\nstruct Resource\n    < id: I64\n\nimpl Drop for Resource\n    ~@drop = -> return\n\napply = value -> value.println!\n\nmain = -> Event::Empty\n',
    });
    await vscode.window.showTextDocument(semanticDocument);
    const extended = await waitFor(async () => {
      const result = await vscode.commands.executeCommand('vscode.provideDocumentSemanticTokens', semanticDocument.uri);
      return result?.data?.length ? result : undefined;
    }, 'extended Rock semantic tokens');
    const categories = new Set();
    for (let index = 3; index < extended.data.length; index += 5) categories.add(tokenTypes[extended.data[index]]);
    for (const type of ['parameter', 'enumMember', 'rockReceiver', 'rockAssignment', 'rockArrow', 'rockAnnotation', 'rockCall']) {
      assert.ok(categories.has(type), `Missing editor category ${type}`);
    }
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    await vscode.window.showTextDocument(document);
    await waitFor(async () => {
      const hovers = await vscode.commands.executeCommand('vscode.executeHoverProvider', document.uri, new vscode.Position(2, 6));
      return hovers?.length > 0;
    }, 'rock-lsp hover');
    const edit = new vscode.WorkspaceEdit();
    edit.replace(document.uri, new vscode.Range(2, 4, 2, 9), 'missing_name');
    assert.ok(await vscode.workspace.applyEdit(edit));
    await waitFor(() => vscode.languages.getDiagnostics(document.uri).some(diagnostic => diagnostic.source === 'rock'), 'rock-lsp diagnostics after an unsaved edit');
    console.log('Rock extension host: file recognition, extended Tree-sitter tokens, LSP hover, and live diagnostics passed.');
  } finally {
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    await rm(directory, { recursive: true, force: true });
  }
};
