/**
 * The proof that `deploy/api` can deploy from Workers Builds without touching production (CD-177):
 * the checkout Workers Builds makes is reproduced — a SHALLOW clone of `deploy/api`, whose tip is a
 * promotion merge (tree of the verified commit, that commit as its second parent), with the
 * WORKERS_CI variables and the Worker-name override Workers Builds sets — and the clone's own
 * `infra/scripts/edge-deploy.sh` is run the way the deploy command runs it, as a dry run: the real
 * pnpm installs `@wg/edge...` and `@wg/admin...`, the real packages and the admin app build, and the real wrangler bundles the three Workers.
 * The gate is not re-checked in Workers Builds (owner, 5 Oct, CD-215), so no mark is asked for. Only
 * the repository is a fixture: the files the Workers build from, and every workspace manifest (the
 * frozen lockfile needs them all).
 */
import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { childEnv } from './hermetic.mjs';

const REPO = fileURLToPath(new URL('../../../', import.meta.url));
const ROOT_FILES = ['package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml', '.npmrc', '.gitignore', 'tsconfig.base.json'];

let root;
let origin;
let clone;
let verified;
let tip;
let envLog;
let result;

const run = (cwd, ...args) => {
  const out = spawnSync('git', ['-C', cwd, '-c', 'user.email=t@t', '-c', 'user.name=t', ...args], { encoding: 'utf8' });
  if (out.status !== 0) throw new Error(`git ${args.join(' ')}: ${out.stderr}`);
  return out.stdout.trim();
};

const fixtureFiles = () => {
  const listed = spawnSync('git', ['-C', REPO, 'ls-files', '-z', '--', ...ROOT_FILES, 'packages', 'apps/edge', 'apps/admin', 'infra', 'apps/*/package.json'], { encoding: 'utf8' }).stdout;
  return listed.split('\0').filter(Boolean);
};

beforeAll(() => {
  root = mkdtempSync(join(tmpdir(), 'edge-workers-builds-'));
  const work = join(root, 'work');
  origin = join(root, 'origin.git');
  clone = join(root, 'clone');
  envLog = join(root, 'wrangler-env.log');
  spawnSync('git', ['init', '-q', '--bare', origin]);
  mkdirSync(work);
  run(work, 'init', '-q', '-b', 'main');
  for (const file of fixtureFiles()) {
    mkdirSync(dirname(join(work, file)), { recursive: true });
    copyFileSync(join(REPO, file), join(work, file));
  }
  run(work, 'add', '-A');
  run(work, 'commit', '-q', '-m', 'the verified head');
  verified = run(work, 'rev-parse', 'HEAD');
  run(work, 'checkout', '-q', '--orphan', 'old-deploy');
  run(work, 'rm', '-rq', '--cached', '.');
  run(work, 'commit', '-q', '--allow-empty', '-m', 'the old deploy/api tip');
  const oldTip = run(work, 'rev-parse', 'HEAD');
  tip = run(work, 'commit-tree', `${verified}^{tree}`, '-p', oldTip, '-p', verified, '-m', `promote(api): ${verified.slice(0, 7)} → deploy/api`);
  run(work, 'remote', 'add', 'origin', origin);
  run(work, 'push', '-q', 'origin', `${verified}:refs/heads/main`,`${tip}:refs/heads/deploy/api`);

  const cloned = spawnSync('git', ['clone', '-q', '--depth=1', '--branch', 'deploy/api', `file://${origin}`, clone], { encoding: 'utf8' });
  if (cloned.status !== 0) throw new Error(cloned.stderr);

  const wrangler = join(root, 'wrangler');
  writeFileSync(
    wrangler,
    `#!/usr/bin/env bash\nprintf '%s|name=%s|tag=%s\\n' "$*" "\${WRANGLER_CI_OVERRIDE_NAME:-}" "\${WRANGLER_CI_MATCH_TAG:-}" >>"${envLog}"\nexec npx wrangler "$@"\n`,
  );
  chmodSync(wrangler, 0o755);

  const env = childEnv();
  for (const name of Object.keys(env)) {
    if (/^(WORKERS_CI|WRANGLER_CI_|EDGE_DEPLOY_|EDGE_DIR$|EDGE_REPO_ROOT$|BUILD_SHA$|CLOUDFLARE_API_TOKEN$)/.test(name)) delete env[name];
  }
  result = spawnSync('bash', ['infra/scripts/edge-deploy.sh', '--env', 'production', '--dry-run', '--gate'], {
    cwd: clone,
    encoding: 'utf8',
    timeout: 280_000,
    env: {
      ...env,
      CI: 'true',
      WORKERS_CI: '1',
      WORKERS_CI_BUILD_UUID: 'proof-build',
      WORKERS_CI_COMMIT_SHA: tip,
      WORKERS_CI_BRANCH: 'deploy/api',
      WRANGLER_CI_OVERRIDE_NAME: 'cd-api',
      WRANGLER_CI_MATCH_TAG: 'tag-of-cd-api',
      WRANGLER: wrangler,
    },
  });
}, 300_000);

afterAll(() => rmSync(root, { recursive: true, force: true }), 120_000);

describe('edge-deploy.sh in a reproduced Workers Builds checkout (CD-177)', () => {
  it('deploys the shallow clone of deploy/api as pushed, without re-checking the gate, and bundles the three Workers stamped with the verified commit', () => {
    expect(spawnSync('git', ['-C', clone, 'rev-parse', '--is-shallow-repository'], { encoding: 'utf8' }).stdout.trim()).toBe('true');
    expect(result.status, `${result.stdout}\n${result.stderr}`).toBe(0);
    expect(result.stdout).toContain(`deploy/api at ${tip.slice(0, 7)}, shallow clone: true`);
    expect(result.stdout).toContain(`BUILD_SHA ${verified.slice(0, 7)}, dry run`);
    expect(result.stdout).toContain(`${verified.slice(0, 7)} passed checks and gate before that push, and the gate is not re-checked here`);
    expect(result.stdout).not.toContain('is verified');
    expect(result.stdout).toContain('dry run ok — the three Workers bundle');
    const bundled = readFileSync(envLog, 'utf8').trim().split('\n');
    expect(bundled).toHaveLength(3);
    expect(bundled[0]).toMatch(new RegExp(`^deploy -c wrangler\\.auth\\.jsonc --env production --var BUILD_SHA:${verified} --dry-run --outdir \\S+\\|name=\\|tag=$`));
    expect(bundled[2]).toMatch(new RegExp(`^deploy -c wrangler\\.api\\.jsonc --env production --var BUILD_SHA:${verified} --dry-run --outdir \\S+\\|name=\\|tag=tag-of-cd-api$`));
    expect(bundled[1]).toMatch(new RegExp(`^deploy -c wrangler\\.admin\\.jsonc --env production --var BUILD_SHA:${verified} --dry-run --outdir \\S+\\|name=\\|tag=$`));
  });
});
