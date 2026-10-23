/**
 * The pull-request workflows and the merge queue (CD-269). The PR workflows run from main's
 * definitions (`pull_request_target`), check out the PR head commit by its sha, refuse forks,
 * run nothing for a draft, cancel a superseded run, and keep every token that writes out of the
 * jobs that run the PR's code. Mergify (`.mergify.yml`) queues on an agent's comment, tests one
 * PR at a time in place (main merged into the branch) and squash-merges it, on the checks the
 * workflows name.
 */
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { GIT_ENV, harnessFixture } from './harness-fixture.mjs';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const read = (name) => parse(readFileSync(join(ROOT, '.github/workflows', name), 'utf8'));

const PR_WORKFLOWS = ['checks.yml', 'pr-issue.yml'];
const HEAD = '${{ github.event.pull_request.head.sha }}';
const NOT_DRAFT = '!github.event.pull_request.draft';

const checkouts = (job) => (job.steps ?? []).filter((step) => String(step.uses ?? '').startsWith('actions/checkout@'));
const needsOf = (job) => (job.needs === undefined ? [] : [job.needs].flat());

/** A job runs nothing for a draft: its `if` says so, or it only follows jobs that do (and does not run regardless). */
const skipsDrafts = (jobs, id) => {
  const job = jobs[id];
  if (String(job.if ?? '').includes(NOT_DRAFT)) return true;
  if (/always\(\)|cancelled\(\)/.test(String(job.if ?? ''))) return false;
  return needsOf(job).length > 0 && needsOf(job).every((need) => skipsDrafts(jobs, need));
};

