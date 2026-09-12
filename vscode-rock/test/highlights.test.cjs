const assert = require('node:assert/strict');
const { after, before, test } = require('node:test');
const { join } = require('node:path');
const { readFileSync, readdirSync } = require('node:fs');
const { createHighlighter, tokenTypes, captureTypes } = require('../dist/highlights.cjs');
const { resourceSource } = require('../../docs/checks/highlight-regression.cjs');
const { rockFences, highlightSources, htmlSource } = require('../../docs/checks/rock-highlight.cjs');

let highlighter;
before(async () => { highlighter = await createHighlighter(join(__dirname, '../dist')); });
after(() => highlighter?.dispose());

function highlighted(text) {
  const lines = text.split('\n');
  return highlighter.tokens(text).map(token => ({
    ...token,
    text: lines[token.line].slice(token.start, token.start + token.length),
    name: tokenTypes[token.type],
  }));
}

test('shared query highlights declarations without generic identifier duplicates', () => {
  const tokens = highlighted('struct Point\n  x: I64\n\nmain = -> 0\n');
  for (const [text, name] of [['struct', 'keyword'], ['Point', 'type'], ['x', 'property'], ['I64', 'type'], ['main', 'function'], ['0', 'number']]) {
    const matches = tokens.filter(token => token.text === text);
    assert.equal(matches.length, 1, text);
    assert.equal(matches[0].name, name, text);
  }
});

test('UTF-16 columns remain correct after non-ASCII text', () => {
  const text = 'main = -> "\u{1f600}\u00e9"; 42\n';
  const tokens = highlighted(text);
  const number = tokens.find(token => token.text === '42');
  assert.ok(number);
  assert.equal(number.start, text.indexOf('42'));
  assert.equal(tokens.find(token => token.name === 'string').text, '"\u{1f600}\u00e9"');
});

test('multiline captures are split into ordered non-overlapping single-line tokens', () => {
  const text = '/* first\r\n\r\nlast */\r\nmain = -> 0\r\n';
  const tokens = highlighted(text);
  assert.deepEqual(tokens.filter(token => token.name === 'comment').map(token => token.text), ['/* first', 'last */']);
  for (let index = 0; index < tokens.length; index++) {
    const token = tokens[index];
    assert.ok(token.length > 0);
    assert.ok(!token.text.includes('\r'));
    const previous = tokens[index - 1];
    if (previous) assert.ok(token.line > previous.line || (token.line === previous.line && token.start >= previous.start + previous.length));
  }
});

test('incomplete edits and successive documents do not break highlighting', () => {
  assert.ok(highlighted('main = -> "unfinished').some(token => token.text === 'main'));
  assert.deepEqual(highlighter.tokens(''), []);
  assert.ok(highlighted('other = -> 7\n').some(token => token.text === 'other' && token.name === 'function'));
});

test('extended roles distinguish receivers, methods, assignments, arrows, and punctuation', () => {
  const tokens = highlighted(resourceSource);
  for (const [text, name] of [
    ['~@', 'rockReceiver'], ['drop', 'function'], ['=', 'rockAssignment'],
    ['->', 'rockArrow'], [':', 'rockAnnotation'], ['.', 'rockPunctuation'],
    ['println', 'function'], ['!', 'rockCall'], ['Resource', 'type'], ['id', 'property'],
  ]) assert.ok(tokens.some(token => token.text === text && token.name === name), `${text}: ${name}`);
});

test('parameters stay parameters while locals and other functions remain independent', () => {
  const tokens = highlighted('apply = value, callback ->\n    local = value + 1\n    callback value\n    inner = extra -> value + extra\n    value = local\n    value\n\nother = ->\n    value = 0\n    value\n');
  assert.deepEqual(tokens.filter(token => token.text === 'value').map(token => token.name),
    [...Array(6).fill('parameter'), 'variable', 'variable']);
  assert.ok(tokens.filter(token => token.text === 'local').every(token => token.name === 'variable'));
  assert.ok(tokens.filter(token => token.text === 'callback' || token.text === 'extra').every(token => token.name === 'parameter'));
});

test('variants resolve per document, including forward, imported, and generic forms', () => {
  const tokens = highlighted('make = -> Data 1\n\nenum Event\n    Data I64\n    Empty\n\nmain = ->\n    value = Event::Data 2\n    match value\n        Data x => x\n        Empty => 0\n    (Event _)::Data 3\n');
  assert.ok(tokens.filter(token => token.text === 'Data' || token.text === 'Empty').every(token => token.name === 'enumMember'));
  assert.ok(tokens.filter(token => token.text === 'Event').every(token => token.name === 'type'));
  assert.equal(highlighted('> library::Event::Data\nmain = -> Data 1\n').filter(token => token.text === 'Data' && token.name === 'enumMember').length, 2);
  assert.equal(highlighted('main = -> Data\n').find(token => token.text === 'Data').name, 'type');
});

test('book and editor classifications agree across every book example', () => {
  const sources = [];
  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) visit(path);
      else if (path.endsWith('.md')) sources.push(...rockFences(readFileSync(path, 'utf8')).map(block => block.source));
    }
  }
  visit(join(__dirname, '../../docs/src'));
  const rendered = highlightSources(sources);
  for (const [index, source] of sources.entries()) {
    const expected = [];
    const stack = [];
    for (const match of rendered[index].matchAll(/<span class='([^']+)'>|<\/span>|([^<]+)/g)) {
      if (match[1]) stack.push(match[1].replaceAll(' ', '.'));
      else if (!match[2]) stack.pop();
      else {
        const role = stack.at(-1) ?? '';
        expected.push(...htmlSource(match[2]).split('').map(() => captureTypes[role] ?? captureTypes[role.split('.')[0]]));
      }
    }
    assert.equal(expected.length, source.length);
    const actual = Array(source.length).fill(undefined);
    const offsets = [0];
    for (const line of source.split('\n')) offsets.push(offsets.at(-1) + line.length + 1);
    for (const token of highlighter.tokens(source)) {
      const start = offsets[token.line] + token.start;
      actual.fill(tokenTypes[token.type], start, start + token.length);
    }
    const mismatch = actual.findIndex((type, position) => !'\r\n'.includes(source[position]) && type !== expected[position]);
    assert.equal(mismatch, -1, `Example ${index + 1}, offset ${mismatch}: editor ${actual[mismatch]}, book ${expected[mismatch]}\n${source}`);
  }
  assert.ok(sources.length > 0);
});

test('optional book themes style every semantic category without changing user defaults', () => {
  const manifest = JSON.parse(readFileSync(join(__dirname, '../package.json'), 'utf8'));
  assert.equal(manifest.contributes.configurationDefaults['workbench.colorTheme'], undefined);
  for (const variant of ['light', 'dark']) {
    const theme = JSON.parse(readFileSync(join(__dirname, `../dist/rock-${variant}.json`), 'utf8'));
    for (const type of tokenTypes) {
      assert.ok(theme.semanticTokenColors[`${type}:rock`]?.foreground, type);
      assert.ok(manifest.contributes.semanticTokenScopes[0].scopes[type]?.length, type);
    }
    for (const [left, right] of [['parameter', 'variable'], ['enumMember', 'type'], ['rockReceiver', 'function'], ['rockArrow', 'rockAssignment']]) {
      assert.notEqual(theme.semanticTokenColors[`${left}:rock`].foreground, theme.semanticTokenColors[`${right}:rock`].foreground);
    }
  }
});
