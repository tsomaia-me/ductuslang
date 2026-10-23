import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { baseRef, gateArgs, gateTests, lintable, localArgs, namedTests, ownerPackage, plan, readPlan, runAtOnce, runSteps } from '../check-touched.mjs';

import { GIT_ENV } from './harness-fixture.mjs';
import { assertStubsRun, childEnv } from './hermetic.mjs';

const BASE = 'abc1234';

const tree = (files) => {
  const paths = new Set(Object.keys(files));
  return {
    has: (path) => paths.has(path),
    readJson: (path) => files[path],
  };
};

const WORKSPACE = tree({
  'package.json': { name: 'root' },
  'apps/admin/package.json': { name: '@wg/admin', scripts: { typecheck: 'tsc -p tsconfig.json --noEmit', test: 'vitest run' }, devDependencies: { vitest: '^5.0.0' } },
  'apps/admin/tsconfig.json': {},
  'apps/admin/src/screen.tsx': null,
  'apps/admin/src/i18n/en/media.json': null,
  'packages/plain/package.json': { name: '@wg/plain' },
  'packages/plain/tsconfig.json': {},
  'packages/plain/src/a.ts': null,
  'packages/untyped/package.json': { name: '@wg/untyped' },
  'packages/untyped/src/b.ts': null,
  'infra/package.json': { name: '@wg/infra', scripts: { test: 'sh scripts/infra-test.sh' }, devDependencies: { vitest: '^5.0.0' } },
  'infra/scripts/x.mjs': null,
  'README.md': null,
});

const TESTS = [
  { path: 'infra/scripts/__tests__/reads-workflows.test.mjs', text: "join(ROOT, '.github/workflows')" },
  { path: 'infra/scripts/__tests__/reads-agents.test.mjs', text: "join(ROOT, '.claude', 'agents')" },
  { path: 'infra/scripts/__tests__/calls-api.test.mjs', text: "fetch('https://api.github.com/repos')" },
  { path: 'infra/scripts/__tests__/other.test.mjs', text: "join(ROOT, 'apps/app')" },
  { path: 'infra/scripts/__tests__/runs-gate.test.mjs', text: "fileURLToPath(new URL('../gate.sh', import.meta.url))" },
  { path: 'infra/scripts/__tests__/globs-workflows.test.mjs', text: "globSync('.github/workflows/*.yml')" },
  { path: 'infra/scripts/__tests__/reads-release.test.mjs', text: "readFileSync('.github/workflows/release.yml')" },
  { path: 'infra/scripts/__tests__/walks-up.test.mjs', text: "join(here, '..', '..')" },
  { path: 'infra/scripts/__tests__/fake-repo.test.mjs', text: "mkdirSync(join(repo, 'apps'))" },
  { path: 'infra/scripts/__tests__/reads-mode.test.mjs', text: "readFileSync(join(ROOT, 'infra/ci/mode.json'))" },
];

describe('baseRef', () => {
  it('is origin/main without --since and the given commit under it, for a fix round on a branch whose earlier tip passed; it refuses an unknown argument, the retired --epic and a --since with no commit', () => {
    expect.soft(baseRef([]), 'no --since').toBe('origin/main');
    expect.soft(baseRef(['--since', 'f00dcafe']), '--since').toBe('f00dcafe');
    for (const args of [['--base', 'x'], ['--epic', 'claude/cd-400-epic'], ['--since'], ['--since', '']]) {
      expect.soft(() => baseRef(args), JSON.stringify(args)).toThrow(/usage/);
    }
  });
});

describe('localArgs', () => {
  it('reads --direct beside --since, in either order, for the base the plain run takes, and refuses --direct twice and every argument baseRef refuses', () => {
    for (const [args, read] of [
      [[], { ref: 'origin/main', direct: false }],
      [['--direct'], { ref: 'origin/main', direct: true }],
      [['--direct', '--since', 'f00dcafe'], { ref: 'f00dcafe', direct: true }],
      [['--since', 'f00dcafe', '--direct'], { ref: 'f00dcafe', direct: true }],
    ]) {
      expect.soft(localArgs(args), JSON.stringify(args)).toEqual(read);
    }
    for (const args of [['--direct', '--direct'], ['--direct', '--since'], ['--direct', '--base', 'x']]) {
      expect.soft(() => localArgs(args), JSON.stringify(args)).toThrow(/usage/);
    }
  });
});

