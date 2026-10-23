/**
 * A repository with one pull request of each kind against `main` (CD-269), each branched from the
 * seed: `harness` (harness-only), `mixed` (harness and infra), `product` (an app file), `moved` (an
 * infra file moved into .claude/) and `deleted` (an infra file deleted). `main` has moved on since
 * with an app file and main's `harness-only.mjs`, so only a diff against the merge base reads the PR
 * alone, and `origin/main` names it, as in a runner's full checkout. The repository is
 * `<root>/tested`, as `pr-issue.yml` checks it out.
 */
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const GIT_ENV = {
  GIT_AUTHOR_NAME: 't',
  GIT_AUTHOR_EMAIL: 't@example.invalid',
  GIT_COMMITTER_NAME: 't',
  GIT_COMMITTER_EMAIL: 't@example.invalid',
  GIT_CONFIG_GLOBAL: '/dev/null',
  GIT_CONFIG_NOSYSTEM: '1',
};

export const PULL_REQUESTS = {
  harness: ['.claude/hooks/x.sh', '.claude/skills/x/SKILL.md', 'CLAUDE.md'],
  mixed: ['.claude/hooks/x.sh', 'infra/scripts/x.mjs'],
  product: ['apps/app/src/x.ts'],
};

/** The PRs that only move or delete a file, and the paths their diff names. */
export const MOVED = ['.claude/scripts/moved.mjs', 'infra/scripts/moved.mjs'];
export const DELETED = ['infra/scripts/gone.mjs'];

const SCRIPT = fileURLToPath(new URL('../harness-only.mjs', import.meta.url));

export function harnessFixture() {
  const root = mkdtempSync(join(tmpdir(), 'harness-only-'));
  const repo = join(root, 'tested');
  mkdirSync(repo);
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: repo, encoding: 'utf8', env: { ...process.env, ...GIT_ENV } });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')}: ${result.stderr}`);
    return result.stdout.trim();
  };
  const write = (path, text) => {
    mkdirSync(join(repo, dirname(path)), { recursive: true });
    writeFileSync(join(repo, path), text);
  };
  const commit = (paths, text) => {
    for (const path of paths) write(path, text);
    git('add', '--', ...paths);
    git('commit', '-q', '-m', text);
  };
  git('init', '-q', '-b', 'main');
  commit([...new Set(Object.values(PULL_REQUESTS).flat()), 'infra/scripts/moved.mjs', ...DELETED], 'seed\n');
  for (const [branch, paths] of Object.entries(PULL_REQUESTS)) {
    git('checkout', '-q', '-b', branch, 'main');
    commit(paths, `${branch}\n`);
  }
  git('checkout', '-q', '-b', 'moved', 'main');
  mkdirSync(join(repo, '.claude/scripts'), { recursive: true });
  git('mv', 'infra/scripts/moved.mjs', '.claude/scripts/moved.mjs');
  git('commit', '-q', '-m', 'moved');
  git('checkout', '-q', '-b', 'deleted', 'main');
  git('rm', '-q', ...DELETED);
  git('commit', '-q', '-m', 'deleted');
  git('checkout', '-q', 'main');
  mkdirSync(join(repo, 'infra/scripts'), { recursive: true });
  copyFileSync(SCRIPT, join(repo, 'infra/scripts/harness-only.mjs'));
  write('apps/app/src/moved-on.ts', 'main moved on\n');
  git('add', '--', 'infra/scripts/harness-only.mjs', 'apps/app/src/moved-on.ts');
  git('commit', '-q', '-m', 'main moved on');
  git('update-ref', 'refs/remotes/origin/main', 'main');
  const main = git('rev-parse', 'main');
  return { root, repo, main, git, checkout: (branch) => git('checkout', '-q', branch) };
}
