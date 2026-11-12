import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, symlinkSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { GUARD_LINE, HOOK_OUTPUT_LIMIT, MENU_LINE } from './session.mjs';
import { LAST_MESSAGES_FILE } from './shared.mjs';

const HOOK = path.join(path.dirname(fileURLToPath(import.meta.url)), 'session.mjs');
const SESSION_ID = 'session-abc';
const LESSONS = 'G-1: Do X, because Y.\nG-2: Do Z, because W.';
const PREFERENCES = '- Prefer X.';

function lessonsFile(project) {
  return path.join(project, '.claude', 'lessons', 'GENERAL.md');
}

function preferencesFile(project) {
  return path.join(project, '.claude', 'OWNER_PREFERENCES.md');
}

function notesLine(project) {
  return `Your notes: ${path.join(project, '.claude', 'notes', '<ROLE>.md')}. Read them whole, then every file they list.`;
}

function makeProject(lessons, preferences = PREFERENCES) {
  const project = mkdtempSync(path.join(os.tmpdir(), 'session-hook-project-'));
  if (lessons !== undefined) {
    mkdirSync(path.join(project, '.claude', 'lessons'), { recursive: true });
    writeFileSync(lessonsFile(project), lessons);
    if (preferences !== null) writeFileSync(preferencesFile(project), preferences);
  }
  return project;
}

function runHook({ project, source = 'startup', stdin, hook = HOOK, harness = mkdtempSync(path.join(os.tmpdir(), 'session-hook-harness-')) }) {
  const input = stdin ?? JSON.stringify({
    session_id: SESSION_ID,
    transcript_path: path.join(harness, 'transcript.jsonl'),
    cwd: project,
    hook_event_name: 'SessionStart',
    source,
  });
  const env = { ...process.env, CLAUDE_PROJECT_DIR: project, CLAUDE_HARNESS_DIR: harness };
  const result = spawnSync(process.execPath, [hook], { input, encoding: 'utf8', env });
  return { ...result, harness };
}

function git(cwd, ...args) {
  execFileSync('git', ['-c', 'user.name=test', '-c', 'user.email=test@example.com', '-c', 'init.defaultBranch=main', ...args], { cwd, stdio: 'pipe' });
}

test('startup prints the lessons and the owner preferences verbatim then the menu line and no last-messages line', () => {
  const project = makeProject(`${LESSONS}\n`, `${PREFERENCES}\n`);
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
  assert.ok(!stdout.includes('messages:'));
  assert.ok(!stdout.includes('Your notes:'));
});

test('resume prints what startup prints', () => {
  const project = makeProject(LESSONS);
  const { stdout, status } = runHook({ project, source: 'resume' });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
});

test('compact prints the guard, the lessons, the preferences, the menu line, the last-messages line and the notes line', () => {
  const project = makeProject(LESSONS);
  const { stdout, status, harness } = runHook({ project, source: 'compact' });
  assert.equal(status, 0);
  const lastMessages = path.join(harness, SESSION_ID, LAST_MESSAGES_FILE);
  assert.equal(
    stdout,
    `${GUARD_LINE}\n${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\nLast 50 messages: ${lastMessages}. Read them whole.\n${notesLine(project)}\n`,
  );
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
});

test('nothing under .memory is read', () => {
  const project = makeProject(LESSONS);
  mkdirSync(path.join(project, '.memory', 'lessons'), { recursive: true });
  writeFileSync(path.join(project, '.memory', 'LESSONS.md'), 'OLD LESSON');
  writeFileSync(path.join(project, '.memory', 'OWNER_PREFERENCES.md'), 'OLD PREFERENCE');
  writeFileSync(path.join(project, '.memory', 'READING.md'), 'OLD READING');
  for (const source of ['startup', 'compact']) {
    const { stdout, status } = runHook({ project, source });
    assert.equal(status, 0);
    assert.ok(!stdout.includes('OLD '));
    assert.ok(!stdout.includes('.memory'));
  }
});

test('the project falls back to the input cwd when CLAUDE_PROJECT_DIR is unset', () => {
  const project = makeProject(LESSONS);
  const harness = mkdtempSync(path.join(os.tmpdir(), 'session-hook-harness-'));
  const input = JSON.stringify({ session_id: SESSION_ID, cwd: project, hook_event_name: 'SessionStart', source: 'compact' });
  const env = { ...process.env, CLAUDE_HARNESS_DIR: harness };
  delete env.CLAUDE_PROJECT_DIR;
  const { stdout, status } = spawnSync(process.execPath, [HOOK], { input, encoding: 'utf8', env });
  assert.equal(status, 0);
  assert.ok(stdout.startsWith(`${GUARD_LINE}\n${LESSONS}\n`));
  assert.ok(stdout.endsWith(`${notesLine(project)}\n`));
});

test('on a compaction the lessons give way once the guard, the menu, the last-messages and the notes lines leave no room', () => {
  const sized = (length) => {
    const project = makeProject('l'.repeat(length), '');
    return runHook({ project, source: 'compact' }).stdout;
  };
  const fixed = sized(0).length;
  const fits = sized(HOOK_OUTPUT_LIMIT - fixed - 60);
  assert.ok(fits.includes('l'.repeat(100)));
  assert.ok(fits.length < HOOK_OUTPUT_LIMIT);
  const over = sized(HOOK_OUTPUT_LIMIT - fixed + 60);
  assert.ok(over.includes('GENERAL.md over the hook limit'));
  assert.ok(over.length < HOOK_OUTPUT_LIMIT);
});

