import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { assertStubsRun, childEnv } from './hermetic.mjs';

const SCRIPT = fileURLToPath(new URL('../explore.sh', import.meta.url));
const CLAUDE_STUB = `#!/usr/bin/env bash
printf '%s\\n' "$@" > "$EXPLORE_STUB_DIR/args"
pwd -P > "$EXPLORE_STUB_DIR/cwd"
cat > "$EXPLORE_STUB_DIR/stdin"
echo 'apps/admin/src/a.ts:4: export const label = "Notice";'
exit "\${EXPLORE_STUB_EXIT:-0}"
`;

const QUESTION = 'Where does the settings form name the maintenance label?';

let root;
let repo;

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'explore-'));
  repo = join(root, 'repo');
  mkdirSync(join(repo, 'apps', 'admin'), { recursive: true });
  spawnSync('git', ['init', '-q', repo]);
  const bin = join(root, 'bin');
  mkdirSync(bin);
  writeFileSync(join(bin, 'claude'), CLAUDE_STUB, { mode: 0o755 });
  assertStubsRun(bin);
});

afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

const explore = (args, { exit = 0 } = {}) => {
  const env = childEnv({ PATH: `${join(root, 'bin')}:${process.env.PATH}`, EXPLORE_STUB_DIR: root, EXPLORE_STUB_EXIT: String(exit) });
  return spawnSync('bash', [SCRIPT, ...args], { cwd: join(repo, 'apps', 'admin'), env, encoding: 'utf8' });
};

const seen = (name) => readFileSync(join(root, name), 'utf8');

describe('explore.sh', () => {
  it("runs the explore agent on Haiku, read-only, at the top of the checkout, asks the question for only the lines that answer it with their file:line, names the memory archives in the main checkout's memory for a question whether a decision was already made, and prints what the agent answers", () => {
    const run = explore([QUESTION]);
    expect.soft(run.status).toBe(0);
    expect.soft(run.stdout).toBe('apps/admin/src/a.ts:4: export const label = "Notice";\n');
    expect.soft(seen('args').trim().split('\n')).toEqual(['-p', '--agent', 'Explore', '--model', 'claude-haiku-5-5', '--allowedTools', 'Read', 'Grep', 'Glob']);
    expect.soft(seen('cwd').trim()).toBe(realpathSync(repo));
    const prompt = seen('stdin');
    expect.soft(prompt).toContain(`The question:\n\n${QUESTION}`);
    expect.soft(prompt).toMatch(/^FACTS: only the lines that answer it, one per line as path:line followed by the line/m);
    expect.soft(prompt).toMatch(/^NOT FOUND: /m);
    expect.soft(prompt).toContain('Read only; change nothing.');
    const memory = join(realpathSync(repo), '.memory');
    expect.soft(prompt).toContain(`The archives, ${join(memory, 'archive')}/ and ${join(memory, 'tasks', 'archive')}/, are read only when the question is whether a decision was already made.`);
    expect.soft(prompt).not.toContain('decisions-archive.md');
    expect.soft(prompt).toContain(`The memory checkout is ${memory}.`);
  });

  it('names the main checkout\'s memory when run in a worktree', () => {
    spawnSync('git', ['-C', repo, '-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '--allow-empty', '-m', 'seed']);
    const worktree = join(root, 'worktree');
    spawnSync('git', ['-C', repo, 'worktree', 'add', '-q', worktree]);
    const env = childEnv({ PATH: `${join(root, 'bin')}:${process.env.PATH}`, EXPLORE_STUB_DIR: root, EXPLORE_STUB_EXIT: '0' });
    expect(spawnSync('bash', [SCRIPT, QUESTION], { cwd: worktree, env, encoding: 'utf8' }).status).toBe(0);
    expect(seen('stdin')).toContain(`The memory checkout is ${join(realpathSync(repo), '.memory')}.`);
  });

  it('keeps the agent to one file when asked with --file', () => {
    const file = join(root, 'cut.txt');
    writeFileSync(file, 'x\n');
    expect(explore(['--file', file, QUESTION]).status).toBe(0);
    const prompt = seen('stdin');
    expect(prompt).toContain(`The question is about one file, ${file}: Grep it and Read it in ranges, and answer from it alone.`);
    expect(prompt).toContain(`The question:\n\n${QUESTION}`);
    expect(seen('args').trim().split('\n')).toEqual(['-p', '--agent', 'Explore', '--model', 'claude-haiku-5-5', '--allowedTools', 'Read', 'Grep', 'Glob']);
  });

  it("exits with the agent's code when it fails", () => {
    expect(explore([QUESTION], { exit: 7 }).status).toBe(7);
  });

  it('refuses a call without exactly one question, or with a --file naming no file, without starting an agent', () => {
    for (const [label, args] of [
      ['no question', []],
      ['an empty question', ['']],
      ['two arguments', [QUESTION, QUESTION]],
      ['--file with no question', ['--file', '/etc/hostname']],
      ['--file naming no file', ['--file', '/nowhere/x.txt', QUESTION]],
    ]) {
      rmSync(join(root, 'args'), { force: true });
      const run = explore(args);
      expect.soft(run.status, label).toBe(2);
      expect.soft(run.stderr, label).toContain('usage: pnpm explore [--file <path>] "<question>"');
      expect.soft(() => seen('args'), label).toThrow();
    }
  });
});
