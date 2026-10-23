import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { GIT_ENV } from './harness-fixture.mjs';
import { assertStubsRun, childEnv } from './hermetic.mjs';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const CHECKS = parse(readFileSync(join(ROOT, '.github/workflows/checks.yml'), 'utf8'));
const { test, checks } = CHECKS.jobs;

const SCOPE_IF = /^steps\.scope\.outputs\.(\w+) != 'false'$/;
const REFUSAL_IF = /^github\.event_name == /;

const scopeStep = test.steps.find((step) => step.id === 'scope');
const gateStep = test.steps.find((step) => step.env?.LANE !== undefined);
const workersStep = test.steps.find((step) => step.id === 'workers');
const ciOnly = {
  web: test.steps.find((step) => String(step.run).includes('export:web')),
  bundle: test.steps.find((step) => String(step.run).includes('edge-deploy.sh')),
  smoke: test.steps.find((step) => String(step.run).includes('e2e:full')),
};
const runsOn = (step, outputs) => {
  const match = SCOPE_IF.exec(step.if);
  return (outputs[match[1]] ?? '') !== 'false';
};
const ciOnlyRan = (outputs) => Object.keys(ciOnly).filter((name) => runsOn(ciOnly[name], outputs));

const PACKAGES = {
  'apps/app': ['@wg/app', ['@wg/shared', '@wg/i18n', '@wg/game-wordgrid']],
  'apps/edge': ['@wg/edge', ['@wg/shared', '@wg/i18n']],
  'apps/admin': ['@wg/admin', ['@wg/shared']],
  'packages/shared': ['@wg/shared', []],
  'packages/i18n': ['@wg/i18n', []],
  'packages/game-wordgrid': ['@wg/game-wordgrid', ['@wg/languages']],
  'packages/languages': ['@wg/languages', []],
};
const INFRA_MANIFEST = JSON.parse(readFileSync(join(ROOT, 'infra/package.json'), 'utf8'));
const infraDeclares = (name) => name in { ...INFRA_MANIFEST.dependencies, ...INFRA_MANIFEST.devDependencies };
PACKAGES.infra = ['@wg/infra', Object.values(PACKAGES).map(([name]) => name).filter(infraDeclares)];

const OWN_TMPDIR_TEST = 'node ../../infra/scripts/own-tmpdir.mjs vitest run';
const TEST_SCRIPT = (dir) => (dir === 'infra' ? 'sh scripts/infra-test.sh' : OWN_TMPDIR_TEST);

const IMPORTED_BY_PATH = {
  'apps/app/src/api/http-client.ts': 'export const client = 1;\n',
  'apps/app/src/api/edge-channel.ts': 'export const channel = 1;\n',
  'apps/edge/test/client-contract.test.ts': "import clientSource from '../../app/src/api/http-client.ts?raw';\n",
  'apps/edge/test/session-sockets.test.ts': "import { channel } from '../../app/src/api/edge-channel.js';\n",
  'packages/languages/src/words.txt': 'duke\n',
  'packages/i18n/src/words.ts': "export { default } from '../../languages/src/words.txt';\n",
  'packages/shared/test/words.test.ts': "import words from '../../i18n/src/words';\n",
  'apps/edge/scripts/transfer-guard.mjs': 'export const guard = 1;\n',
  'infra/scripts/__tests__/transfer-guard.test.mjs': "import { guard } from '../../../apps/edge/scripts/transfer-guard.mjs';\n",
  'infra/scripts/moved.mjs': 'export const moved = 1;\n',
  'infra/scripts/own-tmpdir-lib.mjs': 'export const main = 1;\n',
  'infra/scripts/own-tmpdir.mjs': "import { main } from './own-tmpdir-lib.mjs';\n",
  'infra/scripts/edge-deploy.sh': '. "$SCRIPT_DIR/gate-passed.sh"\n',
  'infra/scripts/gate-passed.sh': 'gate_passed() { :; }\n',
  'infra/reporters/compact-vitest.mjs': 'export default class {}\n',
  'apps/admin/vite.config.mts': "export const reporter = new URL('../../infra/reporters/compact-vitest.mjs', import.meta.url);\n",
  'infra/verify/screens.mjs': 'export const screens = 1;\n',
  'infra/scripts/release-version.mjs': 'export const version = 1;\n',
  '.github/workflows/release.yml': 'name: release\n',
};

