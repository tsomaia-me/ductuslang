import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { NOTE_LIMIT } from '../scripts/grounding.mjs';

import { HOOK_OUTPUT_LIMIT, MENU_LINE } from './session.mjs';
import { LAST_MESSAGES_FILE, NOTE_FILE } from './shared.mjs';

const HOOK = path.join(path.dirname(fileURLToPath(import.meta.url)), 'session.mjs');
const ROOT = path.resolve(path.dirname(HOOK), '..', '..');
const SESSION_ID = 'session-abc';
const LESSONS = 'L1 Do X, because Y.\nL2 Do Z, because W.';

function makeProject(lessons) {
  const project = mkdtempSync(path.join(os.tmpdir(), 'session-hook-project-'));
  if (lessons !== undefined) {
    mkdirSync(path.join(project, '.memory'));
    writeFileSync(path.join(project, '.memory', 'LESSONS.md'), lessons);
  }
  return project;
}

function runHook({ project, source = 'startup', stdin, hook = HOOK, note }) {
  const harness = mkdtempSync(path.join(os.tmpdir(), 'session-hook-harness-'));
  if (note !== undefined) {
    mkdirSync(path.join(harness, SESSION_ID));
    writeFileSync(path.join(harness, SESSION_ID, NOTE_FILE), typeof note === 'string' ? note : JSON.stringify(note));
  }
  const input = stdin ?? JSON.stringify({
    session_id: SESSION_ID,
    transcript_path: path.join(harness, 'transcript.jsonl'),
    cwd: project,
    hook_event_name: 'SessionStart',
    source,
  });
  const result = spawnSync(process.execPath, [hook], {
    input,
    encoding: 'utf8',
    env: { ...process.env, CLAUDE_PROJECT_DIR: project, CLAUDE_HARNESS_DIR: harness },
  });
  return { ...result, harness };
}

