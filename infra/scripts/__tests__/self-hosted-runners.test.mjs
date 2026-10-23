/**
 * Every job runs on the owner's self-hosted runners, never on a GitHub-hosted runner (owner, 8 Oct,
 * CD-269: the account's GitHub Actions minutes are spent, and no more can be bought). Light jobs run
 * on runners labelled `wb`; the test job checks-run runs on runners labelled `heavy` (owner, 12 Oct,
 * CD-354); there is one release-only runner `wb-release`; no job runs on `wb-gate`.
 * RUNNERS pins each job's labels; changing it needs the owner's approval.
 */
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const GITHUB = fileURLToPath(new URL('../../../.github/', import.meta.url));
const WORKFLOWS = join(GITHUB, 'workflows');
const ACTIONS = join(GITHUB, 'actions');
const ACTIONLINT = join(GITHUB, 'actionlint.yaml');

const WB = ['self-hosted', 'wb'];
const HEAVY = ['self-hosted', 'heavy'];
const WB_RELEASE = ['self-hosted', 'wb-release'];

const RUNNERS = {
  'checks.yml:test': HEAVY,
  'checks.yml:lane': WB,
  'checks.yml:checks': WB,
  'deploy.yml:plan': WB_RELEASE,
  'deploy-part.yml:deploy': WB_RELEASE,
  'deploy-part.yml:rollback': WB_RELEASE,
  'gate-main.yml:gate-main': WB,
  'journeys.yml:journeys': HEAVY, // owner, 14 Oct, CD-405
  'nightly.yml:file': WB_RELEASE,
  'proof-branch.yml:delete': WB_RELEASE,
  'pr-issue.yml:linear-issue': WB,
  'pr-issue.yml:screen-proof': WB,
  'release.yml:plan': WB_RELEASE,
  'release.yml:runner-check': WB_RELEASE,
  'release.yml:smoke': WB_RELEASE,
  'release-please.yml:release': WB_RELEASE,
  'release-please.yml:pull-request': WB_RELEASE,
  'release-please.yml:proofs': WB_RELEASE,
  'release-please.yml:proof-branches': WB_RELEASE,
};

const HOSTED = /^(ubuntu|windows|macos)-/;
const INSTALL = /playwright(-core)?(@\S+)?\s+install/;
const BROWSER_ACTION = /playwright|setup-chrom|setup-browser/i;

// GitHub-hosted when ANY label names a GitHub-hosted image, or when runs-on is not a literal
// list of labels at all (an expression or a group can pick a hosted runner at run time).
function githubHosted(runsOn) {
  const labels = Array.isArray(runsOn) ? runsOn : [runsOn];
  if (labels.length === 0) return true;
  return labels.some((label) => typeof label !== 'string' || HOSTED.test(label) || label.includes('${{'));
}

function installsBrowser(step) {
  const run = String(step.run ?? '').replace(/\\\r?\n/g, ' ');
  return INSTALL.test(run) || BROWSER_ACTION.test(String(step.uses ?? ''));
}

/**
 * A job holds a write token (owner, Q35, CD-269) when its GITHUB_TOKEN may write contents, it reads
 * any secret, or it mints a GitHub App token; every such job runs on wb-release, with no exception.
 * `statuses: write` is left out on purpose: the status jobs checks, gate and gate-main hold it and
 * stay on wb (owner, 8 Oct). They post the commit statuses the queue and the release read, and run
 * none of the branch's code.
 */
function holdsWriteToken(job) {
  const text = JSON.stringify(job.steps ?? []) + JSON.stringify(job.env ?? {});
  return job.permissions?.contents === 'write' || /\bsecrets\./.test(text) || /create-github-app-token|"private-key"/.test(text);
}

/** What a wb-release job may check out: main, the release's commit of main, a rollback target on main, or the workflow's own commit. */
const RELEASE_REFS = [undefined, 'main', "${{ github.event.workflow_run.head_sha || 'main' }}", '${{ needs.plan.outputs.sha }}', '${{ matrix.sha }}', '${{ inputs.sha }}', '${{ inputs.live }}'];

const isCheckout = (step) => String(step.uses ?? '').startsWith('actions/checkout@');

