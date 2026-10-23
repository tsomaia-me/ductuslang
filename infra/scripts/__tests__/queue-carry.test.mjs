import { execFile, execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { GIT_ENV } from './harness-fixture.mjs';
import { childEnv } from './hermetic.mjs';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const workflow = (name) => parse(readFileSync(join(ROOT, '.github/workflows', name), 'utf8'));
const CHECKS = workflow('checks.yml');
const carryStep = CHECKS.jobs.test.steps.find((step) => step.id === 'carry');
const postStep = CHECKS.jobs.checks.steps.find((step) => String(step.run).includes('gate-status.mjs post'));
const adoptOnMain = workflow('gate-main.yml').jobs['gate-main'].steps.find((step) => String(step.name).startsWith("Adopt the pull request's result"));
const releaseGate = workflow('release.yml').jobs.plan.steps.find((step) => step.id === 'gate');
const MAIN_SCRIPTS = ['check-touched.mjs', 'gate.sh', 'gate-status.mjs', 'queue-carry.mjs'];

const PACKAGES = {
  'apps/app': ['@wg/app', ['@wg/shared']],
  'apps/admin': ['@wg/admin', ['@wg/shared']],
  'apps/edge': ['@wg/edge', ['@wg/shared']],
  'packages/shared': ['@wg/shared', []],
  infra: ['@wg/infra', []],
};

const MERGIFY = 'mergify[bot]';

describe("the queue carries a pull request's passed run over main's delta only when the delta cannot reach it (CD-355)", { timeout: 60_000 }, () => {
  let root;
  let repo;
  let seed;
  let server;
  let api;
  let cases = 0;
  const statuses = new Map();
  const pulls = new Map();
  const posted = [];
  const { NODE_USE_ENV_PROXY: _proxy, HTTP_PROXY: _h, HTTPS_PROXY: _hs, http_proxy: _l, https_proxy: _ls, ...baseEnv } = childEnv();

  const git = (...args) => execFileSync('git', args, { cwd: repo, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], env: { ...baseEnv, ...GIT_ENV } }).trim();
  const write = (path, text) => {
    mkdirSync(join(repo, dirname(path)), { recursive: true });
    writeFileSync(join(repo, path), text);
  };
  const commit = (paths, text) => {
    for (const path of paths) write(path, `${text}\n`);
    git('add', '-A');
    git('commit', '-q', '-m', text);
    return git('rev-parse', 'HEAD');
  };
  const treeOf = (sha) => git('rev-parse', `${sha}^{tree}`);
  const checksStatus = (sha, state) => ({ context: 'checks/tree', state, description: `tree ${treeOf(sha)} ${state === 'success' ? 'passed' : 'failed'}`, created_at: '2026-10-12T10:00:00Z' });
  const record = (sha, state) => statuses.set(sha, [...(statuses.get(sha) ?? []), checksStatus(sha, state)]);

  const queueUpdate = ({ pullRequest, delta, evil = false }) => {
    cases += 1;
    git('checkout', '-q', '-b', `main-${cases}`, seed);
    const mainCommits = delta.map((paths, index) => commit(paths, `main ${cases}.${index}`));
    git('checkout', '-q', '-b', `pr-${cases}`, seed);
    const previous = commit(pullRequest, `pr ${cases}`);
    git('merge', '-q', '--no-edit', '--no-commit', `main-${cases}`);
    if (evil) write('apps/edge/src/resolved.ts', 'a hand edit in the merge\n');
    git('add', '-A');
    git('commit', '-q', '--no-edit');
    const head = git('rev-parse', 'HEAD');
    git('update-ref', 'refs/remotes/origin/main', `main-${cases}`);
    return { previous, mainCommits, head, mainTip: git('rev-parse', `main-${cases}`) };
  };

  const passEverything = ({ previous, mainCommits }) => {
    for (const sha of [previous, ...mainCommits]) record(sha, 'success');
  };

  const runStep = (step, { cwd = repo, env = {} } = {}) => {
    const temp = mkdtempSync(join(root, 'runner-temp-'));
    const output = join(temp, 'github-output');
    writeFileSync(output, '');
    return new Promise((resolve) => {
      execFile(
        'bash',
        ['-e', '-c', step.run],
        {
          cwd,
          encoding: 'utf8',
          env: { ...baseEnv, ...GIT_ENV, GITHUB_TOKEN: 'token', GITHUB_REPOSITORY: 'o/r', GITHUB_API_URL: api, NO_PROXY: '127.0.0.1', no_proxy: '127.0.0.1', RUNNER_TEMP: temp, GITHUB_OUTPUT: output, ...env },
        },
        (error, stdout, stderr) => {
          const outputs = Object.fromEntries(
            readFileSync(output, 'utf8')
              .split('\n')
              .filter(Boolean)
              .map((line) => [line.slice(0, line.indexOf('=')), line.slice(line.indexOf('=') + 1)]),
          );
          resolve({ code: error ? error.code : 0, stdout, stderr, outputs });
        },
      );
    });
  };

  const carry = async (update, sender = MERGIFY, env = {}) => {
    git('checkout', '-q', '--detach', update.head);
    const result = await runStep(carryStep, { env: { SENDER: sender, ...env } });
    expect.soft(result.code, result.stderr).toBe(0);
    return { carried: result.outputs.carried, log: result.stderr };
  };

  beforeAll(async () => {
    root = mkdtempSync(join(tmpdir(), 'queue-carry-'));
    repo = join(root, 'tested');
    mkdirSync(repo);
    git('init', '-q', '-b', 'main');
    write('package.json', JSON.stringify({ name: 'wg-monorepo', private: true }));
    write('pnpm-workspace.yaml', "packages:\n  - 'apps/*'\n  - 'infra'\n  - 'packages/*'\n");
    write('tsconfig.base.json', '{}\n');
    for (const [dir, [name, deps]] of Object.entries(PACKAGES)) {
      write(`${dir}/package.json`, JSON.stringify({ name, version: '0.0.0', dependencies: Object.fromEntries(deps.map((dep) => [dep, 'workspace:*'])) }));
      write(`${dir}/src/index.ts`, 'export {};\n');
    }
    mkdirSync(join(repo, 'infra/scripts'), { recursive: true });
    for (const script of MAIN_SCRIPTS) copyFileSync(join(ROOT, 'infra/scripts', script), join(repo, 'infra/scripts', script));
    git('add', '-A');
    git('commit', '-q', '-m', 'seed');
    seed = git('rev-parse', 'HEAD');

    server = createServer((request, response) => {
      let body = '';
      request.on('data', (chunk) => (body += chunk));
      request.on('end', () => {
        const path = new URL(request.url, 'http://x').pathname;
        const match = /\/commits\/([0-9a-f]{40})\/(statuses|pulls)$/.exec(path);
        if (request.method === 'POST') {
          const sha = path.split('/').pop();
          const status = { ...JSON.parse(body), created_at: new Date().toISOString() };
          posted.push({ sha, ...status });
          statuses.set(sha, [...(statuses.get(sha) ?? []), status]);
          response.writeHead(201, { 'content-type': 'application/json' }).end('{}');
          return;
        }
        const answer = match?.[2] === 'statuses' ? statuses.get(match[1]) ?? [] : match?.[2] === 'pulls' ? pulls.get(match[1]) ?? [] : undefined;
        response.writeHead(answer ? 200 : 404, { 'content-type': 'application/json' }).end(JSON.stringify(answer ?? {}));
      });
    });
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    api = `http://127.0.0.1:${server.address().port}`;
  }, 60_000);

  afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
    rmSync(root, { recursive: true, force: true });
  });

  it('carries a queue update only when its delta cannot reach the pull request, saying why in the log, and runs in full otherwise', async () => {
    const rows = [
      { label: 'a delta reaching packages the pull request does not', shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts']] }, carried: 'true', says: [/^queue-carry: carried: /m] },
      { label: 'a Markdown and .claude only delta', shape: { pullRequest: ['apps/app/src/x.ts'], delta: [['docs/notes.md', '.claude/x.json', 'CLAUDE.md']] }, carried: 'true', says: [] },
      { label: 'the pull request changes a global file', shape: { pullRequest: ['tsconfig.base.json'], delta: [['apps/edge/src/y.ts']] }, carried: 'false', says: ['queue-carry: no carry: ', 'the pull request changes a global file'] },
      { label: "main's delta changes a global file", shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts'], ['tsconfig.base.json']] }, carried: 'false', says: ['queue-carry: no carry: ', "main's delta changes a global file"] },
      { label: 'both sides reach the same package', shape: { pullRequest: ['apps/app/src/x.ts'], delta: [['packages/shared/src/y.ts']] }, carried: 'false', says: ['queue-carry: no carry: ', "the pull request and main's delta both reach apps/app"] },
      { label: 'the merge carries a hand edit', shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['docs/notes.md']], evil: true }, carried: 'false', says: ['queue-carry: no carry: ', 'is not a clean merge'] },
      {
        label: 'a commit main brought in has no passed checks of its own',
        shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts'], ['docs/direct-push.md']] },
        pass: (update) => {
          record(update.previous, 'success');
          record(update.mainCommits[0], 'success');
        },
        carried: 'false',
        says: [(update) => `main's ${update.mainCommits[1].slice(0, 7)} in the delta has no passed checks/tree`],
      },
      {
        label: "the pull request's own previous run did not pass",
        shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts']] },
        pass: (update) => {
          record(update.previous, 'failure');
          record(update.mainCommits[0], 'success');
        },
        carried: 'false',
        says: [(update) => `previous head ${update.previous.slice(0, 7)} has no passed checks/tree`],
      },
      { label: 'an update anyone but the merge queue pushed, an agent merging main in included', shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts']] }, sender: 'an-agent', carried: 'false', says: ['not a merge-queue update (sent by an-agent)'] },
      {
        label: 'the merged-in parent is not on main',
        shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts']] },
        before: () => git('update-ref', 'refs/remotes/origin/main', seed),
        carried: 'false',
        says: ['is not on main'],
      },
      { label: 'it cannot ask GitHub', shape: { pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts']] }, env: { GITHUB_API_URL: 'http://127.0.0.1:1' }, carried: 'false', says: ['queue-carry: no carry: could not decide'] },
    ];
    for (const row of rows) {
      const update = queueUpdate(row.shape);
      (row.pass ?? passEverything)(update);
      row.before?.();
      const { carried, log } = await carry(update, row.sender, row.env);
      expect.soft(carried, row.label).toBe(row.carried);
      for (const said of row.says) {
        const text = typeof said === 'function' ? said(update) : said;
        if (text instanceof RegExp) expect.soft(log, row.label).toMatch(text);
        else expect.soft(log, row.label).toContain(text);
      }
    }
  });

  it("posts checks/tree for the carried head's own tree, which gate-main adopts after the squash merge and the release accepts", async () => {
    const update = queueUpdate({ pullRequest: ['apps/admin/src/x.ts'], delta: [['apps/edge/src/y.ts']] });
    passEverything(update);
    expect((await carry(update)).carried).toBe('true');

    const job = mkdtempSync(join(root, 'checks-job-'));
    mkdirSync(join(job, 'tools/infra/scripts'), { recursive: true });
    copyFileSync(join(ROOT, 'infra/scripts/gate-status.mjs'), join(job, 'tools/infra/scripts/gate-status.mjs'));
    git('worktree', 'add', '-q', '--detach', join(job, 'tested'), update.head);
    const post = await runStep(postStep, { cwd: job, env: { HEAD_SHA: update.head, JOB_STATUS: 'success' } });
    expect(post.code, post.stderr).toBe(0);
    const tree = treeOf(update.head);
    expect(posted.filter((status) => status.sha === update.head).map((status) => [status.context, status.state, status.description])).toEqual([['checks/tree', 'success', `tree ${tree} passed`]]);

    const squash = git('commit-tree', tree, '-p', update.mainTip, '-m', 'the PR (CD-1) (#9)');
    pulls.set(squash, [{ number: 9, merge_commit_sha: squash, head: { sha: update.head } }]);
    git('worktree', 'add', '-q', '--detach', join(root, 'main-tip'), squash);
    const adopted = await runStep(adoptOnMain, { cwd: join(root, 'main-tip') });
    expect(adopted.code, adopted.stderr).toBe(0);
    expect(adopted.stdout).toContain("main's tree is the tree its pull request's checks passed: adopted.");

    const release = await runStep(releaseGate, { cwd: join(root, 'main-tip'), env: { TIP: squash } });
    expect(release.code, release.stderr).toBe(0);
    expect(release.stdout).toContain(`checks/tree: tree ${tree.slice(0, 7)} passed`);
    expect(release.stdout).toContain(`gate/tree: tree ${tree.slice(0, 7)} passed`);
  });
});
