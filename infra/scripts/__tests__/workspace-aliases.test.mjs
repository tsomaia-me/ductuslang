import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { workspaceAliases } from '../workspace-aliases.mjs';

describe('workspaceAliases', () => {
  let root;
  const write = (path, text = '') => {
    mkdirSync(join(root, dirname(path)), { recursive: true });
    writeFileSync(join(root, path), text);
  };

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'workspace-aliases-'));
    write(
      'packages/kit/package.json',
      JSON.stringify({
        name: '@wg/kit',
        exports: {
          '.': { types: './dist/index.d.ts', default: './dist/index.js' },
          './react': { types: './dist/react.d.ts', default: './dist/react.js' },
          './admin': { types: './dist/admin/index.d.ts', default: './dist/admin/index.js' },
          './en/pack.json': './dist/en/pack.json',
          './locales/*': './dist/locales/*',
          './built': { default: './dist/built.js' },
        },
      }),
    );
    write('packages/kit/src/index.ts');
    write('packages/kit/src/react.tsx');
    write('packages/kit/src/admin/index.ts');
    write('packages/kit/src/en/pack.json', '{}');
    write('packages/plain/package.json', JSON.stringify({ name: '@wg/plain', main: './dist/index.js' }));
    write('packages/plain/src/index.ts');
  });
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const resolve = (aliases, specifier) => aliases.find(({ find }) => find.test(specifier))?.replacement;

  it('points every workspace export at its TypeScript source, so the import graph sees the source and no build is needed, matching a specifier exactly, and leaves to the package a wildcard export and one with no source file', () => {
    const aliases = workspaceAliases(root);
    for (const [specifier, source] of [
      ['@wg/kit', 'packages/kit/src/index.ts'],
      ['@wg/kit/react', 'packages/kit/src/react.tsx'],
      ['@wg/kit/admin', 'packages/kit/src/admin/index.ts'],
      ['@wg/kit/en/pack.json', 'packages/kit/src/en/pack.json'],
      ['@wg/plain', 'packages/plain/src/index.ts'],
    ]) {
      expect.soft(resolve(aliases, specifier), specifier).toBe(join(root, source));
    }
    for (const specifier of ['@wg/kit/missing', '@wg/kitten', '@wg/kit/locales/en.json', '@wg/kit/built']) {
      expect.soft(resolve(aliases, specifier), specifier).toBeUndefined();
    }
  });
});