describe('the pull-request workflows run from main, on the PR head (CD-269)', () => {
  it('each pull-request workflow: pull_request_target on main, no pull_request trigger, ready_for_review and synchronize start it', () => {
    for (const name of PR_WORKFLOWS) {
      const { on } = read(name);
      expect.soft(Object.keys(on).sort(), name).toEqual(['pull_request_target']);
      expect.soft(on.pull_request_target.branches, name).toEqual(['main']);
      const extra = { 'pr-issue.yml': ['edited'] }[name] ?? [];
      expect.soft([...on.pull_request_target.types].sort(), name).toEqual(['opened', 'synchronize', 'reopened', 'ready_for_review', ...extra].sort());
    }
  });

  it('each pull-request workflow: one run per pull request, a newer one cancels the older; no token permission by default', () => {
    for (const name of PR_WORKFLOWS) {
      const workflow = read(name);
      expect.soft(workflow.concurrency.group, name).toContain('${{ github.event.pull_request.number');
      expect.soft(workflow.concurrency['cancel-in-progress'], name).toBe(true);
      expect.soft(workflow.permissions, name).toEqual({});
    }
  });

  it('each pull-request workflow: a draft runs nothing', () => {
    for (const name of PR_WORKFLOWS) {
      const { jobs } = read(name);
      for (const id of Object.keys(jobs)) expect.soft(skipsDrafts(jobs, id), `${name}:${id}`).toBe(true);
    }
  });

  it("each pull-request workflow: a fork is refused before anything is checked out; the job that refuses is what the others follow", () => {
    for (const name of PR_WORKFLOWS) {
      const { jobs } = read(name);
      for (const [id, job] of Object.entries(jobs)) {
        const steps = job.steps ?? [];
        const refusal = steps.findIndex((step) => step.name === 'Refuse a pull request from a fork');
        const first = steps.findIndex((step) => String(step.uses ?? '').startsWith('actions/checkout@'));
        if (refusal >= 0) {
          expect.soft(steps[refusal].if, `${name}:${id}`).toMatch(/github\.event\.pull_request\.head\.repo\.full_name != github\.repository$/);
          expect.soft(steps[refusal].run, `${name}:${id}`).toMatch(/exit 1/);
          if (first >= 0) expect.soft(refusal, `${name}:${id}`).toBeLessThan(first);
          continue;
        }
        // No refusal of its own: it follows a job that refuses, and posts only for this repository's heads.
        expect.soft(needsOf(job).length, `${name}:${id} neither refuses a fork nor follows a job that does`).toBeGreaterThan(0);
        const posts = steps.filter((step) => String(step.run ?? '').includes('gate-status.mjs post'));
        for (const step of posts) expect.soft(step.if, `${name}:${id}`).toBe("${{ !cancelled() && env.POST == 'true' }}");
        if (posts.length > 0) expect.soft(job.env.POST, `${name}:${id}`).toContain('github.event.pull_request.head.repo.full_name == github.repository');
      }
    }
  });

  it('each pull-request workflow: every checkout names the PR head by its sha, or is the workflow\'s own commit under tools/; none keeps credentials, no cache', () => {
    for (const name of PR_WORKFLOWS) {
      const { jobs } = read(name);
      for (const [id, job] of Object.entries(jobs)) {
        for (const step of checkouts(job)) {
          expect.soft(step.with['persist-credentials'], `${name}:${id}`).toBe(false);
          if (step.with.ref === undefined) expect.soft(step.with.path, `${name}:${id}: a checkout of main's code goes to tools/`).toBe('tools');
          else expect.soft(step.with.ref, `${name}:${id}`).toBe(HEAD);
        }
        for (const step of job.steps ?? []) expect.soft(step.with?.cache, `${name}:${id}: no cache in a pull_request_target run`).toBeUndefined();
      }
    }
  });

  it('gives the jobs that run the PR code a token that only reads, and writes statuses only from jobs that run none of it', () => {
    const runsPrCode = [];
    const writers = [];
    for (const name of PR_WORKFLOWS) {
      for (const [id, job] of Object.entries(read(name).jobs)) {
        const full = checkouts(job).some((step) => step.with.ref !== undefined && !step.with['sparse-checkout']);
        if (full) runsPrCode.push(`${name}:${id}`);
        if (Object.values(job.permissions ?? {}).includes('write')) writers.push(`${name}:${id}`);
      }
    }
    expect(runsPrCode.sort()).toEqual(['checks.yml:test']);
    for (const entry of runsPrCode) {
      const [name, id] = entry.split(':');
      expect(read(name).jobs[id].permissions, entry).toEqual({ contents: 'read', statuses: 'read', 'pull-requests': 'read' });
    }
    expect(writers.sort()).toEqual(['checks.yml:checks']);
    for (const entry of writers) {
      const [name, id] = entry.split(':');
      const job = read(name).jobs[id];
      expect(job.permissions, entry).toEqual({ contents: 'read', statuses: 'write' });
      // Two checkouts, exactly: main's poster, and one file of the tested commit for its tree.
      expect(checkouts(job).map((step) => step.with), entry).toEqual([
        { path: 'tools', 'sparse-checkout': 'infra/scripts/gate-status.mjs', 'sparse-checkout-cone-mode': false, 'persist-credentials': false },
        { ref: HEAD, path: 'tested', 'sparse-checkout': 'package.json', 'sparse-checkout-cone-mode': false, 'persist-credentials': false },
      ]);
      const runs = job.steps.map((step) => String(step.run ?? '')).join('\n');
      expect(runs, entry).not.toMatch(/\bpnpm\b|\bnpm\b|\bnpx\b|\bmake\b/);
      expect(runs.match(/\bnode \S+/g), entry).toEqual(['node tools/infra/scripts/gate-status.mjs']);
      expect(runs, entry).toContain(`--tree "$(git -C tested rev-parse 'HEAD^{tree}')"`);
      expect(job.needs, entry).toBeDefined();
    }
  });

  it("pr-issue reads only: main's fast-lane script and one file of the head, run nothing of the PR", () => {
    const issue = read('pr-issue.yml').jobs['linear-issue'];
    // CD-269: it reads the head's history for the fast lane; it ran none of the PR's code before and runs none now.
    expect(checkouts(issue).map((step) => step.with)).toEqual([
      { path: 'tools', 'sparse-checkout': 'infra/scripts/harness-only.mjs', 'sparse-checkout-cone-mode': false, 'persist-credentials': false },
      { ref: HEAD, path: 'tested', 'fetch-depth': 0, 'sparse-checkout': 'package.json', 'sparse-checkout-cone-mode': false, 'persist-credentials': false },
    ]);
    expect(issue.permissions).toEqual({ contents: 'read' });
    const runs = issue.steps.map((step) => String(step.run ?? '')).join('\n');
    expect(runs).not.toMatch(/\bpnpm\b|\bnpm\b|\bnpx\b|\bmake\b/);
    expect(runs.match(/\bnode \S+/g)).toEqual(['node ../tools/infra/scripts/harness-only.mjs']);
    // No action of the head's (`uses: ./tested/...`), and the head's folder is a checkout target and a place to run git, nothing else.
    expect(issue.steps.filter((step) => step.uses !== undefined).map((step) => step.uses.split('@')[0])).toEqual(['actions/checkout', 'actions/checkout', 'actions/setup-node']);
    const mentions = issue.steps.flatMap((step, index) =>
      Object.entries({ ...step, ...step.with }).flatMap(([key, value]) => (/tested/.test(JSON.stringify(value)) && key !== 'with' ? [`${issue.steps[index].id ?? index}.${key}`] : [])),
    );
    const head = issue.steps.findIndex((step) => step.with?.ref === HEAD);
    expect(mentions).toEqual([`${head}.path`, 'lane.working-directory']);
    expect(issue.name).toBe('linear-issue');
  });

  it("linear-issue succeeds for a bot's pull request instead of being skipped (Mergify never matches a skipped check)", () => {
    const issue = read('pr-issue.yml').jobs['linear-issue'];
    expect(issue.if).toBe('${{ !github.event.pull_request.draft }}');
    const title = issue.steps.find((step) => step.name === 'The title names a Linear issue');
    expect(title.env).toEqual({
      TITLE: '${{ github.event.pull_request.title }}',
      AUTHOR: '${{ github.event.pull_request.user.login }}',
      HARNESS: '${{ steps.lane.outputs.harness }}',
    });
    const exit = (env) => spawnSync('bash', ['-e', '-c', title.run], { env: { PATH: process.env.PATH, ...env }, encoding: 'utf8' }).status;
    expect(exit({ AUTHOR: 'renovate[bot]', TITLE: 'chore(deps): bump x' })).toBe(0);
    expect(exit({ AUTHOR: 'someone', TITLE: 'chore(deps): bump x' })).toBe(1);
    expect(exit({ AUTHOR: 'crooked-duke-release[bot]', TITLE: 'release: v5.129.0' })).toBe(0);
    expect(exit({ AUTHOR: 'bot-ish', TITLE: 'no id' })).toBe(1);
    for (const title of ['fix(CD-12): the thing', 'feat(PAR-3): a screen', 'setup(CD-478): a hook', 'test(CD-9): a flow', 'docs(CD-9): a page']) expect.soft(exit({ AUTHOR: 'someone', TITLE: title }), title).toBe(0);
    for (const title of ['fix the thing (CD-12)', 'CD-12: the thing', 'chore(CD-12): the thing', 'feat(CD-12):the thing', 'feat: the thing']) expect.soft(exit({ AUTHOR: 'someone', TITLE: title }), title).toBe(1);
  });
});