const LOCK = {
  lockfileVersion: '9.0',
  settings: { autoInstallPeers: true, excludeLinksFromLockfile: false },
  importers: {
    '.': {},
    'apps/app': {
      dependencies: { '@wg/shared': { specifier: 'workspace:*', version: 'link:../../packages/shared' }, 'left-pad': { specifier: '^1.0.0', version: '1.0.0' } },
    },
    'apps/edge': {},
    'packages/languages': { dependencies: { words: { specifier: '^1.0.0', version: '1.0.0' } } },
  },
  packages: {
    'left-pad@1.0.0': { resolution: { integrity: 'sha512-a' } },
    'words@1.0.0': { resolution: { integrity: 'sha512-w' } },
    'letters@1.0.0': { resolution: { integrity: 'sha512-l' } },
    'glyphs@1.0.0': { resolution: { integrity: 'sha512-g' } },
  },
  snapshots: {
    'left-pad@1.0.0': {},
    'words@1.0.0': { dependencies: { letters: '1.0.0' } },
    'letters@1.0.0': { dependencies: { glyphs: '1.0.0' } },
    'glyphs@1.0.0': {},
  },
};
const LOCK_EDITS = {
  lockAppDevDependency: (lock) => ({
    ...lock,
    importers: { ...lock.importers, 'apps/app': { ...lock.importers['apps/app'], devDependencies: { vitest: { specifier: '^3.0.0', version: '3.0.0' } } } },
    packages: { ...lock.packages, 'vitest@3.0.0': { resolution: { integrity: 'sha512-v' } } },
    snapshots: { ...lock.snapshots, 'vitest@3.0.0': {} },
  }),
  lockTransitive: (lock) => ({
    ...lock,
    packages: { ...lock.packages, 'glyphs@1.1.0': { resolution: { integrity: 'sha512-g2' } } },
    snapshots: { ...lock.snapshots, 'letters@1.0.0': { dependencies: { glyphs: '1.1.0' } }, 'glyphs@1.1.0': {} },
  }),
  lockRoot: (lock) => ({ ...lock, importers: { ...lock.importers, '.': { devDependencies: { oxlint: { specifier: '^1.0.0', version: '1.0.0' } } } } }),
  lockSettings: (lock) => ({ ...lock, settings: { ...lock.settings, autoInstallPeers: false } }),
  lockReformatted: (lock) => JSON.stringify(lock, null, 4),
  lockIntegrity: (lock) => ({ ...lock, packages: { ...lock.packages, 'glyphs@1.0.0': { resolution: { integrity: 'sha512-g-republished' } } } }),
  lockImporterWithoutPackage: (lock) => ({ ...lock, importers: { ...lock.importers, 'apps/ghost': { dependencies: { 'left-pad': { specifier: '^1.0.0', version: '1.0.0' } } } } }),
};
const YAML_FROM_THE_REPO = { NODE_PATH: join(ROOT, 'node_modules') };

const MANIFEST_EDITS = {
  rootScript: [['package.json', (manifest) => ({ ...manifest, scripts: { inventory: 'bash infra/scripts/inventory.sh' } })]],
  appDevDependency: [['apps/app/package.json', (manifest) => ({ ...manifest, devDependencies: { vitest: '^3.0.0' } })]],
  appReorderedDependencies: [['apps/app/package.json', (manifest) => ({ ...manifest, dependencies: Object.fromEntries(Object.entries(manifest.dependencies).reverse()) })]],
  appDependency: [['apps/app/package.json', (manifest) => ({ ...manifest, dependencies: { ...manifest.dependencies, zod: '^4.0.0' } })]],
  appPeerDependency: [['apps/app/package.json', (manifest) => ({ ...manifest, peerDependencies: { react: '^19.0.0' } })]],
  edgeAndRootScript: [
    ['package.json', (manifest) => ({ ...manifest, scripts: { inventory: 'bash infra/scripts/inventory.sh' } })],
    ['apps/edge/src/x.ts', () => 'edgeAndRootScript'],
  ],
  edgeManifestDeleted: [['apps/edge/package.json', () => null]],
  sharedManifestDeleted: [['packages/shared/package.json', () => null]],
  newManifestWithoutDependencies: [['packages/newcomer/package.json', () => JSON.stringify({ name: '@wg/newcomer', version: '0.0.0' })]],
  languagesManifestUnreadable: [['packages/languages/package.json', () => 'not json']],
};

const BRANCHES = {
  text: ['docs/x.md', 'README.md', '.claude/settings.json', 'CLAUDE.md', '.gitignore'],
  client: ['apps/app/src/x.ts'],
  clientAndIgnore: ['apps/app/src/x.ts', '.gitignore'],
  admin: ['apps/admin/src/x.ts'],
  walk: ['apps/app/e2e/live/walk.ts'],
  namesake: ['apps/app/src/http-client.ts'],
  contract: ['apps/app/src/api/http-client.ts'],
  channel: ['apps/app/src/api/edge-channel.ts'],
  chain: ['packages/languages/src/words.txt'],
  guard: ['apps/edge/scripts/transfer-guard.mjs'],
  rootAndContract: ['tsconfig.base.json', 'apps/app/src/api/http-client.ts'],
  edge: ['apps/edge/src/x.ts'],
  shared: ['packages/shared/src/x.ts'],
  i18n: ['packages/i18n/src/x.ts'],
  wordgrid: ['packages/game-wordgrid/src/x.ts'],
  infra: ['infra/scripts/x.mjs'],
  ownTmpdir: ['infra/scripts/own-tmpdir.mjs'],
  ownTmpdirLib: ['infra/scripts/own-tmpdir-lib.mjs'],
  edgeDeploy: ['infra/scripts/edge-deploy.sh'],
  gatePassed: ['infra/scripts/gate-passed.sh'],
  verify: ['infra/verify/screens.mjs'],
  reporter: ['infra/reporters/compact-vitest.mjs'],
  releaseScript: ['infra/scripts/release-version.mjs'],
  workflow: ['.github/workflows/release.yml'],
  infraDoc: ['infra/README.md'],
  root: ['tsconfig.base.json'],
  lockfile: ['pnpm-lock.yaml'],
  rootManifest: ['package.json'],
  appManifest: ['apps/app/package.json'],
  infraManifest: ['infra/package.json'],
  workspace: ['pnpm-workspace.yaml'],
  oxlint: ['.oxlintrc.json'],
  npmrc: ['.npmrc'],
  gateScript: ['infra/scripts/gate.sh'],
  checksWorkflow: ['.github/workflows/checks.yml'],
  mergify: ['.mergify.yml'],
  other: ['release/x.txt'],
};

