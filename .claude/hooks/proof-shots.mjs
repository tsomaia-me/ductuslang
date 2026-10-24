import { isEntryPoint, readStdin } from './shared.mjs';

export const REASON =
  'Never open a proof screenshot: the pictures are the Judge’s, your walk is your check and the summary’s text is your smoke (builder.md, the walk). Read .proof/summary.txt instead.';

export const isProofShot = (filePath) => typeof filePath === 'string' && /\.png$/i.test(filePath) && filePath.replaceAll('\\', '/').includes('/.proof/');

export function decide(input) {
  if (input.tool_name !== 'Read' || !isProofShot(input.tool_input?.file_path)) return null;
  return { hookSpecificOutput: { hookEventName: 'PreToolUse', permissionDecision: 'deny', permissionDecisionReason: REASON } };
}

if (isEntryPoint(import.meta.url)) {
  try {
    const decision = decide(JSON.parse(await readStdin()));
    if (decision) process.stdout.write(`${JSON.stringify(decision)}\n`);
  } catch {
    process.exitCode = 0;
  }
}
