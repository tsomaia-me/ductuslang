import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

import { MESSAGE_COUNT } from './last-messages.mjs';
import { isEntryPoint, lastMessagesPath, mainCheckout, notesPath, readStdin } from './shared.mjs';

export const HOOK_OUTPUT_LIMIT = 10000;
const LESSONS_RELATIVE_PATH = path.join('.claude', 'lessons', 'GENERAL.md');
const PREFERENCES_RELATIVE_PATH = path.join('.claude', 'OWNER_PREFERENCES.md');
const COMPACT_SOURCE = 'compact';
const LINE_BREAK = '\n';
export const GUARD_LINE = 'You were compacted. Ground before doing anything; change nothing while grounding.';
export const MENU_LINE = 'head: load the head skill; peer: load the peer skill; builder: read `.claude/agents/builder.md` whole as your prompt and run its first step; any other role: run your file\'s first step again.';

function lastMessagesLine(sessionId) {
  return `Last ${MESSAGE_COUNT} messages: ${lastMessagesPath(sessionId)}. Read them whole.`;
}

function notesLine(project) {
  return `Your notes: ${notesPath(project)}. Read them whole, then every file they list.`;
}

function filePart(relativePath, root, roomLeft) {
  const name = path.basename(relativePath);
  const file = path.join(root, relativePath);
  if (!existsSync(file)) {
    return `${name} not found at ${file}`;
  }
  const content = readFileSync(file, 'utf8').replace(/\n+$/, '');
  return content.length < roomLeft ? content : `${name} over the hook limit: read ${file}`;
}

function hasSessionId(input) {
  return typeof input.session_id === 'string' && input.session_id.length > 0;
}

function render(parts) {
  return parts.join(LINE_BREAK) + LINE_BREAK;
}

function buildOutput(input) {
  const project = process.env.CLAUDE_PROJECT_DIR || input.cwd;
  const root = mainCheckout(project);
  const compacted = input.source === COMPACT_SOURCE && hasSessionId(input) && !input.agent_id;
  const head = compacted ? [GUARD_LINE] : [];
  const tail = compacted ? [MENU_LINE, lastMessagesLine(input.session_id), notesLine(project)] : [MENU_LINE];
  const lessons = filePart(LESSONS_RELATIVE_PATH, root, HOOK_OUTPUT_LIMIT - render([...head, '', '', ...tail]).length);
  const preferences = filePart(PREFERENCES_RELATIVE_PATH, root, HOOK_OUTPUT_LIMIT - render([...head, lessons, '', ...tail]).length);
  return render([...head, lessons, preferences, ...tail]);
}

async function main() {
  try {
    process.stdout.write(buildOutput(JSON.parse(await readStdin())));
  } catch (error) {
    process.stdout.write(`session hook failed: ${String(error.message).replace(/\s+/g, ' ')}${LINE_BREAK}`);
  }
  process.exitCode = 0;
}

if (isEntryPoint(import.meta.url)) {
  await main();
}