describe('ownerPackage', () => {
  it('is the nearest folder above the path holding a package.json, and none for a file only the repo root owns', () => {
    for (const [path, owner] of [
      ['apps/admin/src/screen.tsx', 'apps/admin'],
      ['infra/scripts/x.mjs', 'infra'],
      ['README.md', null],
      ['docs/a/b.ts', null],
    ]) {
      expect.soft(ownerPackage(path, WORKSPACE.has), path).toBe(owner);
    }
  });
});

describe('lintable', () => {
  it('keeps the JavaScript and TypeScript files oxlint reads', () => {
    expect(lintable(['a.js', 'b.mjs', 'c.cjs', 'd.jsx', 'e.ts', 'f.mts', 'g.cts', 'h.tsx', 'i.md', 'j.json', 'k.sh', 'l.yml', 'm.d.ts'])).toEqual([
      'a.js',
      'b.mjs',
      'c.cjs',
      'd.jsx',
      'e.ts',
      'f.mts',
      'g.cts',
      'h.tsx',
      'm.d.ts',
    ]);
  });
});

describe('namedTests', () => {
  it('picks the infra tests that name the changed path, by its path from the test or a folder of it they read whole, a top folder only under .github or .claude, and none for a folder reached only by climbing with ..', () => {
    for (const [label, changed, picked] of [
      ['a .claude file a test reads the folder of', ['.claude/agents/builder.md'], ['infra/scripts/__tests__/reads-agents.test.mjs']],
      ['a workflow two tests read the folder of', ['.github/workflows/checks.yml'], ['infra/scripts/__tests__/reads-workflows.test.mjs', 'infra/scripts/__tests__/globs-workflows.test.mjs']],
      [
        'a workflow one test also names',
        ['.github/workflows/release.yml'],
        ['infra/scripts/__tests__/reads-workflows.test.mjs', 'infra/scripts/__tests__/globs-workflows.test.mjs', 'infra/scripts/__tests__/reads-release.test.mjs'],
      ],
      ['a script a test names by its path from the test, since a spawned script is outside the import graph', ['infra/scripts/gate.sh'], ['infra/scripts/__tests__/runs-gate.test.mjs']],
      ['files a test reaches only by climbing with ..', ['infra/scripts/other.sh', 'infra/x.mjs'], []],
      ['an app file, named only by the folder of a repo a test fakes', ['apps/app/src/a.ts'], ['infra/scripts/__tests__/other.test.mjs']],
      ['files no test names', ['packages/plain/src/a.ts', 'docs/.github-notes.md'], []],
    ]) {
      expect.soft(namedTests(changed, TESTS), label).toEqual(picked);
    }
  });
});

const names = (steps) => steps.map((step) => step.name);

describe('readPlan', () => {
  it('reads the lane and the packages the change reaches from gate.sh --plan, and refuses a plan that names no lane', () => {
    expect.soft(readPlan('lane=code\nweb=true\nedge=false\nreach=apps/app infra\ntouched=apps/app\nlint=apps/app\n')).toEqual({ lane: 'code', reach: ['apps/app', 'infra'], touched: ['apps/app'] });
    expect.soft(readPlan('lane=full\nweb=true\nedge=true\ntouched=apps/admin\n')).toEqual({ lane: 'full', reach: [], touched: ['apps/admin'] });
    expect.soft(readPlan('lane=lint\nweb=false\nedge=false\nreach=\n')).toEqual({ lane: 'lint', reach: [], touched: [] });
    expect.soft(() => readPlan('web=true\n')).toThrow(/lane/);
  });
});

