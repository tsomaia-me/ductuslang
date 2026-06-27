// ============================================================
// SUBAGENTIC WORKFLOW — REUSABLE TEMPLATE
// ============================================================
//
// A 6-phase orchestrator script for heavy, multi-site documentation
// or code edits that exceed a single context window.
//
// Phases:
//   1. Extract         — parallel explorers produce site list from disk
//   2. Edit            — pipeline (per-file sequential, cross-file parallel)
//   3. Batch gates     — 5 parallel mechanical gates
//   4. Adversarial     — high-effort cold-read by an independent skeptic
//   5. E2E review      — capture unified diff for the orchestrator
//   6. Surface         — return structured summary
//
// HOW TO USE:
//   1. Copy this file.
//   2. Fill in the four CUSTOMIZE blocks (META, HARD_RULES, LEARNINGS,
//      PROJECT_CONTEXT) and the six SCOPE constants.
//   3. Adjust the gate prompts under Phase 3 to match your project's
//      invariants. Each gate is independent; keep or drop as needed.
//   4. Run via Workflow({scriptPath: '<path>'}); read the e2e diff
//      personally before declaring done.
//
// PROTOCOL FILES referenced by this template (must exist at the same
// directory level OR you must rewrite the paths in protocolHeader):
//   - WORKFLOW_PROTOCOL.md
//   - SUBAGENT_PROTOCOL_EXPLORER.md
//   - SUBAGENT_PROTOCOL_EDITOR.md
//   - SUBAGENT_PROTOCOL_GATE.md
//   - SUBAGENT_PROTOCOL_ADVERSARIAL.md
// ============================================================

// ─────────────────────────── CUSTOMIZE: META ───────────────────────────
export const meta = {
  name: '<batch-name>',                         // CHANGE
  description: '<one-line batch description>',  // CHANGE
  phases: [
    { title: 'Extract',     detail: 'Pull site list with verbatim quotes from disk' },
    { title: 'Edit',        detail: 'Per-site editor + spot-verify pipeline' },
    { title: 'Batch gates', detail: 'Parallel mechanical gates over post-edit state' },
    { title: 'Adversarial', detail: 'High-effort cold-read of staged diff against plan' },
    { title: 'E2E review',  detail: 'Capture unified diff for orchestrator personal review' },
    { title: 'Surface',     detail: 'Return structured summary' },
  ],
}

// ─────────────────────── CUSTOMIZE: PROTOCOL CONTENT ───────────────────
// HARD_RULES: absolute rules every agent must respect. Project-specific.
const HARD_RULES = `
== HARD RULES (absolute, no exceptions) ==
- Never use AskUserQuestion. Pick a sensible default + state assumption in one line, OR halt and surface up.
- Approval is literal: only "approve"/"approved" counts. Nothing else.
- <ADD project edit-protocol rule, e.g. "Edit DECISION_LOG.md first, then conforming SPEC.md section.">
- Verbatim quote handling: build old_string from a FRESH READ of the target file at edit time. NEVER from cached memory, the plan's snippets, or another agent's output.
- Discovered contradictions, ambiguities, or incoherence: STOP and surface up via halt. Do NOT silently resolve.
- Branch is <BRANCH-NAME> only. Never push elsewhere.
- No PR creation unless explicitly asked.
- Use the right tool: Edit for in-place changes; Read for content; Grep for content search; Glob for paths. Don't reach for Bash unless necessary.
- Don't narrate. Return structured data per schema; the fields ARE the deliverable.
`

// LEARNINGS: project working rules (e.g., the verbatim contents of LEARNINGS.md).
const LEARNINGS = `
== WORKING RULES (project LEARNINGS) ==
<PASTE THE FULL VERBATIM CONTENTS OF LEARNINGS.md HERE>
`

