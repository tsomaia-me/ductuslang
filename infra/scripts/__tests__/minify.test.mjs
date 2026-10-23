import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { describe, expect, it } from 'vitest';

const ROOT = join(import.meta.dirname, '..', '..', '..');
const ADMIN = join(ROOT, 'apps', 'admin');
const MINIFIED_LINE_BYTES = 500;

const scripts = (dir) => JSON.parse(readFileSync(join(ROOT, dir, 'package.json'), 'utf8')).scripts;
const bytesPerLine = (text) => text.length / text.split('\n').length;

async function adminOutput(mode) {
  const { build } = await import(pathToFileURL(createRequire(join(ADMIN, 'package.json')).resolve('vite')).href);
  const result = await build({ root: ADMIN, configFile: join(ADMIN, 'vite.config.mts'), mode, logLevel: 'silent', build: { write: false } });
  const files = [result].flat().flatMap(({ output }) => output);
  const text = (pattern) => files.filter(({ fileName }) => pattern.test(fileName)).map((file) => String(file.code ?? file.source));
  return { scripts: text(/\.js$/), styles: text(/\.css$/), maps: files.filter(({ fileName }) => fileName.endsWith('.map')) };
}

describe('minification and source maps are the production build’s alone', () => {
  it('the admin’s build outside the release mode writes no minified script or style and no map, and the release mode minifies both', async () => {
    const local = await adminOutput('production');
    expect(local.scripts.length).toBeGreaterThan(0);
    for (const text of [...local.scripts, ...local.styles]) expect(bytesPerLine(text)).toBeLessThan(MINIFIED_LINE_BYTES);
    expect(local.maps).toEqual([]);
    const release = await adminOutput('release');
    for (const text of [...release.scripts, ...release.styles]) expect(bytesPerLine(text)).toBeGreaterThan(MINIFIED_LINE_BYTES);
    expect(release.maps).toEqual([]);
  }, 120_000);

  it('every app export but the release one skips minification, and none writes source maps', () => {
    const exports = Object.entries(scripts('apps/app')).filter(([, command]) => command.includes('expo export'));
    expect(exports.filter(([, command]) => !command.includes('--no-minify')).map(([name]) => name)).toEqual(['export:web:release']);
    for (const [name, command] of exports) expect(command, name).not.toMatch(/--source-maps|\s-s\b/);
  });
});
