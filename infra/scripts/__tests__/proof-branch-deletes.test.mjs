import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { run as releasedProofsCli } from '../release-proofs.mjs';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));
const read = (name) => parse(readFileSync(join(WORKFLOWS, name), 'utf8'));

describe('the proof branches are deleted one by one, each by its own PR branch, and only those', () => {
  let bin;
  let log;

  beforeAll(() => {
    bin = mkdtempSync(join(tmpdir(), 'proof-deletes-'));
    log = join(bin, 'calls');
    writeFileSync(
      join(bin, 'curl'),
      [
        '#!/usr/bin/env bash',
        'url="${@: -1}"',
        'echo "$url" >>"$CURL_LOG"',
        'case "$url" in',
        '  *pulls\\?*unreadable*) exit 22 ;;',
        '  *pulls\\?*reused*) printf \'[\\n  {\\n    "number": 7\\n  }\\n]\\n\' ;;',
        '  *pulls\\?*) printf \'[\\n\\n]\\n\' ;;',
        '  *fail*) printf 000; exit 28 ;;',
        '  *missing*) printf 422 ;;',
        '  *) printf 204 ;;',
        'esac',
        '',
      ].join('\n'),
    );
    chmodSync(join(bin, 'curl'), 0o755);
    mkdirSync(join(bin, 'failing'));
    writeFileSync(join(bin, 'failing', 'node'), '#!/usr/bin/env bash\necho "release-proofs: GitHub answered 502" >&2\nexit 1\n');
    chmodSync(join(bin, 'failing', 'node'), 0o755);
  });

  afterAll(() => rmSync(bin, { recursive: true, force: true }));

  const runStep = (step, env, path = bin) => {
    rmSync(log, { force: true });
    const result = spawnSync('bash', ['-e', '-c', step.run], {
      encoding: 'utf8',
      env: { PATH: `${path}:${process.env.PATH}`, CURL_LOG: log, GITHUB_API_URL: 'https://api.test', GITHUB_REPOSITORY: 'o/r', GITHUB_TOKEN: 't', GITHUB_OUTPUT: join(bin, 'output'), ...env },
    });
    const calls = existsSync(log) ? readFileSync(log, 'utf8').split('\n').filter(Boolean) : [];
    return { ...result, calls };
  };
  const REF = 'https://api.test/repos/o/r/git/refs/heads/';

  const OPEN = (ref) => `https://api.test/repos/o/r/pulls?state=open&per_page=1&head=o:${ref}`;

  it('proof-branch.yml runs for a PR of this repository closed without a merge, and deletes proof/<its branch without claude/> alone, unless an open PR has that branch or the open PRs cannot be read', () => {
    const workflow = read('proof-branch.yml');
    expect.soft(workflow.on).toEqual({ pull_request_target: { types: ['closed'], branches: ['main'] } });
    const job = workflow.jobs.delete;
    expect.soft(job.if).toBe('${{ !github.event.pull_request.merged && github.event.pull_request.head.repo.full_name == github.repository }}');
    const [step] = job.steps;
    for (const [ref, deleted] of [
      ['claude/cd-478-proof-rework', 'proof/cd-478-proof-rework'],
      ['claude/../deploy/api', null],
      ['claude/a/b', null],
      ['claude/', null],
      ['deploy/api', null],
      ['xclaude/cd-1-a', null],
      ['claude/cd-1-a\nclaude/cd-2-b', null],
    ]) {
      const result = runStep(step, { HEAD_REF: ref });
      expect.soft(result.status, ref).toBe(0);
      expect.soft(result.calls, ref).toEqual(deleted ? [OPEN(ref), `${REF}${deleted}`] : []);
    }
    const reused = runStep(step, { HEAD_REF: 'claude/debt-sweep-reused' });
    expect.soft(reused.status, 'reused').toBe(0);
    expect.soft(reused.calls, 'reused').toEqual([OPEN('claude/debt-sweep-reused')]);
    expect.soft(reused.stdout, 'reused').toContain('proof/debt-sweep-reused stays');
    const unreadable = runStep(step, { HEAD_REF: 'claude/debt-sweep-unreadable' });
    expect.soft(unreadable.status, 'unreadable').toBe(1);
    expect.soft(unreadable.calls, 'unreadable').toEqual([OPEN('claude/debt-sweep-unreadable')]);
    expect.soft(unreadable.stdout, 'unreadable').toContain('::error::');
  });

  it('release-please.yml deletes each released PR’s proof branch, goes on past a failed delete, names it, and ends non-zero, and fails the step that names the branches when its script fails', () => {
    const named = read('release-please.yml').jobs.proofs.steps.find((step) => step.id === 'proofs');
    expect.soft(runStep(named, { PRS: '1' }, join(bin, 'failing')).status, 'a failed read').not.toBe(0);
    const job = read('release-please.yml').jobs['proof-branches'];
    const [step] = job.steps;
    expect.soft(step['continue-on-error']).toBe(true);
    const result = runStep(step, { PROOFS: 'cd-1-a cd-2-fail ../x cd-3-missing cd-4-d' });
    expect.soft(result.calls).toEqual(['cd-1-a', 'cd-2-fail', 'cd-3-missing', 'cd-4-d'].map((slug) => `${REF}proof/${slug}`));
    expect.soft(result.stdout).toContain('::warning::proof/cd-2-fail was not deleted (curl exit 28)');
    expect.soft(result.stdout).toContain('::warning::../x names no proof branch');
    expect.soft(result.stdout).toContain('deleted proof/cd-4-d');
    expect.soft(result.status).toBe(1);
  });
});