describe('the fast lane: a harness-only PR runs lint and the harness tests, no e2e, and needs no issue id (CD-269, owner Q43 A)', () => {
  const SCRIPT = fileURLToPath(new URL('../harness-only.mjs', import.meta.url));
  let fixture;

  beforeAll(() => {
    fixture = harnessFixture();
    // main's copy decides: pr-issue reads it from tools/. The head's own copy, untracked and in no diff, says harness-only to anything.
    fixture.checkout('mixed');
    for (const dir of ['tested/infra/scripts', 'tested/infra/verify', 'tools/infra/scripts', 'runner-temp']) mkdirSync(join(fixture.root, dir), { recursive: true });
    writeFileSync(join(fixture.repo, 'infra/scripts/harness-only.mjs'), 'process.exit(3);\n');
    copyFileSync(SCRIPT, join(fixture.root, 'tools/infra/scripts/harness-only.mjs'));
  });
  afterAll(() => rmSync(fixture.root, { recursive: true, force: true }));

  /** Runs a step's script as the runner does (bash -e) in `cwd`; its exit status and outputs. */
  const runStep = (step, cwd, env) => {
    const output = join(fixture.root, 'github-output');
    writeFileSync(output, '');
    const result = spawnSync('bash', ['-e', '-c', step.run], {
      cwd,
      encoding: 'utf8',
      env: { PATH: process.env.PATH, ...GIT_ENV, GITHUB_OUTPUT: output, GITHUB_STEP_SUMMARY: join(fixture.root, 'summary'), RUNNER_TEMP: join(fixture.root, 'runner-temp'), ...env },
    });
    const outputs = Object.fromEntries(
      readFileSync(output, 'utf8')
        .split('\n')
        .filter(Boolean)
        .map((line) => [line.slice(0, line.indexOf('=')), line.slice(line.indexOf('=') + 1)]),
    );
    return { status: result.status, stderr: result.stderr, outputs };
  };

  const laneStep = (job) => job.steps.find((step) => step.id === 'lane');
  const cwdOf = (step) => (step['working-directory'] ? join(fixture.root, step['working-directory']) : fixture.repo);

  /** The job's lane step, run on a PR branch of the fixture against main: its `harness` output. */
  const lane = (job, branch, baseEnv) => {
    fixture.checkout(branch);
    const result = runStep(laneStep(job), cwdOf(laneStep(job)), { [baseEnv]: fixture.main });
    expect(result.status, result.stderr).toBe(0);
    return result.outputs.harness;
  };

  it('pr-issue.yml: a PR whose title names no issue passes linear-issue only when harness-only (not mixed, non-harness, a move into .claude/ or a deletion), one that names an issue always passes, and a lane step that cannot answer (a base it cannot read) passes and is the full run', () => {
    const job = read('pr-issue.yml').jobs['linear-issue'];
    expect.soft(laneStep(job)['working-directory']).toBe('tested');
    expect.soft(laneStep(job).env).toEqual({ BASE_SHA: '${{ github.event.pull_request.base.sha }}' });
    const title = job.steps.find((step) => step.name === 'The title names a Linear issue');
    for (const [kind, branch, harness, status] of [
      ['harness-only', 'harness', 'true', 0],
      ['mixed', 'mixed', 'false', 1],
      ['non-harness', 'product', 'false', 1],
      ['move into .claude/', 'moved', 'false', 1],
      ['deletion', 'deleted', 'false', 1],
    ]) {
      expect.soft(lane(job, branch, 'BASE_SHA'), kind).toBe(harness);
      expect.soft(runStep(title, fixture.root, { AUTHOR: 'someone', TITLE: 'harness: a rule', HARNESS: harness }).status, kind).toBe(status);
      expect.soft(runStep(title, fixture.root, { AUTHOR: 'someone', TITLE: 'setup(CD-12): a rule', HARNESS: harness }).status, `${kind} with an issue`).toBe(0);
    }
    fixture.checkout('harness');
    const unanswered = runStep(laneStep(job), cwdOf(laneStep(job)), { BASE_SHA: 'no-such-ref-cd269' });
    expect.soft(unanswered.status, unanswered.stderr).toBe(0);
    expect.soft(unanswered.outputs, 'a base it cannot read').toEqual({ harness: 'false' });
  });
});

