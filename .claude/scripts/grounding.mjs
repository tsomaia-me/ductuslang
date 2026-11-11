import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { isEntryPoint, isPlainId, memoryCheckout, NOTE_ADVISED_CHARACTERS, noteRelativePath, readStdin, roleFile } from '../hooks/shared.mjs';

export const NOTE_LIMIT = 5000;
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const PUSH_SCRIPT = path.join(ROOT, '.claude', 'scripts', 'memory-push.sh');
const USAGE = 'wrong arguments: run pnpm grounding --role <ROLE> with the note on stdin';

function say(tag, text) {
  process.stdout.write(`${tag.padEnd(11)}  ${text}\n`);
}

function readRole(argv) {
  try {
    const { values, positionals } = parseArgs({ args: argv, options: { role: { type: 'string' } } });
    return values.role && positionals.length === 0 ? values.role : null;
  } catch {
    return null;
  }
}

async function store(argv) {
  const role = readRole(argv);
  if (!role) return { code: 2, line: USAGE };
  if (!roleFile(ROOT, role)) {
    return { code: 1, line: `unknown role ${role}: name your role, a lessons file such as BUILDER or a file under .claude/agents/ or .claude/skills/` };
  }
  const sessionId = process.env.CLAUDE_CODE_SESSION_ID;
  if (!sessionId) return { code: 1, line: "no session id: run it from your session's Bash tool" };
  if (!isPlainId(sessionId)) return { code: 1, line: `session id ${sessionId} is not a plain id: letters, digits, - and _ only` };
  const note = (await readStdin()).trim();
  if (note === '') return { code: 1, line: 'no note on stdin: pipe the note in' };
  if (note.length > NOTE_LIMIT) {
    return { code: 1, line: `${note.length} characters, over ${NOTE_LIMIT}: cut the note under ${NOTE_ADVISED_CHARACTERS} and run again` };
  }
  const checkout = memoryCheckout(process.cwd());
  if (!checkout || !existsSync(checkout)) return { code: 1, line: `no memory checkout ${checkout ?? process.cwd()}: run it from the repository or set MEMORY_DIR` };
  const relative = noteRelativePath(sessionId);
  const target = path.join(checkout, relative);
  mkdirSync(path.dirname(target), { recursive: true });
  writeFileSync(target, JSON.stringify({ role, note, time: new Date().toISOString() }));
  return { code: 0, stored: target, line: push(relative) };
}

function push(relative) {
  const result = spawnSync(PUSH_SCRIPT, [relative], { encoding: 'utf8' });
  if (result.status === 0) return null;
  const said = `${result.stdout ?? ''}\n${result.stderr ?? ''}`.split('\n').find((line) => line.trim() !== '');
  return said ?? `push blocked: memory-push.sh ${relative} failed`;
}

if (isEntryPoint(import.meta.url)) {
  const result = await store(process.argv.slice(2));
  if (result.stored) say('STORED', result.stored);
  if (result.line) say('NEXT', result.line);
  process.exitCode = result.code;
}