describe('plan', () => {
  const local = (changed, lane, reach = [], testFiles = TESTS) => plan({ base: BASE, changed, lane, reach, ...WORKSPACE, testFiles });

  it('has no step for docs, lints the changed files that exist, checks the TypeScript ones for comments, types each package owning a TypeScript or JSON file, runs vitest by --changed in every reached package, runs the infra tests that name a changed .claude path, and runs every package whole when the plan cannot scope the change', () => {
    expect.soft(local(['README.md', '.claude/docs/running.md'], 'lint', [], []), 'docs only').toEqual([]);

    const steps = local(['apps/admin/src/screen.tsx', 'packages/plain/src/a.ts', 'packages/untyped/src/b.ts', 'packages/plain/src/gone.ts', 'infra/scripts/x.mjs', 'README.md'], 'code', ['apps/admin', 'infra']);
    expect.soft(names(steps), 'code').toEqual(['oxlint', 'comments', 'typecheck apps/admin', 'typecheck packages/plain', 'vitest apps/admin', 'vitest infra']);
    expect.soft(steps[0].command, 'code').toEqual(['pnpm', 'exec', 'oxlint', 'apps/admin/src/screen.tsx', 'packages/plain/src/a.ts', 'packages/untyped/src/b.ts', 'infra/scripts/x.mjs']);
    expect.soft(steps[1], 'code').toEqual({ name: 'comments', cwd: '.', command: ['node', 'infra/scripts/strip-comments.mjs', '--check', 'apps/admin/src/screen.tsx', 'packages/plain/src/a.ts', 'packages/untyped/src/b.ts'] });
    expect.soft(steps[2], 'code').toMatchObject({ cwd: 'apps/admin', command: ['pnpm', 'run', 'typecheck'] });
    expect.soft(steps[3], 'code').toMatchObject({ cwd: 'packages/plain', command: ['pnpm', 'exec', 'tsc', '--noEmit'] });
    expect.soft(steps[4]?.cwd, 'code').toBe('apps/admin');
    expect.soft(steps[4]?.command, 'code').toEqual(expect.arrayContaining(['vitest', 'run', '--changed', BASE, '--maxWorkers=4']));
    expect.soft(steps[5]?.cwd, 'code').toBe('infra');
    expect.soft(steps[5]?.command, 'code').toEqual(expect.arrayContaining(['--changed', BASE, '--maxWorkers=4']));

    expect.soft(names(local(['apps/admin/src/i18n/en/media.json'], 'code', ['apps/admin'])), 'a JSON file TypeScript imports').toEqual(['typecheck apps/admin', 'vitest apps/admin']);

    const dependent = local(['packages/plain/src/a.ts'], 'code', ['apps/admin', 'packages/plain']);
    expect.soft(names(dependent), 'a dependent package').toEqual(['oxlint', 'comments', 'typecheck packages/plain', 'vitest apps/admin']);
    expect.soft(dependent[3]?.command, 'a dependent package').toEqual(expect.arrayContaining(['--changed', BASE]));

    const named = local(['.claude/agents/builder.md'], 'lint');
    expect.soft(names(named), 'a .claude path').toEqual(['named tests']);
    expect.soft(named[0]?.cwd, 'a .claude path').toBe('infra');
    expect.soft(named[0]?.command, 'a .claude path').toEqual(['pnpm', 'exec', 'sh', 'scripts/infra-test.sh', 'scripts/__tests__/reads-agents.test.mjs', '--maxWorkers=4']);

    expect.soft(local(['pnpm-workspace.yaml'], 'full'), 'full').toEqual([{ name: 'test every package', cwd: '.', command: ['pnpm', '-r', '--if-present', '--workspace-concurrency=1', 'run', 'test', '--maxWorkers=4'] }]);
  });

  it('under --direct runs vitest by --changed in the touched packages only, never their dependents, the infra tests that name a changed .claude path, and the touched packages whole on lane full, not every package', () => {
    const direct = (changed, lane, reach, touched) => plan({ base: BASE, changed, lane, reach, touched, direct: true, ...WORKSPACE, testFiles: TESTS });
    const changed = ['packages/plain/src/a.ts', 'infra/scripts/x.mjs'];
    const reach = ['apps/admin', 'infra', 'packages/plain'];
    expect.soft(names(local(changed, 'code', reach)), 'the plain run').toEqual(['oxlint', 'comments', 'typecheck packages/plain', 'vitest apps/admin', 'vitest infra']);
    const steps = direct(changed, 'code', reach, ['infra', 'packages/plain']);
    expect.soft(names(steps), 'code').toEqual(['oxlint', 'comments', 'typecheck packages/plain', 'vitest infra']);
    expect.soft(steps[3]?.command, 'code').toEqual(expect.arrayContaining(['--changed', BASE, '--maxWorkers=4']));
    expect.soft(names(direct(['.claude/agents/builder.md'], 'lint', [], [])), 'a .claude path').toEqual(['named tests']);
    const full = direct(['pnpm-workspace.yaml', 'apps/admin/src/screen.tsx'], 'full', [], ['apps/admin']);
    expect.soft(names(full), 'full').toEqual(['oxlint', 'comments', 'typecheck apps/admin', 'test apps/admin']);
    expect.soft(full[3], 'full').toEqual({ name: 'test apps/admin', cwd: 'apps/admin', command: ['pnpm', 'run', 'test', '--maxWorkers=4'] });
    expect.soft(direct(['pnpm-workspace.yaml'], 'full', [], []), 'full, nothing touched').toEqual([]);
  });
});

