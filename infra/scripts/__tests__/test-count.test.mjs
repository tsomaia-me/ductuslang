import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { testKeys } from '../test-count.mjs';

import { GIT_ENV } from './harness-fixture.mjs';
import { childEnv } from './hermetic.mjs';

const SCRIPT = fileURLToPath(new URL('../test-count.mjs', import.meta.url));

const COUNTED = `
it('plain', () => {});
test('async', async () => {});
it.only('only', () => {});
test.skip('skipped', () => {});
it.concurrent.fails('chained', () => {});
it.todo('later');
it('options', { timeout: 5 }, () => {});
it.each([1, 2, 3])('row %s', (n) => {});
test.for([['a'], ['b']])('pair %s', ([x]) => {});
it.each(ROWS)('table', () => {});
it.each([...ROWS, 4])('spread', () => {});
it.each\`
  a | b
  \${1} | \${2}
\`('tagged', () => {});
it.skipIf(process.env.CI)('local only', () => {});
test('playwright', async ({ page }) => {
  test.skip(page.isClosed(), 'closed');
  await test.step('step', async () => {});
});
`;

const NOT_COUNTED = `
describe('suite', () => {
  beforeEach(() => {});
});
describe.each([1, 2])('suite %s', () => {});
test.describe('playwright suite', () => {});
test.beforeEach(async () => {});
test.use({ locale: 'en' });
const extended = test.extend({});
/^x/.test('x');
const text = "it('in a string', () => {})";
// it('in a comment', () => {});
/* test('in a block comment', () => {}); */
`;

describe('testKeys', () => {
  it('counts each it and test definition and each row of a literal table, and nothing in a suite, hook, runtime skip, comment or string', () => {
    const keys = testKeys(COUNTED, 'a.test.ts');
    expect.soft(keys).toHaveLength(17);
    expect.soft(keys.filter((key) => key.startsWith("'row %s'"))).toEqual(["'row %s'\u00001", "'row %s'\u00002", "'row %s'\u00003"]);
    expect.soft(keys.filter((key) => key === "'table'" || key === "'spread'" || key === "'tagged'")).toHaveLength(3);
    expect.soft(testKeys(NOT_COUNTED, 'b.spec.tsx')).toEqual([]);
    expect.soft(testKeys('it("jsx", () => <div />);', 'c.test.jsx')).toEqual(['"jsx"']);
  });
});

describe('test-count.mjs', () => {
  let root;
  let repo;
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: repo, encoding: 'utf8', env: { ...process.env, ...GIT_ENV } });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr}`);
  };
  const write = (files) => {
    for (const [path, text] of Object.entries(files)) {
      mkdirSync(join(repo, dirname(path)), { recursive: true });
      writeFileSync(join(repo, path), text);
    }
  };
  const commit = (files, message) => {
    write(files);
    git('add', '-A');
    git('commit', '-q', '-m', message);
  };
  const count = () => spawnSync(process.execPath, [SCRIPT, '--base', 'main'], { cwd: repo, encoding: 'utf8', env: childEnv(GIT_ENV) });

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'test-count-'));
    repo = join(root, 'repo');
    mkdirSync(repo);
    git('init', '-q', '-b', 'main');
    commit(
      {
        'package.json': '{}\n',
        'README.md': 'base\n',
        'packages/a/package.json': '{}\n',
        'packages/a/src/x.test.ts': "it('one', () => {});\nit('two', () => {});\nit.each([1, 2])('row %s', () => {});\n",
        'packages/a/src/gone.test.ts': "test('gone', () => {});\n",
        'packages/a/src/same.test.tsx': "it('kept', () => {});\n",
        'apps/b/package.json': '{}\n',
        'apps/b/old.spec.mjs': "test('b', async () => {});\n",
        'packages/c/package.json': '{}\n',
        'packages/c/index.ts': 'export const c = 1;\n',
        'packages/c/c.test.ts': "it('c', () => {});\n",
        'packages/d/package.json': '{}\n',
        'packages/d/d.test.ts': "it('d', () => {});\n",
      },
      'base',
    );
    git('checkout', '-q', '-b', 'docs');
    commit({ 'README.md': 'docs\n' }, 'docs');
    git('checkout', '-q', '-b', 'change', 'main');
    git('rm', '-q', 'packages/a/src/gone.test.ts');
    commit({ 'packages/a/src/x.test.ts': "it('one', () => {});\nit('three', () => {});\nit.each([1, 2, 3])('row %s', () => {});\n", 'packages/c/index.ts': 'export const c = 2;\n' }, 'change');
    git('checkout', '-q', 'main');
    commit({ 'packages/a/src/main.test.ts': "it('main', () => {});\n", 'packages/d/d.test.ts': "it('d', () => {});\nit('d2', () => {});\n" }, 'main moved on');
  });

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it('prints each touched package, by the merge base and the working tree, with the tests added and removed, and the total, and that no package is touched when the change is outside every package', () => {
    git('checkout', '-q', '-f', 'change');
    write({ 'packages/a/src/new.test.ts': "it('new', () => {});\nit.todo('later');\n", 'apps/b/old.spec.mjs': "test('b', async () => {});\ntest('b2', async () => {});\n" });
    const run = count();
    expect.soft(run.stderr).toBe('');
    expect.soft(run.status).toBe(0);
    expect
      .soft(run.stdout)
      .toBe(['tests: apps/b 1 -> 2 (+1/-0)', 'tests: packages/a 6 -> 8 (+4/-2)', 'tests: packages/c 1 -> 1 (+0/-0)', 'tests: total 8 -> 11 (+5/-2)', ''].join('\n'));
    git('checkout', '-q', '-f', 'docs');
    git('clean', '-q', '-f', '-d');
    const docs = count();
    expect.soft(docs.status, 'docs').toBe(0);
    expect.soft(docs.stdout, 'docs').toBe('tests: no package touched\n');
  });
});
