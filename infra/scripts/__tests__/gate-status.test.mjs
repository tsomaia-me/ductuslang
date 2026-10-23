/**
 * `gate-status.mjs` (CD-179): the gate's result as GitHub holds it, keyed by the
 * TREE the workflow tested. What has to hold: a status is evidence only for the
 * tree it names; the newest status naming a tree decides (a later failure undoes
 * an earlier pass); evidence may sit on the commit, a parent, or the head of the
 * PR that made it; `adopt` copies it onto the commit; and every way of not being
 * able to ask is exit 2, never a pass. GitHub is a fake `fetch` here.
 */
import { describe, expect, it } from 'vitest';

import {
  CONTEXTS,
  CannotAsk,
  EXIT_CANNOT_ASK,
  EXIT_MISSING,
  EXIT_PASS,
  PULLS_TRIES,
  RETRY_BACKOFF_MS,
  describeTree,
  findEvidence,
  githubApi,
  mergedPrPassed,
  parseArgs,
  repoFromUrl,
  run,
  verdict,
} from '../gate-status.mjs';

const MAIN = 'a'.repeat(40);
const HEAD = 'b'.repeat(40);
const PARENT = 'c'.repeat(40);
const TREE = '1'.repeat(40);
const OTHER_TREE = '2'.repeat(40);
const TOKEN = 'ghs-token-must-not-appear';

const status = (context, tree, state, created_at, target_url = 'https://github.test/run') => ({
  context,
  state,
  description: describeTree(tree, state),
  created_at,
  target_url,
});

const fakeGitHub = ({ statuses = {}, pulls = {}, answer } = {}) => {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url, method: init.method, auth: init.headers.authorization, body: init.body && JSON.parse(init.body) });
    if (answer) return answer(url, init);
    const path = new URL(url).pathname;
    const match = /\/commits\/([0-9a-f]{40})\/(statuses|pulls)$/.exec(path);
    const json = (body) => ({ ok: true, status: 200, json: async () => body });
    if (init.method === 'POST') return { ok: true, status: 201, json: async () => ({}) };
    if (match?.[2] === 'statuses') return json(statuses[match[1]] ?? []);
    if (match?.[2] === 'pulls') return json((pulls[match[1]] ?? []).map((sha) => ({ head: { sha } })));
    return { ok: false, status: 404, json: async () => ({}) };
  };
  return { calls, fetchImpl };
};

const ENV = { GITHUB_TOKEN: TOKEN, GITHUB_REPOSITORY: 'owner/repo', GITHUB_API_URL: 'https://api.github.test' };

const cli = async (argv, github, env = ENV) => {
  const out = [];
  const err = [];
  const code = await run(argv, env, {
    fetchImpl: github.fetchImpl,
    out: (line) => out.push(line),
    err: (line) => err.push(line),
  });
  return { code, out: out.join('\n'), err: err.join('\n') };
};

const refusePulls = (statuses) =>
  fakeGitHub({
    answer: async (url) => {
      const path = new URL(url).pathname;
      if (path.endsWith('/pulls')) return { ok: false, status: 403, json: async () => ({}) };
      const match = /\/commits\/([0-9a-f]{40})\/statuses$/.exec(path);
      return { ok: true, status: 200, json: async () => (match ? (statuses[match[1]] ?? []) : []) };
    },
  });

describe('verdict', () => {
  it('is decided by the newest status that names the tree, and only by those: a later failure undoes an earlier pass, a status naming no tree is ignored', () => {
    const named = [
      status(CONTEXTS.gate, TREE, 'success', '2026-09-30T01:00:00Z'),
      status(CONTEXTS.gate, OTHER_TREE, 'failure', '2026-09-30T03:00:00Z'),
    ];
    expect.soft(verdict(named, CONTEXTS.gate, TREE).passed, 'its own tree').toBe(true);
    expect.soft(verdict(named, CONTEXTS.gate, OTHER_TREE).passed, 'the other tree').toBe(false);
    expect.soft(verdict(named, CONTEXTS.checks, TREE), 'another context').toEqual({ passed: false, reason: 'none' });
    const undone = [
      status(CONTEXTS.gate, TREE, 'success', '2026-09-30T01:00:00Z'),
      status(CONTEXTS.gate, TREE, 'failure', '2026-09-30T02:00:00Z'),
    ];
    expect.soft(verdict(undone, CONTEXTS.gate, TREE), 'a later failure').toMatchObject({ passed: false, reason: 'failure' });
    const forged = { context: CONTEXTS.gate, state: 'success', description: 'looks fine', created_at: '2026-09-30T05:00:00Z' };
    expect.soft(verdict([forged], CONTEXTS.gate, TREE).passed, 'no tree named').toBe(false);
  });
});