test('startup prints the lessons verbatim then the menu line and no last-messages line', () => {
  const project = makeProject(`${LESSONS}\n`);
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${MENU_LINE}\n`);
  assert.ok(!stdout.includes('messages:'));
});

test('compact with no note says so, then the lessons, the menu line and the last-messages line', () => {
  const project = makeProject(LESSONS);
  const { stdout, status, harness } = runHook({ project, source: 'compact' });
  assert.equal(status, 0);
  const expectedPath = path.join(harness, SESSION_ID, LAST_MESSAGES_FILE);
  assert.equal(
    stdout,
    `You were compacted and left no note.\n${LESSONS}\n${MENU_LINE}\nLast 50 messages: ${expectedPath}. Read them whole.\n`,
  );
});

test('compact with a note prints the guard, the reading list and the note under the limit, and uses the note once', () => {
  const project = makeProject(LESSONS);
  mkdirSync(path.join(project, '.memory', 'lessons'));
  writeFileSync(path.join(project, '.memory', 'lessons', 'BUILDER.md'), 'B1 Do X, because Y.');
  const note = 'n'.repeat(NOTE_LIMIT);
  const { stdout, status, harness } = runHook({ project, source: 'compact', note: { role: 'builder', note, time: 'now' } });
  assert.equal(status, 0);
  assert.equal(
    stdout,
    [
      'You were compacted. Ground before doing anything; change nothing while grounding.',
      'Read, in order:',
      `1. ${path.join(ROOT, '.claude', 'agents', 'builder.md')}`,
      `2. ${path.join(project, '.memory', 'LESSONS.md')}`,
      `3. ${path.join(project, '.memory', 'lessons', 'BUILDER.md')}`,
      `4. ${path.join(harness, SESSION_ID, LAST_MESSAGES_FILE)} (the last 50 messages)`,
      '## Note from yourself',
      note,
      'Read every file your note lists, in its order.',
      '',
    ].join('\n'),
  );
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
  assert.ok(!existsSync(path.join(harness, SESSION_ID, NOTE_FILE)));
});

test('a startup or resume session start leaves a stored note in place', () => {
  for (const source of ['startup', 'resume']) {
    const project = makeProject(LESSONS);
    const { stdout, status, harness } = runHook({ project, source, note: { role: 'builder', note: 'kept', time: 'now' } });
    assert.equal(status, 0);
    assert.equal(stdout, `${LESSONS}\n${MENU_LINE}\n`);
    assert.ok(existsSync(path.join(harness, SESSION_ID, NOTE_FILE)));
  }
});

test('compact with a corrupt note says it is corrupt and keeps it', () => {
  const project = makeProject(LESSONS);
  const { stdout, status, harness } = runHook({ project, source: 'compact', note: '{"role": "builder", "no' });
  assert.equal(status, 0);
  const noteFile = path.join(harness, SESSION_ID, NOTE_FILE);
  assert.ok(stdout.startsWith(`You were compacted and your note at ${noteFile} is corrupt: read it as it stands.\n${LESSONS}\n${MENU_LINE}\n`));
  assert.ok(existsSync(noteFile));
});

test('missing lessons file prints the not-found line then the menu line', () => {
  const project = makeProject();
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(stdout, `LESSONS.md not found at ${path.join(project, '.memory', 'LESSONS.md')}\n${MENU_LINE}\n`);
});

test('a lessons file over the limit is replaced by a pointer line', () => {
  const project = makeProject('x'.repeat(HOOK_OUTPUT_LIMIT));
  const { stdout, status } = runHook({ project, source: 'compact' });
  assert.equal(status, 0);
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
  assert.ok(stdout.startsWith(`You were compacted and left no note.\nLESSONS.md over the hook limit: read ${path.join(project, '.memory', 'LESSONS.md')}\n${MENU_LINE}\n`));
  assert.ok(stdout.includes('Last 50 messages:'));
});

test('ordinary output stays under the limit', () => {
  const project = makeProject(LESSONS);
  const { stdout } = runHook({ project, source: 'compact' });
  assert.ok(stdout.length > 0);
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
});

test('malformed stdin prints one failure line and exits 0', () => {
  const project = makeProject(LESSONS);
  const { stdout, status } = runHook({ project, stdin: 'not json' });
  assert.equal(status, 0);
  const lines = stdout.trimEnd().split('\n');
  assert.equal(lines.length, 1);
  assert.ok(lines[0].startsWith('session hook failed:'));
});

function git(cwd, ...args) {
  execFileSync('git', ['-c', 'user.name=test', '-c', 'user.email=test@example.com', '-c', 'init.defaultBranch=main', ...args], { cwd, stdio: 'pipe' });
}

test('a worktree project reads the lessons from the main checkout', () => {
  const repo = makeProject(LESSONS);
  git(repo, 'init');
  writeFileSync(path.join(repo, 'README'), 'readme\n');
  git(repo, 'add', 'README');
  git(repo, 'commit', '-m', 'init');
  const worktree = path.join(repo, 'wt');
  git(repo, 'worktree', 'add', worktree, '-b', 'wt');
  const { stdout, status } = runHook({ project: worktree });
  assert.equal(status, 0);
  assert.ok(stdout.startsWith(`${LESSONS}\n`));
});

test('a project that is not a git repository reads its own lessons', () => {
  const project = makeProject(LESSONS);
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${MENU_LINE}\n`);
});

test('compact without a session id keeps the lessons and the menu line', () => {
  const project = makeProject(LESSONS);
  const stdin = JSON.stringify({ cwd: project, hook_event_name: 'SessionStart', source: 'compact' });
  const { stdout, status } = runHook({ project, stdin });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${MENU_LINE}\n`);
  assert.ok(!stdout.includes('messages:'));
  assert.ok(!stdout.includes('session hook failed'));
});

test('a hook run through a symlinked directory still prints the menu line', () => {
  const project = makeProject(LESSONS);
  const linkedHooks = path.join(mkdtempSync(path.join(os.tmpdir(), 'session-hook-link-')), 'hooks');
  symlinkSync(path.dirname(HOOK), linkedHooks);
  const { stdout, status } = runHook({ project, hook: path.join(linkedHooks, 'session.mjs') });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${MENU_LINE}\n`);
});

test('a session started from the app with no agent file is told to read builder.md as a builder', () => {
  const project = makeProject(LESSONS);
  const stdin = JSON.stringify({
    session_id: SESSION_ID,
    transcript_path: path.join(project, 'transcript.jsonl'),
    cwd: project,
    hook_event_name: 'SessionStart',
    source: 'startup',
    model: 'claude-opus-4-5',
  });
  const { stdout, status } = runHook({ project, stdin });
  assert.equal(status, 0);
  assert.ok(stdout.includes('builder: read `.claude/agents/builder.md` whole as your prompt and run its first step;'));
  assert.ok(stdout.includes('head: load the head skill; consultant: load the consultant skill;'));
});
