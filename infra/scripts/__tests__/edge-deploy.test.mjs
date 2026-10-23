import { spawnSync } from 'node:child_process';
import { chmodSync, cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';

const SCRIPT = fileURLToPath(new URL('../edge-deploy.sh', import.meta.url));
const REAL_EDGE = fileURLToPath(new URL('../../../apps/edge', import.meta.url));
let SHA;
let base;
let template;
let root;
let edge;
let origin;
let log;
let envLog;
let repoRoot;

const stub = (name, body) => {
  const file = join(root, 'bin', name);
  writeFileSync(file, `#!/usr/bin/env bash\n${body}\n`);
  chmodSync(file, 0o755);
  return file;
};

const git = (...args) => spawnSync('git', ['-C', edge, ...args], { encoding: 'utf8' }).stdout.trim();
const g = (...args) => spawnSync('git', ['-C', edge, '-c', 'user.email=t@t', '-c', 'user.name=t', ...args], { encoding: 'utf8' });

const verifiedCheckout = () => {
  origin = join(root, 'origin.git');
  spawnSync('git', ['init', '-q', '--bare', origin]);
  g('init', '-q', '-b', 'main');
  writeFileSync(join(edge, '.gitignore'), '.wrangler/\n');
  g('add', '.');
  g('commit', '-q', '-m', 'verified head');
  g('remote', 'add', 'origin', origin);
  const head = git('rev-parse', 'HEAD');
  g('push', '-q', 'origin', 'main');
  return head;
};

beforeAll(() => {
  base = mkdtempSync(join(tmpdir(), 'edge-deploy-'));
  root = join(base, 'run');
  mkdirSync(join(root, 'bin'), { recursive: true });
  edge = join(root, 'edge');
  mkdirSync(join(edge, 'scripts'), { recursive: true });
  cpSync(join(REAL_EDGE, 'scripts/d1-migrate.mjs'), join(edge, 'scripts/d1-migrate.mjs'));
  cpSync(join(REAL_EDGE, 'wrangler.api.jsonc'), join(edge, 'wrangler.api.jsonc'));
  cpSync(join(REAL_EDGE, 'wrangler.auth.jsonc'), join(edge, 'wrangler.auth.jsonc'));
  cpSync(join(REAL_EDGE, 'wrangler.admin.jsonc'), join(edge, 'wrangler.admin.jsonc'));
  log = join(root, 'calls.log');
  envLog = join(root, 'env.log');
  const seen = (name) => `printf '${name} %s|name=%s|tag=%s|alias=%s|token=%s\\n' "$*" "\${WRANGLER_CI_OVERRIDE_NAME:-}" "\${WRANGLER_CI_MATCH_TAG:-}" "\${WRANGLER_CI_GENERATE_PREVIEW_ALIAS:-}" "\${CLOUDFLARE_API_TOKEN:-}" >>"${envLog}"`;
  stub('wrangler', `printf 'wrangler %s\\n' "$*" >>"${log}"\n${seen('wrangler')}\n[ -n "\${WRANGLER_FAIL:-}" ] && case "$*" in *"$WRANGLER_FAIL"*) exit 7 ;; esac\nexit 0`);
  stub('pnpm', `[ "$1" = --version ] && { echo "\${PNPM_HAS:-10.33.0}"; exit 0; }\nprintf 'pnpm %s\\n' "$*" >>"${log}"\n${seen('pnpm')}\nexit 0`);
  stub('corepack', `case "$*" in "pnpm@10.33.0 --version") [ -n "\${COREPACK_FAILS:-}" ] && exit 1; echo 10.33.0; exit 0 ;; esac\nprintf 'corepack %s\\n' "$*" >>"${log}"\nexit 0`);
  stub('live-vars', `printf 'live-vars %s\\n' "$*" >>"${log}"\n${seen('live-vars')}\nexit "\${LIVE_VARS_EXIT:-0}"`);
  stub('transfer-guard', `printf 'transfer-guard %s\\n' "$*" >>"${log}"\n${seen('transfer-guard')}\nexit "\${TRANSFER_GUARD_EXIT:-0}"`);
  repoRoot = join(root, 'repo-root');
  mkdirSync(repoRoot);
  writeFileSync(join(repoRoot, 'package.json'), JSON.stringify({ packageManager: 'pnpm@10.33.0', engines: { node: '>=22.0.0' } }));
  // The gate's statuses on GitHub (CD-179) pass unless a test hands in its own `node`.
  stub(
    'gate-pass',
    `case "$*" in *gate-status.mjs*) echo "checks/tree: tree x passed on 9f9f9f9"; echo "gate/tree: tree x passed on 9f9f9f9 (https://github.test/run/0)" ;; *) exec node "$@" ;; esac`,
  );
  SHA = verifiedCheckout();
  template = join(base, 'plain');
  renameSync(root, template);
});

beforeEach(() => cpSync(template, root, { recursive: true }));

afterEach(() => rmSync(root, { recursive: true, force: true }));

afterAll(() => rmSync(base, { recursive: true, force: true }));

const REPO = fileURLToPath(new URL('../../../', import.meta.url));

const run = (args, env = {}, { script = SCRIPT, cwd } = {}) =>
  spawnSync('bash', [script, ...args], {
    encoding: 'utf8',
    cwd,
    env: {
      PATH: process.env.PATH,
      HOME: root,
      GIT_CONFIG_NOSYSTEM: '1',
      EDGE_DIR: edge,
      WRANGLER: join(root, 'bin', 'wrangler'),
      PNPM: join(root, 'bin', 'pnpm'),
      COREPACK: join(root, 'bin', 'corepack'),
      LIVE_VARS: join(root, 'bin', 'live-vars'),
      TRANSFER_GUARD: join(root, 'bin', 'transfer-guard'),
      WB_NODE: join(root, 'bin', 'gate-pass'),
      EDGE_REPO_ROOT: repoRoot,
      ...env,
    },
  });

const calls = () => readFileSync(log, 'utf8').trim().split('\n');

const CLASS_REFUSED = 'Cannot create binding for class RuleLab in script that does not export it. [code: 10061]';
const ADMIN_BOOTSTRAP_STEP = 'edge-deploy: the admin Worker binds a Durable Object class the api Worker does not export yet (code 10061): deploying api first, then admin';

const adminRefusedUntilApi = () =>
  stub(
    'wrangler',
    `printf 'wrangler %s\\n' "$*" >>"${log}"\nprintf 'wrangler %s|tag=%s\\n' "$*" "\${WRANGLER_CI_MATCH_TAG:-}" >>"${envLog}"\ncase "$*" in *"deploy -c wrangler.admin.jsonc"*) [ -f "${root}/api-exists" ] || { echo '${CLASS_REFUSED}'; exit 1; } ;; esac\ncase "$*" in *"deploy -c wrangler.api.jsonc"*) touch "${root}/api-exists" ;; esac\nexit 0`,
  );

const fresh = () => {
  rmSync(root, { recursive: true, force: true });
  cpSync(template, root, { recursive: true });
};

const says = (output, expected, label) => {
  for (const text of expected) {
    if (text instanceof RegExp) expect.soft(output, label).toMatch(text);
    else expect.soft(output, label).toContain(text);
  }
};

describe('edge-deploy.sh (§24.9)', () => {
  it('bootstraps the first deploy of an environment: auth without its MailRouter binding, api, then auth in full', () => {
    stub('wrangler', `printf 'wrangler %s\\n' "$*" >>"${log}"\ncase "$*" in *"deploy -c wrangler.auth.jsonc"*) [ -f "${root}/api-exists" ] || { echo 'Cannot create binding for class in script that does not exist. [code: 10061]'; exit 1; } ;; esac\ncase "$*" in *"deploy -c wrangler.api.jsonc"*) case "$*" in *--dry-run*) ;; *) touch "${root}/api-exists" ;; esac ;; esac\nexit 0`);
    const result = run(['--env', 'production', '--sha', SHA]);
    expect(result.status, result.stderr).toBe(0);
    const deploys = calls().filter((line) => line.startsWith('wrangler deploy') && !line.includes('--dry-run'));
    expect(deploys).toEqual([
      `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c .wrangler/auth-bootstrap.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA}`,
    ]);
    const bootstrap = JSON.parse(readFileSync(join(edge, '.wrangler/auth-bootstrap.jsonc'), 'utf8'));
    expect(bootstrap.env.production.durable_objects).toBeUndefined();
    expect(bootstrap.env.production.d1_databases).toBeDefined();
  });

  it('stops at a failed step: a failed migration deploys nothing, a failed auth deploy neither bootstraps nor deploys api', () => {
    const rows = [
      { label: 'migration', failing: 'cd-core', never: (c) => c.startsWith('wrangler deploy') },
      { label: 'auth', failing: 'deploy -c wrangler.auth.jsonc', never: (c) => c.includes('auth-bootstrap') || c.startsWith('wrangler deploy -c wrangler.api.jsonc') },
    ];
    rows.forEach((row, i) => {
      if (i) fresh();
      const result = run(['--env', 'production', '--sha', SHA], { WRANGLER_FAIL: row.failing });
      expect.soft(result.status, row.label).not.toBe(0);
      expect.soft(calls().some(row.never), row.label).toBe(false);
    });
  });

  it('builds the packages and the admin app, migrates every database of the environment, then deploys auth, admin and api, each with BUILD_SHA (CD-291)', () => {
    const result = run(['--env', 'production', '--sha', SHA]);
    expect(result.status, result.stderr).toBe(0);
    expect(calls()).toEqual([
      'pnpm --filter @wg/edge^... run build',
      'pnpm --filter @wg/admin run build:release',
      `live-vars --env production --dir ${edge}`,
      `wrangler d1 migrations apply cd-accounts --remote -c ${join(edge, 'wrangler.api.jsonc')} --env production`,
      `wrangler d1 migrations apply cd-core --remote -c ${join(edge, 'wrangler.api.jsonc')} --env production`,
      `wrangler d1 migrations apply cd-admin --remote -c ${join(edge, 'wrangler.admin.jsonc')} --env production`,
      `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA}`,
    ]);
  });

  it('deploys the admin before api, so a failed admin leaves api, and its /health, on the old build, with no retry but the 10061 one (CD-291 review)', () => {
    const adminFailed = run(['--env', 'production', '--sha', SHA], { WRANGLER_FAIL: 'deploy -c wrangler.admin.jsonc' });
    expect(adminFailed.status).not.toBe(0);
    expect(adminFailed.stdout).not.toContain(ADMIN_BOOTSTRAP_STEP);
    expect(calls().filter((c) => c.startsWith('wrangler deploy'))).toEqual([
      `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}`,
      `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}`,
    ]);
    rmSync(log);
    expect(run(['--env', 'production', '--sha', SHA], { WRANGLER_FAIL: 'deploy -c wrangler.api.jsonc' }).status).not.toBe(0);
    expect(calls().some((c) => c.startsWith('wrangler deploy -c wrangler.admin.jsonc'))).toBe(true);
  });

  it('when the admin binds a class the api does not export yet (code 10061), deploys api first and the admin again, once each: it names the step, and fails loudly when the admin is refused again or fails otherwise', () => {
    const rows = [
      { label: 'api exports it', wrangler: adminRefusedUntilApi, ok: true, out: [ADMIN_BOOTSTRAP_STEP], err: [] },
      {
        label: 'still refused',
        wrangler: () => stub('wrangler', `printf 'wrangler %s\\n' "$*" >>"${log}"\ncase "$*" in *"deploy -c wrangler.admin.jsonc"*) echo '${CLASS_REFUSED}'; exit 1 ;; esac\nexit 0`),
        ok: false,
        out: [],
        err: ['edge-deploy: the admin deploy is still refused (code 10061) after api deployed: a Durable Object class the admin binds is not exported by the api Worker'],
      },
      {
        label: 'another error',
        wrangler: () =>
          stub(
            'wrangler',
            `printf 'wrangler %s\\n' "$*" >>"${log}"\ncase "$*" in *"deploy -c wrangler.admin.jsonc"*) [ -f "${root}/admin-tried" ] && { echo 'Upload failed. [code: 10021]'; exit 1; }; touch "${root}/admin-tried"; echo '${CLASS_REFUSED}'; exit 1 ;; esac\nexit 0`,
          ),
        ok: false,
        out: [],
        err: ['edge-deploy: the admin deploy failed after api deployed'],
      },
    ];
    rows.forEach((row, i) => {
      if (i) fresh();
      row.wrangler();
      const result = run(['--env', 'production', '--sha', SHA]);
      expect.soft(result.status === 0, `${row.label}: ${result.stderr}`).toBe(row.ok);
      says(result.stdout, row.out, row.label);
      says(result.stderr, row.err, row.label);
      if (!row.ok) expect.soft(result.stdout, row.label).not.toContain('edge-deploy: deployed');
      expect.soft(calls().filter((line) => line.startsWith('wrangler deploy')), row.label).toEqual([
        `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}`,
        `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}`,
        `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA}`,
        `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}`,
      ]);
    });
  });

  it('leaves no temporary deploy log behind, on success and when it stops at a failed deploy', () => {
    const temp = join(root, 'tmp');
    for (const failing of ['', 'deploy -c wrangler.auth.jsonc', 'deploy -c wrangler.admin.jsonc', 'deploy -c wrangler.api.jsonc']) {
      mkdirSync(temp);
      const result = run(['--env', 'production', '--sha', SHA], { TMPDIR: temp, WRANGLER_FAIL: failing });
      expect(result.status === 0, `${failing}: ${result.stderr}`).toBe(failing === '');
      expect(readdirSync(temp), failing).toEqual([]);
      rmSync(temp, { recursive: true });
    }
  });

  it("a dev deploy asks the gate for the checkout's tree, ignores ignored build output, compares the live vars, waits on production's transfer, then migrates and deploys dev's own resources (CD-142, CD-179, CD-280, CD-285)", () => {
    mkdirSync(join(edge, '.wrangler'), { recursive: true });
    writeFileSync(join(edge, '.wrangler', 'state.json'), '{}\n');
    const asked = join(root, 'gate.argv');
    const node = stub(
      'gate-node',
      `case "$*" in *gate-status.mjs*) printf '%s\\n' "$*" >>"${asked}"; echo "checks/tree: tree x passed on ${SHA.slice(0, 7)}"; echo "gate/tree: tree x passed on ${SHA.slice(0, 7)} (https://github.test/run/3)" ;; *) exec node "$@" ;; esac`,
    );
    const result = run(['--env', 'dev', '--sha', SHA], { WB_NODE: node });
    expect(result.status, result.stderr).toBe(0);
    expect.soft(readFileSync(asked, 'utf8'), 'the gate is asked for the tree').toContain(`check --sha ${SHA} --tree ${git('rev-parse', 'HEAD^{tree}')}`);
    expect.soft(result.stdout, 'the gate passed').toContain('is verified (gate/tree');
    expect.soft(readFileSync(SCRIPT, 'utf8'), 'marks are retired').not.toMatch(/verified\/\$|ci-ok|wb_origin_mark|wb_fetch_marks|mark\.sh/);
    expect.soft(calls().filter((c) => c.includes('migrations')), 'dev databases').toEqual([
      `wrangler d1 migrations apply cd-accounts-dev --remote -c ${join(edge, 'wrangler.api.jsonc')} --env dev`,
      `wrangler d1 migrations apply cd-core-dev --remote -c ${join(edge, 'wrangler.api.jsonc')} --env dev`,
      `wrangler d1 migrations apply cd-admin-dev --remote -c ${join(edge, 'wrangler.admin.jsonc')} --env dev`,
    ]);
    expect.soft(calls().filter((c) => c.startsWith('wrangler deploy')).every((c) => c.includes('--env dev')), 'dev deploys').toBe(true);
    const order = calls().map((line) => line.split(' ')[0] + (line.includes('d1 migrations') ? ':migrate' : ''));
    expect.soft(calls(), 'live-vars asked').toContain(`live-vars --env dev --dir ${edge}`);
    expect.soft(calls(), 'transfer-guard asked').toContain(`transfer-guard --dir ${edge}`);
    expect.soft(order.indexOf('live-vars'), 'live-vars before the first deploy').toBeLessThan(order.findIndex((entry) => entry === 'wrangler'));
    expect.soft(order.indexOf('transfer-guard'), 'transfer-guard after live-vars').toBeGreaterThan(order.indexOf('live-vars'));
    expect.soft(order.indexOf('transfer-guard'), 'transfer-guard before migrating').toBeLessThan(order.indexOf('wrangler:migrate'));
  });

  it('a dry run bundles the three Workers and touches nothing remote, needing no gate and asking neither the live vars nor the transfer', () => {
    const result = run(['--env', 'production', '--sha', SHA, '--dry-run']);
    expect.soft(result.status, result.stderr).toBe(0);
    const made = calls();
    expect.soft(made).toContain('pnpm --filter @wg/admin run build');
    expect.soft(made.some((c) => c.includes('--remote') || c.includes('migrations'))).toBe(false);
    expect.soft(made.filter((c) => c.startsWith('wrangler deploy')).map((c) => c.replace(/--outdir \S+/, '--outdir <dir>'))).toEqual([
      `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA} --dry-run --outdir <dir>`,
      `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA} --dry-run --outdir <dir>`,
      `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA} --dry-run --outdir <dir>`,
    ]);
    rmSync(log);
    const node = stub('gate-node', 'case "$*" in *gate-status.mjs*) echo "gate-status: gate/tree: no status names this tree" >&2; exit 1 ;; *) exec node "$@" ;; esac');
    const dev = run(['--env', 'dev', '--sha', SHA, '--dry-run'], { WB_NODE: node, LIVE_VARS_EXIT: '3', TRANSFER_GUARD_EXIT: '3' });
    expect.soft(dev.status, `dev: ${dev.stderr}`).toBe(0);
    expect.soft(calls().some((line) => line.startsWith('live-vars') || line.startsWith('transfer-guard')), 'dev asks no guard').toBe(false);
  });

  it('refuses no environment, the local definition, an unknown one, and a BUILD_SHA that is not a full commit', () => {
    const rows = [
      { args: ['--sha', SHA], said: /edge-deploy:/ },
      { args: ['--env', 'local', '--sha', SHA], said: /edge-deploy:/ },
      { args: ['--env', 'staging', '--sha', SHA], said: /edge-deploy:/ },
      { args: ['--env', 'production', '--sha', 'abc1234'], said: 'BUILD_SHA' },
    ];
    for (const row of rows) {
      const result = run(row.args);
      expect.soft(result.status, row.args.join(' ')).not.toBe(0);
      says(result.stderr, [row.said], row.args.join(' '));
    }
    expect(() => calls()).toThrow();
  });

  it('stamps the VERIFIED commit: the promotion merge’s second parent, not the branch tip', () => {
    g('checkout', '-q', '-b', 'deploy');
    g('commit', '-q', '--allow-empty', '-m', 'old tip');
    g('checkout', '-q', '-b', 'verified');
    writeFileSync(join(edge, 'marker'), 'verified');
    g('add', '.');
    g('commit', '-q', '-m', 'verified head');
    const verified = git('rev-parse', 'HEAD');
    g('checkout', '-q', 'deploy');
    g('merge', '-q', '--no-ff', '-m', 'promote', 'verified');
    const tip = git('rev-parse', 'HEAD');
    expect(tip).not.toBe(verified);
    const result = run(['--env', 'production', '--dry-run']);
    expect(result.status, result.stderr).toBe(0);
    expect(calls().filter((c) => c.startsWith('wrangler deploy')).every((c) => c.includes(`BUILD_SHA:${verified}`))).toBe(true);
    const ci = run(['--env', 'production', '--dry-run'], { WORKERS_CI_COMMIT_SHA: tip });
    expect(ci.status).toBe(0);
    expect(calls().at(-1)).toContain(`BUILD_SHA:${verified}`);
  });

  describe("the gate (CD-160, CD-147 review #3; CD-280: the gate's statuses are the only evidence, no marks)", () => {
    const touchedRemote = () => {
      let made = [];
      try {
        made = calls();
      } catch {
        return false;
      }
      return made.some((line) => line.includes('--remote') || (line.startsWith('wrangler deploy') && !line.includes('--dry-run')) || line.startsWith('live-vars') || line.startsWith('pnpm'));
    };

    const failingGate = (said) => stub('gate-node', `case "$*" in *gate-status.mjs*) ${said} ;; *) exec node "$@" ;; esac`);
    const NO_STATUS = 'echo "gate-status: gate/tree: no status names this tree" >&2; exit 1';

    it("refuses a commit the gate's statuses do not pass, one that carries only a verified/<sha> branch (a mark is not evidence), and any when GitHub cannot be asked; says why and moves nothing", () => {
      const rows = [
        { label: 'no status', said: NO_STATUS, err: ['gate/tree: no status names this tree', /no passing gate for the checkout's tree/] },
        {
          label: 'a verified/<sha> branch only',
          said: NO_STATUS,
          before: () => {
            g('push', '-q', 'origin', `${SHA}:refs/heads/verified/${SHA}`);
            expect.soft(spawnSync('git', ['-C', origin, 'rev-parse', '--verify', '--quiet', `refs/heads/verified/${SHA}`]).status, 'the mark is on origin').toBe(0);
          },
          err: [/no passing gate for the checkout's tree/],
        },
        { label: 'GitHub cannot be asked', said: 'echo "gate-status: no GitHub token here" >&2; exit 2', err: [/cannot ask GitHub/] },
      ];
      rows.forEach((row, i) => {
        if (i) fresh();
        row.before?.();
        const result = run(['--env', 'dev', '--sha', SHA], { WB_NODE: failingGate(row.said) });
        expect.soft(result.status, row.label).not.toBe(0);
        says(result.stderr, row.err, row.label);
        expect.soft(touchedRemote(), row.label).toBe(false);
      });
    });

    it('refuses a checkout that is not the verified commit: a later commit’s tree, an untracked file with its own exit code (an unverified migration never reaches D1), an uncommitted change', () => {
      const rows = [
        {
          label: 'a later commit',
          before: () => {
            writeFileSync(join(edge, 'unverified.ts'), 'export const late = 1;\n');
            g('add', '.');
            g('commit', '-q', '-m', 'after the gate');
          },
          err: [/tree/],
        },
        {
          label: 'an untracked migration',
          before: () => {
            mkdirSync(join(edge, 'migrations', 'core'), { recursive: true });
            writeFileSync(join(edge, 'migrations', 'core', '0099_unverified.sql'), 'create table later (id text);\n');
          },
          status: 4,
          err: [/untracked/, 'migrations/core/0099_unverified.sql'],
        },
        {
          label: 'an uncommitted change',
          before: () => writeFileSync(join(edge, 'wrangler.api.jsonc'), `${readFileSync(join(edge, 'wrangler.api.jsonc'), 'utf8')}\n`),
          err: [/uncommitted/],
        },
      ];
      rows.forEach((row, i) => {
        if (i) fresh();
        row.before();
        const result = run(['--env', 'dev', '--sha', SHA]);
        if (row.status === undefined) expect.soft(result.status, row.label).not.toBe(0);
        else expect.soft(result.status, row.label).toBe(row.status);
        says(result.stderr, row.err, row.label);
        expect.soft(touchedRemote(), row.label).toBe(false);
      });
    });

    it('deploys a promotion merge whose tree is the verified commit’s', () => {
      g('checkout', '-q', '--orphan', 'deploy');
      g('commit', '-q', '-m', 'the old deploy tip');
      const oldTip = git('rev-parse', 'HEAD');
      const merge = g('commit-tree', `${SHA}^{tree}`, '-p', oldTip, '-p', SHA, '-m', 'promote').stdout.trim();
      g('reset', '-q', '--hard', merge);
      expect(git('rev-parse', 'HEAD^2')).toBe(SHA);
      const result = run(['--env', 'dev']);
      expect(result.status, result.stderr).toBe(0);
      expect(result.stdout).toContain(`${SHA.slice(0, 7)} is verified`);
    });

    it('--force deploys a commit the gate has not passed, loudly', () => {
      const node = failingGate(NO_STATUS);
      const result = run(['--env', 'dev', '--sha', SHA, '--force'], { WB_NODE: node });
      expect(result.status, result.stderr).toBe(0);
      expect(result.stderr).toMatch(/WITHOUT the gate/);
    });
  });

  describe('the live-vars guard (CD-142) and the wait for production’s Durable Object transfer (CD-285)', () => {
    const deployed = () => calls().filter((line) => line.startsWith('wrangler deploy') && !line.includes('--dry-run'));

    it('refuses dev, migrating and deploying nothing, when its vars differ from the live ones, when those cannot be read, and while production has not taken the objects', () => {
      const rows = [
        { label: 'vars differ', env: { LIVE_VARS_EXIT: '3' }, err: /differ from what runs live/ },
        { label: 'vars unread', env: { LIVE_VARS_EXIT: '2' }, err: /could not be compared/ },
        { label: 'transfer pending', env: { TRANSFER_GUARD_EXIT: '3' }, err: /dev is not deployed before production has taken its Durable Objects/ },
      ];
      rows.forEach((row, i) => {
        if (i) fresh();
        const result = run(['--env', 'dev', '--sha', SHA], row.env);
        expect.soft(result.status, row.label).not.toBe(0);
        expect.soft(result.stderr, row.label).toMatch(row.err);
        expect.soft(calls().some((line) => line.startsWith('wrangler')), row.label).toBe(false);
      });
    });

    it('lets a meant change through with --accept-var-change, without asking', () => {
      const result = run(['--env', 'dev', '--sha', SHA, '--accept-var-change'], { LIVE_VARS_EXIT: '3' });
      expect(result.status, result.stderr).toBe(0);
      expect(calls().some((line) => line.startsWith('live-vars'))).toBe(false);
      expect(deployed()).toHaveLength(3);
    });
  });

  describe('in Workers Builds (CD-177): a shallow clone of deploy/api, the WORKERS_CI variables', () => {
    let tip;
    let clone;

    const promoted = () => {
      g('checkout', '-q', '--orphan', 'old-deploy');
      g('commit', '-q', '-m', 'the old deploy tip');
      const oldTip = git('rev-parse', 'HEAD');
      const merge = g('commit-tree', `${SHA}^{tree}`, '-p', oldTip, '-p', SHA, '-m', `promote(api): ${SHA.slice(0, 7)} → deploy/api`).stdout.trim();
      g('push', '-q', 'origin', `${merge}:refs/heads/deploy/api`);
      g('checkout', '-q', 'main');
      return merge;
    };

    const shallowClone = () => {
      const dir = join(root, 'clone');
      const cloned = spawnSync('git', ['clone', '-q', '--depth=1', '--branch', 'deploy/api', `file://${origin}`, dir], { encoding: 'utf8' });
      expect(cloned.status, cloned.stderr).toBe(0);
      return dir;
    };

    const reclone = () => {
      rmSync(clone, { recursive: true, force: true });
      clone = shallowClone();
    };

    const ciEnv = (extra = {}) => ({
      EDGE_DIR: clone,
      WORKERS_CI: '1',
      CI: 'true',
      WORKERS_CI_BUILD_UUID: 'build-uuid-1',
      WORKERS_CI_COMMIT_SHA: tip,
      WORKERS_CI_BRANCH: 'deploy/api',
      WRANGLER_CI_OVERRIDE_NAME: 'cd-api',
      WRANGLER_CI_MATCH_TAG: 'tag-of-cd-api',
      WRANGLER_CI_GENERATE_PREVIEW_ALIAS: 'true',
      EDGE_DEPLOY_API_TOKEN: 'deploy-token',
      ...extra,
    });

    const envOf = () => {
      try {
        return readFileSync(envLog, 'utf8').trim().split('\n');
      } catch {
        return [];
      }
    };
    const nothingRan = () => {
      try {
        return calls().length === 0;
      } catch {
        return true;
      }
    };

    beforeAll(() => {
      cpSync(template, root, { recursive: true });
      tip = promoted();
      clone = shallowClone();
      template = join(base, 'promoted');
      renameSync(root, template);
    });

    afterAll(() => {
      template = join(base, 'plain');
    });

    beforeEach(() => {
      expect(spawnSync('git', ['-C', clone, 'rev-parse', '--is-shallow-repository'], { encoding: 'utf8' }).stdout.trim()).toBe('true');
      expect(spawnSync('git', ['-C', clone, 'rev-parse', '--verify', '--quiet', 'HEAD^2']).status).not.toBe(0);
    });

    it('started by a relative path from another directory, stamps the tip’s second parent without asking GitHub anything (CD-215), installs, then migrates and deploys auth before api, hands its token to Cloudflare only after the build, matches only api to the connected tag, and leaves EDGE_DEPLOY_GIT_TOKEN unused and unprinted', () => {
      const asked = join(root, 'gate.argv');
      const node = stub(
        'gate-node',
        `case "$*" in *gate-status.mjs*) printf '%s\\n' "$*" >>"${asked}"; echo "gate/tree: tree x passed on ${SHA.slice(0, 7)}" ;; *) exec node "$@" ;; esac`,
      );
      const result = run(['--env', 'production'], ciEnv({ WB_NODE: node, EDGE_DEPLOY_GIT_TOKEN: 'ghp-must-not-appear' }), { script: 'infra/scripts/edge-deploy.sh', cwd: REPO });
      expect(result.status, result.stderr).toBe(0);
      expect.soft(result.stderr, 'finds its own files').not.toContain('Cannot find module');
      expect.soft(result.stdout).toContain(`Workers Builds build-uuid-1, deploy/api at ${tip.slice(0, 7)}, shallow clone: true`);
      expect.soft(result.stdout).toContain(`${SHA.slice(0, 7)} passed checks and gate before that push, and the gate is not re-checked here`);
      expect.soft(result.stdout).toContain(`fetching it from origin for the tree comparison`);
      expect.soft(result.stdout).toContain(`a promotion merge carrying ${SHA.slice(0, 7)}'s tree`);
      expect.soft(result.stdout).not.toContain('fetching origin');
      expect.soft(result.stdout).not.toContain('is verified');
      expect.soft(() => readFileSync(asked, 'utf8'), 'GitHub is not asked').toThrow();
      expect.soft(calls(), 'the calls').toEqual([
        'pnpm install --frozen-lockfile --filter @wg/edge... --filter @wg/admin...',
        'pnpm --filter @wg/edge^... run build',
        'pnpm --filter @wg/admin run build:release',
        `live-vars --env production --dir ${clone}`,
        `wrangler d1 migrations apply cd-accounts --remote -c ${join(clone, 'wrangler.api.jsonc')} --env production`,
        `wrangler d1 migrations apply cd-core --remote -c ${join(clone, 'wrangler.api.jsonc')} --env production`,
        `wrangler d1 migrations apply cd-admin --remote -c ${join(clone, 'wrangler.admin.jsonc')} --env production`,
        `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}`,
        `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}`,
        `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA}`,
      ]);
      const seen = envOf();
      expect.soft(seen.find((line) => line.startsWith('wrangler deploy -c wrangler.auth.jsonc')), 'auth keeps its own name').toMatch(/\|name=\|tag=\|alias=\|/);
      expect.soft(seen.find((line) => line.startsWith('wrangler deploy -c wrangler.api.jsonc')), 'api matches the connected tag').toMatch(/\|name=\|tag=tag-of-cd-api\|alias=\|/);
      expect.soft(seen.filter((line) => line.includes('d1 migrations')).every((line) => line.includes('|name=|tag=|alias=|')), 'migrations carry no name').toBe(true);
      expect.soft(seen.filter((line) => line.startsWith('pnpm ')).every((line) => line.endsWith('|token=')), 'pnpm gets no token').toBe(true);
      expect.soft(seen.filter((line) => !line.startsWith('pnpm ')).every((line) => line.endsWith('|token=deploy-token')), 'Cloudflare gets the token').toBe(true);
      expect.soft(result.stdout, 'git token').toContain('EDGE_DEPLOY_GIT_TOKEN is set; it is used only if origin refuses');
      expect.soft(result.stdout, 'git token').not.toContain('asking again');
      expect.soft(`${result.stdout}${result.stderr}`, 'git token printed').not.toContain('ghp-must-not-appear');
      expect.soft(seen.some((line) => line.includes('ghp-must-not-appear')), 'git token passed on').toBe(false);
    });

    it('a promotion whose admin binds a class api does not export yet deploys api first, still matched to the connected tag, then the admin again', () => {
      adminRefusedUntilApi();
      const result = run(['--env', 'production'], ciEnv());
      expect(result.status, result.stderr).toBe(0);
      expect(result.stdout).toContain(ADMIN_BOOTSTRAP_STEP);
      expect(envOf().filter((line) => line.startsWith('wrangler deploy'))).toEqual([
        `wrangler deploy -c wrangler.auth.jsonc --env production --var BUILD_SHA:${SHA}|tag=`,
        `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}|tag=`,
        `wrangler deploy -c wrangler.api.jsonc --env production --var BUILD_SHA:${SHA}|tag=tag-of-cd-api`,
        `wrangler deploy -c wrangler.admin.jsonc --env production --var BUILD_SHA:${SHA}|tag=`,
      ]);
    });

    it('a dry run with --gate bundles in the clone, touching nothing remote and needing no token, and reads its configs with node, never with the NODE pnpm exports (CD-196)', () => {
      const pnpmExe = stub('pnpm-exe', 'echo " ERR_PNPM_RECURSIVE_EXEC_FIRST_FAIL  Command not found" >&2\nexit 254');
      const result = run(['--env', 'production', '--dry-run', '--gate'], ciEnv({ EDGE_DEPLOY_API_TOKEN: '', NODE: pnpmExe }));
      expect(result.status, result.stderr).toBe(0);
      expect.soft(result.stdout).toContain('connected Worker cd-api;');
      expect.soft(result.stdout).toContain('the gate is not re-checked here');
      expect.soft(calls().some((line) => line.includes('--remote') || line.startsWith('live-vars'))).toBe(false);
      expect.soft(calls().filter((line) => line.startsWith('wrangler deploy')).every((line) => line.includes('--dry-run'))).toBe(true);
    });

    it('deploys dev from deploy/api-dev as cd-api-dev (CD-285), adds origin from EDGE_DEPLOY_ORIGIN when the clone has none, and runs the pinned pnpm through corepack when the image’s pnpm is another version', () => {
      const dev = run(['--env', 'dev'], ciEnv({ WORKERS_CI_BRANCH: 'deploy/api-dev', WRANGLER_CI_OVERRIDE_NAME: 'cd-api-dev' }));
      expect.soft(dev.status, `dev: ${dev.stderr}`).toBe(0);
      expect.soft(dev.stdout, 'dev').toContain(`deploy/api-dev at ${tip.slice(0, 7)}`);
      expect.soft(calls(), 'dev').toContain(`wrangler d1 migrations apply cd-accounts-dev --remote -c ${join(clone, 'wrangler.api.jsonc')} --env dev`);
      expect.soft(calls().filter((line) => line.startsWith('wrangler deploy')).every((line) => line.includes('--env dev')), 'dev').toBe(true);
      fresh();
      spawnSync('git', ['-C', clone, 'remote', 'remove', 'origin']);
      const withOrigin = run(['--env', 'production'], ciEnv({ EDGE_DEPLOY_ORIGIN: `file://${origin}` }));
      expect.soft(withOrigin.status, `EDGE_DEPLOY_ORIGIN: ${withOrigin.stderr}`).toBe(0);
      fresh();
      const corepack = run(['--env', 'production'], ciEnv({ PNPM_HAS: '10.11.1' }));
      expect.soft(corepack.status, `corepack: ${corepack.stderr}`).toBe(0);
      expect.soft(calls().slice(0, 3), 'corepack').toEqual(['corepack pnpm@10.33.0 install --frozen-lockfile --filter @wg/edge... --filter @wg/admin...', 'corepack pnpm@10.33.0 --filter @wg/edge^... run build', 'corepack pnpm@10.33.0 --filter @wg/admin run build:release']);
    });

    it('retries the fetch with EDGE_DEPLOY_GIT_TOKEN as a basic x-access-token header for github.com only after origin refused the clone’s own access', () => {
      spawnSync('git', ['-C', clone, 'remote', 'set-url', 'origin', join(root, 'nowhere.git')]);
      const realGit = spawnSync('bash', ['-c', 'command -v git'], { encoding: 'utf8' }).stdout.trim();
      const wire = join(root, 'git-wire.log');
      stub('git', `if [ -n "\${GIT_CONFIG_KEY_0:-}" ]; then printf '%s|%s\\n' "$GIT_CONFIG_KEY_0" "$(printf %s "\${GIT_CONFIG_VALUE_0#AUTHORIZATION: basic }" | base64 -d)" >>"${wire}"; fi\nexec "${realGit}" "$@"`);
      const result = run(['--env', 'production'], ciEnv({ EDGE_DEPLOY_GIT_TOKEN: 'ghp-must-not-appear', PATH: `${join(root, 'bin')}:${process.env.PATH}` }));
      expect(result.status).not.toBe(0);
      expect(result.stdout).toContain('the fetch with the clone\'s own access failed (above); asking again with EDGE_DEPLOY_GIT_TOKEN');
      expect(result.stderr.match(/nowhere\.git/g)).toHaveLength(2);
      expect(result.stderr).toMatch(/cannot fetch .{7} from origin/);
      expect(readFileSync(wire, 'utf8').trim().split('\n')).toEqual(['http.https://github.com/.extraheader|x-access-token:ghp-must-not-appear']);
      expect(`${result.stdout}${result.stderr}`).not.toContain('ghp-must-not-appear');
      expect(nothingRan()).toBe(true);
    });

    it('refuses before anything runs: a tip that is not a promotion merge, a merge whose tree is not its second parent’s, a stamped commit origin cannot give, no origin and no EDGE_DEPLOY_ORIGIN, a dirty clone, the wrong branch for the environment, a clone off the named commit, another Worker, no token, --force or --accept-var-change, a pnpm corepack cannot pin, an older Node', () => {
      const rows = [
        {
          label: 'a plain push (CD-215)',
          before: () => {
            g('push', '-q', '-f', 'origin', `${SHA}:refs/heads/deploy/api`);
            reclone();
            return { WORKERS_CI_COMMIT_SHA: SHA };
          },
          err: [/is not a promotion merge stamping/],
        },
        {
          label: 'another tree (CD-215)',
          before: () => {
            g('checkout', '-q', '--orphan', 'other-tree');
            writeFileSync(join(edge, 'extra.txt'), 'not the verified tree\n');
            g('add', '-A');
            g('commit', '-q', '-m', 'another tree');
            const otherTree = git('rev-parse', 'HEAD^{tree}');
            g('checkout', '-q', 'main');
            const bad = g('commit-tree', otherTree, '-p', tip, '-p', SHA, '-m', `promote(api): ${SHA.slice(0, 7)} → deploy/api`).stdout.trim();
            g('push', '-q', '-f', 'origin', `${bad}:refs/heads/deploy/api`);
            reclone();
            return { WORKERS_CI_COMMIT_SHA: bad };
          },
          err: [/the checkout's tree is not .{7}'s tree/],
        },
        {
          label: 'origin cannot give the stamped commit',
          before: () => void spawnSync('git', ['-C', clone, 'remote', 'set-url', 'origin', join(root, 'nowhere.git')]),
          err: [/cannot fetch .{7} from origin/],
        },
        {
          label: 'no origin',
          before: () => void spawnSync('git', ['-C', clone, 'remote', 'remove', 'origin']),
          err: ['EDGE_DEPLOY_ORIGIN'],
        },
        { label: 'a dirty clone', before: () => writeFileSync(join(clone, 'stray.sql'), '-- not committed\n'), status: 4, err: ['stray.sql'] },
        { label: 'main', env: { WORKERS_CI_BRANCH: 'main' }, err: [/only deploy\/api deploys/] },
        { label: 'production from deploy/api-dev', env: { WORKERS_CI_BRANCH: 'deploy/api-dev' }, err: [/only deploy\/api deploys/] },
        { label: 'dev from deploy/api', args: ['--env', 'dev'], env: { WRANGLER_CI_OVERRIDE_NAME: 'cd-api-dev' }, err: [/only deploy\/api-dev deploys/] },
        { label: 'a clone off the named commit', env: { WORKERS_CI_COMMIT_SHA: SHA }, err: [/the clone is at/] },
        { label: 'another Worker', env: { WRANGLER_CI_OVERRIDE_NAME: 'cd-auth' }, err: ['"cd-auth"', '"cd-api"'] },
        { label: 'no token', env: { EDGE_DEPLOY_API_TOKEN: '' }, err: ['EDGE_DEPLOY_API_TOKEN'] },
        { label: '--force', args: ['--env', 'production', '--force'], err: ['--force is refused in Workers Builds'] },
        { label: '--accept-var-change', args: ['--env', 'production', '--accept-var-change'], err: ['--accept-var-change is refused in Workers Builds'] },
        { label: 'corepack cannot pin pnpm', env: { PNPM_HAS: '10.11.1', COREPACK_FAILS: '1' }, err: ['PNPM_VERSION=10.33.0'] },
        {
          label: 'an older Node',
          before: () => writeFileSync(join(repoRoot, 'package.json'), JSON.stringify({ packageManager: 'pnpm@10.33.0', engines: { node: '>=99' } })),
          err: ['NODE_VERSION=99'],
        },
      ];
      rows.forEach((row, i) => {
        if (i) fresh();
        const extra = { ...row.env, ...row.before?.() };
        const result = run(row.args ?? ['--env', 'production'], ciEnv(extra));
        if (row.status === undefined) expect.soft(result.status, row.label).not.toBe(0);
        else expect.soft(result.status, row.label).toBe(row.status);
        says(result.stderr, row.err, row.label);
        expect.soft(nothingRan(), row.label).toBe(true);
      });
    });
  });
});