describe('arguments and the repository', () => {
  it('takes full ids only and knows its verbs and contexts', () => {
    expect(parseArgs(['check', '--sha', MAIN]).need).toEqual(['checks', 'gate']);
    expect(parseArgs(['check', '--sha', MAIN, '--need', 'gate']).need).toEqual(['gate']);
    expect(() => parseArgs(['check', '--sha', 'abc1234'])).toThrow(/40-hex/);
    expect(() => parseArgs(['check', '--sha', MAIN, '--need', 'ai-review'])).toThrow(/unknown context/);
    expect(() => parseArgs(['post', '--sha', MAIN, '--tree', TREE, '--context', 'gate'])).toThrow(/--state/);
    expect(() => parseArgs(['merge', '--sha', MAIN])).toThrow(/check \| post \| adopt/);
  });

  it('reads the repository out of either form of a github.com origin', () => {
    expect(repoFromUrl('https://github.com/parlornights/CrookedDuke.git')).toBe('parlornights/CrookedDuke');
    expect(repoFromUrl('git@github.com:parlornights/CrookedDuke.git')).toBe('parlornights/CrookedDuke');
    expect(repoFromUrl('http://127.0.0.1:1234/git/x')).toBeNull();
  });
});

describe('check', () => {
  it("passes on the head of the PR that made the commit, and names where", async () => {
    const github = fakeGitHub({
      pulls: { [MAIN]: [HEAD] },
      statuses: {
        [HEAD]: [
          status(CONTEXTS.checks, TREE, 'success', '2026-09-30T01:00:00Z'),
          status(CONTEXTS.gate, TREE, 'success', '2026-09-30T02:00:00Z', 'https://github.test/run/7'),
        ],
      },
    });
    const result = await cli(['check', '--sha', MAIN, '--tree', TREE], github);
    expect(result.code, result.err).toBe(EXIT_PASS);
    expect(result.out).toContain(`gate/tree: tree 1111111 passed on bbbbbbb (https://github.test/run/7)`);
    expect(github.calls.every((call) => call.auth === `Bearer ${TOKEN}`)).toBe(true);
    expect(github.calls.map((call) => new URL(call.url).pathname)).toContain(`/repos/owner/repo/commits/${MAIN}/pulls`);
  });

  it('is not a pass for a missing context (a passing gate without passing checks), a tree nobody tested whatever passed on the same commits, or evidence on neither the commit nor a parent when the PR lookup is refused', async () => {
    const gateOnly = fakeGitHub({ statuses: { [MAIN]: [status(CONTEXTS.gate, TREE, 'success', '2026-09-30T02:00:00Z')] } });
    const result = await cli(['check', '--sha', MAIN, '--tree', TREE], gateOnly);
    expect.soft(result.code, 'gate only').toBe(EXIT_MISSING);
    expect.soft(result.err, 'gate only').toContain('checks/tree: no status names this tree');
    expect.soft((await cli(['check', '--sha', MAIN, '--tree', TREE, '--need', 'gate'], gateOnly)).code, 'gate only, needing the gate').toBe(EXIT_PASS);
    const otherTree = fakeGitHub({
      statuses: {
        [MAIN]: [
          status(CONTEXTS.checks, OTHER_TREE, 'success', '2026-09-30T01:00:00Z'),
          status(CONTEXTS.gate, OTHER_TREE, 'success', '2026-09-30T01:00:00Z'),
        ],
      },
    });
    expect.soft((await cli(['check', '--sha', MAIN, '--tree', TREE], otherTree)).code, 'a tree nobody tested').toBe(EXIT_MISSING);
    expect.soft((await cli(['check', '--sha', MAIN, '--tree', TREE], refusePulls({}))).code, 'pulls refused, no evidence').toBe(EXIT_MISSING);
  });

  it('cannot ask without a token, without a repository, when GitHub refuses, when it is down, or when the statuses themselves are refused, and never says pass or prints the token', async () => {
    const github = fakeGitHub();
    expect.soft((await cli(['check', '--sha', MAIN, '--tree', TREE], github, { GITHUB_REPOSITORY: 'owner/repo' })).code, 'no token').toBe(EXIT_CANNOT_ASK);
    expect.soft(github.calls, 'no token').toEqual([]);
    const refused = fakeGitHub({ answer: async () => ({ ok: false, status: 401, json: async () => ({}) }) });
    const result = await cli(['check', '--sha', MAIN, '--tree', TREE], refused);
    expect.soft(result.code, '401').toBe(EXIT_CANNOT_ASK);
    expect.soft(result.err, '401').toContain('GitHub answered 401');
    expect.soft(result.err, '401').not.toContain(TOKEN);
    const down = fakeGitHub({
      answer: async () => {
        throw Object.assign(new Error('boom'), { name: 'TypeError' });
      },
    });
    expect.soft((await cli(['check', '--sha', MAIN, '--tree', TREE], down)).code, 'down').toBe(EXIT_CANNOT_ASK);
    const statusesRefused = fakeGitHub({ answer: async () => ({ ok: false, status: 403, json: async () => ({}) }) });
    expect.soft((await cli(['check', '--sha', MAIN, '--tree', TREE], statusesRefused)).code, 'statuses refused').toBe(EXIT_CANNOT_ASK);
  });
});

