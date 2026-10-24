import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import { REASON } from './proof-shots.mjs';

const HOOK_PATH = path.join(path.dirname(fileURLToPath(import.meta.url)), 'proof-shots.mjs');

function runHook(input) {
  return spawnSync(process.execPath, [HOOK_PATH], { input: typeof input === 'string' ? input : JSON.stringify(input), encoding: 'utf8' });
}

const read = (filePath) => ({ hook_event_name: 'PreToolUse', tool_name: 'Read', tool_input: { file_path: filePath } });

test('a Read of a screenshot under a worktree’s .proof folder is refused, and the refusal names the rule', () => {
  assert.match(REASON, /Never open a proof screenshot/);
  assert.match(REASON, /builder\.md, the walk/);
  for (const filePath of ['/home/cdagent/wt/.proof/shots/admin-spend-390-dark-default.png', '/home/cdagent/wt/.proof/base/app-home-1440-light-error.PNG']) {
    const result = runHook(read(filePath));
    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(result.stdout), {
      hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'deny', permissionDecisionReason: REASON },
    });
  }
});

test('the summary, a screenshot outside a .proof folder and every other tool pass untouched', () => {
  for (const input of [
    read('/home/cdagent/wt/.proof/summary.txt'),
    read('/home/cdagent/scratch/proof-page/shots/admin-spend-390-dark-default.png'),
    { hook_event_name: 'PreToolUse', tool_name: 'Bash', tool_input: { command: 'ls /home/cdagent/wt/.proof/shots/a.png' } },
    'not json',
  ]) {
    const result = runHook(input);
    assert.equal(result.status, 0);
    assert.equal(result.stdout, '');
  }
});