const SOURCED = /^\. "\$SCRIPT_DIR\/([^"]+)"$/gm;
const withSourced = (path) => [path, ...[...readFileSync(join(ROOT, path), 'utf8').matchAll(SOURCED)].map(([, name]) => `${dirname(path)}/${name}`)];
const RUN_BY_SCOPED_STEPS = test.steps
  .filter((step) => SCOPE_IF.test(step.if ?? ''))
  .flatMap((step) => [...String(step.run).matchAll(/infra\/[\w./-]+/g)].flatMap(([path]) => withSourced(path).map((file) => [file, SCOPE_IF.exec(step.if)[1]])));
for (const [index, [file]] of RUN_BY_SCOPED_STEPS.entries()) BRANCHES[`scopedStep${index}`] = [file];

const MOVES = {
  renamed: ['apps/app/src/api/http-client.ts', 'apps/app/src/api/api-client.ts'],
  movedOutOfInfra: ['infra/scripts/moved.mjs', '.claude/scripts/moved.mjs'],
};

const STRIP_STUB = `import { appendFileSync } from 'node:fs';
appendFileSync(process.env.PNPM_LOG, \`\${JSON.stringify({ args: ['strip-comments', ...process.argv.slice(2)], workers: process.env.VITEST_MAX_WORKERS ?? null, cwd: process.cwd() })}\\n\`);
`;
const LINT = 'run lint';
const EACH = '-r --if-present --no-bail --report-summary';
const lintTouched = (dir) => [`exec oxlint ${dir}`, `strip-comments --check --type ts,tsx ${dir}`];
const build = (...dirs) => `${EACH} ${dirs.map((dir) => `--filter {./${dir}}^...`).join(' ')} run build`;
const typecheck = (...dirs) => `${EACH} ${dirs.map((dir) => `--filter ...{./${dir}}`).join(' ')} run typecheck`;
const changed = (dir) =>
  dir === 'infra'
    ? 'infra: exec sh scripts/infra-test.sh --changed BASE --passWithNoTests'
    : dir === 'apps/edge'
      ? 'apps/edge: exec node ../../infra/scripts/own-tmpdir.mjs node scripts/test.mjs --changed BASE'
      : `${dir}: exec node ../../infra/scripts/own-tmpdir.mjs vitest run --changed BASE --passWithNoTests`;
const whole = (dir) => `${dir}: run test`;
const I18N = 'run i18n:check';
const BALANCE = '--filter @wg/game-wordgrid run test:balance';
const FULL_GATE = [LINT, `${EACH} run build`, `${EACH} run typecheck`, I18N, `${EACH} run test`, BALANCE];
const JOURNEYS = '--filter @wg/infra run test verify/__tests__/journeys-map.test.mjs';
const INFRA_LANE = { lane: 'code', web: 'false', edge: 'false' };
const INFRA_ONLY = [...lintTouched('infra'), build('infra'), typecheck('infra'), changed('infra')];
const INFRA_BUNDLE = [...lintTouched('infra'), build('infra', 'apps/edge'), typecheck('infra'), changed('infra')];
const EVERY_PACKAGE = Object.keys(PACKAGES).sort();
const CLIENT = [...lintTouched('apps/app'), build('apps/app', 'infra'), typecheck('apps/app'), changed('apps/app'), changed('infra'), I18N, JOURNEYS];
const APP_AND_EDGE = [...lintTouched('apps/app'), build('apps/app', 'apps/edge', 'infra'), typecheck('apps/app', 'apps/edge'), changed('apps/app'), changed('apps/edge'), changed('infra'), I18N, JOURNEYS];
const tested = (gate) => gate.flatMap((command) => /^([\w/-]+): /.exec(command)?.[1] ?? []).sort();

