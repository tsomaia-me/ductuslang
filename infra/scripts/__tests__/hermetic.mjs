/**
 * What a script under test may inherit from the run that tests it (CD-196).
 *
 * `pnpm run test` exports its own state to every script it starts: `NODE` and
 * `npm_node_execpath` name pnpm's OWN executable, which on the wb runners is the
 * standalone `@pnpm/exe` and not a node at all, and `npm_config_*` carry its
 * configuration into any `pnpm` the script calls. A script that read `NODE`
 * answered differently under `pnpm run test` than under `sh infra-test.sh`, and
 * when that executable was half-written by a parallel job, an unmarked commit
 * passed the gate. `childEnv` is the environment with all of that, and every
 * GitHub token a gate could ask with, taken out; the test adds what it means.
 *
 * `assertStubsRun` is the other half: the stubs live under TMPDIR, and on a
 * `noexec` TMPDIR (wb's `/tmp`) bash skips a stub it cannot execute and runs the
 * REAL `git` next on PATH — in the real checkout, against its real origin, where
 * a promotion test pushes. A run that cannot execute its stubs stops here.
 */
import { spawnSync } from 'node:child_process';
import { rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

/** 40 hex digits no repository of ours has: a fixture never resolves upstream. */
export const FIXTURE_SHA = 'c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00';

export const GITHUB_TOKENS = ['GITHUB_TOKEN', 'GH_TOKEN', 'WB_GATE_TOKEN', 'EDGE_DEPLOY_GIT_TOKEN'];

const PACKAGE_MANAGER = /^npm_/i;
const INJECTED = new Set(['NODE', 'WB_NODE', 'INIT_CWD', 'PNPM_SCRIPT_SRC_DIR', 'TEST_MAX_WORKERS', 'TEST_PACKAGES_AT_ONCE', 'VITEST_MAX_WORKERS', ...GITHUB_TOKENS]);

export const inherited = (name) => !PACKAGE_MANAGER.test(name) && !INJECTED.has(name);

export const childEnv = (overrides = {}) => ({
  ...Object.fromEntries(Object.entries(process.env).filter(([name]) => inherited(name))),
  ...overrides,
});

export const assertStubsRun = (bin) => {
  const probe = join(bin, '.stub-probe');
  writeFileSync(probe, '#!/bin/sh\nexit 0\n', { mode: 0o755 });
  const ran = spawnSync(probe);
  rmSync(probe, { force: true });
  if (ran.error || ran.status !== 0) {
    throw new Error(
      `the stubs in ${bin} cannot be executed (${ran.error?.code ?? `exit ${ran.status}`}), so the script under test would run the real binaries next on PATH. TMPDIR is on a noexec mount: point it at an executable folder.`,
    );
  }
};