describe('gateArgs', () => {
  it('reads the base and the reached packages, and refuses no base and no reached package', () => {
    expect.soft(gateArgs(['--gate', BASE, 'apps/admin', 'infra'])).toEqual({ base: BASE, reached: ['apps/admin', 'infra'] });
    expect.soft(() => gateArgs(['--gate'])).toThrow(/usage/);
    expect.soft(() => gateArgs(['--gate', BASE])).toThrow(/usage/);
  });
});

describe('gateTests', () => {
  const EDGE = tree({
    'apps/edge/package.json': { name: '@wg/edge', scripts: { test: 'node scripts/test.mjs' }, devDependencies: { vitest: '^5.0.0' } },
    'apps/edge/src/w.ts': null,
    'apps/admin/package.json': { name: '@wg/admin', scripts: { test: 'vitest run' }, devDependencies: { vitest: '^5.0.0' } },
    'apps/admin/src/screen.tsx': null,
    'packages/plain/package.json': { name: '@wg/plain', scripts: { test: 'vitest run' }, devDependencies: { vitest: '^5.0.0' } },
    'packages/plain/src/a.ts': null,
    'packages/grid/package.json': { name: '@wg/grid', scripts: { test: 'vitest run' }, devDependencies: { vitest: '^5.0.0' } },
    'packages/grid/src/g.ts': null,
    'packages/bare/package.json': { name: '@wg/bare' },
    'infra/package.json': { name: '@wg/infra', scripts: { test: 'sh scripts/infra-test.sh' }, devDependencies: { vitest: '^5.0.0' } },
    'infra/scripts/x.mjs': null,
  });
  const gate = (changed, reached) => gateTests({ base: BASE, changed, reached, ...EDGE, testFiles: TESTS });

  it('runs by --changed only the tests the change reaches in each reached package, with the workers the environment sets; whole a package whose tests vitest does not run, every package on a lockfile change, and infra when a config only its tests read changed; and beside infra the infra tests that name a changed .github, infra script or infra/ci path, unless infra runs whole', () => {
    const steps = gate(['apps/admin/src/screen.tsx', 'apps/edge/src/w.ts'], ['apps/admin', 'apps/edge']);
    expect.soft(names(steps), 'packages owning a change').toEqual(['vitest apps/admin', 'vitest apps/edge']);
    for (const step of steps) {
      expect.soft(step.command, step.name).toEqual(expect.arrayContaining(['--changed', BASE]));
      expect.soft(step.command.some((arg) => arg.startsWith('--maxWorkers')), step.name).toBe(false);
    }
    expect.soft(steps[0]?.command, 'admin').toEqual(expect.arrayContaining(['vitest', 'run']));
    expect.soft(steps[1]?.command, 'edge').toEqual(expect.arrayContaining(['node', 'scripts/test.mjs', '--changed', BASE]));
    expect.soft(steps[1]?.command, 'edge').not.toContain('vitest');

    const dependents = gate(['packages/plain/src/a.ts'], ['packages/bare', 'packages/grid', 'packages/plain']);
    expect.soft(names(dependents), 'a reached package owning no change').toEqual(['vitest packages/grid', 'vitest packages/plain']);
    expect.soft(dependents[0]?.command, 'a reached package owning no change').toEqual(expect.arrayContaining(['vitest', 'run', '--changed', BASE]));

    const odd = gateTests({ base: BASE, changed: ['packages/odd/src/o.ts'], reached: ['packages/odd'], ...tree({ 'packages/odd/package.json': { name: '@wg/odd', scripts: { test: 'node --test' } } }), testFiles: [] });
    expect.soft(odd, 'tests that run without vitest').toEqual([{ name: 'test packages/odd', cwd: 'packages/odd', command: ['pnpm', 'run', 'test'] }]);

    const infraTest = (...files) => ['pnpm', 'exec', 'sh', 'scripts/infra-test.sh', ...files.map((file) => `scripts/__tests__/${file}.test.mjs`)];
    for (const [label, changed, reached, picked, named] of [
      ['the lockfile', ['pnpm-lock.yaml'], ['packages/plain'], ['test packages/plain']],
      ['the lockfile beside a source', ['pnpm-lock.yaml', 'packages/plain/src/a.ts'], ['packages/plain'], ['test packages/plain']],
      ['a .github path', ['.github/workflows/checks.yml'], ['infra'], ['vitest infra', 'named tests'], infraTest('reads-workflows', 'globs-workflows')],
      ['an app path a test names', ['apps/app/src/a.ts'], ['infra'], ['vitest infra']],
      ['an infra script', ['infra/scripts/gate.sh'], ['infra'], ['vitest infra', 'named tests'], infraTest('runs-gate')],
      ['an infra test', ['infra/scripts/__tests__/other.test.mjs'], ['infra'], ['vitest infra']],
      ['an infra/ci file', ['infra/ci/mode.json'], ['infra'], ['vitest infra', 'named tests'], infraTest('reads-mode')],
      ['a config only infra tests read', ['.github/workflows/checks.yml', '.oxlintrc.json'], ['infra'], ['test infra']],
      ['infra whole, holding the named tests', ['apps/admin/src/screen.tsx', '.github/workflows/checks.yml', '.mergify.yml'], ['apps/admin', 'infra'], ['vitest apps/admin', 'test infra']],
    ]) {
      const planned = gate(changed, reached);
      expect.soft(names(planned), label).toEqual(picked);
      if (named) expect.soft(planned[1]?.command, label).toEqual(named);
    }
  });

  it('runs the tests related to a given file list in every reached package, by path from the package, through the edge and infra runners too', () => {
    const related = ['apps/edge/src/w.ts', 'infra/scripts/x.mjs'];
    const steps = gateTests({ base: BASE, changed: related, related, reached: ['apps/admin', 'apps/edge', 'infra'], ...EDGE, testFiles: [] });
    expect(steps.map((step) => step.command)).toEqual([
      ['pnpm', 'exec', 'node', '../../infra/scripts/own-tmpdir.mjs', 'vitest', 'related', '../edge/src/w.ts', '../../infra/scripts/x.mjs', '--run', '--passWithNoTests'],
      ['pnpm', 'exec', 'node', '../../infra/scripts/own-tmpdir.mjs', 'node', 'scripts/test.mjs', 'related', 'src/w.ts', '../../infra/scripts/x.mjs', '--run'],
      ['pnpm', 'exec', 'sh', 'scripts/infra-test.sh', 'related', '../apps/edge/src/w.ts', 'scripts/x.mjs', '--run', '--passWithNoTests'],
    ]);
  });
});

