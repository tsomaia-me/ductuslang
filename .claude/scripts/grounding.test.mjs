import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { NOTE_LIMIT } from './grounding.mjs';

const SCRIPT = path.join(path.dirname(fileURLToPath(import.meta.url)), 'grounding.mjs');
const SESSION_ID = 'session-abc';
const BRANCH = 'memory';

const gitEnv = { ...process.env, GIT_CONFIG_GLOBAL: '/dev/null', GIT_CONFIG_NOSYSTEM: '1' };
delete gitEnv.MEMORY_SESSION;
delete gitEnv.MEMORY_DIR;
delete gitEnv.CLAUDE_CODE_SESSION_ID;

function git(cwd, ...args) {
  return execFileSync('git', ['-c', 'user.name=Test', '-c', 'user.email=test@example.com', ...args], { cwd, env: gitEnv, encoding: 'utf8', stdio: 'pipe' });
}

function setup(t) {
  const root = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'grounding-test-')));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const origin = path.join(root, 'origin.git');
  const seed = path.join(root, 'seed');
  git(root, 'init', '-q', '--bare', origin);
  git(root, 'init', '-q', '-b', BRANCH, seed);
  writeFileSync(path.join(seed, 'LESSONS.md'), 'L1\n');
  git(seed, 'add', 'LESSONS.md');
  git(seed, 'commit', '-q', '-m', 'seed');
  git(seed, 'push', '-q', origin, `${BRANCH}:${BRANCH}`);
  git(origin, 'symbolic-ref', 'HEAD', `refs/heads/${BRANCH}`);
  const product = path.join(root, 'product');
  git(root, 'init', '-q', '-b', 'main', product);
  writeFileSync(path.join(product, 'README'), 'p\n');
  git(product, 'add', 'README');
  git(product, 'commit', '-q', '-m', 'product');
  const memory = path.join(product, '.memory');
  git(root, 'clone', '-q', origin, memory);
  git(memory, 'config', 'user.name', 'Test');
  git(memory, 'config', 'user.email', 'test@example.com');
  return { root, origin, product, memory };
}

function run(env, { args = ['--role', 'BUILDER'], note = 'Doing X.\nNext: Y.\n', sessionId = SESSION_ID, cwd = env.root, memoryDir = env.memory } = {}) {
  const childEnv = { ...gitEnv, CLAUDE_HARNESS_DIR: path.join(env.root, 'harness'), CLAUDE_CODE_SESSION_ID: sessionId };
  if (memoryDir !== null) childEnv.MEMORY_DIR = memoryDir;
  return spawnSync(process.execPath, [SCRIPT, ...args], { cwd, input: note, env: childEnv, encoding: 'utf8' });
}

function originFile(env, file) {
  return git(env.origin, 'show', `${BRANCH}:${file}`);
}

function originHas(env, file) {
  return spawnSync('git', ['show', `${BRANCH}:${file}`], { cwd: env.origin, env: gitEnv }).status === 0;
}

test('stores the role, the note and the time in the memory checkout under grounding/<session id>.json and pushes that path alone', (t) => {
  const env = setup(t);
  writeFileSync(path.join(env.memory, 'other.md'), 'not mine\n');
  const { status, stdout } = run(env);
  const stored = path.join(env.memory, 'grounding', `${SESSION_ID}.json`);
  assert.equal(status, 0);
  assert.equal(stdout, `STORED       ${stored}\n`);
  const note = JSON.parse(readFileSync(stored, 'utf8'));
  assert.equal(note.role, 'BUILDER');
  assert.equal(note.note, 'Doing X.\nNext: Y.');
  assert.ok(!Number.isNaN(Date.parse(note.time)));
  assert.equal(originFile(env, `grounding/${SESSION_ID}.json`), readFileSync(stored, 'utf8'));
  assert.equal(git(env.origin, 'diff-tree', '--no-commit-id', '--name-only', '-r', BRANCH), `grounding/${SESSION_ID}.json\n`);
  assert.ok(!originHas(env, 'other.md'));
});

