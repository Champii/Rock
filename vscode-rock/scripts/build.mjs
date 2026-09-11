import { execFileSync } from 'node:child_process';
import { copyFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const root = fileURLToPath(new URL('../', import.meta.url));
const grammar = fileURLToPath(new URL('../../tree-sitter-rock/', import.meta.url));
await mkdir(`${root}dist`, { recursive: true });
execFileSync('tree-sitter', ['generate'], { cwd: grammar, stdio: 'inherit' });
execFileSync('tree-sitter', ['build', '--wasm', '--output', `${root}dist/tree-sitter-rock.wasm`], { cwd: grammar, stdio: 'inherit' });
await copyFile(`${grammar}queries/highlights.scm`, `${root}dist/highlights.scm`);
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
