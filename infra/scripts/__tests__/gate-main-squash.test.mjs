/**
 * gate-main after a Mergify merge (CD-269). Mergify merges main into the PR branch in place,
 * the PR's `checks` posts `checks/tree` for that head, then Mergify squash-merges it: main's
 * new commit is not the head and has another parent, but its TREE is the head's. gate-main's
 * adopt step, run as the workflow runs it (bash, in a checkout of main's tip, against a fake
 * GitHub), finds the PR's record by that tree and posts gate/tree and checks/tree on main's tip,
 * which is what release.yml and the deploy scripts ask for.
 */
import { execFile, execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { childEnv } from './hermetic.mjs';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const adopt = parse(readFileSync(join(ROOT, '.github/workflows/gate-main.yml'), 'utf8'))
  .jobs['gate-main'].steps.find((step) => String(step.name).startsWith("Adopt the pull request's result")).run;

let dir;
let server;
afterEach(async () => {
  if (server) await new Promise((resolve) => server.close(resolve));
  server = undefined;
  rmSync(dir, { recursive: true, force: true });
});

const fakeGitHub = async (routes) => {
  const posted = [];
  server = createServer((request, response) => {
    let body = '';
    request.on('data', (chunk) => (body += chunk));
    request.on('end', () => {
      const path = new URL(request.url, 'http://x').pathname;
      if (request.method === 'POST') {
        posted.push({ path, body: JSON.parse(body) });
        response.writeHead(201, { 'content-type': 'application/json' }).end('{}');
        return;
      }
      const answer = routes[path];
      response.writeHead(answer ? 200 : 404, { 'content-type': 'application/json' }).end(JSON.stringify(answer ?? {}));
    });
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return { url: `http://127.0.0.1:${server.address().port}`, posted };
};

describe('gate-main adopts the PR checks after a Mergify squash merge of an up-to-date head (CD-269)', () => {
  it('finds checks/tree on the PR head by the squash commit\'s tree, and posts gate/tree and checks/tree on main\'s tip', async () => {
    dir = mkdtempSync(join(tmpdir(), 'gate-main-squash-'));
    const git = (...args) => execFileSync('git', ['-C', dir, '-c', 'user.name=t', '-c', 'user.email=t@example.com', '-c', 'commit.gpgsign=false', ...args], { encoding: 'utf8' }).trim();
    const commit = (file, text) => {
      writeFileSync(join(dir, file), text);
      git('add', file);
      git('commit', '-q', '-m', file);
      return git('rev-parse', 'HEAD');
    };
    git('init', '-q', '-b', 'main');
    commit('a.txt', 'a');
    git('checkout', '-q', '-b', 'wt/x');
    commit('b.txt', 'the PR');
    git('checkout', '-q', 'main');
    const mainMoved = commit('c.txt', 'another PR, merged first');
    // Mergify's in-place update: main merged into the PR branch; checks runs on this head.
    git('checkout', '-q', 'wt/x');
    git('merge', '-q', '--no-edit', 'main');
    const head = git('rev-parse', 'HEAD');
    const tree = git('rev-parse', 'HEAD^{tree}');
    // Mergify's squash merge: one new commit on main, parent main's tip, the head's tree.
    const squash = git('commit-tree', tree, '-p', mainMoved, '-m', 'the PR (CD-1) (#9)');
    git('checkout', '-q', '--detach', squash);
    expect(git('rev-parse', 'HEAD^{tree}')).toBe(tree);
    expect(squash).not.toBe(head);
    mkdirSync(join(dir, 'infra/scripts'), { recursive: true });
    copyFileSync(join(ROOT, 'infra/scripts/gate-status.mjs'), join(dir, 'infra/scripts/gate-status.mjs'));

    const github = await fakeGitHub({
      [`/repos/o/r/commits/${squash}/statuses`]: [],
      [`/repos/o/r/commits/${mainMoved}/statuses`]: [],
      [`/repos/o/r/commits/${squash}/pulls`]: [{ number: 9, merge_commit_sha: squash, head: { sha: head } }],
      [`/repos/o/r/commits/${head}/statuses`]: [
        { context: 'checks/tree', state: 'success', description: `tree ${tree} passed`, created_at: '2026-10-08T10:00:00Z', target_url: 'https://github.test/run/1' },
      ],
    });
    const { NODE_USE_ENV_PROXY: _proxy, HTTP_PROXY: _h, HTTPS_PROXY: _hs, http_proxy: _l, https_proxy: _ls, ...env } = childEnv();
    const result = await new Promise((resolve) => {
      execFile('bash', ['-c', adopt], {
        cwd: dir,
        encoding: 'utf8',
        env: { ...env, GITHUB_TOKEN: 'token', GITHUB_REPOSITORY: 'o/r', GITHUB_API_URL: github.url, NO_PROXY: '127.0.0.1', no_proxy: '127.0.0.1' },
      }, (error, stdout, stderr) => resolve({ code: error ? error.code : 0, stdout, stderr }));
    });

    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("main's tree is the tree its pull request's checks passed: adopted.");
    const onTip = github.posted.filter((post) => post.path === `/repos/o/r/statuses/${squash}`).map((post) => [post.body.context, post.body.state, post.body.description]);
    expect(onTip).toEqual([
      ['gate/tree', 'success', `tree ${tree} passed`],
      ['checks/tree', 'success', `tree ${tree} passed`],
    ]);
  }, 30_000);
});