describe('pr-issue.yml: a change to app screens carries a Proof: link to its own proof branch in its description (PAR-30)', () => {
  const job = read('pr-issue.yml').jobs['screen-proof'];
  const step = job.steps.find((entry) => entry.name === 'A change to app screens links its proof');
  const REPO = 'parlornights/CrookedDuke';
  const HEAD_REF = 'claude/cd-478-proof-rework';
  const PROOF = `Proof: https://github.com/${REPO}/tree/proof/cd-478-proof-rework`;
  let root;
  let base;

  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: join(root, 'tested'), encoding: 'utf8', env: { PATH: process.env.PATH, ...GIT_ENV } });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr}`);
    return result.stdout.trim();
  };
  const write = (path, text) => {
    mkdirSync(join(root, 'tested', path, '..'), { recursive: true });
    writeFileSync(join(root, 'tested', path), text);
  };
  const branch = (name, files) => {
    git('checkout', '-q', '-b', name, base);
    for (const [path, text] of Object.entries(files)) write(path, text);
    git('add', '-A');
    git('commit', '-qm', name);
  };
  const check = (name, body) => {
    git('checkout', '-q', name);
    return spawnSync('bash', ['-e', '-c', step.run], {
      cwd: join(root, step['working-directory']),
      encoding: 'utf8',
      env: { PATH: process.env.PATH, ...GIT_ENV, BASE_SHA: base, REPO, HEAD_REF, ...(body === undefined ? {} : { BODY: body }) },
    });
  };

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'par30-proof-'));
    mkdirSync(join(root, 'tested'));
    git('init', '-q', '-b', 'main');
    write('apps/app/src/screens/home.tsx', 'export const home = 1;\n');
    write('apps/edge/src/api.ts', 'export const api = 1;\n');
    git('add', '-A');
    git('commit', '-qm', 'base');
    base = git('rev-parse', 'HEAD');
    branch('screen', { 'apps/app/src/screens/home.tsx': 'export const home = 2;\n', 'apps/app/src/screens/lobby.tsx': 'export const lobby = 1;\n' });
    branch('edge', { 'apps/edge/src/api.ts': 'export const api = 2;\n', 'apps/app/e2e/home.spec.ts': 'export {};\n', 'docs/TESTING.md': 'x\n' });
    branch('tests', {
      'apps/app/src/screens/home.test.tsx': 'export {};\n',
      'apps/app/src/state/wallet.test.ts': 'export {};\n',
      'apps/app/src/screens/__tests__/lobby.tsx': 'export {};\n',
      'apps/app/src/__tests__/fixtures/user.json': '{}\n',
    });
    branch('mixed', { 'apps/app/src/screens/home.test.tsx': 'export {};\n', 'apps/app/src/screens/home.tsx': 'export const home = 3;\n' });
  });
  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it('is its own job, named screen-proof, on wb like linear-issue: no draft, no fork, a token that only reads; text only: it checks out one file of the head for its history, installs and runs nothing of the PR, and starts no browser', () => {
    expect.soft(job.name).toBe('screen-proof');
    expect.soft(job['runs-on']).toEqual(['self-hosted', 'wb']);
    expect.soft(job.if).toBe('${{ !github.event.pull_request.draft }}');
    expect.soft(job.permissions).toEqual({ contents: 'read' });
    expect.soft(step.env).toEqual({ BASE_SHA: '${{ github.event.pull_request.base.sha }}', BODY: '${{ github.event.pull_request.body }}', REPO: '${{ github.repository }}', HEAD_REF: '${{ github.event.pull_request.head.ref }}' });
    expect.soft(checkouts(job).map((entry) => entry.with)).toEqual([
      { ref: HEAD, path: 'tested', 'fetch-depth': 0, 'sparse-checkout': 'package.json', 'sparse-checkout-cone-mode': false, 'persist-credentials': false },
    ]);
    expect.soft(job.steps.filter((entry) => entry.uses !== undefined).map((entry) => entry.uses.split('@')[0])).toEqual(['actions/checkout']);
    const runs = job.steps.map((entry) => String(entry.run ?? '')).join('\n');
    expect.soft(runs).not.toMatch(/\bpnpm\b|\bnpm\b|\bnpx\b|\bnode\b|\bplaywright\b/);
  });

  it('fails a change under apps/app/src/ whose description has no line Proof: with the link to its own proof branch, saying what is missing and naming only the screen beside tests; passes it with one wherever it sits, and passes a change with nothing under apps/app/src/ but tests with or without one', () => {
    const MISSING_SCREEN = `::error::This pull request changes 2 file(s) under apps/app/src/ (first: apps/app/src/screens/home.tsx) and its description has no line '${PROOF}'`;
    const elsewhere = [
      PROOF.toLowerCase(),
      `See ${PROOF}`,
      ` ${PROOF}`,
      `${PROOF}x`,
      `Proof: https://github.com/${REPO}/tree/proof/cd-478-other`,
      'Proof: https://claude.ai/public/artifacts/0f3a',
      'Proof: https://github.com/someone/else/tree/proof/cd-478-proof-rework',
      `Proof: https://github.com/${REPO}/tree/main`,
    ];
    for (const body of [undefined, '', ...elsewhere]) {
      const result = check('screen', body);
      const label = `screen, ${JSON.stringify(body)}`;
      expect.soft(result.status, label).toBe(1);
      expect.soft(result.stdout, label).toContain(MISSING_SCREEN);
      expect.soft(result.stdout, label).toContain("add the line 'Proof: <the link the proof printed>' to the description");
    }
    for (const body of [PROOF, `Summary.\n\n${PROOF}\n\nMore.`, `Summary.\r\n${PROOF}\r\n`]) {
      const result = check('screen', body);
      const label = `screen, ${JSON.stringify(body)}`;
      expect.soft(result.status, `${label}: ${result.stdout}${result.stderr}`).toBe(0);
      expect.soft(result.stdout, label).toContain('2 file(s) under apps/app/src/ change, and the description has its Proof: line');
    }
    for (const name of ['edge', 'tests']) {
      for (const body of [undefined, '', PROOF]) {
        const result = check(name, body);
        const label = `${name}, ${JSON.stringify(body)}`;
        expect.soft(result.status, `${label}: ${result.stdout}${result.stderr}`).toBe(0);
        expect.soft(result.stdout, label).toContain('no file under apps/app/src/ changes but tests: no proof needed');
      }
    }
    const mixed = check('mixed', 'Tests and a screen.');
    expect.soft(mixed.status, 'mixed').toBe(1);
    expect.soft(mixed.stdout, 'mixed').toContain('changes 1 file(s) under apps/app/src/ (first: apps/app/src/screens/home.tsx)');
    expect.soft(check('mixed', PROOF).status, 'mixed with a Proof: line').toBe(0);
  });
});

