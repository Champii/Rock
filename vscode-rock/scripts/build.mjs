import { execFileSync } from 'node:child_process';
import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const root = fileURLToPath(new URL('../', import.meta.url));
const grammar = fileURLToPath(new URL('../../tree-sitter-rock/', import.meta.url));
await mkdir(`${root}dist`, { recursive: true });
execFileSync('tree-sitter', ['generate'], { cwd: grammar, stdio: 'inherit' });
execFileSync('tree-sitter', ['build', '--wasm', '--output', `${root}dist/tree-sitter-rock.wasm`], { cwd: grammar, stdio: 'inherit' });
await copyFile(`${grammar}queries/highlights.scm`, `${root}dist/highlights.scm`);
await copyFile(`${grammar}queries/locals.scm`, `${root}dist/locals.scm`);
await copyFile(`${root}node_modules/web-tree-sitter/web-tree-sitter.wasm`, `${root}dist/web-tree-sitter.wasm`);
await build({
  absWorkingDir: root,
  entryPoints: ['src/extension.ts'],
  outfile: 'dist/extension.js',
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node18',
  external: ['vscode'],
});
await build({
  absWorkingDir: root,
  entryPoints: ['src/highlights.ts'],
  outfile: 'dist/highlights.cjs',
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node18',
});

// Generate optional editor themes from the book palette, not a second set of hex values.
const { tokenColors } = createRequire(import.meta.url)(`${root}dist/highlights.cjs`);
const manifest = JSON.parse(await readFile(`${root}package.json`, 'utf8'));
const css = await readFile(new URL('../../docs/theme/rock.css', import.meta.url), 'utf8');
const palettes = [...css.matchAll(/\{([^{}]*--rock-code-bg:[^{}]*)\}/g)].map((match) =>
  Object.fromEntries([...match[1].matchAll(/--rock-code-([\w-]+):\s*(#[\da-f]{6})/g)]
    .map((color) => [color[1], color[2]])));
if (palettes.length !== 2) throw new Error('Expected light and dark Rock book palettes');
const scopes = manifest.contributes.semanticTokenScopes[0].scopes;
for (const [index, variant] of ['light', 'dark'].entries()) {
  const palette = palettes[index];
  for (const key of Object.values(tokenColors)) {
    if (!palette[key]) throw new Error(`Missing book palette color: ${key}`);
  }
  await writeFile(`${root}dist/rock-${variant}.json`, JSON.stringify({
    name: `Rock Book ${variant === 'dark' ? 'Dark' : 'Light'}`,
    semanticHighlighting: true,
    colors: {
      'editor.background': palette.bg, 'editor.foreground': palette.fg,
      'editorLineNumber.foreground': palette.comment,
      'editorLineNumber.activeForeground': palette.fg,
      'editorCursor.foreground': palette.type,
    },
    tokenColors: Object.entries(tokenColors).filter(([token]) => !token.startsWith('rock')).map(([token, key]) => ({
      scope: scopes[token], settings: { foreground: palette[key], ...(token === 'comment' ? { fontStyle: 'italic' } : {}) },
    })),
    semanticTokenColors: Object.fromEntries(Object.entries(tokenColors).map(([token, key]) =>
      [`${token}:rock`, { foreground: palette[key], ...(token === 'comment' ? { italic: true } : {}) }])),
  }, null, 2) + '\n');
}