test('finds the memory checkout beside the main checkout, from the main checkout and from a worktree of it', (t) => {
  const env = setup(t);
  const worktree = path.join(env.root, 'worktree');
  git(env.product, 'worktree', 'add', '-q', worktree, '-b', 'wt');
  for (const [cwd, sessionId] of [[env.product, 'from-main'], [worktree, 'from-worktree']]) {
    const { status, stdout } = run(env, { cwd, sessionId, memoryDir: null });
    const stored = path.join(env.memory, 'grounding', `${sessionId}.json`);
    assert.equal(status, 0);
    assert.equal(stdout, `STORED       ${stored}\n`);
    assert.ok(existsSync(stored));
    assert.ok(originHas(env, `grounding/${sessionId}.json`));
  }
  assert.ok(!existsSync(path.join(worktree, '.memory')));
});

test('a blocked push still stores the note and says the push script block line', (t) => {
  const env = setup(t);
  const hook = path.join(env.origin, 'hooks', 'pre-receive');
  writeFileSync(hook, '#!/bin/sh\nexit 1\n');
  chmodSync(hook, 0o755);
  const { status, stdout } = run(env);
  const stored = path.join(env.memory, 'grounding', `${SESSION_ID}.json`);
  assert.equal(status, 0);
  assert.equal(stdout, `STORED       ${stored}\nNEXT         push blocked: push rejected twice grounding/${SESSION_ID}.json\n`);
  assert.equal(JSON.parse(readFileSync(stored, 'utf8')).note, 'Doing X.\nNext: Y.');
  assert.ok(!originHas(env, `grounding/${SESSION_ID}.json`));
});

test('refuses and stores nothing', (t) => {
  const env = setup(t);
  const missing = path.join(env.root, 'missing');
  const rows = [
    { args: ['--role', 'builder'], note: 'x'.repeat(NOTE_LIMIT + 1), status: 1, line: `NEXT         ${NOTE_LIMIT + 1} characters, over ${NOTE_LIMIT}: cut the note under 4,000 and run again` },
    { args: ['--role', 'shared'], note: 'n', status: 1, line: 'NEXT         unknown role shared: name your role, a lessons file such as BUILDER or a file under .claude/agents/ or .claude/skills/' },
    { args: ['--role', '../agents/builder'], note: 'n', status: 1, line: 'NEXT         unknown role ../agents/builder: name your role, a lessons file such as BUILDER or a file under .claude/agents/ or .claude/skills/' },
    { args: [], note: 'n', status: 2, line: 'NEXT         wrong arguments: run pnpm grounding --role <ROLE> with the note on stdin' },
    { args: ['--role', 'builder'], note: 'n', sessionId: '', status: 1, line: "NEXT         no session id: run it from your session's Bash tool" },
    { args: ['--role', 'builder'], note: ' \n', status: 1, line: 'NEXT         no note on stdin: pipe the note in' },
    { args: ['--role', 'builder'], note: 'n', sessionId: '../escape', status: 1, line: 'NEXT         session id ../escape is not a plain id: letters, digits, - and _ only' },
    { args: ['--role', 'builder'], note: 'n', sessionId: 'a/b', status: 1, line: 'NEXT         session id a/b is not a plain id: letters, digits, - and _ only' },
    { args: ['--role', 'builder'], note: 'n', memoryDir: missing, status: 1, line: `NEXT         no memory checkout ${missing}: run it from the repository or set MEMORY_DIR` },
  ];
  for (const { args, note, sessionId, memoryDir, status, line } of rows) {
    const result = run(env, { args, note, sessionId, memoryDir });
    assert.equal(result.status, status);
    assert.equal(result.stdout, `${line}\n`);
    assert.ok(!existsSync(path.join(env.memory, 'grounding')));
    assert.ok(!existsSync(path.join(env.product, 'escape.json')));
    assert.ok(!existsSync(missing));
    assert.equal(git(env.memory, 'status', '--porcelain'), '');
  }
});