describe('.mergify.yml: one PR at a time, in place, squashed (CD-269)', () => {
  const config = parse(readFileSync(join(ROOT, '.mergify.yml'), 'utf8'));
  const CONDITIONS = [
    '-draft',
    'check-success = @github-actions/checks',
    'check-success = @github-actions/linear-issue',
    'check-success = @github-actions/screen-proof',
  ];

  it('has one queue: squash, one PR per batch, main merged into the PR when main moved; checks in place, serially, merge conditions the same as the queue conditions (else Mergify tests a draft PR of its own, which runs nothing here); queues on the comment alone, sets no label, and names checks that are jobs of the pull-request workflows', () => {
    expect.soft(config.queue_rules).toHaveLength(1);
    expect.soft(config.queue_rules[0]).toEqual({
      name: 'default',
      queue_conditions: CONDITIONS,
      merge_conditions: CONDITIONS,
      merge_method: 'squash',
      batch_size: 1,
      update_method: 'merge',
      checks_timeout: '60 minutes',
    });
    expect.soft(config.merge_queue).toEqual({ mode: 'serial', max_parallel_checks: 1, queued_label: null, dequeued_label: null });
    expect.soft(config.queue_rules[0].merge_conditions).toEqual(config.queue_rules[0].queue_conditions);
    expect.soft(JSON.stringify(config)).not.toContain('test-integrity');
    expect.soft(config.queue_rules[0]).not.toHaveProperty('max_checks_retries');
    expect.soft(Object.keys(config).sort()).toEqual(['merge_queue', 'queue_rules', 'shared']);
    expect.soft(JSON.stringify(config)).not.toMatch(/autoqueue|auto_merge|label\s*[=!~]/);
    const names = PR_WORKFLOWS.flatMap((name) => Object.values(read(name).jobs).map((job) => job.name));
    for (const condition of CONDITIONS.filter((entry) => entry.startsWith('check-success = @github-actions/'))) {
      expect.soft(names, condition).toContain(condition.split('/').at(-1));
    }
  });
});