describe('checks.yml: checks-run runs what the change reaches, as gate.sh decides (PAR-30)', { timeout: 60_000 }, () => {
  let root;
  let repo;
  let bin;
  let main;
  let seed;
  const realPnpm = spawnSync('sh', ['-c', 'command -v pnpm'], { encoding: 'utf8' }).stdout.trim();
  const realGit = spawnSync('sh', ['-c', 'command -v git'], { encoding: 'utf8' }).stdout.trim();
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: repo, encoding: 'utf8', env: { ...childEnv(), ...GIT_ENV } });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr}`);
    return result.stdout.trim();
  };
  const write = (path, text) => {
    mkdirSync(join(repo, dirname(path)), { recursive: true });
    writeFileSync(join(repo, path), text);
  };
  const branch = (name, paths) => {
    git('checkout', '-q', '-b', name, 'main');
    for (const path of paths) write(path, `${name}\n`);
    git('add', '-A');
    git('commit', '-q', '-m', name);
  };

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'checks-scope-'));
    repo = join(root, 'tested');
    bin = join(root, 'bin');
    mkdirSync(repo);
    mkdirSync(bin);
    mkdirSync(join(root, 'runner-temp'));
    git('init', '-q', '-b', 'main');
    write('package.json', JSON.stringify({ name: 'wg-monorepo', private: true }));
    write('pnpm-workspace.yaml', "packages:\n  - 'apps/*'\n  - 'infra'\n  - 'packages/*'\n");
    write('tsconfig.base.json', '{}\n');
    for (const [dir, [name, deps]] of Object.entries(PACKAGES)) {
      write(`${dir}/package.json`, JSON.stringify({ name, version: '0.0.0', scripts: { test: TEST_SCRIPT(dir) }, dependencies: Object.fromEntries(deps.map((dep) => [dep, 'workspace:*'])), devDependencies: { vitest: '^5.0.0' } }));
    }
    for (const [path, text] of Object.entries(IMPORTED_BY_PATH)) write(path, text);
    write('pnpm-lock.yaml', JSON.stringify(LOCK));
    writeFileSync(join(root, 'languages-manifest.json'), readFileSync(join(repo, 'packages/languages/package.json'), 'utf8'));
    mkdirSync(join(repo, 'infra/scripts'), { recursive: true });
    for (const script of ['gate.sh', 'check-touched.mjs']) copyFileSync(join(ROOT, 'infra/scripts', script), join(repo, 'infra/scripts', script));
    write('infra/scripts/strip-comments.mjs', STRIP_STUB);
    git('add', '-A');
    git('commit', '-q', '-m', 'seed');
    for (const [name, paths] of Object.entries(BRANCHES)) branch(name, paths);
    for (const [name, edits] of Object.entries(MANIFEST_EDITS)) {
      git('checkout', '-q', '-b', name, 'main');
      for (const [path, edit] of edits) {
        const before = existsSync(join(repo, path)) && path.endsWith('.json') ? JSON.parse(readFileSync(join(repo, path), 'utf8')) : null;
        const after = edit(before);
        if (after === null) rmSync(join(repo, path));
        else write(path, typeof after === 'string' ? `${after}\n` : JSON.stringify(after));
      }
      git('add', '-A');
      git('commit', '-q', '-m', name);
    }
    git('checkout', '-q', '-b', 'languagesManifestReadableAgain', 'languagesManifestUnreadable');
    write('packages/languages/package.json', readFileSync(join(repo, '..', 'languages-manifest.json'), 'utf8'));
    git('commit', '-q', '-am', 'languagesManifestReadableAgain');
    for (const [name, edit] of Object.entries(LOCK_EDITS)) {
      git('checkout', '-q', '-b', name, 'main');
      const after = edit(LOCK);
      write('pnpm-lock.yaml', typeof after === 'string' ? after : JSON.stringify(after));
      git('commit', '-q', '-am', name);
    }
    for (const [name, [from, to]] of Object.entries(MOVES)) {
      git('checkout', '-q', '-b', name, 'main');
      mkdirSync(join(repo, dirname(to)), { recursive: true });
      git('mv', from, to);
      git('commit', '-q', '-m', name);
    }
    git('checkout', '-q', '-b', 'narrowed', 'main');
    write('infra/scripts/gate.sh', 'printf "lane=lint\\nweb=false\\nedge=false\\n"\n');
    git('commit', '-q', '-am', 'narrowed');
    git('checkout', '-q', '-b', 'empty', 'main');
    git('checkout', '-q', 'main');
    write('apps/edge/src/main-moved-on.ts', 'main\n');
    git('add', '-A');
    git('commit', '-q', '-m', 'main moved on');
    git('update-ref', 'refs/remotes/origin/main', 'main');
    main = git('rev-parse', 'main');
    seed = git('rev-parse', 'main^');
    repo = realpathSync(repo);
    writeFileSync(
      join(bin, 'pnpm'),
      `#!/usr/bin/env bash
for arg in "$@"; do
  if [ "$arg" = ls ]; then
    [ -n "$FAKE_PNPM_LS_FAILS" ] && exit 1
    exec ${JSON.stringify(realPnpm)} "$@"
  fi
