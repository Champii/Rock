const assert = require('node:assert/strict');
const { after, before, test } = require('node:test');
const { join } = require('node:path');
const { createHighlighter, tokenTypes } = require('../dist/highlights.cjs');

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