describe('a token that may not read pull requests (CD-204, #126)', () => {
  const passing = [
    status(CONTEXTS.checks, TREE, 'success', '2026-10-04T01:00:00Z'),
    status(CONTEXTS.gate, TREE, 'success', '2026-10-04T02:00:00Z'),
  ];

  it('still passes on the commit itself, and says the PR lookup was refused', async () => {
    const result = await cli(['check', '--sha', MAIN, '--tree', TREE], refusePulls({ [MAIN]: passing }));
    expect(result.code, result.err).toBe(EXIT_PASS);
    expect(result.err).toBe(
      `gate-status: GitHub answered 403 to GET /commits/${MAIN}/pulls; looked at aaaaaaa and its parents only`,
    );
  });

  it("finds the evidence on a parent, as the gate's Scope step needs for a merge commit's base", async () => {
    const github = refusePulls({ [PARENT]: passing });
    const { githubApi } = await import('../gate-status.mjs');
    const api = githubApi({ token: TOKEN, repo: 'owner/repo', base: 'https://api.github.test', fetchImpl: github.fetchImpl });
    const evidence = await findEvidence({ api, sha: MAIN, tree: TREE, need: ['checks', 'gate'], parents: [PARENT] });
    expect(evidence.passed).toBe(true);
    expect(evidence.pullsSkipped).toMatch(/403/);
    expect(evidence.results.map((result) => result.status.on)).toEqual([PARENT, PARENT]);
  });

});

describe('post and adopt', () => {
  it('posts the tree and the verdict as the status the gate workflow ends with', async () => {
    const github = fakeGitHub();
    const env = { ...ENV, GITHUB_RUN_ID: '42', GITHUB_SERVER_URL: 'https://github.test' };
    const result = await cli(['post', '--sha', HEAD, '--tree', TREE, '--context', 'gate', '--state', 'failure'], github, env);
    expect(result.code).toBe(EXIT_PASS);
    expect(github.calls).toHaveLength(1);
    expect(new URL(github.calls[0].url).pathname).toBe(`/repos/owner/repo/statuses/${HEAD}`);
    expect(github.calls[0].body).toEqual({
      context: 'gate/tree',
      state: 'failure',
      description: `tree ${TREE} failed`,
      target_url: 'https://github.test/owner/repo/actions/runs/42',
    });
  });

  it("adopts evidence found elsewhere onto the commit, posts nothing it already carries, and adopts nothing when the tree has not passed", async () => {
    const github = fakeGitHub({
      pulls: { [MAIN]: [HEAD] },
      statuses: {
        [MAIN]: [status(CONTEXTS.checks, TREE, 'success', '2026-09-30T01:00:00Z')],
        [HEAD]: [status(CONTEXTS.gate, TREE, 'success', '2026-09-30T02:00:00Z', 'https://github.test/run/9')],
      },
    });
    const result = await cli(['adopt', '--sha', MAIN, '--tree', TREE], github);
    expect(result.code, result.err).toBe(EXIT_PASS);
    const posts = github.calls.filter((call) => call.method === 'POST');
    expect(posts).toHaveLength(1);
    expect.soft(new URL(posts[0].url).pathname).toBe(`/repos/owner/repo/statuses/${MAIN}`);
    expect.soft(posts[0].body).toEqual({
      context: 'gate/tree',
      state: 'success',
      description: `tree ${TREE} passed`,
      target_url: 'https://github.test/run/9',
    });
    const unpassed = fakeGitHub({ pulls: { [MAIN]: [PARENT] } });
    const nothing = await cli(['adopt', '--sha', MAIN, '--tree', TREE], unpassed);
    expect.soft(nothing.code, 'not passed').toBe(EXIT_MISSING);
    expect.soft(unpassed.calls.filter((call) => call.method === 'POST'), 'not passed').toEqual([]);
  });
});

