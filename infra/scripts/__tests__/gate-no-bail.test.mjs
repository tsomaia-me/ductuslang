import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { GIT_ENV } from './harness-fixture.mjs';
import { childEnv } from './hermetic.mjs';

const GATE = fileURLToPath(new URL('../gate.sh', import.meta.url));
const CHECK_TOUCHED = fileURLToPath(new URL('../check-touched.mjs', import.meta.url));
const { packageManager } = JSON.parse(readFileSync(fileURLToPath(new URL('../../../package.json', import.meta.url)), 'utf8'));
const PASSED_MARK = 'tests of packages/passing ran';
const FAILED_MARK = 'tests of packages/failing ran';
const PASSED_TYPECHECK_MARK = 'typecheck of packages/passing ran';
const FAILED_TYPECHECK_MARK = 'typecheck of packages/failing ran';
const LINES = [1, 2, 3];
const testScript = (name, exit) => `for line in ${LINES.join(' ')}; do echo "${name} line $line"; sleep 0.2; done && echo "tests of packages/${name} ran" && exit ${exit}`;
const block = (output, name, exit) => new RegExp(`^check: test packages/${name}: pnpm run test\\n((?:(?!check: ).*\\n)*)check: test packages/${name} exit ${exit}$`, 'm').exec(output)?.[1] ?? '';