// PROJECT_CONTEXT: batch-specific scope, files in play, decision IDs,
// invariants, floor table (next-free IDs), etc.
const PROJECT_CONTEXT = `
== PROJECT CONTEXT — <BATCH NAME> ==

Project: <project>
Branch: <branch-name>
Plan: <absolute path>
Master change-list: <absolute path>
Per-batch verification report: <absolute path>

Files in scope:
- <absolute path 1>
- <absolute path 2>

Floor table (next-free integers, verified):
- <§NNN: NNN-XXX>

Batch scope (<N> verified sites):
<describe each item A, B, C, ... with concrete decision IDs and intent>

Decisions that govern this batch:
- <D-id or F-id>: <one-line summary>

Invariants to preserve post-batch:
- <invariant 1>
- <invariant 2>
`

// HALT_PROTOCOL: universal, do not edit unless your project diverges.
const HALT_PROTOCOL = `
== HALT PROTOCOL ==
Every structured output MUST include:
- protocol_acknowledged: boolean (true if you read your per-kind protocol file)
- halt: boolean (true if you cannot complete within protocol)
- halt_reason: string (short, specific, when halt=true)

When to halt:
- A hard rule or LEARNINGS rule would be violated by proceeding.
- A discovered contradiction, ambiguity, or incoherence.
- A required precondition is absent (file doesn't exist, section is missing, etc.).
- The agent's verbatim old_string build doesn't match disk content.
- The task requires deciding something the user hasn't decided.

How to halt:
Return structured output with halt=true and halt_reason set. Do NOT attempt to fix. Do NOT continue with a workaround. The orchestrator surfaces halts to the user.

Forbidden patterns:
- "I encountered ambiguity, so I picked the most likely interpretation and continued." → Should have halted.
- "I noticed a contradiction with another rule, but I proceeded with the locked plan." → Should have halted.
- "I couldn't find the verbatim quote, so I used a close paraphrase." → Should have halted.
- "I'll mark this task complete and flag the issue in chat." → Should have halted.

Halting is NEVER a failure. Continuing past ambiguity IS a failure.
`

// PROTOCOL_DIR: where per-kind protocol files live (no trailing slash).
const PROTOCOL_DIR = '<absolute path to repo root containing SUBAGENT_PROTOCOL_*.md files>'

const protocolHeader = (kind) => `${HARD_RULES}\n${LEARNINGS}\n${PROJECT_CONTEXT}\n${HALT_PROTOCOL}\n
== PER-KIND PROTOCOL ==
Your role-specific protocol is at ${PROTOCOL_DIR}/SUBAGENT_PROTOCOL_${kind}.md.
Read it now, before starting your task. It defines your role's specific gates, schemas, and forbidden patterns.

`

// ─────────────────────────── CUSTOMIZE: SCOPES ─────────────────────────
// One SCOPE constant per file you extract sites from. Free-form prose:
// describe what the explorer should pull from THIS file for THIS batch.
const FILE_A_PATH = '<absolute path>'   // CHANGE
const FILE_B_PATH = '<absolute path>'   // CHANGE
const FILE_C_PATH = '<absolute path>'   // CHANGE optional, remove block if not used

const FILE_A_SCOPE = `<what to extract from file A>`
const FILE_B_SCOPE = `<what to extract from file B>`
const FILE_C_SCOPE = `<what to extract from file C>`

// Filtration: list any string fragments inside an extract entry's `intent`
// that should be treated as a no-op site (skip the editor; gate-count covers
// it). Common: rule-count headers, "verify only", "no edit needed".
const NOOP_INTENT_FRAGMENTS = [
  'rule count', 'rule-count', 'no edit', 'no-op', 'verify only', 'leave alone',
]

// Editor-side benign-halt fragments. When an editor returns halt=true with a
// reason that matches one of these, treat it as "site already in intended
// state, no edit needed" and continue, not "real halt".
const NOOP_HALT_FRAGMENTS = [
  'no-op', 'noop', 'no literal', 'no change', 'no edit', 'already clean',
  'may not need', 'does not contain', 'not found in the file',
]

// ============================================================
// SCHEMAS (universal — do not customize unless your project demands it)
// ============================================================

