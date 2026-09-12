import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import TreeSitter = require('web-tree-sitter');

const { Language, Parser, Query } = TreeSitter;

// Palette keys come from docs/theme/rock.css; the build uses them for the
// optional Rock themes. Other VS Code themes retain control of token colors.
export const tokenColors: Record<string, string> = {
  comment: 'comment', keyword: 'keyword', number: 'number', string: 'string',
  operator: 'operator', function: 'function', macro: 'meta', variable: 'fg',
  type: 'type', property: 'property', decorator: 'meta', parameter: 'property',
  enumMember: 'variant', namespace: 'module', rockReceiver: 'receiver',
  rockAssignment: 'assignment', rockArrow: 'arrow', rockPunctuation: 'punctuation',
  rockAnnotation: 'annotation', rockCall: 'receiver', rockIntrinsic: 'meta',
  rockBoolean: 'number',
};
export const tokenTypes = Object.keys(tokenColors);

export const captureTypes: Record<string, string> = {
  comment: 'comment', keyword: 'keyword', number: 'number', string: 'string',
  operator: 'operator', function: 'function', 'function.macro': 'macro',
  'function.builtin': 'rockIntrinsic', variable: 'variable', type: 'type',
  property: 'property', attribute: 'decorator', module: 'namespace',
  'variable.parameter': 'parameter', 'variable.builtin': 'rockReceiver',
  'constant.variant': 'enumMember', 'constant.builtin': 'rockBoolean',
  'operator.assignment': 'rockAssignment', 'operator.arrow': 'rockArrow',
  punctuation: 'rockPunctuation', 'punctuation.annotation': 'rockAnnotation',
  'punctuation.special': 'rockCall',
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
  const [highlightSource, localSource] = await Promise.all([
    readFile(join(directory, 'highlights.scm'), 'utf8'),
    readFile(join(directory, 'locals.scm'), 'utf8'),
  ]);
  const query = new Query(language, highlightSource);
  let locals: TreeSitter.Query;
  try {
    locals = new Query(language, localSource);
  } catch (error) {
    query.delete();
    throw error;
  }
  const parser = new Parser();
  parser.setLanguage(language);

  return {
    tokens(text: string): Token[] {
      const tree = parser.parse(text);
      if (!tree) throw new Error('Tree-sitter could not parse the document');
      try {
        const lines = text.split('\n');
        const captures = new Map<number, TreeSitter.QueryCapture>();
        const variants = new Set<string>();
        for (const capture of query.captures(tree.rootNode)) {
          const previous = captures.get(capture.node.id);
          if (!previous || capture.patternIndex >= previous.patternIndex) captures.set(capture.node.id, capture);
          if (capture.name === 'constant.variant.definition') variants.add(capture.node.text);
        }
        const roles = new Map<number, string>();
        for (const { name, node } of captures.values()) {
          roles.set(node.id, name === 'constant.variant.definition' ? 'constant.variant'
            : name === 'type.variant_reference' ? (variants.has(node.text) ? 'constant.variant' : 'type') : name);
        }

        // Use AST ancestry, not ranges, so adjacent function scopes cannot leak.
        const localCaptures = locals.captures(tree.rootNode);
        const scopes = new Map<number, { inherits: boolean; definitions: Map<string, TreeSitter.Node[]> }>();
        for (const capture of localCaptures) {
          if (capture.name === 'local.scope') scopes.set(capture.node.id, {
            inherits: capture.setProperties?.['local.scope-inherits'] !== 'false', definitions: new Map(),
          });
        }
        for (const capture of localCaptures) {
          if (capture.name !== 'local.definition') continue;
          for (let parent = capture.node.parent; parent; parent = parent.parent) {
            const scope = scopes.get(parent.id);
            if (!scope) continue;
            const definitions = scope.definitions.get(capture.node.text) ?? [];
            definitions.push(capture.node);
            scope.definitions.set(capture.node.text, definitions);
            break;
          }
        }
        for (const capture of localCaptures) {
          if (capture.name !== 'local.reference') continue;
          for (let parent = capture.node.parent; parent; parent = parent.parent) {
            const scope = scopes.get(parent.id);
            if (!scope) continue;
            const definition = scope.definitions.get(capture.node.text)
              ?.filter(node => node.startIndex <= capture.node.startIndex).at(-1);
            if (definition) {
              roles.set(capture.node.id, roles.get(definition.id) ?? 'variable');
              break;
            }
            if (!scope.inherits) break;
          }
        }

        const candidates: (Token & { priority: number })[] = [];
        for (const capture of captures.values()) {
          const role = roles.get(capture.node.id)!;
          const name = captureTypes[role] ?? captureTypes[role.split('.')[0]];
          if (!name) continue;
          const type = tokenTypes.indexOf(name);
          const { startPosition: start, endPosition: end } = capture.node;
          // web-tree-sitter's JavaScript input uses UTF-16, like VS Code.
          for (let line = start.row; line <= end.row; line++) {
            const column = line === start.row ? start.column : 0;
            const limit = line === end.row ? end.column : lines[line].replace(/\r$/, '').length;
            if (limit > column) candidates.push({
              line, start: column, length: limit - column, type,
              priority: capture.patternIndex,
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
      locals.delete();
    },
  };
}
