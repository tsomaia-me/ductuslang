import { execFileSync } from 'node:child_process';
import { mkdirSync, realpathSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const LAST_MESSAGES_FILE = 'last-messages.md';
const NOTES_RELATIVE_PATH = path.join('.claude', 'notes', '<ROLE>.md');

export function mainCheckout(project) {
  try {
    const commonDir = execFileSync('git', ['-C', project, 'rev-parse', '--path-format=absolute', '--git-common-dir'], {
      encoding: 'utf8',
      stdio: 'pipe',
    }).trim();
    return path.dirname(commonDir);
  } catch {
    return project;
  }
}

export function notesPath(project) {
  return path.join(mainCheckout(project), NOTES_RELATIVE_PATH);
}

export function sessionFolder(sessionId) {
  const folder = path.join(process.env.CLAUDE_HARNESS_DIR || path.join(os.tmpdir(), 'claude-harness'), sessionId);
  mkdirSync(folder, { recursive: true });
  return folder;
}

export function lastMessagesPath(sessionId) {
  return path.join(sessionFolder(sessionId), LAST_MESSAGES_FILE);
}

export async function readStdin() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString('utf8');
}

export function parseLine(line) {
  try {
    return JSON.parse(line);
  } catch {
    return null;
  }
}

export function isEntryPoint(moduleUrl) {
  if (!process.argv[1]) return false;
  try {
    return moduleUrl === pathToFileURL(realpathSync(process.argv[1])).href;
  } catch {
    return false;
  }
}