const SITE_LIST_SCHEMA = {
  type: 'object',
  required: ['protocol_acknowledged', 'halt', 'entries'],
  properties: {
    protocol_acknowledged: { type: 'boolean' },
    halt: { type: 'boolean' },
    halt_reason: { type: ['string', 'null'] },
    entries: {
      type: 'array',
      items: {
        type: 'object',
        required: ['site_id', 'file', 'locator', 'verbatim_old', 'intent', 'decision_id', 'batch_role'],
        properties: {
          site_id: { type: 'string' },
          file: { type: 'string' },
          locator: { type: 'string' },
          verbatim_old: { type: 'string' },
          intent: { type: 'string' },
          decision_id: { type: 'string' },
          batch_role: { type: 'string' },
          neighbors_before: { type: ['string', 'null'] },
          neighbors_after: { type: ['string', 'null'] },
        },
      },
    },
    files_read: { type: 'array', items: { type: 'string' } },
    notes_for_orchestrator: { type: ['string', 'null'] },
  },
}

const EDIT_SCHEMA = {
  type: 'object',
  required: ['protocol_acknowledged', 'halt', 'site_id', 'gates_passed'],
  properties: {
    protocol_acknowledged: { type: 'boolean' },
    halt: { type: 'boolean' },
    halt_reason: { type: ['string', 'null'] },
    site_id: { type: 'string' },
    file: { type: ['string', 'null'] },
    locator: { type: ['string', 'null'] },
    old_string: { type: ['string', 'null'] },
    new_string: { type: ['string', 'null'] },
    diff_summary: { type: ['string', 'null'] },
    gates_passed: { type: 'array', items: { type: 'string' } },
  },
}

const VERIFY_SCHEMA = {
  type: 'object',
  required: ['protocol_acknowledged', 'halt', 'site_id', 'verified'],
  properties: {
    protocol_acknowledged: { type: 'boolean' },
    halt: { type: 'boolean' },
    halt_reason: { type: ['string', 'null'] },
    site_id: { type: 'string' },
    verified: { type: 'boolean' },
    reason: { type: ['string', 'null'] },
  },
}

const GATE_SCHEMA = {
  type: 'object',
  required: ['protocol_acknowledged', 'halt', 'gate_kind', 'pass', 'findings'],
  properties: {
    protocol_acknowledged: { type: 'boolean' },
    halt: { type: 'boolean' },
    halt_reason: { type: ['string', 'null'] },
    gate_kind: { type: 'string' },
    scope: { type: ['string', 'null'] },
    pass: { type: ['boolean', 'null'] },
    findings: { type: 'array' },
    files_searched: { type: 'array', items: { type: 'string' } },
    search_patterns: { type: 'array', items: { type: 'string' } },
  },
}

const ADVERSARIAL_SCHEMA = {
  type: 'object',
  required: ['protocol_acknowledged', 'halt', 'verdict', 'findings', 'summary_for_user'],
  properties: {
    protocol_acknowledged: { type: 'boolean' },
    halt: { type: 'boolean' },
    halt_reason: { type: ['string', 'null'] },
    verdict: { type: ['string', 'null'] },        // GREEN | YELLOW | RED
    verdict_one_line: { type: ['string', 'null'] },
    findings: { type: 'array' },
    missed_sites: { type: 'array' },
    composition_issues: { type: 'array' },
    summary_for_user: { type: ['string', 'null'] },
  },
}

// ============================================================
// PHASE 1 — EXTRACT (parallel explorers per file)
// ============================================================

phase('Extract')
log('Phase 1: Extracting site list — parallel explorers per file')

