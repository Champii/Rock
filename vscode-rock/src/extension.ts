import * as vscode from 'vscode';
import { isAbsolute, resolve } from 'node:path';
import { LanguageClient } from 'vscode-languageclient/node';
import { createHighlighter, tokenTypes } from './highlights';

const clients: LanguageClient[] = [];

export async function activate(context: vscode.ExtensionContext) {
  const output = vscode.window.createOutputChannel('Rock');
  context.subscriptions.push(output);
  const report = (feature: string, error: unknown) => {
    const message = `Rock ${feature}: ${error instanceof Error ? error.message : String(error)}`;
    output.appendLine(message);
    void vscode.window.showErrorMessage(message);
  };

  try {
    const highlighter = await createHighlighter(context.asAbsolutePath('dist'));
    context.subscriptions.push(highlighter);
    const legend = new vscode.SemanticTokensLegend(tokenTypes);
    context.subscriptions.push(vscode.languages.registerDocumentSemanticTokensProvider(
      { language: 'rock' },
      {
        provideDocumentSemanticTokens(document, cancellation) {
          if (cancellation.isCancellationRequested) return;
          const builder = new vscode.SemanticTokensBuilder(legend);
          for (const token of highlighter.tokens(document.getText())) {
            builder.push(token.line, token.start, token.length, token.type, 0);
          }
          return builder.build();
        },
      },
      legend,
    ));
  } catch (error) {
    report('highlighting failed', error);
  }

  if (!vscode.workspace.isTrusted) return;
  let started = false;
  async function start(document: vscode.TextDocument) {
    if (document.languageId !== 'rock' || document.uri.scheme !== 'file') return;
    if (started) return;
    started = true;
    const folder = vscode.workspace.workspaceFolders?.[0];
    const config = vscode.workspace.getConfiguration('rock');
    let command = config.get<string>('server.path', 'rock-lsp');
    if (!isAbsolute(command) && /[/\\]/.test(command)) {
      if (!folder) {
        report('server failed', new Error('Use an absolute rock.server.path when no workspace folder is open.'));
        return;
      }
      command = resolve(folder.uri.fsPath, command);
    }
    const client = new LanguageClient('rock', 'Rock', {
      command,
      args: config.get<string[]>('server.args', []),
      options: { cwd: folder?.uri.fsPath },
    }, {
      documentSelector: [{ language: 'rock', scheme: 'file' }],
      outputChannel: output,
    });
    clients.push(client);
    try {
      await client.start();
    } catch (error) {
      report('server failed; check rock.server.path and reload the window', error);
    }
  }
  context.subscriptions.push(vscode.workspace.onDidOpenTextDocument(document => void start(document)));
  await Promise.all(vscode.workspace.textDocuments.map(start));
}

export async function deactivate() {
  await Promise.all(clients.map(client => client.stop()));
}