done
json() {
  local text=\${1//\\\\/\\\\\\\\}
  text=\${text//\\"/\\\\\\"}
  text=\${text//$'\\n'/\\\\n}
  text=\${text//$'\\r'/\\\\r}
  text=\${text//$'\\t'/\\\\t}
  printf -v quoted '"%s"' "$text"
}
args=
for arg in "$@"; do
  json "$arg"
  args+=\${args:+,}$quoted
done
workers=null
if [ -n "\${VITEST_MAX_WORKERS+set}" ]; then
  json "$VITEST_MAX_WORKERS"
  workers=$quoted
fi
json "$(pwd -P)"
printf '{"args":[%s],"workers":%s,"cwd":%s}\\n' "$args" "$workers" "$quoted" >>"$PNPM_LOG"
`,
      { mode: 0o755 },
    );
    writeFileSync(
      join(bin, 'git'),
      `#!/bin/sh
[ "$1" = grep ] && [ -n "$FAKE_GIT_GREP_FAILS" ] && exit 2
exec ${JSON.stringify(realGit)} "$@"
`,
      { mode: 0o755 },
    );
    assertStubsRun(bin);
  }, 60_000);
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const runStep = (step, env = {}) => {
    const output = join(root, 'github-output');
    const githubEnv = join(root, 'github-env');
    const log = join(root, 'pnpm-log');
    writeFileSync(output, '');
    writeFileSync(githubEnv, '');
    writeFileSync(log, '');
    const result = spawnSync('bash', ['-e', '-c', step.run], {
      cwd: repo,
      encoding: 'utf8',
      env: childEnv({ ...GIT_ENV, PATH: `${bin}:${process.env.PATH}`, GITHUB_OUTPUT: output, GITHUB_ENV: githubEnv, RUNNER_TEMP: join(root, 'runner-temp'), PNPM_LOG: log, ...step.env, ...env }),
    });
    const pairs = (file) =>
      Object.fromEntries(
        readFileSync(file, 'utf8')
          .split('\n')
          .filter(Boolean)
          .map((line) => [line.slice(0, line.indexOf('=')), line.slice(line.indexOf('=') + 1)]),
      );
    const outputs = pairs(output);
    const exported = pairs(githubEnv);
    const calls = readFileSync(log, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line));
    const pnpm = calls.map((call) => `${call.cwd === repo ? '' : `${relative(repo, call.cwd)}: `}${call.args.join(' ')}`.replaceAll(seed, 'BASE'));
    const workers = calls.map((call) => call.workers);
    return { status: result.status, stdout: result.stdout, stderr: result.stderr, outputs, exported, pnpm, workers };
  };

  const pr = (name, env = {}) => {
    git('checkout', '-q', name);
    const scope = runStep(scopeStep, { BASE: main, CARRIED: 'false', ...env });
    expect(scope.status, `${name}: ${scope.stderr}`).toBe(0);
    const gate = runStep(gateStep, { BASE: main, CARRIED: 'false', LANE: scope.outputs.lane ?? '', ...env });
    expect(gate.status, `${name}: ${gate.stdout}${gate.stderr}`).toBe(0);
    return { outputs: scope.outputs, stdout: scope.stdout, gate: gate.pnpm, workers: gate.workers, ciOnly: ciOnlyRan(scope.outputs) };
  };

  it('every condition in checks-run is a scope output that runs unless it says false, or a refusal', () => {
    for (const step of test.steps.filter((entry) => entry.if !== undefined)) {
      expect(SCOPE_IF.test(step.if) || REFUSAL_IF.test(step.if), step.if).toBe(true);
    }
    expect(gateStep.if).toBeUndefined();
    expect(test.steps.indexOf(scopeStep)).toBeLessThan(test.steps.indexOf(gateStep));
    for (const step of Object.values(ciOnly)) expect(test.steps.indexOf(step)).toBeGreaterThan(test.steps.indexOf(gateStep));
    expect(scopeStep.env).toEqual({ BASE: "${{ github.event.pull_request.base.sha || 'origin/main' }}", CARRIED: '${{ steps.carry.outputs.carried }}' });
    expect(gateStep.env.BASE).toBe(scopeStep.env.BASE);
    expect(gateStep.env.CARRIED).toBe(scopeStep.env.CARRIED);
    expect(test.steps.findIndex((step) => step.id === 'carry')).toBeLessThan(test.steps.indexOf(scopeStep));
  });

  const FULL = { lane: 'full', web: 'true', edge: 'true' };
  const LINT_ONLY = { lane: 'lint', web: 'false', edge: 'false' };
  const CODE_ALL = { lane: 'code', web: 'true', edge: 'true' };
  const ALL_CI = ['web', 'bundle', 'smoke'];
  const CLIENT_ONLY = { lane: 'code', web: 'true', edge: 'false' };
  const EDGE_ONLY = { lane: 'code', web: 'false', edge: 'true' };
  const EDGE = [...lintTouched('apps/edge'), build('apps/edge'), typecheck('apps/edge'), changed('apps/edge'), I18N];
  const CANNOT = 'The scope could not be computed: everything runs.';

  const expectScopes = (rows) => {
    for (const [label, name, want] of rows) {
      const run = pr(name, want.env?.() ?? {});
      expect.soft(run.outputs, label).toEqual(want.outputs);
      if (want.gate) {
        expect.soft(run.gate, label).toEqual(want.gate);
        expect.soft(run.workers.filter((workers) => workers !== null), label).toEqual([]);
      }
      if (want.ci) expect.soft(run.ciOnly, label).toEqual(want.ci);
      if (want.tested) expect.soft(tested(run.gate), label).toEqual(want.tested);
      if (want.contains) expect.soft(run.gate, label).toContain(want.contains);
      if (want.whole) expect.soft(run.gate.filter((command) => tested([command]).length > 0), label).toEqual(want.whole.map(whole));
      if (want.says) expect.soft(run.stdout, label).toContain(want.says);
    }
  };

  it('an app, admin, edge or package change gets its scope, the gate steps it reaches and the CI-only steps it turns on', () => {
    expectScopes([
        ['Markdown and .claude only', 'text', { outputs: LINT_ONLY, gate: [LINT], ci: [] }],
        ['client-only', 'client', { outputs: CLIENT_ONLY, gate: CLIENT, ci: ['web'] }],
        ['client file with a .gitignore change, which adds nothing to it', 'clientAndIgnore', { outputs: CLIENT_ONLY, gate: CLIENT, ci: ['web'] }],
        ['admin-only', 'admin', { outputs: { lane: 'code', web: 'false', edge: 'false' }, gate: [...lintTouched('apps/admin'), build('apps/admin'), typecheck('apps/admin'), changed('apps/admin'), I18N, JOURNEYS], ci: [] }],
        ['client file no edge test imports, named like one they do', 'namesake', { outputs: CLIENT_ONLY, gate: CLIENT, ci: ['web'] }],
        ['client file an edge test reads by path (?raw)', 'contract', { outputs: CODE_ALL, gate: APP_AND_EDGE, ci: ALL_CI }],
        ['client file an edge test imports by path', 'channel', { outputs: CODE_ALL, gate: APP_AND_EDGE, ci: ALL_CI }],
        [
          'file imported by path from a file another package imports by path',
          'chain',
          { outputs: CODE_ALL, gate: [...lintTouched('packages/languages'), build(...EVERY_PACKAGE), typecheck('packages/i18n', 'packages/languages', 'packages/shared'), ...EVERY_PACKAGE.map(changed), I18N, BALANCE], ci: ALL_CI },
        ],
        ['renamed client file an edge test still imports by its old path', 'renamed', { outputs: CODE_ALL, gate: APP_AND_EDGE, ci: ALL_CI }],
        ['infra file moved into .claude', 'movedOutOfInfra', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ['edge script an infra test imports by path', 'guard', { outputs: EDGE_ONLY, gate: [...lintTouched('apps/edge'), build('apps/edge', 'infra'), typecheck('apps/edge', 'infra'), changed('apps/edge'), changed('infra'), I18N], ci: ['bundle', 'smoke'] }],
        ['edge-only', 'edge', { outputs: EDGE_ONLY, gate: EDGE, ci: ['bundle', 'smoke'] }],
        ['edge file with a manifest change outside dependencies and peerDependencies, which adds nothing to it', 'edgeAndRootScript', { outputs: EDGE_ONLY, gate: EDGE, ci: ['bundle', 'smoke'] }],
        [
          'shared package',
          'shared',
          { outputs: CODE_ALL, gate: [...lintTouched('packages/shared'), build('apps/admin', 'apps/app', 'apps/edge', 'infra', 'packages/shared'), typecheck('packages/shared'), ...['apps/admin', 'apps/app', 'apps/edge', 'infra', 'packages/shared'].map(changed)], ci: ALL_CI },
        ],
        [
          'i18n package',
          'i18n',
          { outputs: CODE_ALL, gate: [...lintTouched('packages/i18n'), build('apps/app', 'apps/edge', 'infra', 'packages/i18n'), typecheck('packages/i18n'), ...['apps/app', 'apps/edge', 'infra', 'packages/i18n'].map(changed), I18N], ci: ALL_CI },
        ],
        [
          'wordgrid engine',
          'wordgrid',
          { outputs: CLIENT_ONLY, gate: [...lintTouched('packages/game-wordgrid'), build('apps/app', 'infra', 'packages/game-wordgrid'), typecheck('packages/game-wordgrid'), ...['apps/app', 'infra', 'packages/game-wordgrid'].map(changed), BALANCE], ci: ['web'] },
        ],
    ]);
  });

  it('an infra change gets its scope, the gate steps it reaches and the CI-only steps it turns on', () => {
    expectScopes([
        ["app e2e file, which infra's tests read through paths no import shows", 'walk', { outputs: CLIENT_ONLY, contains: changed('infra') }],
        ['infra script nothing uses by path', 'infra', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ['infra script only another workflow runs', 'releaseScript', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ['workflow other than checks.yml', 'workflow', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ['Markdown under infra', 'infraDoc', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ['Workers bundle script', 'edgeDeploy', { outputs: EDGE_ONLY, gate: INFRA_BUNDLE, ci: ['bundle', 'smoke'] }],
        ['script the Workers bundle script sources', 'gatePassed', { outputs: EDGE_ONLY, gate: INFRA_BUNDLE, ci: ['bundle', 'smoke'] }],
        [
          'infra/verify file',
          'verify',
          { outputs: CODE_ALL, gate: [...lintTouched('infra'), build('apps/app', 'apps/edge', 'infra'), typecheck('apps/app', 'apps/edge', 'infra'), changed('apps/app'), changed('apps/edge'), changed('infra'), I18N, JOURNEYS], ci: ALL_CI },
        ],
        [
          'infra file a package config reads by quoted path',
          'reporter',
          { outputs: { lane: 'code', web: 'false', edge: 'false' }, gate: [...lintTouched('infra'), build('apps/admin', 'infra'), typecheck('apps/admin', 'infra'), changed('apps/admin'), changed('infra'), I18N, JOURNEYS], ci: [] },
        ],
        ["own-tmpdir.mjs, which every package's test script runs by path", 'ownTmpdir', { outputs: CODE_ALL, tested: EVERY_PACKAGE, ci: ALL_CI }],
        ["the module own-tmpdir.mjs imports, which every package's test script runs", 'ownTmpdirLib', { outputs: CODE_ALL, tested: EVERY_PACKAGE, ci: ALL_CI }],
    ]);
  });

  it('a root file or manifest change gets its scope, the gate steps it reaches and the CI-only steps it turns on', () => {
    expectScopes([
        ['global root file', 'root', { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }],
        ...['lockfile', 'rootManifest', 'appManifest', 'infraManifest', 'workspace', 'npmrc'].map((name) => [`still-global ${BRANCHES[name][0]}`, name, { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }]),
        ['root file the rules do not name', 'other', { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }],
        ['root manifest whose scripts changed', 'rootScript', { outputs: LINT_ONLY, gate: [LINT], ci: [] }],
        ['manifest whose devDependencies changed', 'appDevDependency', { outputs: LINT_ONLY, gate: [LINT], ci: [] }],
        ['manifest whose dependencies were only reordered', 'appReorderedDependencies', { outputs: LINT_ONLY, gate: [LINT], ci: [] }],
        ['manifest whose dependencies changed', 'appDependency', { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }],
        ['manifest whose peerDependencies changed', 'appPeerDependency', { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }],
        ['manifest with dependencies deleted', 'edgeManifestDeleted', { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }],
        ['manifest without dependencies deleted', 'sharedManifestDeleted', { outputs: LINT_ONLY, gate: [LINT], ci: [] }],
        ['new manifest without dependencies', 'newManifestWithoutDependencies', { outputs: LINT_ONLY, gate: [LINT], ci: [] }],
        ['manifest changed into one that cannot be read', 'languagesManifestUnreadable', { outputs: FULL, gate: FULL_GATE, ci: ALL_CI }],
        ['manifest that could not be read on the base side counts as global, whatever it reads now', 'languagesManifestReadableAgain', { env: () => ({ BASE: git('rev-parse', 'languagesManifestUnreadable') }), outputs: FULL, gate: FULL_GATE }],
    ]);
  });

  it('a change to the gate, checks.yml, the lint configs, nothing at all, or one planned without grep gets its scope and gate steps', () => {
    expectScopes([
        ['gate.sh, which CI runs only from main', 'gateScript', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ["PR that narrows gate.sh to lint: main's gate.sh decides, never the PR's own", 'narrowed', { outputs: INFRA_LANE, gate: INFRA_ONLY }],
        ['checks.yml, which CI runs only from main', 'checksWorkflow', { outputs: INFRA_LANE, gate: INFRA_ONLY, ci: [] }],
        ['.oxlintrc.json, which lint and the infra lint-rule tests read', 'oxlint', { outputs: INFRA_LANE, gate: [LINT, build('infra'), typecheck('infra'), whole('infra')], ci: [] }],
        ['.mergify.yml, which only the infra workflow tests read', 'mergify', { outputs: INFRA_LANE, gate: [...lintTouched('infra'), build('infra'), typecheck('infra'), whole('infra')], ci: [] }],
        ['change to nothing', 'empty', { outputs: { lane: 'none', web: 'false', edge: 'false' }, gate: [], ci: [] }],
        ['full-lane PR, planned without looking for files that import its changes', 'rootAndContract', { env: () => ({ FAKE_GIT_GREP_FAILS: '1' }), outputs: FULL, gate: FULL_GATE }],
    ]);
  });

  it('a carried merge-queue update, a lockfile change and a scope that cannot be computed get their scope, gate steps and CI-only steps', () => {
    expectScopes([
        ...[
          ['client', 'client'],
          ['edge', 'edge'],
          ['global root file', 'root'],
        ].map(([kind, name]) => [`merge-queue update the carry step carried, even for a ${kind} change`, name, { env: () => ({ CARRIED: 'true' }), outputs: LINT_ONLY, gate: [LINT], ci: [] }]),
        ['lockfile change to the root importer', 'lockRoot', { env: () => YAML_FROM_THE_REPO, outputs: FULL, gate: FULL_GATE }],
        ['lockfile change to its settings', 'lockSettings', { env: () => YAML_FROM_THE_REPO, outputs: FULL, gate: FULL_GATE }],
        ['lockfile importer whose folder has no manifest', 'lockImporterWithoutPackage', { env: () => YAML_FROM_THE_REPO, outputs: LINT_ONLY, gate: [LINT] }],
        ['lockfile that only changed its layout', 'lockReformatted', { env: () => YAML_FROM_THE_REPO, outputs: LINT_ONLY, gate: [LINT] }],
        ['lockfile the gate cannot parse, for want of its YAML parser', 'lockAppDevDependency', { env: () => ({ NODE_PATH: '' }), outputs: FULL }],
        ['lockfile change to a devDependency of one package', 'lockAppDevDependency', { env: () => YAML_FROM_THE_REPO, outputs: CLIENT_ONLY, whole: ['apps/app', 'infra'] }],
        ['lockfile change to a version two levels under one package', 'lockTransitive', { env: () => YAML_FROM_THE_REPO, outputs: CLIENT_ONLY, whole: ['apps/app', 'infra', 'packages/game-wordgrid', 'packages/languages'] }],
        ['lockfile change to only the integrity of a package two levels under one package', 'lockIntegrity', { env: () => YAML_FROM_THE_REPO, outputs: CLIENT_ONLY, whole: ['apps/app', 'infra', 'packages/game-wordgrid', 'packages/languages'] }],
        ...[
          ['a base it cannot read', { BASE: 'no-such-ref-par30' }],
          ['pnpm failing to list what the change reaches', { FAKE_PNPM_LS_FAILS: '1' }],
          ['git grep failing while it looks for files that import a changed file', { FAKE_GIT_GREP_FAILS: '1' }],
        ].map(([kind, env]) => [`client change whose scope cannot be computed (${kind})`, 'client', { env: () => env, outputs: {}, says: CANNOT, gate: FULL_GATE, ci: ALL_CI }]),
    ]);
  });

  it('a change to a file a checks-run step runs or sources by path turns on the step gated on its scope output', () => {
    expect(RUN_BY_SCOPED_STEPS.length).toBeGreaterThan(0);
    for (const [index, [file, output]] of RUN_BY_SCOPED_STEPS.entries()) {
      expect.soft(pr(`scopedStep${index}`).outputs[output], `${file} turns on the ${output} step`).toBe('true');
    }
  });

  const CAPPED_FULL = FULL_GATE.map((command) => (command === `${EACH} run test` ? `${EACH} --workspace-concurrency=1 run test` : command));
  const fullAtOnce = (count) => FULL_GATE.map((command) => (command === `${EACH} run test` ? `${EACH} --workspace-concurrency=${count} --no-sort --aggregate-output --reporter-hide-prefix run test` : command));
  const inPackage = (command) => tested([command]).length > 0;
  const anyTestOrder = (gate) => {
    const runs = [];
    for (const command of gate) {
      const last = runs.at(-1);
      if (last && inPackage(command) && inPackage(last[0])) last.push(command);
      else runs.push([command]);
    }
    return runs.flatMap((run) => run.sort());
  };

  const onRunner = (name, runner) => {
    git('checkout', '-q', name);
    const workflow = { ...CHECKS.env, ...test.env };
    const workers = runStep(workersStep, { ...runner, ...workflow });
    expect(workers.status, `${name} ${JSON.stringify(runner)}: ${workers.stderr}`).toBe(0);
    const scope = runStep(scopeStep, { BASE: main, CARRIED: 'false' });
    const gate = runStep(gateStep, { ...runner, ...workers.exported, ...workflow, BASE: main, CARRIED: 'false', LANE: scope.outputs.lane ?? '' });
    return { said: workers.stdout, status: gate.status, output: gate.stdout + gate.stderr, gate: gate.pnpm, workers: gate.workers };
  };

  it("checks-run runs 2 packages' tests at once on at most 3 workers each unless the runner sets its own TEST_MAX_WORKERS or TEST_PACKAGES_AT_ONCE, and says so", () => {
    for (const [label, runner, said, atOnce, workers] of [
      ['a runner that sets none', {}, 'at most 2 packages at once, each on at most 3 test workers', 2, '3'],
      ['a runner that sets TEST_MAX_WORKERS empty', { TEST_MAX_WORKERS: '' }, 'at most 2 packages at once, each on at most 3 test workers', 2, '3'],
      ['the big-* runners’ TEST_MAX_WORKERS of 4', { TEST_MAX_WORKERS: '4' }, 'at most 2 packages at once, each on at most 4 test workers', undefined, '4'],
      ['a runner TEST_PACKAGES_AT_ONCE of 3', { TEST_PACKAGES_AT_ONCE: '3' }, 'at most 3 packages at once, each on at most 3 test workers', 3, undefined],
    ]) {
      const run = onRunner('client', runner);
      expect.soft(run.status, `${label}: ${run.output}`).toBe(0);
      expect.soft(run.said, label).toContain(`checks-run: ${said}`);
      expect.soft(anyTestOrder(run.gate), label).toEqual(anyTestOrder(CLIENT));
      if (workers) expect.soft(new Set(run.workers), label).toEqual(new Set([workers]));
      if (atOnce) {
        expect.soft(run.output, label).toMatch(new RegExp(`, ${atOnce} at once$`, 'm'));
        expect.soft(onRunner('root', runner).gate, label).toEqual(fullAtOnce(atOnce));
      }
    }
  });

  it('a gate step given TEST_MAX_WORKERS runs one package at a time on at most that many workers, and given TEST_PACKAGES_AT_ONCE runs that many packages at once: the full lane through pnpm with each package printed whole, the code lane through check-touched.mjs', () => {
    for (const cap of ['4', '7']) {
      for (const [name, gate] of [
        ['client', CLIENT],
        ['root', CAPPED_FULL],
      ]) {
        const run = pr(name, { TEST_MAX_WORKERS: cap });
        expect.soft(run.gate, `${name}, TEST_MAX_WORKERS ${cap}`).toEqual(gate);
        expect.soft(new Set(run.workers), `${name}, TEST_MAX_WORKERS ${cap}`).toEqual(new Set([cap]));
      }
    }
    for (const count of ['1', '4']) {
      const label = `TEST_PACKAGES_AT_ONCE ${count}`;
      expect.soft(pr('root', { TEST_MAX_WORKERS: '2', TEST_PACKAGES_AT_ONCE: count }).gate, label).toEqual(fullAtOnce(count));
      git('checkout', '-q', 'client');
      const gate = runStep(gateStep, { BASE: main, CARRIED: 'false', LANE: 'code', TEST_PACKAGES_AT_ONCE: count });
      expect.soft(gate.status, `${label}: ${gate.stdout}${gate.stderr}`).toBe(0);
      expect.soft(gate.stdout, label).toMatch(count === '1' ? /^check: gate tests against \S+, \d+ changed files, reach [^,]+$/m : new RegExp(`, ${count} at once$`, 'm'));
      expect.soft(anyTestOrder(gate.pnpm), label).toEqual(anyTestOrder(CLIENT));
    }
  });

  it('a TEST_MAX_WORKERS or TEST_PACKAGES_AT_ONCE that is not a whole number of at least 1 fails checks-run and the gate before any step runs', () => {
    for (const knob of ['TEST_MAX_WORKERS', 'TEST_PACKAGES_AT_ONCE']) {
      const run = onRunner('client', { [knob]: 'four' });
      expect.soft(run.status, `runner ${knob}`).not.toBe(0);
      expect.soft(run.output, `runner ${knob}`).toContain(`${knob} must be a whole number of at least 1`);
      expect.soft(run.gate, `runner ${knob}`).toEqual([]);
      for (const cap of ['0', '-1', '2.5', 'three', '3 ']) {
        const label = `gate ${knob} ${JSON.stringify(cap)}`;
        git('checkout', '-q', 'client');
        const gate = runStep(gateStep, { BASE: main, LANE: 'code', [knob]: cap });
        expect.soft(gate.status, label).not.toBe(0);
        expect.soft(gate.stdout + gate.stderr, label).toContain(`${knob} must be a whole number of at least 1`);
        expect.soft(gate.pnpm, label).toEqual([]);
      }
    }
  });

  it('checks stays the one required check: it follows checks-run whatever the lane and never reads the scope', () => {
    expect(checks.name).toBe('checks');
    expect(checks.needs).toEqual(['lane', 'test']);
    expect(JSON.stringify(checks)).not.toContain('steps.scope');
    expect(checks.steps.map((step) => step.if)).toEqual([undefined, ...Array(4).fill("${{ !cancelled() && env.POST == 'true' }}")]);
    const exit = (TEST, DOCS = '') => spawnSync('bash', ['-e', '-c', checks.steps[0].run], { env: { PATH: process.env.PATH, TEST, DOCS }, encoding: 'utf8' }).status;
    expect(exit('success')).toBe(0);
    expect(exit('skipped')).toBe(1);
    expect(exit('failure')).toBe(1);
    expect(exit('skipped', 'false')).toBe(1);
    expect(exit('skipped', 'true')).toBe(0);
    expect(exit('failure', 'true')).toBe(1);
    expect(exit('cancelled', 'true')).toBe(1);
  });

  it('a failing gate fails checks-run', () => {
    git('checkout', '-q', 'client');
    const failing = join(root, 'failing-bin');
    mkdirSync(failing, { recursive: true });
    writeFileSync(join(failing, 'pnpm'), '#!/bin/sh\nexit 1\n', { mode: 0o755 });
    const gate = runStep(gateStep, { BASE: main, LANE: 'lint', PATH: `${failing}:${process.env.PATH}` });
    expect(gate.status).not.toBe(0);
  });
});