describe('gate.sh --no-bail on the full lane runs every package past a failure and still fails', { timeout: 120_000 }, () => {
  let root;
  let repo;
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: repo, encoding: 'utf8', env: { ...childEnv(), ...GIT_ENV } });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr}`);
    return result.stdout.trim();
  };
  const write = (path, text) => {
    mkdirSync(join(repo, dirname(path)), { recursive: true });
    writeFileSync(join(repo, path), text);
  };

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'gate-no-bail-'));
    repo = join(root, 'tested');
    mkdirSync(repo);
    git('init', '-q', '-b', 'main');
    write('package.json', JSON.stringify({ name: 'fixture', private: true, packageManager, scripts: { lint: 'test -z "$FAIL_LINT"', 'i18n:check': 'true' } }));
    write('pnpm-workspace.yaml', "packages:\n  - 'packages/*'\n");
    write('.gitignore', 'node_modules\n');
    write('packages/failing/package.json', JSON.stringify({ name: 'failing', version: '0.0.0', scripts: { test: testScript('failing', 1), typecheck: `echo "${FAILED_TYPECHECK_MARK}" && test -z "$FAIL_TYPECHECK"` } }));
    write('packages/passing/package.json', JSON.stringify({ name: 'passing', version: '0.0.0', scripts: { test: testScript('passing', 0), typecheck: `echo "${PASSED_TYPECHECK_MARK}"` } }));
    write('infra/scripts/gate.sh', readFileSync(GATE, 'utf8'));
    write('infra/scripts/check-touched.mjs', readFileSync(CHECK_TOUCHED, 'utf8'));
    write('infra/scripts/strip-comments.mjs', '\n');
    write('infra/scripts/__tests__/none.txt', '\n');
    write('bin/oxlint', '#!/bin/sh\nexit 0\n');
    chmodSync(join(repo, 'bin/oxlint'), 0o755);
    git('add', '-A');
    git('commit', '-q', '-m', 'seed');
    git('update-ref', 'refs/remotes/origin/main', 'main');
    git('checkout', '-q', '-b', 'change');
    write('packages/failing/notes.txt', 'change\n');
    write('packages/passing/notes.txt', 'change\n');
    git('add', '-A');
    git('commit', '-q', '-m', 'change');
  }, 120_000);
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const gate = (args, env = {}) => {
    const result = spawnSync('bash', ['infra/scripts/gate.sh', '--base', 'main', ...args], {
      cwd: repo,
      encoding: 'utf8',
      env: childEnv({ ...GIT_ENV, PATH: `${join(repo, 'bin')}:${process.env.PATH}`, TEST_MAX_WORKERS: '1', ...env }),
    });
    return { status: result.status, output: result.stdout + result.stderr };
  };

  it('a failing package no longer hides the next one: both run, both results are listed, and the gate fails', () => {
    const run = gate(['--full', '--no-bail']);
    expect(run.status, run.output).toBe(1);
    expect(run.output).toContain(FAILED_MARK);
    expect(run.output).toContain(PASSED_MARK);
    expect(run.output).toMatch(/^gate: test packages\/failing failure$/m);
    expect(run.output).toMatch(/^gate: test packages\/passing passed$/m);
    expect(run.output).toMatch(/^gate: RED \(failed: test\)$/m);
    expect(run.output).not.toContain('gate: GREEN');
    expect(existsSync(join(repo, 'pnpm-exec-summary.json'))).toBe(false);
  });

  it('a failing step before the tests still lets the tests run, and the gate names it', () => {
    const run = gate(['--full', '--no-bail'], { FAIL_LINT: '1' });
    expect(run.status, run.output).toBe(1);
    expect(run.output).toContain(PASSED_MARK);
    expect(run.output).toMatch(/^gate: RED \(failed: lint test\)$/m);
  });

  it('a package whose typecheck fails no longer hides the next one: both run, both results are listed, and the gate fails', () => {
    const run = gate(['--full', '--no-bail'], { FAIL_TYPECHECK: '1' });
    expect(run.status, run.output).toBe(1);
    expect(run.output).toContain(FAILED_TYPECHECK_MARK);
    expect(run.output).toContain(PASSED_TYPECHECK_MARK);
    expect(run.output).toMatch(/^gate: typecheck packages\/failing failure$/m);
    expect(run.output).toMatch(/^gate: typecheck packages\/passing passed$/m);
    expect(run.output).toMatch(/^gate: RED \(failed: typecheck test\)$/m);
  });

  it('without the flag the gate fails on the failing package and prints neither the per-package results nor the RED summary', () => {
    const run = gate(['--full']);
    expect(run.status, run.output).not.toBe(0);
    expect(run.output).toContain(FAILED_MARK);
    expect(run.output).toMatch(/failing@0\.0\.0 test/);
    expect(run.output).not.toMatch(/^gate: test packages\//m);
    expect(run.output).not.toMatch(/^gate: RED/m);
  });

  it('given TEST_PACKAGES_AT_ONCE, the full lane runs every package, prints each one whole and unprefixed, and still fails naming the failed one', () => {
    const run = gate(['--full', '--no-bail'], { TEST_PACKAGES_AT_ONCE: '2' });
    expect(run.status, run.output).toBe(1);
    const lines = run.output.split('\n');
    for (const name of ['failing', 'passing']) {
      const first = lines.indexOf(`${name} line 1`);
      expect(first, run.output).toBeGreaterThan(-1);
      expect(lines.slice(first, first + LINES.length)).toEqual(LINES.map((line) => `${name} line ${line}`));
    }
    expect(run.output).toMatch(/^gate: test packages\/failing failure$/m);
    expect(run.output).toMatch(/^gate: test packages\/passing passed$/m);
    expect(run.output).toMatch(/^gate: RED \(failed: test\)$/m);
  });

  it('given TEST_PACKAGES_AT_ONCE, the code lane runs every reached package that many at once, prints each one whole, and still fails naming the failed one', () => {
    const run = gate(['--no-bail'], { TEST_PACKAGES_AT_ONCE: '2' });
    expect(run.status, run.output).toBe(1);
    expect(run.output).toMatch(/, 2 at once$/m);
    expect(run.output).toMatch(/^check: test packages\/failing started$/m);
    expect(run.output).toMatch(/^check: test packages\/passing started$/m);
    for (const [name, exit] of [
      ['failing', 1],
      ['passing', 0],
    ]) {
      const output = block(run.output, name, exit);
      expect(output.split('\n').filter((line) => / line \d$/.test(line))).toEqual(LINES.map((line) => `${name} line ${line}`));
      expect(output).toContain(`tests of packages/${name} ran`);
    }
    expect(run.output).toMatch(/^check: failed: test packages\/failing$/m);
    expect(run.output).toMatch(/^gate: RED \(failed: test\)$/m);
  });
});