describe('merged-pr: main runs no tests (owner, 5 Oct)', () => {
  const json = (body) => ({ ok: true, status: 200, json: async () => body });
  const github = ({ pulls, statuses }) => fakeGitHub({
    answer: (url) => {
      const path = new URL(url).pathname;
      if (path.endsWith(`/commits/${MAIN}/pulls`)) return json(pulls);
      const match = /\/commits\/([0-9a-f]{40})\/statuses$/.exec(path);
      if (match) return json(statuses[match[1]] ?? []);
      return { ok: false, status: 404, json: async () => ({}) };
    },
  });
  const merged = [{ number: 7, merge_commit_sha: MAIN, head: { sha: HEAD } }];

  it("passes only when the merged PR's newest checks on its head passed, whatever tree it names; not when they failed, when there is none, for a direct push, or for a PR merged as another commit", async () => {
    const result = await cli(['merged-pr', '--sha', MAIN], github({
      pulls: merged,
      statuses: { [HEAD]: [status(CONTEXTS.checks, OTHER_TREE, 'success', 't1')] },
    }));
    expect.soft(result.code, 'passed').toBe(EXIT_PASS);
    expect.soft(result.out, 'passed').toMatch(/#7's head bbbbbbb/);
    const missing = [
      ['the newest failed', { pulls: merged, statuses: { [HEAD]: [status(CONTEXTS.checks, TREE, 'success', 't1'), status(CONTEXTS.checks, TREE, 'failure', 't2')] } }],
      ['none', { pulls: merged, statuses: { [HEAD]: [] } }],
      ['a direct push', { pulls: [], statuses: {} }],
      ['merged as another commit', { pulls: [{ number: 8, merge_commit_sha: PARENT, head: { sha: HEAD } }], statuses: {} }],
    ];
    for (const [label, answers] of missing) expect.soft((await cli(['merged-pr', '--sha', MAIN], github(answers))).code, label).toBe(EXIT_MISSING);
  });

  it("takes a merge commit's second parent as the PR head when GitHub cannot list the commit's pull requests, and still fails a single-parent commit", async () => {
    const api = {
      pulls: async () => {
        throw new CannotAsk('GitHub answered 500 to GET /commits/x/pulls');
      },
      statuses: async (sha) => (sha === HEAD ? [status(CONTEXTS.checks, OTHER_TREE, 'success', 't1')] : []),
    };
    const merge = await mergedPrPassed({ api, sha: MAIN, need: ['checks'], parents: [PARENT, HEAD] });
    expect.soft(merge.passed).toBe(true);
    expect.soft(merge.pull.head.sha).toBe(HEAD);
    await expect(mergedPrPassed({ api, sha: MAIN, need: ['checks'], parents: [PARENT] })).rejects.toThrow(CannotAsk);
  });

  it("asks GitHub for the commit's pull requests again after a 5xx, with a growing pause, and takes the second parent only after three 5xx answers", async () => {
    const answering = (pullAnswers) => {
      const asked = [];
      const fetchImpl = async (url) => {
        const path = new URL(url).pathname;
        asked.push(path);
        if (path.endsWith('/pulls')) return pullAnswers.shift();
        const match = /\/commits\/([0-9a-f]{40})\/statuses$/.exec(path);
        return json(match?.[1] === HEAD ? [status(CONTEXTS.checks, OTHER_TREE, 'success', 't1')] : []);
      };
      const pauses = [];
      const api = githubApi({ token: TOKEN, repo: 'owner/repo', base: 'https://api.github.test', fetchImpl, sleep: async (ms) => pauses.push(ms) });
      return { api, asked, pauses };
    };
    const failing = { ok: false, status: 500, json: async () => ({}) };
    const pullsAsked = (asked) => asked.filter((path) => path.endsWith('/pulls')).length;

    const recovered = answering([failing, json([{ number: 9, merge_commit_sha: MAIN, head: { sha: HEAD } }])]);
    const passed = await mergedPrPassed({ api: recovered.api, sha: MAIN, need: ['checks'], parents: [PARENT, PARENT] });
    expect.soft(passed.passed, 'a 500 then the list').toBe(true);
    expect.soft(passed.pull.number, 'the listed PR, not the fallback').toBe(9);
    expect.soft(pullsAsked(recovered.asked), 'asked twice').toBe(2);
    expect.soft(recovered.pauses, 'one pause').toEqual([RETRY_BACKOFF_MS]);

    const down = answering([failing, failing, failing]);
    const fallback = await mergedPrPassed({ api: down.api, sha: MAIN, need: ['checks'], parents: [PARENT, HEAD] });
    expect.soft(fallback.passed, 'three 500s').toBe(true);
    expect.soft(fallback.pull, 'the second parent').toEqual({ number: null, head: { sha: HEAD } });
    expect.soft(pullsAsked(down.asked), 'asked three times').toBe(PULLS_TRIES);
    expect.soft(down.pauses, 'two growing pauses').toEqual([RETRY_BACKOFF_MS, 2 * RETRY_BACKOFF_MS]);

    const refused = answering([{ ok: false, status: 403, json: async () => ({}) }]);
    await expect(refused.api.pulls(MAIN), 'a 4xx is not asked again').rejects.toThrow(/403/);
    expect.soft(pullsAsked(refused.asked), 'a 4xx once').toBe(1);
  });
});
