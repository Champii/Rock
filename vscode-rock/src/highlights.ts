import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import TreeSitter = require('web-tree-sitter');

const { Language, Parser, Query } = TreeSitter;

export const tokenTypes = [
  'comment', 'keyword', 'number', 'string', 'operator', 'function',
  'macro', 'variable', 'type', 'property', 'decorator',
];

const captureTypes: Record<string, string> = {
  comment: 'comment', keyword: 'keyword', boolean: 'keyword', number: 'number',
  string: 'string', character: 'string', operator: 'operator', function: 'function',
  'function.macro': 'macro', variable: 'variable', type: 'type', property: 'property',
  attribute: 'decorator',
};

export interface Token {
  line: number;
  start: number;
  length: number;
  type: number;
}

export async function createHighlighter(directory: string) {
  await Parser.init({ locateFile: () => join(directory, 'web-tree-sitter.wasm') });
  const language = await Language.load(join(directory, 'tree-sitter-rock.wasm'));
  const query = new Query(language, await readFile(join(directory, 'highlights.scm'), 'utf8'));
  const parser = new Parser();
  parser.setLanguage(language);

  return {
    tokens(text: string): Token[] {
      const tree = parser.parse(text);
      if (!tree) throw new Error('Tree-sitter could not parse the document');
      try {
        const lines = text.split('\n');
        const candidates: (Token & { priority: number })[] = [];
        for (const capture of query.captures(tree.rootNode)) {
          const name = captureTypes[capture.name] ?? captureTypes[capture.name.split('.')[0]];
          if (!name) continue;
          const type = tokenTypes.indexOf(name);
          const { startPosition: start, endPosition: end } = capture.node;
          // web-tree-sitter's JavaScript input uses UTF-16, like VS Code.
          for (let line = start.row; line <= end.row; line++) {
            const column = line === start.row ? start.column : 0;
            const limit = line === end.row ? end.column : lines[line].replace(/\r$/, '').length;
            if (limit > column) candidates.push({
              line, start: column, length: limit - column, type,
              // The shared query ends in a generic identifier capture; specific captures win.
              priority: capture.name === 'variable' ? 0 : 1,
            });
          }
        }
        candidates.sort((a, b) => a.line - b.line || a.start - b.start || b.priority - a.priority || b.length - a.length);
        const tokens: Token[] = [];
        for (const candidate of candidates) {
          const previous = tokens.at(-1);
          if (previous && previous.line === candidate.line && previous.start + previous.length > candidate.start) continue;
          const { priority: _, ...token } = candidate;
          tokens.push(token);
        }
        return tokens;
      } finally {
        tree.delete();
      }
    },
    dispose() {
      parser.delete();
      query.delete();
    },
  };
}
