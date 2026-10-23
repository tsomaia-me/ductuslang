/**
 * The fast lane's one definition (CD-269, owner Q43 A, 8 Oct): a PR is harness-only when every
 * file it changes against the merge base is under `.claude/` or is `CLAUDE.md`.
 * Exit 3 is the only fast lane: anything else, a crash included, is the full run.
 */
import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { runsGateMain } from '../deploy-ref.mjs';
import { COMPARE_FILE_LIMIT, DOCS_ONLY, HARNESS_ONLY, changedSinceMergeBase, comparedPaths, docsOnly, harnessOnly, isHarnessPath } from '../harness-only.mjs';

import { DELETED, GIT_ENV, MOVED, PULL_REQUESTS, harnessFixture } from './harness-fixture.mjs';
import { childEnv } from './hermetic.mjs';

const SCRIPT = fileURLToPath(new URL('../harness-only.mjs', import.meta.url));
const GATE_MAIN = fileURLToPath(new URL('../../../.github/workflows/gate-main.yml', import.meta.url));

const HARNESS_PATHS = ['.claude/hooks/ground.sh', '.claude/skills/orchestrator/SKILL.md', '.claude/settings.json', '.claude/docs/CODING.md', 'CLAUDE.md'];

const DEPLOYS_NOTHING_ONLY = [
  'README.md',
  '.github/workflows/checks.yml',
  '.github/workflows/journeys.yml',
  '.github/workflows/gate-main.yml',
  'infra/scripts/__tests__/harness-only.test.mjs',
  'apps/edge/src/__tests__/wallet.ts',
  'apps/admin/src/routes/users.test.tsx',
  'packages/shared/src/csv.test.ts',
  'infra/verify/qa-plan.mjs',
  'infra/verify/__tests__/journeys-map.test.mjs',
  'infra/README.md',
  'docs/ARCHITECTURE.md',
  'docs/papercuts.md',
];

const RELEASED_PATHS = [
  'apps/app/CLAUDE.md',
  'apps/app/src/x.ts',
  'apps/edge/README.md',
  'harness/gate.mjs',
  'CLAUDE.md.bak',
  'harness-tools/x.mjs',
  '.claude-x/y',
  'infra/scripts/harness-only.mjs',
  'infra/scripts/edge-deploy.sh',
  'infra/scripts/gate.sh',
  '.github/workflows/release.yml',
  '.mergify.yml',
  'package.json',
  'apps/app/src/testing.ts',
];

describe('harnessOnly (CD-269)', () => {
  it('a path is harness only under .claude/ or as the root CLAUDE.md, and a pull request is harness-only only when every file is harness, never for tests alone or an empty diff', () => {
    for (const path of HARNESS_PATHS) expect.soft(isHarnessPath(path), path).toBe(true);
    for (const path of [...DEPLOYS_NOTHING_ONLY, ...RELEASED_PATHS]) expect.soft(isHarnessPath(path), path).toBe(false);
    expect.soft(harnessOnly(['packages/shared/src/csv.test.ts', 'apps/edge/src/__tests__/wallet.ts']).harness, 'tests alone').toBe(false);
    expect.soft(harnessOnly(PULL_REQUESTS.harness).harness, 'harness').toBe(true);
    expect.soft(harnessOnly(PULL_REQUESTS.mixed), 'mixed').toEqual({ harness: false, why: 'infra/scripts/x.mjs is not harness: everything runs' });
    expect.soft(harnessOnly(PULL_REQUESTS.product).harness, 'product').toBe(false);
    expect.soft(harnessOnly([]).harness, 'empty').toBe(false);
  });
});

describe("gate-main's push filter: the release skip", () => {
  it('starts a release on a push to main for every released path and for no harness or deploys-nothing path', () => {
    const push = parse(readFileSync(GATE_MAIN, 'utf8')).on.push;
    expect.soft(push.branches).toEqual(['main']);
    expect.soft(push['paths-ignore']).toBeUndefined();
    for (const path of [...HARNESS_PATHS, ...DEPLOYS_NOTHING_ONLY]) expect.soft(runsGateMain(push.paths, path), path).toBe(false);
    for (const path of RELEASED_PATHS) expect.soft(runsGateMain(push.paths, path), path).toBe(true);
  });
});

