import { execFile } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { childEnv } from './hermetic.mjs';

const REPO = fileURLToPath(new URL('../../../', import.meta.url));
const FIXTURE_PROJECTS = ['one', 'two'];
const FIXTURE_CONFIG = `export default { test: { projects: ${JSON.stringify(FIXTURE_PROJECTS.map((name) => ({ extends: true, test: { name } })))} } };\n`;
const RESOLVE_WORKERS = `
const { createVitest, version } = await import('vitest/node');
const root = process.argv[1];
const vitest = await createVitest('test', { root, watch: false, reporters: [], config: root + '/vitest.config.mjs' });
const workers = [vitest.config, ...vitest.projects.map((project) => project.config)].map((config) => config.maxWorkers ?? null);
await vitest.close();
console.log(JSON.stringify({ version, workers }));
`;
const CAP_UNDER_TEST = 2;
const RESOLVE_TIMEOUT_MS = 60_000;

const packagesWithVitest = () =>
  ['infra', ...['apps', 'packages'].flatMap((group) => readdirSync(join(REPO, group)).map((name) => join(group, name)))]
    .filter((dir) => existsSync(join(REPO, dir, 'package.json')))
    .filter((dir) => {
      const { dependencies, devDependencies } = JSON.parse(readFileSync(join(REPO, dir, 'package.json'), 'utf8'));
      return 'vitest' in { ...dependencies, ...devDependencies };
    });

const vitestInstalls = () => {
  const byVersion = new Map();
  for (const dir of packagesWithVitest()) {
    const manifest = createRequire(join(REPO, dir, 'package.json')).resolve('vitest/package.json');
    const { version } = JSON.parse(readFileSync(manifest, 'utf8'));
    byVersion.set(version, [...(byVersion.get(version) ?? []), dir]);
  }
  return [...byVersion];
};

describe('a real vitest honours VITEST_MAX_WORKERS, the variable gate.sh exports (CD-309)', () => {
  let project;
  beforeAll(() => {
    project = mkdtempSync(join(tmpdir(), 'wg-vitest-cap-'));
    writeFileSync(join(project, 'vitest.config.mjs'), FIXTURE_CONFIG);
  });
  afterAll(() => rmSync(project, { recursive: true, force: true }));

  const resolveIn = async (dir, env) => {
    const { stdout } = await promisify(execFile)(process.execPath, ['--input-type=module', '-e', RESOLVE_WORKERS, project], { cwd: join(REPO, dir), env: childEnv(env) });
    return JSON.parse(stdout.trim().split('\n').at(-1));
  };

  it('every vitest version the workspace runs resolves the cap for the root config and every project', async () => {
    const installs = vitestInstalls();
    expect(installs.length).toBeGreaterThan(0);
    for (const [version, dirs] of installs) {
      const [uncapped, capped] = await Promise.all([resolveIn(dirs[0], {}), resolveIn(dirs[0], { VITEST_MAX_WORKERS: String(CAP_UNDER_TEST) })]);
      expect.soft(capped, version).toEqual({ version, workers: Array(1 + FIXTURE_PROJECTS.length).fill(CAP_UNDER_TEST) });
      expect.soft(uncapped.workers, version).not.toEqual(capped.workers);
    }
  }, RESOLVE_TIMEOUT_MS);
});
