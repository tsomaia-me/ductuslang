import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, realpathSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const LAST_MESSAGES_FILE = 'last-messages.md';
export const NOTES_FOLDER = 'grounding';
export const USED_NOTE_FILE = 'grounding-used.json';
export const NOTE_ADVISED_CHARACTERS = (4000).toLocaleString('en-US');
const ROLE_NAME = /^[a-z][a-z-]*$/;
const PLAIN_ID = /^[A-Za-z0-9_-]+$/;
const MEMORY_DIR_SCRIPT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'scripts', 'memory-dir.sh');

export function sessionFolder(sessionId) {
  const folder = path.join(process.env.CLAUDE_HARNESS_DIR || path.join(os.tmpdir(), 'claude-harness'), sessionId);
  mkdirSync(folder, { recursive: true });
  return folder;
}

export function lastMessagesPath(sessionId) {
  return path.join(sessionFolder(sessionId), LAST_MESSAGES_FILE);
}

export function usedNotePath(sessionId) {
  return path.join(sessionFolder(sessionId), USED_NOTE_FILE);
}

export function isPlainId(sessionId) {
  return typeof sessionId === 'string' && PLAIN_ID.test(sessionId);
}

export function memoryCheckout(cwd) {
  try {
    const found = execFileSync('bash', ['-c', 'source "$1" && memory_checkout', 'memory-dir', MEMORY_DIR_SCRIPT], { cwd, encoding: 'utf8', stdio: 'pipe' }).trim();
    return found === '' ? null : path.resolve(cwd, found);
  } catch {
    return null;
  }
}

export function noteRelativePath(sessionId) {
  return path.join(NOTES_FOLDER, `${sessionId}.json`);
}

export function notePath(sessionId, cwd) {
  if (!isPlainId(sessionId)) return null;
  const checkout = memoryCheckout(cwd);
  return checkout ? path.join(checkout, noteRelativePath(sessionId)) : null;
}

export function roleFile(root, role) {
  const name = role.toLowerCase();
  if (!ROLE_NAME.test(name)) return null;
  const candidates = [path.join(root, '.claude', 'agents', `${name}.md`), path.join(root, '.claude', 'skills', name, 'SKILL.md')];
  return candidates.find((candidate) => existsSync(candidate)) ?? null;
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