const files = readdirSync(WORKFLOWS).filter((file) => /\.ya?ml$/.test(file));
const jobs = files.flatMap((file) => {
  const workflow = parse(readFileSync(join(WORKFLOWS, file), 'utf8'));
  return Object.entries(workflow.jobs ?? {}).map(([id, job]) => ({
    name: `${file}:${id}`,
    on: workflow.on ?? {},
    runsOn: job['runs-on'],
    uses: job.uses,
    permissions: job.permissions,
    env: job.env,
    steps: job.steps ?? [],
  }));
});

const composite = existsSync(ACTIONS)
  ? readdirSync(ACTIONS, { recursive: true })
      .filter((path) => /(^|[\\/])action\.ya?ml$/.test(path))
      .map((path) => ({ name: `actions/${path}`, steps: parse(readFileSync(join(ACTIONS, path), 'utf8')).runs?.steps ?? [] }))
  : [];

describe('every job runs on the owner\'s self-hosted runners (checks-run on heavy, light jobs on wb), none on a GitHub-hosted runner (owner, 8 Oct, CD-269; 12 Oct, CD-354)', () => {
  it('runs no job on a GitHub-hosted label (ubuntu-*, windows-*, macos-*) or on a runner chosen at run time', () => {
    const hosted = jobs.filter((job) => !job.uses && githubHosted(job.runsOn)).map((job) => `${job.name} (${JSON.stringify(job.runsOn)})`);
    expect(hosted, `${hosted.join(', ')}: the GitHub-hosted minutes are spent; every job runs on [self-hosted, wb], [self-hosted, heavy] or [self-hosted, wb-release].`).toEqual([]);
  });

  it('runs each job on exactly its pinned runner, and every job is pinned (owner, 12 Oct, CD-354: checks-run on heavy, light jobs on wb)', () => {
    const actual = Object.fromEntries(jobs.filter((job) => !job.uses).map((job) => [job.name, job.runsOn]));
    expect(actual, 'a job was added, removed or moved to another runner: RUNNERS changes with the owner\'s approval').toEqual(RUNNERS);
  });

  it("lists every label a job's runs-on uses among actionlint's self-hosted-runner labels", () => {
    const listed = parse(readFileSync(ACTIONLINT, 'utf8'))['self-hosted-runner']?.labels ?? [];
    const used = new Set(jobs.filter((job) => !job.uses).flatMap((job) => [job.runsOn].flat()));
    const missing = [...used].filter((label) => label !== 'self-hosted' && !listed.includes(label));
    expect(missing, `${missing.join(', ')}: add each label to .github/actionlint.yaml, or actionlint reports it as unknown.`).toEqual([]);
  });

  it('calls no reusable workflow from outside this repository', () => {
    const outside = jobs.filter((job) => job.uses && !String(job.uses).startsWith('./.github/workflows/')).map((job) => `${job.name} uses ${job.uses}`);
    expect(outside, `${outside.join(', ')}: a reusable workflow elsewhere can pick its own runner; adding one needs the owner's approval.`).toEqual([]);
  });

  it('installs no browser in any job or composite action: wb has its own Chromium', () => {
    const offenders = [
      ...jobs.filter((job) => job.steps.some(installsBrowser)).map((job) => job.name),
      ...composite.filter((action) => action.steps.some(installsBrowser)).map((action) => action.name),
    ];
    expect(offenders, `${offenders.join(', ')} installs a browser: the wb runners carry theirs.`).toEqual([]);
  });

  it('runs every job that holds a write token (contents: write, a secret, an App key) on wb-release (owner, Q35)', () => {
    const misplaced = jobs
      .filter((job) => !job.uses && holdsWriteToken(job))
      .filter((job) => JSON.stringify(job.runsOn) !== JSON.stringify(WB_RELEASE))
      .map((job) => `${job.name} (${JSON.stringify(job.runsOn)})`);
    expect(misplaced, `${misplaced.join(', ')}: a job holding a write token runs on [self-hosted, wb-release], runners that never run a branch's code.`).toEqual([]);
    expect(jobs.filter((job) => holdsWriteToken(job)).map((job) => job.name)).toEqual(
      expect.arrayContaining(['deploy-part.yml:deploy', 'release.yml:smoke', 'release-please.yml:release', 'release-please.yml:pull-request', 'release-please.yml:proof-branches', 'proof-branch.yml:delete']),
    );
  });

  it("runs no pull-request code on wb-release: no pull_request trigger, a pull_request_target job that runs no action, and every checkout is main, the release's commit or the workflow's own", () => {
    const release = jobs.filter((job) => JSON.stringify(job.runsOn) === JSON.stringify(WB_RELEASE));
    expect(release.map((job) => job.name)).toEqual(expect.arrayContaining(['release.yml:smoke', 'deploy-part.yml:deploy', 'release-please.yml:release']));
    const onTarget = release.filter((job) => Object.keys(job.on).includes('pull_request_target'));
    expect(onTarget.map((job) => job.name)).toEqual(['proof-branch.yml:delete']);
    for (const job of onTarget) expect(job.steps.map((step) => step.uses ?? null), job.name).toEqual([null]);
    for (const job of release) {
      expect(Object.keys(job.on), job.name).not.toContain('pull_request');
      for (const step of job.steps.filter(isCheckout)) expect(RELEASE_REFS, `${job.name} checks out ${step.with?.ref}`).toContain(step.with?.ref);
    }
  });

  it("points no wb-release job at another account's HOME or a shared cache: ghrelease's own HOME, tool cache and pnpm store only (owner, 8 Oct)", () => {
    const release = jobs.filter((job) => JSON.stringify(job.runsOn) === JSON.stringify(WB_RELEASE));
    for (const job of release) {
      const text = JSON.stringify({ env: job.env ?? {}, steps: job.steps });
      expect(text, `${job.name} names a fixed home`).not.toMatch(/\/home\/|\/root\/|~\/|~[a-z]/);
      expect(text, `${job.name} names a store, tool cache or runner folder of its own choosing`).not.toMatch(/store-dir|store_dir|RUNNER_TOOL_CACHE|AGENT_TOOLSDIRECTORY|actions-runner/i);
      for (const step of job.steps) {
        expect(String(step.uses ?? ''), `${job.name} restores a cache`).not.toMatch(/^actions\/cache/);
        expect(step.with?.cache, `${job.name}: ${step.uses} restores a cache`).toBeUndefined();
      }
    }
  });

  it('knows a write token when it sees one (fixtures)', () => {
    expect(holdsWriteToken({ permissions: { contents: 'write' }, steps: [] })).toBe(true);
    expect(holdsWriteToken({ permissions: { contents: 'read' }, steps: [{ env: { T: '${{ secrets.CLOUDFLARE_API_TOKEN }}' } }] })).toBe(true);
    expect(holdsWriteToken({ permissions: {}, env: { T: '${{ secrets.X }}' }, steps: [] })).toBe(true);
    expect(holdsWriteToken({ permissions: {}, steps: [{ uses: 'actions/create-github-app-token@v2', with: { 'client-id': 'x' } }] })).toBe(true);
    expect(holdsWriteToken({ permissions: {}, steps: [{ with: { 'private-key': '${{ vars.KEY }}' } }] })).toBe(true);
    expect(holdsWriteToken({ permissions: { contents: 'read', statuses: 'write' }, steps: [{ env: { GITHUB_TOKEN: '${{ github.token }}' } }] })).toBe(false);
  });

  it('reads every runs-on form the right way (fixtures)', () => {
    for (const value of ['ubuntu-latest', ['ubuntu-22.04'], ['self-hosted', 'ubuntu-latest'], 'macos-14', ['windows-2022'], '${{ matrix.os }}', "${{ fromJSON('[\"self-hosted\"]') }}", '${{ vars.RUNNER }}', [], [{ group: 'x' }]]) {
      expect(githubHosted(value), JSON.stringify(value)).toBe(true);
    }
    for (const value of [WB, HEAVY, WB_RELEASE, 'wb']) expect(githubHosted(value), JSON.stringify(value)).toBe(false);
  });

  it('catches every way of installing a browser (fixtures)', () => {
    const caught = [
      { run: 'pnpm exec playwright install chromium' },
      { run: 'npx playwright@1.50.0 install --with-deps' },
      { run: 'npx playwright-core install' },
      { run: 'pnpm exec playwright \\\n  install chromium' },
      { uses: 'microsoft/playwright-github-action@v1' },
      { uses: 'browser-actions/setup-chrome@v1' },
      { uses: 'browser-actions/setup-chromium@v1' },
      { uses: 'some/setup-browser@v2' },
    ];
    for (const step of caught) expect(installsBrowser(step), JSON.stringify(step)).toBe(true);
    for (const step of [{ run: 'pnpm test' }, { uses: 'actions/checkout@v4' }, { run: 'echo playwright' }]) {
      expect(installsBrowser(step), JSON.stringify(step)).toBe(false);
    }
  });
});