test('missing lessons and preferences files print their not-found lines then the menu line', () => {
  const project = makeProject();
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(
    stdout,
    `GENERAL.md not found at ${lessonsFile(project)}\nOWNER_PREFERENCES.md not found at ${preferencesFile(project)}\n${MENU_LINE}\n`,
  );
});

test('a missing owner preferences file prints the not-found line after the lessons', () => {
  const project = makeProject(LESSONS, null);
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\nOWNER_PREFERENCES.md not found at ${preferencesFile(project)}\n${MENU_LINE}\n`);
});

test('a lessons file over the limit is replaced by a pointer line', () => {
  const project = makeProject('x'.repeat(HOOK_OUTPUT_LIMIT));
  const { stdout, status } = runHook({ project, source: 'compact' });
  assert.equal(status, 0);
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
  assert.ok(stdout.startsWith(`${GUARD_LINE}\nGENERAL.md over the hook limit: read ${lessonsFile(project)}\n${PREFERENCES}\n${MENU_LINE}\n`));
  assert.ok(stdout.includes('Last 50 messages:'));
  assert.ok(stdout.endsWith(`${notesLine(project)}\n`));
});

test('an owner preferences file over the limit is replaced by a pointer line', () => {
  const project = makeProject(LESSONS, 'x'.repeat(HOOK_OUTPUT_LIMIT));
  const { stdout, status } = runHook({ project, source: 'compact' });
  assert.equal(status, 0);
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
  assert.ok(stdout.startsWith(`${GUARD_LINE}\n${LESSONS}\nOWNER_PREFERENCES.md over the hook limit: read ${preferencesFile(project)}\n${MENU_LINE}\n`));
});

test('the lessons keep priority over the owner preferences for the room left', () => {
  const lessons = 'l'.repeat(HOOK_OUTPUT_LIMIT - 500);
  const project = makeProject(lessons, 'p'.repeat(400));
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.ok(stdout.length < HOOK_OUTPUT_LIMIT);
  assert.equal(stdout, `${lessons}\nOWNER_PREFERENCES.md over the hook limit: read ${preferencesFile(project)}\n${MENU_LINE}\n`);
});

test('malformed stdin prints one failure line and exits 0', () => {
  const project = makeProject(LESSONS);
  const { stdout, status } = runHook({ project, stdin: 'not json' });
  assert.equal(status, 0);
  const lines = stdout.trimEnd().split('\n');
  assert.equal(lines.length, 1);
  assert.ok(lines[0].startsWith('session hook failed:'));
});

test('a worktree project reads the lessons from the main checkout and names its notes there', () => {
  const repo = makeProject(LESSONS);
  git(repo, 'init');
  writeFileSync(path.join(repo, 'README'), 'readme\n');
  git(repo, 'add', 'README');
  git(repo, 'commit', '-m', 'init');
  const worktree = path.join(repo, 'wt');
  git(repo, 'worktree', 'add', worktree, '-b', 'wt');
  const started = runHook({ project: worktree });
  assert.equal(started.status, 0);
  assert.equal(started.stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
  const compacted = runHook({ project: worktree, source: 'compact' });
  assert.equal(compacted.status, 0);
  assert.ok(compacted.stdout.startsWith(`${GUARD_LINE}\n${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\nLast 50 messages: `));
  assert.ok(compacted.stdout.endsWith(`${notesLine(repo)}\n`));
});

test('a project that is not a git repository reads its own lessons', () => {
  const project = makeProject(LESSONS);
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
});

test('compact without a session id keeps the lessons and the menu line', () => {
  const project = makeProject(LESSONS);
  const stdin = JSON.stringify({ cwd: project, hook_event_name: 'SessionStart', source: 'compact' });
  const { stdout, status } = runHook({ project, stdin });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
  assert.ok(!stdout.includes('session hook failed'));
});

test('a subagent compacting in the session gets the lessons and the menu line and no guard', () => {
  const project = makeProject(LESSONS);
  const harness = mkdtempSync(path.join(os.tmpdir(), 'session-hook-harness-'));
  const stdin = JSON.stringify({
    session_id: SESSION_ID,
    agent_id: 'agent-123',
    transcript_path: path.join(harness, 'transcript.jsonl'),
    cwd: project,
    hook_event_name: 'SessionStart',
    source: 'compact',
  });
  const { stdout, status } = runHook({ project, stdin, harness });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
});

test('a hook run through a symlinked directory still prints the menu line', () => {
  const project = makeProject(LESSONS);
  const linkedHooks = path.join(mkdtempSync(path.join(os.tmpdir(), 'session-hook-link-')), 'hooks');
  symlinkSync(path.dirname(HOOK), linkedHooks);
  const { stdout, status } = runHook({ project, hook: path.join(linkedHooks, 'session.mjs') });
  assert.equal(status, 0);
  assert.equal(stdout, `${LESSONS}\n${PREFERENCES}\n${MENU_LINE}\n`);
});

test('the menu line tells a session with no agent file to read builder.md as a builder', () => {
  const project = makeProject(LESSONS);
  const { stdout, status } = runHook({ project });
  assert.equal(status, 0);
  assert.ok(stdout.includes('builder: read `.claude/agents/builder.md` whole as your prompt and run its first step;'));
  assert.ok(stdout.includes('head: load the head skill; peer: load the peer skill;'));
});
