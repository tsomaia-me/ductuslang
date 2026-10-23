import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { childEnv } from './hermetic.mjs';

const SCRIPT = fileURLToPath(new URL('../journeys-local.sh', import.meta.url));
const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const WORKFLOWS = join(ROOT, '.github/workflows');
const COPIED = ['infra/scripts', 'infra/verify'];
const ADMIN_HOSTS_FILE = 'packages/shared/src/admin/hosts.json';

const surface = (e2e) => ({ screens: [], unplaceable: [], served: false, e2e, verify: null });

function secretNames() {
  const names = new Set();
  for (const file of readdirSync(WORKFLOWS)) {
    const text = readFileSync(join(WORKFLOWS, file), 'utf8');
    for (const [, name] of text.matchAll(/secrets\.([A-Z0-9_]+)/g)) names.add(name);
    for (const [, name] of text.matchAll(/^\s*([A-Z0-9_]+):\s*\$\{\{\s*secrets\./gm)) names.add(name);
  }
  return [...names, 'GITHUB_TOKEN', 'GH_TOKEN'];
}

describe('journeys-local.sh: the journeys a dispatched ref runs, on the local stack (CD-405)', () => {
  let out;
  let root;
  let fake;
  const journeysOf = (name, files) => {
    const dir = join(root, 'apps', name, 'e2e', 'journeys');
    mkdirSync(dir, { recursive: true });
    for (const file of files) writeFileSync(join(dir, file), '');
  };
  const call = () => JSON.parse(readFileSync(join(out, 'call.json'), 'utf8'));
  const nextRow = () => rmSync(join(out, 'call.json'), { force: true });

  beforeEach(() => {
    out = mkdtempSync(join(tmpdir(), 'cd405-local-'));
    root = join(out, 'repo');
    mkdirSync(root);
    spawnSync('git', ['init', '-q'], { cwd: root });
    for (const dir of COPIED) cpSync(join(ROOT, dir), join(root, dir), { recursive: true, filter: (path) => !path.includes('__tests__') });
    mkdirSync(dirname(join(root, ADMIN_HOSTS_FILE)), { recursive: true });
    cpSync(join(ROOT, ADMIN_HOSTS_FILE), join(root, ADMIN_HOSTS_FILE));
    symlinkSync(join(ROOT, 'node_modules'), join(root, 'node_modules'));
    journeysOf('app', ['local.config.ts', 'auth.setup.ts', 'store.spec.ts']);
    journeysOf('admin', ['local.config.ts', 'access.setup.ts', 'users.spec.ts']);
    fake = join(out, 'pnpm');
    writeFileSync(
      fake,
      `#!/usr/bin/env node
const { writeFileSync } = require('node:fs');
writeFileSync(${JSON.stringify(join(out, 'call.json'))}, JSON.stringify({ args: process.argv.slice(2), env: process.env }));
if (process.env.FAKE_OUTPUT) require(process.env.FAKE_OUTPUT)(require('node:path').join(process.argv[3], 'test-results'));
if (process.env.FAKE_REPORT !== 'none') writeFileSync(process.env.LIVE_REPORT, JSON.stringify({ suites: [{ specs: [{ title: 'users: list', file: 'users.spec.ts', tags: ['@screen:/users'], tests: [{ projectName: 'desktop', status: 'unexpected', results: [{ errors: [{ message: 'Error: timed out' }] }] }] }] }] }));
process.exit(Number(process.env.FAKE_EXIT ?? 1));
`,
      { mode: 0o755 },
    );
  });
  afterEach(() => rmSync(out, { recursive: true, force: true }));

  const run = (name, plan, env = {}) => {
    writeFileSync(join(out, 'plan.json'), JSON.stringify({ screenChange: false, surfaces: { app: surface({ all: false, files: [] }), admin: surface({ all: false, files: [] }), [name]: plan } }));
    const result = spawnSync('bash', [SCRIPT, name], { cwd: root, env: { ...childEnv(), OUT: out, JOURNEYS_PNPM: fake, ...env }, encoding: 'utf8' });
    return { ...result, written: JSON.parse(readFileSync(join(out, `${name}-e2e.json`), 'utf8')) };
  };

  it("runs a surface's journeys with its local script: the selected files as paths after it, filtered to the tests the plan greps for, or the whole suite with no file, on a port of its own, passing the exit through", () => {
    const admin = run('admin', surface({ all: false, files: ['apps/admin/e2e/journeys/users.spec.ts'] }));
    expect.soft(admin.status, 'admin files').toBe(1);
    const { args, env } = call();
    expect.soft(args, 'admin files').toEqual(['--dir', join(root, 'apps/admin'), 'run', 'e2e:local', 'e2e/journeys/users.spec.ts']);
    expect.soft(env.E2E_PORT, 'admin files').toMatch(/^[1-9][0-9]*$/);
    expect.soft(env.LIVE_REPORT, 'admin files').toBe(join(out, 'admin-report.json'));
    expect.soft(admin.written, 'admin files').toEqual({ ran: 1, infrastructure: [], findings: [{ severity: 'blocking', screen: '/users', what: 'Error: timed out', source: 'e2e apps/admin/e2e/journeys/users.spec.ts › users: list' }] });

    nextRow();
    writeFileSync(join(out, 'plan.json'), JSON.stringify({ screenChange: false, grep: '@smoke', surfaces: { app: surface({ all: false, files: ['apps/app/e2e/journeys/store.spec.ts'] }), admin: surface({ all: false, files: [] }) } }));
    spawnSync('bash', [SCRIPT, 'app'], { cwd: root, env: { ...childEnv(), OUT: out, JOURNEYS_PNPM: fake, FAKE_EXIT: '0' }, encoding: 'utf8' });
    expect.soft(call().args, 'app files, grep').toEqual(['--dir', join(root, 'apps/app'), 'run', 'e2e:journeys:local', 'e2e/journeys/store.spec.ts', '--grep', '@smoke']);

    nextRow();
    const whole = run('app', surface({ all: true, files: [] }), { FAKE_EXIT: '0' });
    expect.soft(whole.status, 'app whole').toBe(0);
    expect.soft(call().args, 'app whole').toEqual(['--dir', join(root, 'apps/app'), 'run', 'e2e:journeys:local']);
  });

  it('hands the journeys no secret any workflow uses, even when the runner exports them', () => {
    const names = secretNames();
    expect(names).toEqual(expect.arrayContaining(['CF_ACCESS_CLIENT_ID', 'CF_ACCESS_CLIENT_SECRET', 'WALK_SEED_KEY']));
    const exported = Object.fromEntries(names.map((name) => [name, 'a-real-secret']));
    for (const name of ['app', 'admin']) {
      nextRow();
      run(name, surface({ all: true, files: [] }), exported);
      expect(Object.keys(call().env).filter((key) => names.includes(key)), name).toEqual([]);
    }
  });

  it("copies only the failing tests' screenshots into a fresh folder: no other PNG, trace, video, HAR, state, hidden file or symlink (head16)", () => {
    const outside = join(out, 'outside');
    mkdirSync(outside);
    writeFileSync(join(outside, 'test-failed-1.png'), 'outside');
    for (const stale of ['repo/apps/app/test-results/old/test-failed-1.png', 'app-screenshots/planted/test-failed-1.png']) {
      mkdirSync(join(out, stale, '..'), { recursive: true });
      writeFileSync(join(out, stale), 'stale');
    }
    const writer = join(out, 'output.cjs');
    writeFileSync(
      writer,
      `const { mkdirSync, symlinkSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');
module.exports = (dir) => {
  const test = join(dir, 'store-buy-journeys');
  mkdirSync(join(test, '.hidden'), { recursive: true });
  for (const name of ['test-failed-1.png', 'shot.png', 'trace.zip', 'video.webm', 'error-context.md', 'network.har', 'state.json', '.hidden/test-failed-1.png', '.test-failed-3.png']) writeFileSync(join(test, name), name);
  symlinkSync(${JSON.stringify(join(outside, 'test-failed-1.png'))}, join(test, 'test-failed-2.png'));
  symlinkSync(${JSON.stringify(outside)}, join(dir, 'linked'));
};
`,
    );
    run('app', surface({ all: true, files: [] }), { FAKE_OUTPUT: writer });
    const listed = spawnSync('find', ['.', '-mindepth', '1', '-printf', '%y %P\\n'], { cwd: join(out, 'app-screenshots'), encoding: 'utf8' });
    expect(listed.stdout.trim().split('\n').sort()).toEqual(['d store-buy-journeys', 'f store-buy-journeys/test-failed-1.png']);
    expect(readFileSync(join(out, 'app-screenshots', 'store-buy-journeys', 'test-failed-1.png'), 'utf8')).toBe('test-failed-1.png');
    expect(existsSync(join(root, 'apps/app/test-results/old'))).toBe(false);
  });

  it('runs nothing when the plan selects no journey for the surface, emptying its screenshots folder so nothing stale is uploaded (head16), or when its journeys folder holds no spec yet', () => {
    mkdirSync(join(out, 'admin-screenshots'));
    writeFileSync(join(out, 'admin-screenshots', 'stale.png'), 'stale');
    const none = run('admin', surface({ all: false, files: [] }));
    expect.soft(none.status, 'none selected').toBe(0);
    expect.soft(existsSync(join(out, 'call.json')), 'none selected').toBe(false);
    expect.soft(none.written, 'none selected').toEqual({ ran: 0, findings: [], infrastructure: [], note: 'no journeys selected' });
    expect.soft(readdirSync(join(out, 'admin-screenshots')), 'none selected').toEqual([]);

    nextRow();
    rmSync(join(root, 'apps/app/e2e/journeys/store.spec.ts'));
    const unwritten = run('app', surface({ all: true, files: [] }));
    expect.soft(unwritten.status, 'no spec yet').toBe(0);
    expect.soft(existsSync(join(out, 'call.json')), 'no spec yet').toBe(false);
    expect.soft(unwritten.written, 'no spec yet').toEqual({ ran: 0, findings: [], infrastructure: [], note: 'no journeys yet' });
  });

  it('says the run itself failed when Playwright wrote no report', () => {
    const { status, written } = run('admin', surface({ all: true, files: [] }), { FAKE_REPORT: 'none' });
    expect(status).toBe(1);
    expect(written).toEqual({ ran: 0, findings: [], infrastructure: ['the journeys did not report (exit 1)'] });
  });

  it('refuses a surface it does not know', () => {
    const result = spawnSync('bash', [SCRIPT, 'edge'], { cwd: root, env: { ...childEnv(), OUT: out }, encoding: 'utf8' });
    expect(result.status).toBe(2);
    expect(result.stderr).toContain('usage');
  });
});
