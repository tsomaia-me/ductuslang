import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { GIT_ENV } from './harness-fixture.mjs';
import { assertStubsRun, childEnv } from './hermetic.mjs';

const GATE = fileURLToPath(new URL('../gate.sh', import.meta.url));
const FOLDERS = 750;
const LONG = 'z'.repeat(245);
const many = (prefix, suffix) => Array.from({ length: FOLDERS }, (_, index) => `${prefix}${LONG}${index}${suffix}`);

const BRANCHES = {
  apps: ['apps/admin/x.txt', ...many('apps/', '/x.txt')],
  packages: ['packages/game-wordgrid/x.txt', ...many('packages/', '/x.txt')],
};

describe('gate.sh picks every step its changed folders call for, however long the list (CD-362)', { timeout: 120_000 }, () => {
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
    root = mkdtempSync(join(tmpdir(), 'gate-step-choice-'));
    repo = join(root, 'tested');
    bin = join(root, 'bin');
    mkdirSync(repo);
    mkdirSync(bin);
    git('init', '-q', '-b', 'main');
    write('package.json', '{}\n');
    write('infra/scripts/gate.sh', readFileSync(GATE, 'utf8'));
    write('infra/scripts/strip-comments.mjs', '\n');
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
    writeFileSync(join(bin, 'pnpm'), '#!/bin/sh\nprintf "%s\\n" "$*" >>"$PNPM_LOG"\n', { mode: 0o755 });
    assertStubsRun(bin);
  }, 120_000);
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const gate = (branch) => {
    git('checkout', '-q', branch);
    const log = join(root, `${branch}.log`);
    writeFileSync(log, '');
    const result = spawnSync('bash', ['infra/scripts/gate.sh', '--base', 'main'], {
      cwd: repo,
      encoding: 'utf8',
      env: childEnv({ ...GIT_ENV, PATH: `${bin}:${process.env.PATH}`, PNPM_LOG: log }),
    });
    expect(result.status, result.stdout + result.stderr).toBe(0);
    return readFileSync(log, 'utf8').split('\n').filter(Boolean);
  };

  it('an app among a long list of changed folders still runs the i18n check and the journeys map, and the wordgrid engine still runs the balance suite', () => {
    for (const [branch, wanted] of [
      ['apps', ['run i18n:check', '--filter @wg/infra run test verify/__tests__/journeys-map.test.mjs']],
      ['packages', ['--filter @wg/game-wordgrid run test:balance']],
    ]) {
      const steps = gate(branch);
      for (const step of wanted) expect.soft(steps, branch).toContain(step);
    }
  });
});