describe('the release names the proof branches of the PRs it merged, from GitHub', () => {
  it('prints the proof branch of each merged PR from a claude/ branch of this repository with no open PR on that branch, and skips a PR GitHub refuses with a warning naming it', async () => {
    const pulls = {
      1: { merged_at: '2026-10-20T10:00:00Z', head: { ref: 'claude/cd-440-admin-restructure', repo: { full_name: 'o/r' } } },
      2: { merged_at: null, head: { ref: 'claude/cd-441-open', repo: { full_name: 'o/r' } } },
      3: { merged_at: '2026-10-20T10:00:00Z', head: { ref: 'feature-x', repo: { full_name: 'o/r' } } },
      4: { merged_at: '2026-10-20T10:00:00Z', head: { ref: 'claude/cd-442-fork', repo: { full_name: 'fork/r' } } },
      5: { merged_at: '2026-10-20T10:00:00Z', head: { ref: 'claude/proof-rework', repo: { full_name: 'o/r' } } },
      6: { merged_at: '2026-10-20T10:00:00Z', head: { ref: 'claude/debt-sweep', repo: { full_name: 'o/r' } } },
      7: { merged_at: '2026-10-20T10:00:00Z', head: { ref: 'claude/unreadable-open', repo: { full_name: 'o/r' } } },
    };
    const open = { 'o:claude/debt-sweep': [{ number: 12 }] };
    const fetchImpl = async (url) => {
      const { pathname, searchParams } = new URL(url);
      const number = Number(pathname.split('/').at(-1));
      if (number === 501) return { ok: false, status: 500, json: async () => ({}) };
      if (number === 504) throw Object.assign(new Error('timed out'), { name: 'TimeoutError' });
      if (searchParams.get('head') === 'o:claude/unreadable-open') return { ok: false, status: 502, json: async () => ({}) };
      if (searchParams.has('head')) return { ok: true, status: 200, json: async () => open[searchParams.get('head')] ?? [] };
      if (!pulls[number]) return { ok: false, status: 404, json: async () => ({}) };
      return { ok: true, status: 200, json: async () => ({ title: '', body: '', ...pulls[number] }) };
    };
    const out = [];
    const err = [];
    const code = await releasedProofsCli(['--prs', '1 501 2 3 404 4 504 5 6 7'], { GITHUB_REPOSITORY: 'o/r', GITHUB_TOKEN: 't', GITHUB_API_URL: 'https://api.test' }, { fetchImpl, out: (line) => out.push(line), err: (line) => err.push(line) });
    expect.soft(code).toBe(0);
    expect.soft(out).toEqual(['proofs=cd-440-admin-restructure proof-rework']);
    for (const pr of [501, 404, 504]) expect.soft(err.join('\n'), `PR #${pr}`).toMatch(new RegExp(`^::warning::.*PR #${pr}\\b`, 'm'));
    expect.soft(err.join('\n'), 'reused branch').toMatch(/proof\/debt-sweep stays/);
    expect.soft(err.join('\n'), 'unreadable open PRs').toMatch(/^::warning::.*proof\/unreadable-open stays/m);
  });
});