describe('harness-only.mjs against the merge base', () => {
  let fixture;
  beforeAll(() => {
    fixture = harnessFixture();
  });
  afterAll(() => rmSync(fixture.root, { recursive: true, force: true }));

  const run = (...args) => spawnSync('node', [SCRIPT, ...args], { cwd: fixture.repo, encoding: 'utf8', env: { ...process.env, ...GIT_ENV } });

  it('reads the PR alone, not what main did since it branched, naming both sides of a move (no rename detection) and a deleted file', () => {
    fixture.checkout('harness');
    expect.soft(changedSinceMergeBase(fixture.main, 'HEAD', fixture.repo).sort(), 'harness').toEqual([...PULL_REQUESTS.harness].sort());
    fixture.checkout('moved');
    expect.soft(changedSinceMergeBase(fixture.main, 'HEAD', fixture.repo).sort(), 'moved').toEqual(MOVED);
    fixture.checkout('deleted');
    expect.soft(changedSinceMergeBase(fixture.main, 'HEAD', fixture.repo), 'deleted').toEqual(DELETED);
  });

  it('exits 3 only for a harness-only branch, 0 for the others, never 3 or 0 for a base it cannot read, and 2 without a base', () => {
    for (const [branch, status] of [
      ['harness', HARNESS_ONLY],
      ['mixed', 0],
      ['product', 0],
      ['moved', 0],
      ['deleted', 0],
    ]) {
      fixture.checkout(branch);
      const result = run(fixture.main, 'HEAD');
      expect.soft(result.status, `${branch}: ${result.stderr}`).toBe(status);
    }
    fixture.checkout('harness');
    const crash = run('no-such-ref-cd269', 'HEAD');
    expect.soft(crash.status, 'unreadable base').not.toBe(HARNESS_ONLY);
    expect.soft(crash.status, 'unreadable base').not.toBe(0);
    expect.soft(run().status, 'no base').toBe(2);
  });
});

describe('docsOnly: the docs-only lane (CD-436)', () => {
  it('only when every changed path is Markdown, and never for an empty list', () => {
    expect(docsOnly(['.claude/docs/running.md', 'README.md', 'apps/app/README.md'])).toBe(true);
    expect(docsOnly(['.claude/docs/running.md', 'infra/scripts/gate.mjs', 'README.md'])).toBe(false);
    expect(docsOnly(['.claude/hooks/x.mjs'])).toBe(false);
    expect(docsOnly(['docs/x.md.bak'])).toBe(false);
    expect(docsOnly(['docs/md'])).toBe(false);
    expect(docsOnly([])).toBe(false);
  });

  it('reads both sides of a rename from a comparison, and no list from a truncated one', () => {
    const file = (filename, extra = {}) => ({ filename, status: 'modified', ...extra });
    expect(comparedPaths({ files: [file('a.md'), file('b.md', { status: 'renamed', previous_filename: 'b.mjs' })] })).toEqual(['a.md', 'b.md', 'b.mjs']);
    expect(comparedPaths({ files: Array.from({ length: COMPARE_FILE_LIMIT }, (_, index) => file(`d/${index}.md`)) })).toEqual([]);
    expect(comparedPaths({})).toEqual([]);
  });
});

