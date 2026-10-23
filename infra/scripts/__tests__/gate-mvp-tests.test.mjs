import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { GIT_ENV } from './harness-fixture.mjs';
import { assertStubsRun, childEnv } from './hermetic.mjs';

const GATE = fileURLToPath(new URL('../gate.sh', import.meta.url));
const CHECK_TOUCHED = fileURLToPath(new URL('../check-touched.mjs', import.meta.url));

const manifest = (name, scripts = {}) => `${JSON.stringify({ name, scripts: { test: 'vitest run', ...scripts }, devDependencies: { vitest: '^5.0.0' } })}\n`;

const BRANCHES = {
  core: ['packages/game-core/src/index.ts'],
  both: ['packages/game-core/src/index.ts', 'packages/game-wordgrid/src/index.ts'],
  workspace: ['pnpm-workspace.yaml'],
  workspacecore: ['pnpm-workspace.yaml', 'packages/game-core/src/index.ts'],
  lintconfig: ['.oxlintrc.json', 'packages/game-core/src/index.ts'],
};

const STRIP_STUB = "import { appendFileSync } from 'node:fs';\nappendFileSync(process.env.PNPM_LOG, `${process.cwd()}: strip-comments ${process.argv.slice(2).join(' ')}\\n`);\n";
const PNPM_STUB = `#!/bin/sh
printf "%s: %s\\n" "$(pwd -P)" "$*" >>"$PNPM_LOG"
case "$*" in
  "ls "*"}^..."*) printf "%s\\n" "$ROOT/packages/game-core" ;;
  "ls "*) printf "%s\\n" "$ROOT/packages/game-core" "$ROOT/packages/game-wordgrid" ;;
esac
`;

describe('gate.sh tests only what the change reaches, and whole what the selection cannot prove (CD-478)', { timeout: 120_000 }, () => {
  let root;
  let repo;
  let bin;
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
    root = mkdtempSync(join(tmpdir(), 'gate-mvp-tests-'));
    repo = join(root, 'tested');
    bin = join(root, 'bin');
    mkdirSync(repo);
    mkdirSync(bin);
    git('init', '-q', '-b', 'main');
    repo = spawnSync('pwd', ['-P'], { cwd: repo, encoding: 'utf8' }).stdout.trim();
    write('package.json', '{}\n');
    write('infra/scripts/gate.sh', readFileSync(GATE, 'utf8'));
    write('infra/scripts/check-touched.mjs', readFileSync(CHECK_TOUCHED, 'utf8'));
    write('infra/scripts/__tests__/none.txt', '\n');
    write('infra/scripts/strip-comments.mjs', STRIP_STUB);
    write('packages/game-core/package.json', manifest('@wg/game-core', { build: 'tsc' }));
    write('packages/game-wordgrid/package.json', manifest('@wg/game-wordgrid'));
    git('add', '-A');
    git('commit', '-q', '-m', 'seed');
    for (const [name, paths] of Object.entries(BRANCHES)) {
      git('checkout', '-q', '-b', name, 'main');
      for (const path of paths) write(path, `${name}\n`);
      git('add', '-A');
      git('commit', '-q', '-m', name);
    }
    git('checkout', '-q', 'main');
    git('update-ref', 'refs/remotes/origin/main', 'main');
    writeFileSync(join(bin, 'pnpm'), PNPM_STUB, { mode: 0o755 });
    assertStubsRun(bin);
  }, 120_000);
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const run = (branch, ...args) => {
    git('checkout', '-q', branch);
    const log = join(root, `${branch}.log`);
    writeFileSync(log, '');
    const result = spawnSync('bash', ['infra/scripts/gate.sh', '--base', 'main', ...args], {
      cwd: repo,
      encoding: 'utf8',
      env: childEnv({ ...GIT_ENV, PATH: `${bin}:${process.env.PATH}`, PNPM_LOG: log, ROOT: repo }),
    });
    expect(result.status, result.stdout + result.stderr).toBe(0);
    return { stdout: result.stdout, steps: readFileSync(log, 'utf8').split('\n').filter(Boolean) };
  };
  const gate = (...args) => run(...args).steps;
  const ran = (steps, dir, args) => steps.includes(`${join(repo, dir)}: ${args}`);
  const testRuns = (steps) => steps.filter((step) => / run test$|vitest run --changed/.test(step));

  it('runs the changed package and its dependent by --changed, since the dependent imports the workspace sources, lints the touched packages whole and builds only what the reached packages depend on', () => {
    const steps = gate('core');
    expect.soft(testRuns(steps)).toHaveLength(2);
    for (const dir of ['packages/game-core', 'packages/game-wordgrid']) expect.soft(testRuns(steps).find((step) => step.startsWith(join(repo, dir))), dir).toContain('vitest run --changed');
    expect.soft(ran(steps, '.', 'exec oxlint packages/game-core')).toBe(true);
    expect.soft(ran(steps, '.', 'strip-comments --check --type ts,tsx packages/game-core')).toBe(true);
    expect.soft(ran(steps, '.', 'run lint')).toBe(false);
    expect.soft(ran(steps, '.', '-r --if-present --filter {./packages/game-core}^... --filter {./packages/game-wordgrid}^... run build')).toBe(true);
    expect.soft(ran(steps, '.', '-r --if-present --filter ...{./packages/game-core} run typecheck')).toBe(true);
  });

  it('runs a changed package downstream of another changed package by --changed as well', () => {
    const steps = gate('both');
    expect(ran(steps, 'packages/game-wordgrid', 'run test')).toBe(false);
    expect(testRuns(steps)).toHaveLength(2);
  });

  it('lints everything when the lint config changed', () => {
    const steps = gate('lintconfig');
    expect(ran(steps, '.', 'run lint')).toBe(true);
    expect(steps.some((step) => step.includes('exec oxlint'))).toBe(false);
  });

  it('names in the plan what it lints and builds, and the packages owning a changed file on lane full too, for check:touched --direct', () => {
    expect.soft(run('core', '--plan').stdout.split('\n'), 'code').toEqual(expect.arrayContaining(['lane=code', 'reach=packages/game-core packages/game-wordgrid', 'touched=packages/game-core', 'lint=packages/game-core', 'build=packages/game-core']));
    expect.soft(run('workspacecore', '--plan').stdout.split('\n'), 'full, a package touched').toEqual(expect.arrayContaining(['lane=full', 'touched=packages/game-core']));
    expect.soft(run('workspace', '--plan').stdout.split('\n'), 'full, none touched').toEqual(expect.arrayContaining(['lane=full', 'touched=']));
  });

  it('runs every package whole when the reach is unknown', () => {
    expect(testRuns(gate('workspace'))).toEqual([`${repo}: -r --if-present run test`]);
    expect(testRuns(gate('workspace', '--full'))).toEqual([`${repo}: -r --if-present run test`]);
  });
});