const extractBrief = (fileLabel, filePath, scope) => protocolHeader('EXPLORER') + `
== TASK ==
You are extracting the site list for batch ${meta.name}, file ${fileLabel}.

File in scope: ${filePath}

Your job: enumerate every concrete site in this file that this batch must edit. For each site, fresh-read the target lines from disk and build verbatim_old verbatim from the bytes you read. Do NOT use plan snippets as verbatim_old — the plan can drift; disk is truth.

Batch scope for ${fileLabel}:
${scope}

For each site, return:
- site_id: stable string like "${meta.name}-${fileLabel.toLowerCase()}-<seq>"
- file: absolute path
- locator: line number or section header
- verbatim_old: exact bytes a future Edit tool call would replace (include 1-3 lines of context to make old_string unique in the file)
- intent: semantic description of what should change
- decision_id: which decision from the plan governs this edit
- batch_role: "${meta.name}"
- neighbors_before / neighbors_after: 1-2 lines of context above/below

CRITICAL: every verbatim_old field must come from a fresh disk read. If a promised site is not found, halt with halt_reason: "site X promised by plan but not found in ${fileLabel}".

If you find sites the plan didn't list but the batch's scope implies, include them with a note in notes_for_orchestrator.

Return per SITE_LIST_SCHEMA.
`

const [aSites, bSites, cSites] = await parallel([
  () => agent(extractBrief('FILE_A', FILE_A_PATH, FILE_A_SCOPE),
              { phase: 'Extract', schema: SITE_LIST_SCHEMA, label: 'extract:FILE_A' }),
  () => agent(extractBrief('FILE_B', FILE_B_PATH, FILE_B_SCOPE),
              { phase: 'Extract', schema: SITE_LIST_SCHEMA, label: 'extract:FILE_B' }),
  () => agent(extractBrief('FILE_C', FILE_C_PATH, FILE_C_SCOPE),
              { phase: 'Extract', schema: SITE_LIST_SCHEMA, label: 'extract:FILE_C' }),
])

const extractHalts = [aSites, bSites, cSites].filter(r => !r || r.halt)
if (extractHalts.length > 0) {
  return {
    halt: true,
    phase: 'Extract',
    reasons: extractHalts.map(r => r?.halt_reason || 'agent returned null'),
    extract: { aSites, bSites, cSites },
  }
}

const allSites = [
  ...(aSites.entries || []),
  ...(bSites.entries || []),
  ...(cSites.entries || []),
]
log(`Extracted ${allSites.length} sites total`)
if (allSites.length === 0) {
  return { halt: true, phase: 'Extract', reasons: ['no sites extracted across any file'] }
}

// ============================================================
// PHASE 2 — EDIT PIPELINE (per-file sequential, cross-file parallel)
// ============================================================

phase('Edit')
log('Phase 2: Per-site editor + spot-verify pipeline')