describe('harness-only.mjs --docs-only, on a comparison read from stdin', () => {
  const run = (input) => spawnSync('node', [SCRIPT, '--docs-only'], { input, encoding: 'utf8' });
  const compare = (...pairs) => JSON.stringify({ files: pairs.map(([filename, previous_filename]) => ({ filename, status: previous_filename ? 'renamed' : 'modified', previous_filename })) });

  it('exits 3 only for Markdown alone, 0 for a script among it, renamed to it, or no file, and never 3 or 0 for a comparison it cannot read', () => {
    for (const [label, input, status] of [
      ['Markdown only', compare(['.claude/docs/running.md'], ['apps/app/README.md']), DOCS_ONLY],
      ['a script among Markdown', compare(['.claude/docs/running.md'], ['infra/scripts/x.mjs']), 0],
      ['a script renamed to Markdown', compare(['notes.md', 'notes.mjs']), 0],
      ['no file', compare(), 0],
    ]) {
      const result = run(input);
      expect.soft(result.status, `${label}: ${result.stderr}`).toBe(status);
    }
    const result = run('{"files": [');
    expect.soft(result.status, 'unreadable').not.toBe(DOCS_ONLY);
    expect.soft(result.status, 'unreadable').not.toBe(0);
    expect.soft(run('').status, 'empty input').not.toBe(DOCS_ONLY);
  });
});

describe.each(['checks.yml'])('%s: the lane job asks the comparison of the PR head and its base, and says docs=true only for Markdown alone', (name) => {
  const { jobs } = parse(readFileSync(fileURLToPath(new URL(`../../../.github/workflows/${name}`, import.meta.url)), 'utf8'));
  const step = jobs.lane.steps.find((each) => each.id === 'docs');
  let dir;
  beforeAll(() => {
    dir = mkdtempSync(join(tmpdir(), 'docs-lane-'));
    mkdirSync(join(dir, 'tools/infra/scripts'), { recursive: true });
    mkdirSync(join(dir, 'bin'));
    cpSync(SCRIPT, join(dir, 'tools/infra/scripts/harness-only.mjs'));
  });
  afterAll(() => rmSync(dir, { recursive: true, force: true }));

  const lane = (curl) => {
    writeFileSync(join(dir, 'bin/curl'), `#!/bin/sh\nprintf '%s\\n' "$*" >"${dir}/curl-args"\n${curl}\n`, { mode: 0o755 });
    const output = join(dir, 'output');
    writeFileSync(output, '');
    const env = { ...childEnv(), PATH: `${join(dir, 'bin')}:${process.env.PATH}`, GITHUB_OUTPUT: output, GITHUB_API_URL: 'https://api.test', GITHUB_REPOSITORY: 'o/r', GITHUB_TOKEN: 'tok', BASE_SHA: 'b0', HEAD_SHA: 'h1' };
    const result = spawnSync('bash', ['-e', '-c', step.run], { cwd: dir, env, encoding: 'utf8' });
    return { status: result.status, output: readFileSync(output, 'utf8'), args: readFileSync(join(dir, 'curl-args'), 'utf8').trim() };
  };
  const reply = (...filenames) => `cat <<'JSON'\n${JSON.stringify({ files: filenames.map((filename) => ({ filename, status: 'modified' })) })}\nJSON`;

  it('docs=true for Markdown alone, from the comparison of the base and the head shas; docs=false for a script among the Markdown, and when the API call fails', () => {
    expect.soft(lane(reply('.claude/docs/running.md', 'apps/app/README.md')), 'Markdown alone').toEqual({
      status: 0,
      output: 'docs=true\n',
      args: '-fsS -H Authorization: Bearer tok -H Accept: application/vnd.github+json https://api.test/repos/o/r/compare/b0...h1',
    });
    expect.soft(lane(reply('.claude/docs/running.md', 'infra/scripts/x.mjs')).output, 'a script').toBe('docs=false\n');
    expect.soft(lane('echo boom >&2; exit 1').output, 'the API fails').toBe('docs=false\n');
  });

  it('runs only on a pull request, with a token that reads', () => {
    expect(step.if).toBe("github.event_name == 'pull_request_target'");
    expect(step.env.GITHUB_TOKEN).toBe('${{ github.token }}');
    expect(jobs.lane.permissions).toEqual({ contents: 'read' });
    expect(jobs.lane.outputs.docs).toBe('${{ steps.docs.outputs.docs }}');
  });
});
