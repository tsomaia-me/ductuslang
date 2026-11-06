import { closeSync, existsSync, fstatSync, openSync, readSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';

import { isEntryPoint, NOTE_ADVISED_CHARACTERS, parseLine, readStdin, sessionFolder } from './shared.mjs';

export const TAIL_BYTES = 262144;
const NOTE_ROOM_TOKENS = 60000;
const OUTPUT_RESERVE_TOKENS = 20000;
const COMPACT_BUFFER_TOKENS = 13000;
const STATE_FILE = 'context-watch.json';
export const SUBAGENT_WRAP_UP_TOKENS = 600000;
const POST_TOOL_USE = 'PostToolUse';

function readRange(fd, start, end) {
  const buffer = Buffer.alloc(end - start);
  readSync(fd, buffer, 0, buffer.length, start);
  return buffer.toString('utf8');
}

function isMainUsageEntry(entry) {
  return entry?.type === 'assistant' && entry.isSidechain !== true && entry.message?.usage != null;
}

function isAnyUsageEntry(entry) {
  return entry?.type === 'assistant' && entry.message?.usage != null;
}

function lastUsageContext(text, isUsageEntry) {
  const lines = text.split('\n');
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const entry = parseLine(lines[index]);
    if (!isUsageEntry(entry)) continue;
    const usage = entry.message.usage;
    return (
      (usage.input_tokens || 0) +
      (usage.cache_creation_input_tokens || 0) +
      (usage.cache_read_input_tokens || 0) +
      (usage.output_tokens || 0)
    );
  }
  return null;
}

function contextInUse(transcriptPath, isUsageEntry) {
  const fd = openSync(transcriptPath, 'r');
  try {
    const size = fstatSync(fd).size;
    for (let tailBytes = TAIL_BYTES; ; tailBytes *= 2) {
      const start = Math.max(0, size - tailBytes);
      const context = lastUsageContext(readRange(fd, start, size), isUsageEntry);
      if (context != null || start === 0) return context;
    }
  } finally {
    closeSync(fd);
  }
}

function compactionPoint(env) {
  const window = Number(env.CLAUDE_CODE_AUTO_COMPACT_WINDOW);
  if (!Number.isInteger(window) || window <= 0) return null;
  return window - OUTPUT_RESERVE_TOKENS - COMPACT_BUFFER_TOKENS;
}

function thousands(tokens) {
  return `${Math.round(tokens / 1000)}k`;
}

function noticeText(compactAt) {
  return `**Compaction at ${thousands(compactAt)}; you are at ${thousands(compactAt - NOTE_ROOM_TOKENS)}.** Write your note now, before any other tool call: \`pnpm grounding --role <ROLE>\`, with the note on stdin, under ${NOTE_ADVISED_CHARACTERS} characters. Say what you are doing, the decisions in force, the next step, and at the end the files to re-read in order. Then carry on with your work; nothing else changes, and ${thousands(NOTE_ROOM_TOKENS)} is left after the note.`;
}

function formatOutput(eventName, notice) {
  if (eventName === POST_TOOL_USE) {
    return JSON.stringify({ hookSpecificOutput: { hookEventName: POST_TOOL_USE, additionalContext: notice } });
  }
  return notice;
}

function subagentNoticeText(context, compactAt, threshold) {
  const compaction = compactAt == null ? '' : `it compacts near ${thousands(compactAt)}, and `;
  return `**Your context holds ${thousands(context)} tokens, past the ${thousands(threshold)} at which you wrap up; ${compaction}you must not reach compaction.** Wrap up now: finish or set aside the piece in hand, commit what passes if you commit, write where you stopped and what is next where your brief keeps your progress, run your hand-back checks, and hand back. Start nothing new.`;
}

function subagentTranscriptPath(input) {
  if (input.agent_transcript_path) return input.agent_transcript_path;
  const agentId = String(input.agent_id);
  const fileName = `${agentId.startsWith('agent-') ? agentId : `agent-${agentId}`}.jsonl`;
  if (path.basename(input.transcript_path) === fileName) return input.transcript_path;
  return path.join(path.dirname(input.transcript_path), input.session_id, 'subagents', fileName);
}

async function watchSubagent(input) {
  const transcriptPath = subagentTranscriptPath(input);
  if (!existsSync(transcriptPath)) return;
  const compactAt = compactionPoint(process.env);
  const threshold = compactAt == null ? SUBAGENT_WRAP_UP_TOKENS : Math.min(SUBAGENT_WRAP_UP_TOKENS, compactAt - NOTE_ROOM_TOKENS);
  const context = contextInUse(transcriptPath, isAnyUsageEntry);
  if (context == null) return;
  const statePath = path.join(sessionFolder(input.session_id), `context-watch-${input.agent_id}.json`);
  if (context < threshold) {
    rmSync(statePath, { force: true });
    return;
  }
  try {
    writeFileSync(statePath, JSON.stringify({ threshold }), { flag: 'wx' });
  } catch (error) {
    if (error.code === 'EEXIST') return;
    throw error;
  }
  process.stdout.write(formatOutput(input.hook_event_name, subagentNoticeText(context, compactAt, threshold)) + '\n');
}

async function main() {
  const input = JSON.parse(await readStdin());
  if (input.agent_id) return watchSubagent(input);
  const compactAt = compactionPoint(process.env);
  if (compactAt == null) return;
  const context = contextInUse(input.transcript_path, isMainUsageEntry);
  if (context == null) return;
  const statePath = path.join(sessionFolder(input.session_id), STATE_FILE);
  if (context < compactAt - NOTE_ROOM_TOKENS) {
    rmSync(statePath, { force: true });
    return;
  }
  try {
    writeFileSync(statePath, JSON.stringify({ compactAt }), { flag: 'wx' });
  } catch (error) {
    if (error.code === 'EEXIST') return;
    throw error;
  }
  process.stdout.write(formatOutput(input.hook_event_name, noticeText(compactAt)) + '\n');
}

if (isEntryPoint(import.meta.url)) {
  try {
    await main();
  } catch {
    process.exitCode = 0;
  }
}