const isNoopSite = (s) => {
  const intent = (s.intent || '').toLowerCase()
  for (const f of NOOP_INTENT_FRAGMENTS) if (intent.includes(f)) return true
  if (/^##\s+\d{3}\./.test(s.locator || '')) return true     // section-header locators
  return false
}
const filteredSites = allSites.filter(s => {
  const noop = isNoopSite(s)
  if (noop) log(`Filtering no-op site ${s.site_id} (intent: ${s.intent?.slice(0, 80)})`)
  return !noop
})
log(`Site filter: ${allSites.length} → ${filteredSites.length} after dropping ${allSites.length - filteredSites.length} no-op site(s)`)

const sortKey = (s) => `${s.file}|${s.locator}`
const sortedSites = filteredSites.slice().sort((a, b) => sortKey(a).localeCompare(sortKey(b)))

const editorBrief = (s) => protocolHeader('EDITOR') + `
== TASK ==
You are editing ONE site of batch ${meta.name}.

Site brief:
- site_id: ${s.site_id}
- file: ${s.file}
- locator: ${s.locator}
- intent: ${s.intent}
- decision_id: ${s.decision_id}
- batch_role: ${s.batch_role}

Plan-asserted reference quote (REFERENCE ONLY — do NOT use as old_string):
"""
${s.verbatim_old}
"""

What you MUST do:
1. Fresh-read the target file at the locator with Read tool.
2. Build old_string from the bytes you just read — verbatim. Confirm it matches what the plan asserts. If it does NOT match, halt with halt_reason: "fresh disk read does not match plan reference snippet".
3. Compose new_string per the intent. Respect project edit-protocol rules (HARD_RULES + LEARNINGS).
4. Apply ONE Edit call with old_string + new_string.
5. After Edit success, read 5-10 lines of context around the edited region. Confirm intent matches outcome and no adjacent paragraph/reference broke.
6. Return structured output per EDIT_SCHEMA. gates_passed lists which gates you ran.

If your old_string isn't unique in the file, expand context until it is. Don't use replace_all.
ONE edit per agent run. ONE site. If you can't complete cleanly, halt — do not continue with a workaround.
`

const verifyBrief = (s, e) => protocolHeader('GATE') + `
== TASK ==
You are spot-verifying ONE already-applied edit.

Site:
- site_id: ${s.site_id}
- file: ${s.file}
- locator: ${s.locator}
- intent: ${s.intent}

The editor reported:
- diff_summary: ${e?.diff_summary || '(none)'}
- gates_passed: ${(e?.gates_passed || []).join(', ')}

Your job:
1. Read the file at the locator with Read tool, capturing 10 lines before + 10 lines after.
2. Confirm (a) the edit matches the intent semantically, (b) no adjacent paragraph or reference broke from adjacent-text bleed.
3. Return per VERIFY_SCHEMA. verified=true if both pass; false otherwise.

This is read-only spot verification — make NO edits.
`

const sitesByFile = {}
for (const s of sortedSites) {
  if (!sitesByFile[s.file]) sitesByFile[s.file] = []
  sitesByFile[s.file].push(s)
}
const filesInOrder = [FILE_A_PATH, FILE_B_PATH, FILE_C_PATH].filter(f => sitesByFile[f]?.length)

const isNoopHalt = (h) => {
  if (!h) return false
  const r = (h || '').toLowerCase()
  for (const f of NOOP_HALT_FRAGMENTS) if (r.includes(f)) return true
  return false
}

const editResults = []
let editHalt = null
for (const file of filesInOrder) {
  if (editHalt) break
  const fileSites = sitesByFile[file]
  log(`Editing ${fileSites.length} sites in ${file.split('/').pop()} (sequential within file)`)
  for (const s of fileSites) {
    if (editHalt) break
    const edit = await agent(editorBrief(s),
                             { phase: 'Edit', schema: EDIT_SCHEMA, label: `edit:${s.site_id}` })
    editResults.push({ kind: 'edit', site_id: s.site_id, result: edit })
    if (!edit) { editHalt = { site_id: s.site_id, reason: 'agent returned null' }; break }
    if (edit.halt) {
      if (isNoopHalt(edit.halt_reason)) {
        log(`No-op halt at ${s.site_id} (benign): ${edit.halt_reason?.slice(0, 100)}`)
        continue
      }
      editHalt = { site_id: s.site_id, reason: edit.halt_reason }
      break
    }
    const verify = await agent(verifyBrief(s, edit),
                               { phase: 'Edit', schema: VERIFY_SCHEMA, label: `verify:${s.site_id}` })
    editResults.push({ kind: 'verify', site_id: s.site_id, result: verify })
    if (!verify || verify.halt || verify.verified === false) {
      editHalt = { site_id: s.site_id, reason: verify?.halt_reason || verify?.reason || 'verify failed' }
      break
    }
  }
}
if (editHalt) {
  return { halt: true, phase: 'Edit', haltSite: editHalt, editResults }
}
log(`Edit phase complete: ${editResults.filter(r => r.kind === 'verify' && r.result.verified).length} sites verified`)

// ============================================================
// PHASE 3 — BATCH GATES (parallel barrier)
// ============================================================
//
// Five gates run concurrently. Customize each prompt to your project's
// invariants. Keep the gate-kind contract identical (returns GATE_SCHEMA).

phase('Batch gates')
log('Phase 3: Parallel mechanical gates over post-edit state')

const gates = await parallel([
  // gate-count: rule/section-count reconciliation, if your project tracks counts.
  () => agent(protocolHeader('GATE') + `
== TASK ==
Gate kind: count
Scope: <describe what counts must reconcile post-batch>
Files: <paths>
Pass condition: zero drift across all checked counts.
Return per GATE_SCHEMA.
`, { phase: 'Batch gates', schema: GATE_SCHEMA, label: 'gate:count' }),

  // gate-grep: retired-term residue sweep.
  () => agent(protocolHeader('GATE') + `
== TASK ==
Gate kind: grep
Scope: retired-term residue across files in scope.
Search for: <list retired terms / patterns that should be gone>
Files: <paths>
Pass condition: zero unexpected live occurrences. Classify each find as live|transitional|stale|uncertain.
Return per GATE_SCHEMA.
`, { phase: 'Batch gates', schema: GATE_SCHEMA, label: 'gate:grep' }),

  // gate-xref: cross-reference integrity.
  () => agent(protocolHeader('GATE') + `
== TASK ==
Gate kind: xref
Scope: cross-reference integrity for batch edits.
Checks:
  1. Every new ref added by this batch resolves to a real target.
  2. <project-specific xref rule>
Files: <paths>
Pass condition: zero dangling refs, zero conflicts.
Return per GATE_SCHEMA.
`, { phase: 'Batch gates', schema: GATE_SCHEMA, label: 'gate:xref' }),

  // gate-invariant: project-specific invariants the batch must preserve.
  () => agent(protocolHeader('GATE') + `
== TASK ==
Gate kind: invariant
Scope: batch foundation invariants post-edit.
Verify ALL of these hold:
  1. <invariant 1>
  2. <invariant 2>
Files: <paths>
Pass condition: every invariant verified.
Return per GATE_SCHEMA.
`, { phase: 'Batch gates', schema: GATE_SCHEMA, label: 'gate:invariant' }),

  // gate-compat: cross-batch compatibility (skip on the first batch).
  () => agent(protocolHeader('GATE') + `
== TASK ==
Gate kind: compat
Scope: this batch's compatibility with earlier batches in the plan.
Checks:
  1. This batch's edits don't contradict any decision still LIVE from earlier batches.
  2. <project-specific compat rule>
File: <path>
Pass condition: zero contradictions, zero ID collisions, zero broken cross-refs.
Return per GATE_SCHEMA.
`, { phase: 'Batch gates', schema: GATE_SCHEMA, label: 'gate:compat' }),
])

const gateHalts    = gates.filter(g => !g || g.halt)
const gateFailures = gates.filter(g => g && !g.halt && g.pass === false)
if (gateHalts.length > 0 || gateFailures.length > 0) {
  log(`Gate findings: ${gateHalts.length} halts, ${gateFailures.length} pass=false — continuing to adversarial for full picture`)
}

// ============================================================
// PHASE 4 — ADVERSARIAL COLD-READ (high-effort skeptic)
// ============================================================

phase('Adversarial')
log('Phase 4: Adversarial cold-read — high-effort skeptical pass')

const adversarial = await agent(protocolHeader('ADVERSARIAL') + `
== TASK ==
You are the adversarial cold-read for batch ${meta.name}.

Read the staged diff end-to-end against the locked plan. Be skeptical. Verify before conceding.

Sources to read:
- Plan: <absolute path>
- Per-batch verification report: <absolute path>
- Master change-list: <absolute path>
- Current state of files in scope: <paths>

Use \`git diff HEAD\` (via Bash) or compare against the last commit to see exactly what this batch changed.

Check for:
1. Fabricated quotes — plan asserts a verbatim_old that disk doesn't have, or batch introduced wording that says something other than the plan's intent.
2. Missed sites — plan implies an edit at site S, no edit at S.
3. Contradictions — batch edits that conflict with each other or with rules elsewhere.
4. Drift from plan intent — edits pass mechanical gates but semantically deviate.
5. Composition failures — individual edits OK, together they create a defect.

Project-specific adversarial checks:
<list invariants the cold-read must spot-check>

Verdict:
- GREEN: every plan intent landed; no fabricated quotes; no contradictions; no missed sites; no composition issues.
- YELLOW: contained defects exist but batch is mostly sound.
- RED: foundational issue — fabricated quote propagated, broken invariant, contradiction with earlier batch.

Return per ADVERSARIAL_SCHEMA. summary_for_user is 2-4 sentences synthesizing the verdict.
`, { schema: ADVERSARIAL_SCHEMA, effort: 'high', label: `adversarial:${meta.name}` })

// ============================================================
// PHASE 5 — E2E REVIEW (orchestrator personal-read forcing function)
// ============================================================
//
// The workflow does NOT make this decision. It captures the unified diff
// + post-edit file sizes + retired-term spot-grep results so the
// orchestrator has everything needed to do one final personal e2e read.
//
// The orchestrator MUST read `e2eReview.unifiedDiff` themselves before
// declaring the batch ready to commit — trust-but-verify, do not delegate
// the final read.

phase('E2E review')
log('Phase 5: Capturing unified diff for orchestrator personal review')

const e2eReview = await agent(protocolHeader('GATE') + `
== TASK ==
Gate kind: e2e-capture
Scope: capture artifacts the orchestrator needs to do one final personal e2e review.

Your job is mechanical: gather data, do not judge.

Steps:
1. Run \`git diff HEAD --stat\` and capture the stats (files changed, insertions, deletions).
2. Run \`git diff HEAD\` and capture the full unified diff.
3. For each retired term listed by this batch's plan (see PROJECT_CONTEXT for the list), run a grep against files in scope and capture the count + locations. Tag each occurrence as live | transitional-prose | uncertain.
4. Return a structured payload with: diff_stats, unified_diff, retired_term_sweep[], files_in_scope[], byte_counts_after_edit{}.

Pass condition: payload is complete (all four sections present). This gate does NOT judge correctness — it stages evidence for the orchestrator's personal read.

Return per GATE_SCHEMA. Put the diff in findings as a single-element array {kind: "unified_diff", content: "<full diff>"} and the other sections similarly. pass=true once payload is staged.
`, { phase: 'E2E review', schema: GATE_SCHEMA, label: 'e2e:capture' })

// ============================================================
// PHASE 6 — SURFACE
// ============================================================

phase('Surface')
log('Phase 6: Returning structured summary — orchestrator must personally read e2eReview.unifiedDiff before commit')

return {
  batch: meta.name,
  totalSites: allSites.length,
  sitesByFile: {
    FILE_A: aSites.entries?.length || 0,
    FILE_B: bSites.entries?.length || 0,
    FILE_C: cSites.entries?.length || 0,
  },
  edits: editResults,
  gates: {
    count:     gates[0],
    grep:      gates[1],
    xref:      gates[2],
    invariant: gates[3],
    compat:    gates[4],
  },
  adversarial,
  e2eReview,
  orchestrator_required_step:
    'Read e2eReview.findings[].content (the unified diff) end-to-end personally before approving. ' +
    'A GREEN adversarial verdict is the agent\'s judgment, not yours. Trust but verify.',
  haltReasons: [
    ...(gateHalts.length > 0 ? gateHalts.map(g => `gate halt: ${g?.halt_reason || 'unknown'}`) : []),
    ...(gateFailures.length > 0 ? gateFailures.map(g => `gate failure (${g.gate_kind}): ${g.findings?.length || 0} findings`) : []),
    ...(adversarial?.halt ? [`adversarial halt: ${adversarial.halt_reason}`] : []),
    ...(adversarial?.verdict === 'RED' ? [`adversarial RED: ${adversarial.verdict_one_line}`] : []),
    ...(e2eReview?.halt ? [`e2e capture halt: ${e2eReview.halt_reason}`] : []),
  ],
}
