import { readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { describe, expect, it, vi } from 'vitest';

vi.mock('esbuild', async (importOriginal) => {
  const esbuild = await importOriginal();
  return { ...esbuild, build: (options) => esbuild.build({ ...options, external: [...(options.external ?? []), '@wg/*'] }) };
});

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const SKIP = new Set(['node_modules', 'fixtures', '.git', '.work', '.claude', 'dist']);

function configs(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory()) return SKIP.has(entry.name) ? [] : configs(path.join(dir, entry.name));
    return /^vitest.*\.config\.[cm]?[jt]s$/.test(entry.name) ? [path.join(dir, entry.name)] : [];
  });
}

const found = configs(ROOT);

describe('every vitest config shuffles (CD-259: unit tests never rely on order)', () => {
  it('finds the workspace configs, and each sets sequence.shuffle and pins no seed', async () => {
    expect.soft(found.length).toBeGreaterThanOrEqual(11);
    for (const file of found) {
      const name = path.relative(ROOT, file);
      const loaded = (await import(pathToFileURL(file).href)).default;
      const config = typeof loaded === 'function' ? await loaded({ command: 'test', mode: 'test' }) : loaded;
      expect.soft(config.test?.sequence?.shuffle, name).toBe(true);
      expect.soft(config.test?.sequence?.seed, name).toBeUndefined();
    }
  }, 60_000);
});