describe('check-touched --since on a branch that merged main after the given commit', { timeout: 60_000 }, () => {
  const CHECK = fileURLToPath(new URL('../check-touched.mjs', import.meta.url));
  const GATE = fileURLToPath(new URL('../gate.sh', import.meta.url));
  const PACKAGE = { scripts: { test: 'vitest run' }, devDependencies: { vitest: '^5.0.0' } };
  const STRIP_STUB = "import { appendFileSync } from 'node:fs';\nappendFileSync(process.env.PNPM_LOG, `strip-comments ${process.argv.slice(2).join(' ')}\\n`);\n";
  const PNPM = [
    '#!/bin/sh',
    'if [ "$1" = ls ]; then',
    '  for arg; do case "$arg" in "...{./"*) dir=${arg#"...{./"}; echo "$(pwd -P)/${dir%\\}}" ;; esac; done',
    '  exit 0',
    'fi',
    'printf "%s\\n" "$*" >>"$PNPM_LOG"',
    '',
  ].join('\n');
  let root;
  let repo;
  let bin;
  let earlier;
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: repo, encoding: 'utf8', env: { ...childEnv(), ...GIT_ENV } });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr}`);
    return result.stdout.trim();
  };
  const commit = (files) => {
    for (const [path, text] of Object.entries(files)) {
      mkdirSync(join(repo, dirname(path)), { recursive: true });
      writeFileSync(join(repo, path), text);
    }
    git('add', '-A');
    git('commit', '-q', '-m', Object.keys(files).join(' '));
  };

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'check-touched-since-'));
    repo = join(root, 'tested');
    bin = join(root, 'bin');
    mkdirSync(join(repo, 'infra/scripts/__tests__'), { recursive: true });
    mkdirSync(bin);
    git('init', '-q', '-b', 'main');
    commit({
      'package.json': '{}\n',
      'infra/scripts/gate.sh': readFileSync(GATE, 'utf8'),
      'infra/scripts/strip-comments.mjs': STRIP_STUB,
      'packages/a/package.json': JSON.stringify({ name: '@wg/a', ...PACKAGE }),
      'packages/a/src/a.ts': 'export const a = 1;\n',
      'packages/b/package.json': JSON.stringify({ name: '@wg/b', ...PACKAGE }),
      'packages/b/src/b.ts': 'export const b = 1;\n',
    });
    git('checkout', '-q', '-b', 'merged');
    commit({ 'packages/a/src/a.ts': 'export const a = 2;\n' });
    earlier = git('rev-parse', 'HEAD');
    git('checkout', '-q', 'main');
    commit({ 'packages/b/src/b.ts': 'export const b = 2;\n' });
    git('update-ref', 'refs/remotes/origin/main', 'main');
    git('checkout', '-q', 'merged');
    git('merge', '-q', '--no-edit', 'main');
    git('checkout', '-q', '-b', 'fixed');
    commit({ 'packages/a/src/a.ts': 'export const a = 3;\n' });
    writeFileSync(join(bin, 'pnpm'), PNPM, { mode: 0o755 });
    assertStubsRun(bin);
  });
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const check = (branch, flags = []) => {
    git('checkout', '-q', branch);
    const log = join(root, `${branch}.log`);
    writeFileSync(log, '');
    const result = spawnSync('node', [CHECK, ...flags, '--since', earlier], {
      cwd: repo,
      encoding: 'utf8',
      env: childEnv({ ...GIT_ENV, PATH: `${bin}:${process.env.PATH}`, PNPM_LOG: log }),
    });
    expect(result.status, result.stdout + result.stderr).toBe(0);
    return { said: result.stdout, steps: readFileSync(log, 'utf8').split('\n').filter(Boolean) };
  };

  it('lints and tests only the file the branch changed since the commit, never the one the merge of main brought, with or without --direct, and runs nothing and says so when only the merge of main came after the commit', () => {
    const steps = ['exec oxlint packages/a/src/a.ts', 'strip-comments --check packages/a/src/a.ts', 'exec node ../../infra/scripts/own-tmpdir.mjs vitest related src/a.ts --run --maxWorkers=4 --passWithNoTests'];
    expect.soft(check('fixed').steps, 'fixed').toEqual(steps);
    expect.soft(check('fixed', ['--direct']).steps, 'fixed, --direct').toEqual(steps);
    const { said, steps: none } = check('merged');
    expect.soft(none, 'merged').toEqual([]);
    expect.soft(said, 'merged').toContain(`check: nothing changed since ${earlier} that differs from origin/main, so nothing to run`);
  });
});

describe('runSteps', () => {
  const step = (name) => ({ name, command: ['run', name] });

  it('runs every step after a failure and exits with the first failing code, and exits 0 saying so when there is nothing to run', async () => {
    const said = [];
    const codes = { lint: 0, types: 2, tests: 1 };
    const code = await runSteps([step('lint'), step('types'), step('tests')], async ({ name }) => codes[name], (line) => said.push(line));
    expect.soft(code, 'a failure').toBe(2);
    expect.soft(said.filter((line) => line.includes(' exit ')), 'a failure').toEqual(['check: lint exit 0', 'check: types exit 2', 'check: tests exit 1']);
    const quiet = [];
    expect.soft(await runSteps([], async () => 1, (line) => quiet.push(line)), 'nothing to run').toBe(0);
    expect.soft(quiet, 'nothing to run').toEqual(['check: nothing to lint, type or test in this change']);
  });
});

describe('runAtOnce', () => {
  const step = (name) => ({ name, command: ['run', name] });
  const controlled = () => {
    const started = [];
    const finish = {};
    const runStep = ({ name }) =>
      new Promise((done) => {
        started.push(name);
        finish[name] = (code, output) => done({ code, output });
      });
    return { started, finish, runStep };
  };
  const settle = () => new Promise((done) => setImmediate(done));

  it('runs at most the given number of steps at once and starts the next one as soon as one ends', async () => {
    const { started, finish, runStep } = controlled();
    const run = runAtOnce([step('admin'), step('app'), step('edge'), step('infra')], runStep, () => {}, 2);
    await settle();
    expect(started).toEqual(['admin', 'app']);
    finish.app(0, '');
    await settle();
    expect(started).toEqual(['admin', 'app', 'edge']);
    finish.edge(0, '');
    await settle();
    expect(started).toEqual(['admin', 'app', 'edge', 'infra']);
    finish.infra(0, '');
    finish.admin(0, '');
    expect(await run).toBe(0);
  });

  it("prints each step's output whole, between its command and its exit, in the order the steps end", async () => {
    const { finish, runStep } = controlled();
    const said = [];
    const run = runAtOnce([step('admin'), step('app')], runStep, (line) => said.push(line), 2);
    await settle();
    finish.app(0, 'app line 1\n::error file=a.ts::app line 2\n');
    await settle();
    finish.admin(0, 'admin line 1\nadmin line 2\n');
    expect(await run).toBe(0);
    expect(said).toEqual([
      'check: admin started',
      'check: app started',
      'check: app: run app',
      'app line 1\n::error file=a.ts::app line 2',
      'check: app exit 0',
      'check: admin: run admin',
      'admin line 1\nadmin line 2',
      'check: admin exit 0',
    ]);
  });

  it('runs every step past a failure, exits with the first failing code in step order and names every failed step, and exits 0 saying so when there is nothing to run', async () => {
    const said = [];
    const codes = { lint: 0, types: 2, tests: 1 };
    const code = await runAtOnce([step('lint'), step('types'), step('tests')], async ({ name }) => ({ code: codes[name], output: '' }), (line) => said.push(line), 3);
    expect.soft(code, 'a failure').toBe(2);
    expect.soft(said.filter((line) => line.includes(' exit ')).sort(), 'a failure').toEqual(['check: lint exit 0', 'check: tests exit 1', 'check: types exit 2']);
    expect.soft(said.at(-1), 'a failure').toBe('check: failed: types, tests');
    const quiet = [];
    expect.soft(await runAtOnce([], async () => ({ code: 1, output: '' }), (line) => quiet.push(line), 3), 'nothing to run').toBe(0);
    expect.soft(quiet, 'nothing to run').toEqual(['check: nothing to lint, type or test in this change']);
  });
});
