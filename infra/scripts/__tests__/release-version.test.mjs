import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { notesFrom, parseArgs, run } from '../release-version.mjs';

const IDENTITY = {
  GIT_AUTHOR_NAME: 'release test',
  GIT_AUTHOR_EMAIL: 'release@test.invalid',
  GIT_COMMITTER_NAME: 'release test',
  GIT_COMMITTER_EMAIL: 'release@test.invalid',
  GIT_CONFIG_NOSYSTEM: '1',
  GIT_CONFIG_GLOBAL: '/dev/null',
};

describe('the notes', () => {
  it('names the PRs (merge or squash) and the issues of the range, each once, in order', () => {
    expect(
      notesFrom([
        { subject: 'Merge pull request #131 from parlornights/wt/a', body: 'feat: a (CD-203)' },
        { subject: 'fix(app): b (#129)', body: 'Refs: CD-12, PAR-3' },
        { subject: 'chore: c CD-203 and CD-9', body: '' },
      ]),
    ).toEqual({ prs: [129, 131], issues: ['CD-9', 'CD-12', 'CD-203', 'PAR-3'] });
  });

  it('refuses a usage it cannot honour', () => {
    expect(() => parseArgs(['plan', '--sha', 'x'])).toThrow();
    expect(() => parseArgs(['notes'])).toThrow(/--sha/);
    expect(() => parseArgs(['notes', '--sha'])).toThrow(/needs a value/);
    expect(parseArgs(['notes', '--sha', 'x', '--from', 'y'])).toMatchObject({ sha: 'x', from: 'y' });
  });
});

describe('against a real repository', () => {
  let work;
  const git = (...args) => execFileSync('git', args, { cwd: work, encoding: 'utf8', env: { ...process.env, ...IDENTITY } }).trim();
  const cli = (argv) => {
    const out = [];
    const err = [];
    const code = run(argv, { cwd: work, env: { ...process.env, ...IDENTITY }, out: (l) => out.push(l), err: (l) => err.push(l) });
    return { code, out: out.join('\n'), err: err.join('\n') };
  };
  const commit = (message) => {
    writeFileSync(join(work, 'file.txt'), `${message}\n`);
    git('add', '.');
    git('commit', '--quiet', '-m', message);
    return git('rev-parse', 'HEAD');
  };

  beforeEach(() => {
    work = mkdtempSync(join(tmpdir(), 'release-version-'));
    git('init', '--quiet', '--initial-branch=main');
    commit('chore: start');
  });

  afterEach(() => rmSync(work, { recursive: true, force: true }));

  it('prints the PRs and issues between --from and --sha, and of the tip alone without --from', () => {
    const first = commit('Merge pull request #7 from parlornights/wt/x\n\nfeat: x (CD-201)');
    const second = commit('fix: y (#8)\n\nRefs: CD-203');
    expect(cli(['notes', '--sha', second, '--from', first])).toEqual({ code: 0, out: 'prs=8\nissues=CD-203', err: '' });
    expect(cli(['notes', '--sha', second, '--from', 'HEAD~2']).out).toBe('prs=7 8\nissues=CD-201 CD-203');
    expect(cli(['notes', '--sha', first]).out).toBe('prs=7\nissues=CD-201');
  });

  it('refuses a commit it cannot find, and a usage error, by their exit codes', () => {
    expect(cli(['notes', '--sha', 'nowhere'])).toMatchObject({ code: 2, err: 'release-version: usage: "nowhere" names no commit here' });
    expect(cli(['line', '--sha', 'HEAD']).code).toBe(2);
  });
});
