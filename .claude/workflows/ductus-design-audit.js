// =============================================================================
// DUCTUS DESIGN AUDIT — reusable workflow
// =============================================================================
// Comprehensive READ-ONLY audit of DECISION_LOG.md + SPEC.md: coherency,
// completeness, contradictions, gaps, logical flaws, design smells,
// ambiguities, vague terms.
//
// Architecture: ONE Fable "Audit Director" (memo-threaded across charter →
// triage → deepening → synthesis) + Opus worker fleets; deterministic script
// plumbing (maps, buckets, coverage ledger, retry, report assembly).
//
// RUN:   Workflow({ name: 'ductus-design-audit', args: { date: 'YYYY-MM-DD' } })
//        (args.date stamps the report filename; scripts cannot call Date)
// COST:  ~200-500 agent calls, ~8-15M tokens, 1-2h wall clock. Opt-in only.
// OUTPUT: packages/ductus-lang/docs/scratchpad/audits/<date>-design-audit.md
//         Read-only otherwise; findings NEVER license editing LOG/SPEC.
//
// PROVENANCE: first executed as run wf_888290b7-86d on 2026-07-06 (plan:
// ~/.claude/plans/create-audit-workflow-for-snazzy-charm.md, user-approved).
// The protocol supersessions below (halt-on-contradiction disabled for
// audited-document content; Invariant-1 authority over the protocol file's
// stale "never renumber" rule) were approved in that plan.
//
// MAINTENANCE NOTES:
// - LEARNINGS snapshot below was embedded 2026-07-06. Agents are also told to
//   read LEARNINGS.md for rules added later, but refresh the snapshot
//   occasionally.
// - Decision ids cited in prompts (e.g. 002-16, 001-25) are pointers as of
//   2026-07-06. The LOG renumbers positionally (Invariant 1) — prompts
//   instruct agents to locate rules by CONTENT when an id has drifted.
// - Corpus sizes are measured at runtime (wc -l in the map phase); nothing
//   about document length is hardcoded.
// - The corpus must not be edited while the audit runs (pin check will halt
//   verification on mismatch).
// =============================================================================

export const meta = {
  name: 'ductus-design-audit',
  description: 'Comprehensive read-only audit of DECISION_LOG.md + SPEC.md: one Fable Director, Opus fleets',
  whenToUse: 'When the user asks for a full design audit of the Ductus decision log + spec (expensive: ~1-2h, millions of tokens). Pass args.date = today as YYYY-MM-DD.',
  phases: [
    { title: 'Map', detail: '5 mappers: structure, churn, terms, fences' },
    { title: 'Charter', detail: 'Director shards the corpus', model: 'fable' },
    { title: 'Claims index', detail: 'normalize all LOG entries, collide subjects' },
    { title: 'Shard audits', detail: 'deep per-shard defect hunt' },
    { title: 'Sweeps & probes', detail: 'pairs, terms, smells, completeness, parser, witnesses, fences, losslessness' },
    { title: 'Gates', detail: '8 mechanical invariant gates' },
    { title: 'Triage', detail: 'bucket, dedup, Director ranks', model: 'fable' },
    { title: 'Deepening', detail: 'max 2 rounds + completeness critic' },
    { title: 'Verification', detail: 'batched V-evidence + V-charity' },
    { title: 'Synthesis', detail: 'Director narrative + script assembly', model: 'fable' },
    { title: 'Persist', detail: 'chunked writers + concat + verify' },
  ],
}

// ============ CONSTANTS ============
const REPORT_DATE = (args && args.date) ? String(args.date) : 'undated'
const ROOT = '/Users/torniketsomaia/projects/@tsomaia.tech/ductuslang'
const LOG = ROOT + '/packages/ductus-lang/docs/DECISION_LOG.md'
const SPEC = ROOT + '/packages/ductus-lang/docs/SPEC.md'
const BACKLOG = ROOT + '/packages/ductus-lang/docs/BACKLOG.md'
const AMEND_PLANS_DIR = ROOT + '/packages/ductus-lang/docs/scratchpad/plans'
const PROTOCOL = ROOT + '/WORKFLOW_PROTOCOL.md'
const REPORT_DIR = ROOT + '/packages/ductus-lang/docs/scratchpad/audits'
const REPORT_PATH = REPORT_DIR + '/' + REPORT_DATE + '-design-audit.md'

const LEARNINGS = `1. Never decide unilaterally: Any deviation from what was agreed, and any design or terminology choice the user hasn't made, must be brought back to the user — not resolved on my own. A tool/subagent finding (even "HIGH severity") does not license re-deciding; surface it as a question.
2. Plain language, more examples: Keep prose terse and simple; lead with concrete code examples rather than dense, jargon-heavy explanation. Example: explain a "view" by showing \`view channels: Channel+\` and \`channels[0].gain\`, not a paragraph of abstractions.
3. Don't trust memory across the large spec: With thousands of decisions in DECISION_LOG.md and a huge SPEC.md, do not cite IDs or "the spec says X" from recall — verify, and delegate exhaustive sweeps to read-only subagents.
4. Approval is literal: Only "approve"/"approved" counts as approval. "yes do it", "resume", etc. do not authorize implementation.
5. Edit protocol is DECISION_LOG-first: For any spec change, amend DECISION_LOG.md first, then conform the referenced SPEC.md section. Divergence between log and spec is a defect.
6. Capture design before it evaporates: write decisions down before doing expensive follow-on work.
7. Never use AskUserQuestion; ask in plain prose.
8. Ground in DECISION_LOG before answering, not SPEC or memory: The LOG is the self-contained decision-of-record; read the construct's full decision block there first, not a narrow grep and not the SPEC.
9. Don't report work as done unless it is: Never claim a file was edited/read without actually doing it.
10. Don't pattern-match syntax into a data model: Naming an access form is not a data-structure design. State what structure actually backs each construct, or say it is unpinned.
11. Verify before conceding or defending: On a challenge, check DECISION_LOG/BACKLOG before caving or doubling down.
12. Re-anchor on redirection.
13. Lead with a code example, then one line; cite at most one decision-log id per point.
14. Don't pass coined terms off as defined: Cite the decision that defines a term, or drop the term.
15. Don't let a sub-question balloon scope.
16. Placements always bind to borrows — that is uniform: \`as <name>\` on a child placement, \`as <name>\` on a bundle, every form. The Handle/auto-coerce step happens at the receiving view's element type, NOT at the placement.`

const SEVERITY_RUBRIC = `SEVERITY RUBRIC (mandatory; do not invent your own scale):
- HIGH: soundness hole; a contradiction an implementer cannot satisfy both sides of; jointly unsatisfiable rules; an implementer-blocking gap with no legal boundary (per the LOG's "no postponed decisions" identity rules, the only legal boundaries are standard-library delegation or implementation-defined behavior).
- MED: behavior-changing ambiguity (two careful readings yield different concrete program behavior); substantive LOG-SPEC divergence.
- LOW: terminology drift, style, redundancy, design smell without a concrete witness.`

const ID_DRIFT_NOTE = `NOTE on decision ids: ids cited in this brief (like 002-16) are pointers as of the script's writing. The LOG renumbers positionally — if a cited id does not match the described content, locate the rule by its CONTENT (grep for its distinctive phrases) and use its CURRENT id in your findings.`

function preamble(pin, sizes) {
  const sz = sizes || { logLines: '(measure via wc -l)', specLines: '(measure via wc -l)' }
  return `You are one agent in a large READ-ONLY audit of the Ductus language design documents. This audit runs a user-approved plan.

STEP 1 (mandatory): Read ${PROTOCOL} in full. Your output schema includes protocol_acknowledged; returning true without reading it is forbidden. ALSO read ${ROOT}/LEARNINGS.md — the snapshot below may lag it.

PRECEDENCE OVERRIDES (user-approved; they override WORKFLOW_PROTOCOL.md where they conflict):
(a) HALT SUPERSESSION: The protocol's Discovery rules say to HALT on discovered contradiction/ambiguity/incoherence. For THIS audit, contradictions, ambiguities, and incoherences INSIDE the audited documents are your DELIVERABLE — report them as findings, never halt on them. The ONLY legal halt reasons are: scope insufficient, context exhausted, undecidable brief, or corpus changed under you (pin mismatch).
(b) STALE RULE: The protocol's "never renumber or reuse stable IDs" rule is stale. DECISION_LOG.md Invariant 1 (dense positional numbering; a decision's number is its position, renumbering on insert/remove is required) is the CURRENT authority. Do not flag Invariant 1 itself as a defect.
(c) You are read-only regardless of protocol sections about editing: NEVER edit, create, or delete any file. Never run mutating git/Bash. Report only.

PROJECT CONTEXT:
- Decision-of-record: ${LOG} (${sz.logLines} lines; sections '## NNN. Title — N Rules'; entries 'NNN-M. sentence (SECTION-REF)'). The LOG is the lossless WHAT.
- Normative elaboration: ${SPEC} (${sz.specLines} lines). SPEC must conform to LOG; divergence is a defect. SPEC never references LOG entries; LOG entries each carry one (SECTION-REF) into SPEC.
- LOG Invariants: (1) dense positional numbering per section, no gaps; (2) entries are atomic and self-contained, never referencing other entries by number — required restatement instead; (3) refs flow LOG-to-SPEC only.
- Recently churned zones (see git history) are high-risk.
- Corpus pin: git HEAD ${pin.sha}; DECISION_LOG.md ${pin.log_bytes} bytes; SPEC.md ${pin.spec_bytes} bytes. If a file you read appears to differ materially from this pin, halt with reason 'pin mismatch'.

PROJECT WORKING RULES (LEARNINGS snapshot, verbatim — binding):
${LEARNINGS}

${SEVERITY_RUBRIC}

EVIDENCE DISCIPLINE:
- Every claim carries verbatim quotes plus file path and line numbers. NO paraphrase in evidence fields. Build quotes from a fresh Read at report time.
- The Read tool returns at most 2000 lines per call. Any range larger than 2000 lines REQUIRES multiple offset Reads; confirm the final line you read equals the end of your assigned range. Attest honestly in coverage_attestation: read_fully / skimmed / unread line ranges. "No findings" in an unread range is a lie; attest the gap instead.
- Do NOT fix anything. Do NOT decide anything. Do NOT spawn agents. Do NOT use AskUserQuestion. Direction-of-change only, never edit text.
- Findings cap: at most 40 findings per agent. If you have more, keep the 40 most severe and state the overflow (with line ranges needing a follow-up pass) in notes_for_orchestrator.`
}

// ============ SCHEMAS ============
const PROTO_FIELDS = {
  protocol_acknowledged: { type: 'boolean', description: 'true only if you actually read WORKFLOW_PROTOCOL.md this run' },
  halt: { type: 'boolean', description: 'true ONLY for: scope insufficient, context exhausted, undecidable brief, pin mismatch. NEVER because you found contradictions/ambiguities in the audited docs - those are findings.' },
  halt_reason: { type: ['string', 'null'] },
}
const ATTESTATION = {
  type: 'object',
  description: 'Honest read attestation. Ranges as "file:start-end" strings.',
  properties: {
    read_fully: { type: 'array', items: { type: 'string' } },
    skimmed: { type: 'array', items: { type: 'string' } },
    unread: { type: 'array', items: { type: 'string' } },
  },
  required: ['read_fully', 'skimmed', 'unread'],
}
const FINDING_ITEM = {
  type: 'object',
  properties: {
    one_line: { type: 'string', description: 'The defect in <= 25 words, self-contained' },
    category: { type: 'string', enum: ['contradiction', 'divergence', 'ambiguity', 'vague_term', 'undefined_term', 'gap', 'logical_flaw', 'design_smell', 'observation', 'invariant_violation', 'stale_example', 'spec_only_normative', 'parse_ambiguity', 'unsatisfiable', 'unfalsifiable', 'other'] },
    severity: { type: 'string', enum: ['HIGH', 'MED', 'LOW'] },
    log_ids: { type: 'array', items: { type: 'string', pattern: '^[0-9]{3}-[0-9]+$' }, description: 'Every LOG entry id this finding anchors to, strict NNN-M' },
    spec_sections: { type: 'array', items: { type: 'string', pattern: '^[0-9]+(\\.[0-9]+)*$' }, description: 'Every SPEC section number this finding anchors to, dotted, no section sign' },
    evidence: { type: 'array', minItems: 1, items: { type: 'object', properties: { file: { type: 'string' }, line_start: { type: 'integer' }, line_end: { type: 'integer' }, verbatim: { type: 'string', description: 'exact quote from disk, no paraphrase' } }, required: ['file', 'line_start', 'line_end', 'verbatim'] } },
    why_defect: { type: 'string' },
    direction_of_change: { type: 'string', description: 'direction only, never concrete edit text' },
  },
  required: ['one_line', 'category', 'severity', 'log_ids', 'spec_sections', 'evidence', 'why_defect', 'direction_of_change'],
}
const FINDINGS_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    coverage_attestation: ATTESTATION,
    findings: { type: 'array', maxItems: 40, items: FINDING_ITEM },
    notes_for_orchestrator: { type: 'string' },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'coverage_attestation', 'findings', 'notes_for_orchestrator'],
}

const MAP_LOG_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    head_sha: { type: 'string' },
    log_bytes: { type: 'integer' },
    spec_bytes: { type: 'integer' },
    log_total_lines: { type: 'integer' },
    sections: { type: 'array', items: { type: 'object', properties: { id: { type: 'string', pattern: '^[0-9]{3}$' }, title: { type: 'string' }, line_start: { type: 'integer' }, line_end: { type: 'integer' }, declared_count: { type: 'integer' }, actual_count: { type: 'integer' }, refs: { type: 'array', items: { type: 'object', properties: { section: { type: 'string' }, n: { type: 'integer' } }, required: ['section', 'n'] } } }, required: ['id', 'title', 'line_start', 'line_end', 'declared_count', 'actual_count', 'refs'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'head_sha', 'log_bytes', 'spec_bytes', 'log_total_lines', 'sections'],
}
const MAP_SPEC_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    spec_total_lines: { type: 'integer', description: 'wc -l of SPEC.md, measured by you' },
    headings: { type: 'array', items: { type: 'object', properties: { num: { type: 'string' }, title: { type: 'string' }, line: { type: 'integer' } }, required: ['num', 'title', 'line'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'spec_total_lines', 'headings'],
}
const MAP_CONTEXT_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    churn_zones: { type: 'array', items: { type: 'object', properties: { file: { type: 'string' }, description: { type: 'string' }, log_sections: { type: 'array', items: { type: 'string' } }, spec_sections: { type: 'array', items: { type: 'string' } } }, required: ['file', 'description'] } },
    register: { type: 'array', items: { type: 'object', properties: { key: { type: 'string' }, claim: { type: 'string', description: 'the specific defect/question, claim-level, not topic-level' }, rule_ids: { type: 'array', items: { type: 'string' } }, source: { type: 'string' }, quote: { type: 'string' }, status: { type: 'string' } }, required: ['key', 'claim', 'source', 'quote'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'churn_zones', 'register'],
}
const MAP_TERMS_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    defined_terms: { type: 'array', maxItems: 250, items: { type: 'object', properties: { term: { type: 'string' }, defined_at: { type: 'string' } }, required: ['term', 'defined_at'] } },
    undefined_candidates: { type: 'array', maxItems: 150, items: { type: 'object', properties: { term: { type: 'string' }, sample_locations: { type: 'array', items: { type: 'string' } }, note: { type: 'string' } }, required: ['term', 'sample_locations'] } },
    alias_suspects: { type: 'array', maxItems: 80, items: { type: 'object', properties: { terms: { type: 'array', items: { type: 'string' } }, note: { type: 'string' } }, required: ['terms'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'defined_terms', 'undefined_candidates', 'alias_suspects'],
}
const MAP_FENCES_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    fences: { type: 'array', items: { type: 'object', properties: { line_start: { type: 'integer' }, line_end: { type: 'integer' }, lang: { type: 'string' } }, required: ['line_start', 'line_end'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'fences'],
}
const CLAIMS_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    coverage_attestation: ATTESTATION,
    claims: { type: 'array', items: { type: 'object', properties: { log_id: { type: 'string', pattern: '^[0-9]{3}-[0-9]+$' }, subjects: { type: 'array', minItems: 1, maxItems: 3, items: { type: 'string' } }, predicate: { type: 'string', description: 'the constraint in <= 20 words' }, modality: { type: 'string', enum: ['must', 'must_not', 'may', 'is', 'defines'] } }, required: ['log_id', 'subjects', 'predicate', 'modality'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'coverage_attestation', 'claims'],
}
const CHARTER_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    shards: { type: 'array', items: { type: 'object', properties: { shard_id: { type: 'string' }, log_sections: { type: 'array', minItems: 1, items: { type: 'string', pattern: '^[0-9]{3}$' } }, log_entry_split: { type: ['string', 'null'], description: 'if a giant section is split: "NNN:from-to" entry number range, else null' }, spec_sections: { type: 'array', items: { type: 'string' } }, risk_note: { type: 'string' }, seed_questions: { type: 'array', items: { type: 'string' } } }, required: ['shard_id', 'log_sections', 'log_entry_split', 'spec_sections', 'risk_note', 'seed_questions'] } },
    cross_pairs: { type: 'array', maxItems: 15, items: { type: 'object', properties: { a: { type: 'string' }, b: { type: 'string' }, why: { type: 'string' }, seed_questions: { type: 'array', items: { type: 'string' } } }, required: ['a', 'b', 'why'] } },
    clusters: { type: 'array', minItems: 6, maxItems: 10, items: { type: 'object', properties: { name: { type: 'string' }, log_sections: { type: 'array', items: { type: 'string' } }, focus: { type: 'string' } }, required: ['name', 'log_sections', 'focus'] } },
    scenario_seeds: { type: 'array', maxItems: 10, items: { type: 'object', properties: { title: { type: 'string' }, scenario: { type: 'string' }, touching_sections: { type: 'array', items: { type: 'string' } } }, required: ['title', 'scenario', 'touching_sections'] } },
    term_seeds: { type: 'array', items: { type: 'string' } },
    memo: { type: 'object', properties: { risk_map: { type: 'string' }, decisions: { type: 'array', items: { type: 'string' } }, open_questions: { type: 'array', items: { type: 'string' } } }, required: ['risk_map', 'decisions', 'open_questions'] },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'shards', 'cross_pairs', 'clusters', 'scenario_seeds', 'term_seeds', 'memo'],
}
const DEDUP_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    clusters: { type: 'array', items: { type: 'object', properties: { representative_id: { type: 'string' }, member_ids: { type: 'array', minItems: 1, items: { type: 'string' } }, merged_one_line: { type: 'string' }, severity: { type: 'string', enum: ['HIGH', 'MED', 'LOW'] }, category: { type: 'string' }, note: { type: 'string' } }, required: ['representative_id', 'member_ids', 'merged_one_line', 'severity', 'category'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'clusters'],
}
const TRIAGE_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    severity_overrides: { type: 'array', items: { type: 'object', properties: { finding_id: { type: 'string' }, severity: { type: 'string', enum: ['HIGH', 'MED', 'LOW'] }, why: { type: 'string' } }, required: ['finding_id', 'severity', 'why'] } },
    known_open_tags: { type: 'array', items: { type: 'object', properties: { finding_id: { type: 'string' }, register_key: { type: 'string' }, register_quote: { type: 'string', description: 'quote of the register entry proving same-defect match; adjacency is NOT a match' } }, required: ['finding_id', 'register_key', 'register_quote'] } },
    drop_as_duplicate: { type: 'array', items: { type: 'object', properties: { finding_id: { type: 'string' }, duplicate_of: { type: 'string' } }, required: ['finding_id', 'duplicate_of'] } },
    deepening_charters: { type: 'array', maxItems: 14, items: { type: 'object', properties: { title: { type: 'string' }, question: { type: 'string' }, log_ranges: { type: 'array', items: { type: 'string' } }, spec_sections: { type: 'array', items: { type: 'string' } }, lens: { type: 'string' } }, required: ['title', 'question', 'lens'] } },
    memo: { type: 'object', properties: { risk_map: { type: 'string' }, decisions: { type: 'array', items: { type: 'string' } }, open_questions: { type: 'array', items: { type: 'string' } } }, required: ['risk_map', 'decisions', 'open_questions'] },
    done_deepening: { type: 'boolean' },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'severity_overrides', 'known_open_tags', 'drop_as_duplicate', 'deepening_charters', 'memo', 'done_deepening'],
}
const VEVIDENCE_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    pin_ok: { type: 'boolean' },
    results: { type: 'array', items: { type: 'object', properties: { finding_id: { type: 'string' }, evidence_verdict: { type: 'string', enum: ['pass', 'partial', 'fail'] }, quote_notes: { type: 'string', description: 'per-citation: exists verbatim? supports the claim? corrected line numbers if drifted <= 3 lines' }, crux: { type: 'string' } }, required: ['finding_id', 'evidence_verdict', 'quote_notes', 'crux'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'pin_ok', 'results'],
}
const VCHARITY_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    pin_ok: { type: 'boolean' },
    results: { type: 'array', items: { type: 'object', properties: { finding_id: { type: 'string' }, verdict: { type: 'string', enum: ['sustain', 'refute', 'refile_divergence'] }, dissolving_evidence: { type: 'string', description: 'REQUIRED if refute/refile: file:line + verbatim quote of the passage; for ambiguity findings the reading must be FORCED by quoted text, not merely available' }, crux: { type: 'string' } }, required: ['finding_id', 'verdict', 'crux'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'pin_ok', 'results'],
}
const TIEBREAK_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    verdicts: { type: 'array', items: { type: 'object', properties: { finding_id: { type: 'string' }, verdict: { type: 'string', enum: ['sustain', 'refute'] }, crux: { type: 'string' } }, required: ['finding_id', 'verdict', 'crux'] } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'verdicts'],
}
const CRITIC_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    gaps: { type: 'array', items: { type: 'object', properties: { what: { type: 'string' }, why_missed: { type: 'string' }, suggested_charter: { type: 'object', properties: { title: { type: 'string' }, question: { type: 'string' }, log_ranges: { type: 'array', items: { type: 'string' } }, spec_sections: { type: 'array', items: { type: 'string' } }, lens: { type: 'string' } }, required: ['title', 'question', 'lens'] } }, required: ['what', 'why_missed', 'suggested_charter'] } },
    honest_disclaimers: { type: 'array', items: { type: 'string' } },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'gaps', 'honest_disclaimers'],
}
const SYNTH_SCHEMA = {
  type: 'object',
  properties: {
    ...PROTO_FIELDS,
    verdict: { type: 'string', enum: ['GREEN', 'YELLOW', 'RED'] },
    verdict_one_line: { type: 'string' },
    exec_summary: { type: 'string', description: 'plain language, terse, 300-600 words, leads with the most important confirmed problems' },
    reading_guide: { type: 'string' },
    themes: { type: 'array', items: { type: 'object', properties: { title: { type: 'string' }, finding_ids: { type: 'array', items: { type: 'string' } }, note: { type: 'string' } }, required: ['title', 'finding_ids'] } },
    coverage_narrative: { type: 'string' },
    known_open_note: { type: 'string' },
  },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'verdict', 'verdict_one_line', 'exec_summary', 'reading_guide', 'themes', 'coverage_narrative', 'known_open_note'],
}
const WRITER_SCHEMA = {
  type: 'object',
  properties: { ...PROTO_FIELDS, bytes_written: { type: 'integer' }, path: { type: 'string' } },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'bytes_written', 'path'],
}
const CONCAT_SCHEMA = {
  type: 'object',
  properties: { ...PROTO_FIELDS, path: { type: 'string' }, bytes: { type: 'integer' }, pin_ok: { type: 'boolean' }, parts_removed: { type: 'boolean' } },
  required: ['protocol_acknowledged', 'halt', 'halt_reason', 'path', 'bytes', 'pin_ok', 'parts_removed'],
}

// ============ LEDGER & HELPERS ============
const ledger = []
let PIN = { sha: 'unpinned', log_bytes: 0, spec_bytes: 0 }
async function run(label, phaseName, prompt, opts) {
  let res = null
  let attempts = 0
  while (attempts < 2 && !res) {
    attempts++
    try {
      res = await agent(prompt, { ...(opts || {}), label: attempts > 1 ? label + ':retry' : label, phase: phaseName })
    } catch (e) {
      res = null
      if (String(e).indexOf('budget') >= 0) throw e
    }
  }
  ledger.push({ label, phase: phaseName, status: res ? (res.halt ? 'halt' : 'ok') : 'dead', retries: attempts - 1, halt_reason: res && res.halt ? res.halt_reason : null })
  if (res && res.halt) log('HALT [' + label + ']: ' + res.halt_reason)
  if (!res) log('DEAD [' + label + '] after retry — recorded as coverage hole')
  return res
}
const allFindings = []
let findingSeq = 0
function collect(family, sourceLabel, res) {
  if (!res || !Array.isArray(res.findings)) return 0
  let n = 0
  for (const f of res.findings) {
    findingSeq++
    allFindings.push({ ...f, finding_id: 'F' + String(findingSeq).padStart(3, '0'), family, source: sourceLabel })
    n++
  }
  return n
}
function utf8len(s) {
  let n = 0
  for (let i = 0; i < s.length; i++) {
    const c = s.codePointAt(i)
    if (c > 0xffff) i++
    n += c < 0x80 ? 1 : c < 0x800 ? 2 : c < 0x10000 ? 3 : 4
  }
  return n
}
function fmtEvidence(f) {
  return (f.evidence || []).map(e => '  - `' + e.file.replace(ROOT + '/', '') + ':' + e.line_start + '-' + e.line_end + '`\n    > ' + String(e.verbatim).split('\n').join('\n    > ')).join('\n')
}
function digestOf(f) {
  return f.finding_id + ' [' + f.severity + '/' + f.category + '] anchors(' + (f.log_ids || []).join(',') + ';' + (f.spec_sections || []).join(',') + ') ' + f.one_line
}

// ============ PHASE: MAP ============
phase('Map')
log('Mapping corpus structure, churn, terms, fences')
const mapPreamble = preamble(PIN, null)
const [mapLog, mapSpecA, mapSpecB, mapContext, mapTerms, mapFences] = await parallel([
  () => run('map-log', 'Map', `${mapPreamble}

ROLE: map-log explorer (read-only). Build the structural map of ${LOG}.
1. Run: git -C ${ROOT} rev-parse HEAD ; wc -c on both ${LOG} and ${SPEC} ; wc -l on ${LOG}. Report as head_sha, log_bytes, spec_bytes, log_total_lines.
2. For each section '## NNN. Title — N Rules': report id, title, line_start (header line), line_end (line before next header or EOF), declared_count (the N in the header), actual_count (count of lines matching ^NNN- in that range via grep -c), and refs: the UNIQUE SPEC section numbers cited in that section's entries as (SECTION-REF), with occurrence counts. Extract the dotted number only (e.g. from '(§13.3.6.1)' report section '13.3.6.1'). Use grep -o piped to sort|uniq -c via Bash for exactness; do not do this from memory.
Return compact structured data only.`, { model: 'opus', effort: 'medium', schema: MAP_LOG_SCHEMA }),
  () => run('map-spec-a', 'Map', `${mapPreamble}

ROLE: map-spec explorer (read-only), FIRST half. Run wc -l on ${SPEC}; call the result N and report it as spec_total_lines. Your range is lines 1 through floor(N/2). In that range, find EVERY markdown heading that carries a section number (patterns like '# 13.', '## 13.3', '### 13.3.6.1' — inspect the file's actual heading convention by Reading the first 80 lines first). Report each as {num: dotted number string, title, line}. Use Grep with -n. Completeness matters more than anything: a missed heading corrupts downstream sharding. Cross-check your count against grep -c of the heading pattern in the range.`, { model: 'opus', effort: 'medium', schema: MAP_SPEC_SCHEMA }),
  () => run('map-spec-b', 'Map', `${mapPreamble}

ROLE: map-spec explorer (read-only), SECOND half. Run wc -l on ${SPEC}; call the result N and report it as spec_total_lines. Your range is lines floor(N/2)+1 through N. Same task: find EVERY numbered markdown heading in your range, report {num, title, line}. Inspect the heading convention by reading a small sample at the start of your range first. Cross-check count via grep -c.`, { model: 'opus', effort: 'medium', schema: MAP_SPEC_SCHEMA }),
  () => run('map-context', 'Map', `${mapPreamble}

ROLE: map-context explorer (read-only). Two deliverables:
1. churn_zones: run git -C ${ROOT} log --stat -15 -- packages/ductus-lang/docs/DECISION_LOG.md packages/ductus-lang/docs/SPEC.md and (for recent large commits) git -C ${ROOT} show <sha> --stat plus a look at which LOG sections / SPEC sections the diff hunks touch (read hunk headers only — do NOT read full diffs into context). Report which LOG sections (NNN) and SPEC sections were recently churned.
2. register: read ${BACKLOG} in full and every plan document in ${AMEND_PLANS_DIR} (ls it first; read each). Extract every known-open/deferred/parked design issue and review-finding with its fix status. Each register entry must be CLAIM-LEVEL: the specific defect or behavioral question (with rule ids if stated), not a topic. Quote the source line(s) verbatim. Status: open/locked/landed/parked/discarded as stated by the doc.`, { model: 'opus', effort: 'medium', schema: MAP_CONTEXT_SCHEMA }),
  () => run('map-terms', 'Map', `${mapPreamble}

ROLE: map-terms explorer (read-only). Build a mechanical term inventory over BOTH ${LOG} and ${SPEC} using Grep (not memory, not judgment):
- defined_terms: terms with definitional sentences — grep patterns like 'is a ', 'is the ', 'means ', '**term**' bold-definitions, 'A <term> is'. Record term + one defining location (file:line).
- undefined_candidates: recurring technical terms (multi-use, load-bearing: backticked terms, Capitalized Nominals, hyphenated terms of art) that have NO definitional sentence you could find. Include 2-3 sample locations each.
- alias_suspects: clusters of similar terms that may denote the same concept (e.g. variants sharing a head noun with different modifiers). Mechanical closeness is enough to report; adjudication happens later.
Bound your output to the schema caps by keeping the most load-bearing (most-used) items.`, { model: 'opus', effort: 'medium', schema: MAP_TERMS_SCHEMA }),
  () => run('map-fences', 'Map', `${mapPreamble}

ROLE: map-fences explorer (read-only). List EVERY fenced code block in ${SPEC}: grep -n for fence delimiters (three backticks), pair them up in order (odd opens, even closes), report {line_start, line_end, lang} where lang is the info string on the opening fence (empty string if none). Verify the total fence-delimiter count is even; if odd, halt with 'undecidable brief: unbalanced fences at <line>'. Do not read the fence contents.`, { model: 'opus', effort: 'low', schema: MAP_FENCES_SCHEMA }),
])

if (!mapLog || mapLog.halt) throw new Error('map-log failed/halted: audit cannot proceed without the LOG map')
if (!mapSpecA || !mapSpecB) throw new Error('map-spec failed: audit cannot proceed without the SPEC tree')
PIN = { sha: mapLog.head_sha, log_bytes: mapLog.log_bytes, spec_bytes: mapLog.spec_bytes }
const LOG_LINES = mapLog.log_total_lines
const SPEC_LINES = mapSpecA.spec_total_lines
if (mapSpecB.spec_total_lines !== SPEC_LINES) throw new Error('map-spec halves disagree on SPEC length: ' + SPEC_LINES + ' vs ' + mapSpecB.spec_total_lines)
const P = preamble(PIN, { logLines: LOG_LINES, specLines: SPEC_LINES })

// --- deterministic map validation ---
const logSections = [...mapLog.sections].sort((a, b) => a.line_start - b.line_start)
if (logSections.length < 20) throw new Error('map-log returned only ' + logSections.length + ' sections — implausible')
for (let i = 1; i < logSections.length; i++) {
  if (logSections[i].line_start <= logSections[i - 1].line_end) throw new Error('map-log ranges not monotonic at section ' + logSections[i].id)
}
if (logSections[logSections.length - 1].line_end > LOG_LINES) throw new Error('map-log range exceeds file length')
const SECTION_COUNT = logSections.length
function secOf(id) { return logSections.find(s => s.id === id) }
function secText(id) { const s = secOf(id); return s ? 'lines ' + s.line_start + '-' + s.line_end : "(locate by grepping '^## ' headers)" }
const headings = [...(mapSpecA.headings || []), ...(mapSpecB.headings || [])].sort((a, b) => a.line - b.line)
if (headings.length < 300) throw new Error('map-spec suspiciously small: ' + headings.length + ' headings')
// compute leaf and subtree ranges
const specTree = headings.map((h, i) => {
  const depth = h.num.split('.').length
  let leafEnd = SPEC_LINES
  if (i + 1 < headings.length) leafEnd = headings[i + 1].line - 1
  let subEnd = SPEC_LINES
  for (let j = i + 1; j < headings.length; j++) {
    if (headings[j].num.split('.').length <= depth) { subEnd = headings[j].line - 1; break }
  }
  return { num: h.num, title: h.title, line: h.line, leaf_end: leafEnd, subtree_end: subEnd, depth }
})
const specByNum = {}
for (const s of specTree) specByNum[s.num] = s
log('Maps validated: ' + SECTION_COUNT + ' LOG sections, ' + specTree.length + ' SPEC headings. Pinned at ' + PIN.sha.slice(0, 10))

// cited set + orphan computation (covered = self, ancestor, or descendant cited)
const citedSet = new Set()
for (const sec of logSections) for (const r of (sec.refs || [])) citedSet.add(r.section)
function isCovered(num) {
  if (citedSet.has(num)) return true
  for (const c of citedSet) { if (c.startsWith(num + '.') || num.startsWith(c + '.')) return true }
  return false
}
const uncovered = specTree.filter(s => !isCovered(s.num))
const maximalOrphans = uncovered.filter(s => { const parent = s.num.split('.').slice(0, -1).join('.'); return !parent || isCovered(parent) || !uncovered.some(u => u.num === parent) })
log('Orphan analysis (script): ' + uncovered.length + ' uncovered headings, ' + maximalOrphans.length + ' maximal orphan subtrees')

function specRangesFor(nums) {
  const out = []
  for (const n of (nums || [])) {
    const s = specByNum[n]
    if (!s) continue
    if (s.depth <= 1) {
      out.push({ num: n, from: s.line, to: Math.min(s.leaf_end, s.line + 120), note: 'chapter intro only' })
    } else {
      out.push({ num: n, from: s.line, to: s.leaf_end, note: 'leaf' })
    }
  }
  out.sort((a, b) => a.from - b.from)
  return out
}
function estTokens(logLines, specLines) { return Math.round((logLines * 150 + specLines * 45) / 4) }

// ============ PHASE: CHARTER ============
phase('Charter')
const sectionSummary = logSections.map(s => s.id + ' "' + s.title + '" lines ' + s.line_start + '-' + s.line_end + ' (' + s.actual_count + ' entries) cites: ' + (s.refs || []).slice(0, 40).map(r => r.section).join(',')).join('\n')
const registerText = (mapContext && mapContext.register ? mapContext.register : []).map(r => '- [' + r.key + '] (' + (r.status || '?') + ') ' + r.claim + ' | rules: ' + ((r.rule_ids || []).join(',') || '-') + ' | src: ' + r.source).join('\n')
const churnText = (mapContext && mapContext.churn_zones ? mapContext.churn_zones : []).map(c => '- ' + c.file + ': ' + c.description + ' | LOG ' + ((c.log_sections || []).join(',') || '?') + ' | SPEC ' + ((c.spec_sections || []).join(',') || '?')).join('\n')
const giantsText = logSections.filter(s => s.actual_count > 200).map(s => s.id + ' has ' + s.actual_count + ' entries').join(', ') || 'none'

const DIRECTOR_ROLE = `You are the AUDIT DIRECTOR — the single orchestrating mind of this audit, running across several calls. Your memo threads your state between calls; write it for your future self: decisions made, risk map, open questions. You never read the corpus wholesale yourself; you direct Opus worker fleets and judge their output. The mandate: honest, rigorous, comprehensive audit of the Ductus design docs — coherency, completeness, contradictions, gaps, logical flaws, design smells, ambiguities, vague terms, and anything else. Findings never license re-deciding any design question; everything surfaces to the user.`

const charter = await run('director-charter', 'Charter', `${P}

${DIRECTOR_ROLE}

THIS CALL: charter the audit. You may Read small samples of either doc to calibrate (a few hundred lines max), but the maps below are your primary input.

LOG SECTION MAP (id, title, lines, entry count, cited SPEC sections):
${sectionSummary}

CHURN (recently amended, higher risk):
${churnText}

KNOWN-OPEN REGISTER (already-known issues; the audit distinguishes new findings from these):
${registerText}

DELIVER:
1. shards[]: LOG-section-centric audit units covering ALL ${SECTION_COUNT} sections exactly once. Merge small adjacent-topic sections; split giant sections (${giantsText}) via log_entry_split (e.g. "017:1-160"). Each shard lists the SPEC sections its entries cite (from the map; leaf reading — chapter-level refs like bare '13' resolve to chapter intro + specifically cited descendants). Budget: one Opus agent must read the shard's LOG lines + SPEC leaf ranges within ~100k tokens. Give each shard a risk_note and 3-6 seed_questions targeting its specific risks (use churn + register + your own reading of the section map).
2. cross_pairs[]: up to 15 section pairs with the highest semantic-conflict risk that a subject-collision index would NOT catch (different vocabulary, same semantics). Justify each.
3. clusters[]: 6-10 subsystem clusters (grouping LOG sections) for the design-smell and implementer-completeness panels.
4. scenario_seeds[]: up to 10 constructive multi-section witness scenarios (3+ sections interacting, e.g. reactive constructs under keyed scopes, hot-reload against ownership, streams under gates). State the scenario concretely.
5. term_seeds[]: additional load-bearing terms for the terminology sweep beyond the mechanical inventory.
6. memo: your threaded state.`, { model: 'fable', effort: 'xhigh', schema: CHARTER_SCHEMA })
if (!charter || charter.halt) throw new Error('Director charter failed/halted: ' + (charter ? charter.halt_reason : 'dead'))

// --- validate charter coverage; fallback-patch missing sections ---
const coveredSecs = new Set()
for (const sh of charter.shards) for (const s of sh.log_sections) coveredSecs.add(s)
const missing = logSections.filter(s => !coveredSecs.has(s.id))
for (const m of missing) {
  charter.shards.push({ shard_id: 'auto-' + m.id, log_sections: [m.id], log_entry_split: null, spec_sections: (m.refs || []).map(r => r.section), risk_note: 'AUTO-ADDED by script: Director charter missed this section', seed_questions: [] })
  log('Charter gap: section ' + m.id + ' auto-sharded')
}
log('Charter: ' + charter.shards.length + ' shards, ' + charter.cross_pairs.length + ' pairs, ' + charter.clusters.length + ' clusters, ' + charter.scenario_seeds.length + ' scenarios')

// build concrete shard briefs with line ranges + budget check
function shardBrief(sh) {
  const secs = sh.log_sections.map(id => secOf(id)).filter(Boolean)
  let logRanges = secs.map(s => ({ from: s.line_start, to: s.line_end, id: s.id }))
  let splitNote = ''
  if (sh.log_entry_split) splitNote = 'NOTE: you own ONLY entries ' + sh.log_entry_split + ' of that section (locate them by entry numbers inside the section range).'
  const sranges = specRangesFor(sh.spec_sections)
  let logLineCount = logRanges.reduce((a, r) => a + (r.to - r.from + 1), 0)
  let specLineCount = sranges.reduce((a, r) => a + (r.to - r.from + 1), 0)
  let est = estTokens(logLineCount, specLineCount)
  let trimmed = []
  while (est > 100000 && sranges.length > 1) {
    trimmed.push(sranges.pop())
    specLineCount = sranges.reduce((a, r) => a + (r.to - r.from + 1), 0)
    est = estTokens(logLineCount, specLineCount)
  }
  return {
    text: 'LOG ranges (read fully): ' + logRanges.map(r => LOG.replace(ROOT + '/', '') + ':' + r.from + '-' + r.to + ' (section ' + r.id + ')').join(', ') + '. ' + splitNote + '\nSPEC ranges (read fully, leaf-level): ' + (sranges.map(r => 'SPEC.md:' + r.from + '-' + r.to + ' (sec ' + r.num + ', ' + r.note + ')').join(', ') || 'none cited') + (trimmed.length ? '\nTRIMMED for context budget (attest as unread; they may become deepening charters): ' + trimmed.map(r => r.num).join(', ') : ''),
    est,
  }
}

// ============ FLEETS (concurrent families) ============
const familyErrors = []

// ---- FAMILY A: claims index ----
const claimsWork = (async () => {
  try {
    const NORMALIZER_COUNT = 12
    const totalRange = { from: logSections[0].line_start, to: logSections[logSections.length - 1].line_end }
    const chunk = Math.ceil((totalRange.to - totalRange.from + 1) / NORMALIZER_COUNT)
    const ranges = []
    for (let i = 0; i < NORMALIZER_COUNT; i++) {
      const from = totalRange.from + i * chunk
      const to = Math.min(totalRange.to, from + chunk - 1)
      if (from <= to) ranges.push({ from, to })
    }
    const normRes = await parallel(ranges.map((r, i) => () => run('normalize-' + (i + 1), 'Claims index', `${P}

ROLE: claims normalizer (read-only). Read ${LOG} lines ${r.from}-${r.to} COMPLETELY (multiple offset Reads as needed; ranges over 2000 lines require it). For EVERY decision entry line (format 'NNN-M. sentence (SECTION-REF)') in your range, emit one claim tuple:
- log_id: the NNN-M id.
- subjects: 1-3 canonical subjects — the construct(s)/term(s) the rule constrains. Conventions (mandatory, they make cross-agent collision detection work): lowercase; singular; the term as the LOG itself names it (e.g. 'borrow', 'keyword', 'visibility', 'use statement', 'interpretation closure', 'signal', 'cell', 'placement', 'view', 'handle', 'stream', 'effect', 'hot reload'); strip adjectives unless load-bearing.
- predicate: the constraint in <= 20 words, plain language.
- modality: must | must_not | may | is | defines.
Entries in your range that are partially cut off at range boundaries: include them if the entry LINE starts in your range. Emit every entry — no skipping, no judging. Attest coverage.`, { model: 'opus', effort: 'medium', schema: CLAIMS_SCHEMA })))
    const claims = []
    for (const r of normRes) if (r && r.claims) claims.push(...r.claims)
    log('Claims index: ' + claims.length + ' tuples normalized')
    // deterministic merge by subject (Map, not {}: model-generated subjects like
    // 'constructor' collide with Object.prototype and crashed the first run)
    const bySubject = new Map()
    for (const c of claims) for (const s of (c.subjects || [])) {
      const k = s.toLowerCase().trim()
      if (!bySubject.has(k)) bySubject.set(k, [])
      bySubject.get(k).push(c)
    }
    const groups = []
    for (const [k, g] of bySubject) {
      const sections = new Set(g.map(c => c.log_id.slice(0, 3)))
      const preds = new Set(g.map(c => c.modality + '|' + c.predicate.toLowerCase()))
      if (sections.size >= 2 && preds.size >= 2) groups.push({ subject: k, claims: g })
    }
    groups.sort((a, b) => b.claims.length - a.claims.length)
    log('Claims index: ' + groups.length + ' cross-section collision groups')
    // adjudicate in batches
    const batches = []
    let cur = [], curClaims = 0
    for (const g of groups) {
      const gc = Math.min(g.claims.length, 60)
      if (cur.length >= 10 || curClaims + gc > 70) { if (cur.length) batches.push(cur); cur = []; curClaims = 0 }
      cur.push(g); curClaims += gc
    }
    if (cur.length) batches.push(cur)
    const adjRes = await parallel(batches.map((batch, i) => () => run('collide-' + (i + 1), 'Claims index', `${P}

ROLE: collision adjudicator (read-only). Below are groups of normalized claims that share a subject across DIFFERENT LOG sections with differing predicates. LOG Invariant 2 forces rules to be RESTATED across sections instead of cross-referenced — so restatements that drifted apart (especially after recent amendments) are exactly what you hunt. For each group: Read the actual LOG entries by their ids (grep '^NNN-M\\.' in ${LOG} to locate, then Read the lines). Judge: consistent (same rule, compatible wordings) / CONFLICT (the entries disagree on semantics) / drift (restatements that no longer say the same thing — a defect even if reconcilable). Report every CONFLICT and drift as a finding (category 'contradiction'; for restatement drift explain that in why_defect). Consistent groups: no finding. Anchor findings with ALL involved log_ids and quote each entry verbatim.

GROUPS:
${batch.map(g => 'SUBJECT "' + g.subject + '":\n' + g.claims.slice(0, 60).map(c => '  ' + c.log_id + ' [' + c.modality + '] ' + c.predicate).join('\n')).join('\n\n')}`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA })))
    adjRes.forEach((r, i) => collect('claims-index', 'collide-' + (i + 1), r))
    // scenario candidates: groups spanning 3+ sections
    const multi = groups.filter(g => new Set(g.claims.map(c => c.log_id.slice(0, 3))).size >= 3).slice(0, 6)
    return { collisionScenarios: multi.map(g => ({ title: 'collision:' + g.subject, scenario: 'Subject "' + g.subject + '" is constrained by 3+ sections: ' + g.claims.slice(0, 25).map(c => c.log_id).join(', ') + '. Construct a concrete program scenario exercising these rules jointly; check joint satisfiability.', touching_sections: [...new Set(g.claims.map(c => c.log_id.slice(0, 3)))] })) }
  } catch (e) { familyErrors.push('claims-index: ' + String(e)); return { collisionScenarios: [] } }
})()

// ---- FAMILY B: shard audits ----
const shardWork = (async () => {
  try {
    await pipeline(charter.shards, (sh) => {
      const brief = shardBrief(sh)
      return run('shard-' + sh.shard_id, 'Shard audits', `${P}

ROLE: shard deep auditor (read-only, adversarial mindset per the repo's SUBAGENT_PROTOCOL_ADVERSARIAL.md stance — skeptical, cold, exhaustive within scope).
SCOPE:
${brief.text}
RISK NOTE from the Audit Director: ${sh.risk_note}
SEED QUESTIONS (investigate each, but do not stop at them):
${(sh.seed_questions || []).map(q => '- ' + q).join('\n') || '- none'}

HUNT (all of these, within your scope):
1. contradiction: entries in your scope that disagree with each other.
2. divergence: a LOG entry vs the SPEC section it cites — SPEC must conform to LOG; ALSO citation-aptness: the claim is true but the cited section does not actually elaborate it (pointer contract broken).
3. ambiguity: two careful readings yield different concrete program behavior. Show both readings.
4. vague_term / undefined_term: load-bearing words with no pinned meaning at point of use.
5. gap: a case an implementer must handle that no rule in scope covers, with no legal boundary (only std-delegation or implementation-defined are legal stopping points per the LOG's identity rules).
6. logical_flaw: non-sequiturs, circular definitions, rules that cannot mean what they say.
7. invariant_violation: an entry referencing another entry by number (Invariant 2 forbids).
8. unfalsifiable: a rule for which no conformance test could be constructed.
For categories 1-3: quote BOTH sides verbatim. Every finding needs anchors (log_ids, spec_sections), evidence quotes with file:line, why_defect, direction_of_change (never edit text). Cap 40; overflow goes to notes_for_orchestrator with ranges.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('shard', 'shard-' + sh.shard_id, r); return r })
    })
  } catch (e) { familyErrors.push('shards: ' + String(e)) }
})()

// ---- FAMILY C: sweeps & probes ----
const sweepsWork = (async () => {
  try {
    const tasks = []
    // cross-pair hunters
    for (const p of charter.cross_pairs) {
      const sa = secOf(p.a), sb = secOf(p.b)
      if (!sa || !sb) continue
      tasks.push(() => run('pair-' + p.a + 'x' + p.b, 'Sweeps & probes', `${P}

ROLE: cross-pair contradiction hunter (read-only). Read ${LOG} sections ${p.a} (lines ${sa.line_start}-${sa.line_end}) and ${p.b} (lines ${sb.line_start}-${sb.line_end}) fully. Director's reason for this pairing: ${p.why}
Seed questions: ${(p.seed_questions || []).join(' | ') || 'none'}
Hunt SEMANTIC conflicts between the two sections — cases where different vocabulary hides the same subject, or where rules compose into impossibility or surprise. Where a suspected conflict hinges on SPEC elaboration, follow the specific cited sections into SPEC.md (Read those leaf ranges only). Findings need both sides quoted verbatim.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('cross-pair', 'pair-' + p.a + 'x' + p.b, r); return r }))
    }
    // terminology residue
    const termItems = [...((mapTerms && mapTerms.undefined_candidates) || []).map(t => ({ kind: 'undefined?', term: t.term, loc: (t.sample_locations || []).join(', ') })), ...((mapTerms && mapTerms.alias_suspects) || []).map(t => ({ kind: 'alias?', term: (t.terms || []).join(' ~ '), loc: t.note || '' })), ...((charter.term_seeds || []).map(t => ({ kind: 'seed', term: t, loc: '' })))]
    const termBatches = []
    for (let i = 0; i < termItems.length; i += 25) termBatches.push(termItems.slice(i, i + 25))
    termBatches.forEach((batch, i) => tasks.push(() => run('terms-' + (i + 1), 'Sweeps & probes', `${P}

ROLE: terminology adjudicator (read-only). For each term below: grep BOTH docs for it; decide (a) defined (cite the defining decision/section — a definition must be an actual definitional statement, not usage), (b) undefined but load-bearing (finding: undefined_term), (c) meaning drifts between uses (finding: vague_term; quote conflicting uses), (d) alias cluster — multiple terms, one concept (finding: vague_term; quote both) or genuinely distinct concepts (no finding, but say so in notes). Per LEARNINGS 14: usage is not definition.
TERMS:
${batch.map(t => '- [' + t.kind + '] ' + t.term + (t.loc ? ' @ ' + t.loc : '')).join('\n')}`, { model: 'opus', effort: 'medium', schema: FINDINGS_SCHEMA }).then(r => { collect('terminology', 'terms-' + (i + 1), r); return r })))
    // smell + implementer panels per cluster
    for (const cl of charter.clusters) {
      const secs = cl.log_sections.map(id => secOf(id)).filter(Boolean)
      const rangeTxt = secs.map(s => 'DECISION_LOG.md:' + s.line_start + '-' + s.line_end + ' (sec ' + s.id + ' ' + s.title + ')').join(', ')
      tasks.push(() => run('smell-' + cl.name.replace(/[^a-z0-9]+/gi, '-').toLowerCase(), 'Sweeps & probes', `${P}

ROLE: design-smell reviewer (read-only), subsystem cluster "${cl.name}" — focus: ${cl.focus}. Read ${rangeTxt} fully. FIRST read the LOG's Identity & Philosophy section (001, ${secText('001')}): those are the commitments the language sets for itself; they are your ONLY admissible quality standard.
ADMISSIBILITY RULE (hard): a design_smell finding is admissible ONLY if (i) it cites a specific 001-x commitment (or another LOG-stated design value) the design violates or tensions, OR (ii) it exhibits a concrete footgun witness: a plausible Ductus program snippet + the surprising behavior + the quoted rules producing it. Aesthetic disagreement with a deliberate, documented decision is NOT a finding — emit it as category 'observation' (those go to an appendix, unverified). Hunt: irregularity (same concept, different rules in different places), special cases that break composability, footguns, rules fighting the section-001 philosophy.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('smell', 'smell-' + cl.name, r); return r }))
      tasks.push(() => run('impl-' + cl.name.replace(/[^a-z0-9]+/gi, '-').toLowerCase(), 'Sweeps & probes', `${P}

ROLE: implementer-completeness auditor (read-only), subsystem cluster "${cl.name}" — focus: ${cl.focus}. Read ${rangeTxt} fully. Stance: you are about to implement this subsystem. Hunt gaps: error paths, evaluation/commit ordering, initialization and teardown, lifecycle across hot-reload, boundary interactions, resource behavior — every question you would have to ask before writing the code, for which no rule in the corpus answers and no legal boundary (std-delegation or implementation-defined) is declared. Grep the wider corpus before declaring a gap: the answer may live in another section (cite it in notes instead of a finding). ALSO: for rules in scope that look untestable — state the conformance test you would write; if none is constructible, category 'unfalsifiable'.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('implementer', 'impl-' + cl.name, r); return r }))
    }
    // parser adversaries
    tasks.push(() => run('parser-lexical', 'Sweeps & probes', `${P}

ROLE: parser adversary, lexical (read-only). The language has NO grammar document; syntax lives in prose. ${ID_DRIFT_NOTE}
Read the LOG's Source Form & Lexical Rules section (002, ${secText('002')}) fully plus every lexically-relevant rule you can grep (identifier rules, '#' in identifiers, single-token forms like 'handle!', comment form, indentation, layout suspension). Construct minimal source fragments where two lexical rules both claim the characters or where tokenization is undecidable from the stated rules. Category parse_ambiguity; the evidence quotes are the RULES that conflict; put the witness fragment in why_defect. The LOG carries 'takes precedence over any conflicting grammar' scars — the class is live.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('parser', 'parser-lexical', r); return r }))
    tasks.push(() => run('parser-structure', 'Sweeps & probes', `${P}

ROLE: parser adversary, block structure (read-only). ${ID_DRIFT_NOTE}
Read the LOG sections on lexical rules (002, ${secText('002')}), calls (006, ${secText('006')}), iteration (014, ${secText('014')}), and any collect/fold-form sections (034/035 if present: ${secText('034')} / ${secText('035')}), and grep for syntax-bearing rules elsewhere ('as', 'else', 'otherwise', 'default', inline-vs-block bodies). Focus on overloaded constructs: 'as' with many roles; multiple 'else' senses; keywords doubling as field-likes; expression forms with clause arms. Construct minimal programs with two structurally valid parses under the stated prose rules. Category parse_ambiguity; quote the conflicting rules; witness in why_defect.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('parser', 'parser-structure', r); return r }))
    // witness probes from Director seeds
    for (const sc of charter.scenario_seeds) {
      tasks.push(() => run('witness-' + sc.title.replace(/[^a-z0-9]+/gi, '-').toLowerCase().slice(0, 40), 'Sweeps & probes', `${P}

ROLE: witness prober (read-only, constructive). SCENARIO: ${sc.scenario}
Touching sections: ${(sc.touching_sections || []).join(', ')} (locate their line ranges by grepping '^## ' headers in ${LOG}).
Method: (1) write the concrete Ductus program sketch for the scenario; (2) gather EVERY rule that touches it — read the touching sections' relevant entries and follow their cited SPEC leaf sections; (3) determine what the rules jointly require. Report: contradiction (rules disagree), unsatisfiable (jointly impossible but nothing forbids the construct), gap (construct is legal, behavior unspecified), or clean (say so in notes — a clean witness is valuable). Quote every rule you rely on verbatim.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('witness', 'witness-' + sc.title, r); return r }))
    }
    // fence linter
    const fences = (mapFences && mapFences.fences) || []
    const fenceBatches = []
    for (let i = 0; i < fences.length; i += 80) fenceBatches.push(fences.slice(i, i + 80))
    fenceBatches.forEach((batch, i) => tasks.push(() => run('fences-' + (i + 1), 'Sweeps & probes', `${P}

ROLE: example-conformance linter (read-only). ${ID_DRIFT_NOTE}
Below are code fences in ${SPEC}. For each fence whose language is Ductus source (info string 'ductus' or empty-with-Ductus-looking-content; skip obviously non-Ductus fences like shell/json but attest which you skipped): Read it and lint against this charter (quote the violated rule from the LOG as evidence — grep it fresh; ids are pointers as of 2026-07):
- 002-16: ';' anywhere is a lex error. 002-22: tab in leading whitespace is a lex error. 002-2: keywords always lowercase. 002-19: only '//' line comments, no block comments. 002-21: identifiers cannot begin with a digit. 002-25: trait/type/enum/node/connection bodies are always indented blocks, never inline. 001-25: a borrow cannot be stored in a record field, tuple component, enum payload, or indexed slot. 002-10: 'as' is not a cast; conversion is T(value).
Also flag examples that plainly contradict a rule stated in their own surrounding section (read a few lines around the fence for context). Category stale_example. Anchor spec_sections by nearest enclosing numbered heading.
FENCES (SPEC.md line ranges): ${batch.map(f => f.line_start + '-' + f.line_end + '(' + (f.lang || '?') + ')').join(', ')}`, { model: 'opus', effort: 'medium', schema: FINDINGS_SCHEMA }).then(r => { collect('fences', 'fences-' + (i + 1), r); return r })))
    // reverse-losslessness: churn zones + orphan subtrees
    const rlZones = []
    for (const c of ((mapContext && mapContext.churn_zones) || [])) if ((c.spec_sections || []).length) rlZones.push({ name: 'churn:' + c.description.slice(0, 40), ranges: specRangesFor(c.spec_sections) })
    const orphanRanges = maximalOrphans.map(o => ({ num: o.num, from: o.line, to: o.subtree_end }))
    for (let i = 0; i < orphanRanges.length; i += 12) rlZones.push({ name: 'orphans-' + (i / 12 + 1), ranges: orphanRanges.slice(i, i + 12).map(r => ({ num: r.num, from: r.from, to: r.to, note: 'orphan subtree (no LOG entry cites it or its ancestors/descendants)' })) })
    rlZones.slice(0, 10).forEach((z, i) => tasks.push(() => run('lossless-' + (i + 1), 'Sweeps & probes', `${P}

ROLE: reverse-losslessness auditor (read-only). The LOG claims to be the lossless WHAT: 'SPEC.md must conform to this log'. SPEC content making normative commitments that NO LOG entry states is a defect (category spec_only_normative). Zone "${z.name}". Read these SPEC.md ranges fully: ${z.ranges.map(r => r.from + '-' + r.to + ' (sec ' + r.num + (r.note ? ', ' + r.note : '') + ')').join(', ')}.
Method: extract sentences with normative force (must, must not, is a compile error, traps, is rejected, is forbidden, shall, always, never — in normative position, not narration). For each, grep ${LOG} for an entry entailing it (search by the construct's key terms, not exact wording). Entailed: no finding. Not entailed: finding with the SPEC sentence quoted + the searches you ran. Batch related sentences into one finding per topic. If a zone is an orphan subtree, ALSO judge whether the whole subtree is normative-but-unindexed vs purely explanatory (one summary finding if the former).`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA }).then(r => { collect('losslessness', 'lossless-' + (i + 1), r); return r })))
    await parallel(tasks)
  } catch (e) { familyErrors.push('sweeps: ' + String(e)) }
})()

// ---- FAMILY D: gates ----
const gatesWork = (async () => {
  try {
    const gatePrompt = (kind, body) => `${P}

ROLE: mechanical gate '${kind}' (read-only, per the repo's SUBAGENT_PROTOCOL_GATE.md: one gate = one check; verify, cite, never fix; no speculation). ${ID_DRIFT_NOTE} Findings use category 'invariant_violation' unless told otherwise; severity LOW unless the defect changes meaning (then MED).
${body}`
    const gateTasks = [
      () => run('gate-count', 'Gates', gatePrompt('count', `For ALL ${SECTION_COUNT} LOG sections: compare the header's declared '— N Rules' against the actual entry count (grep -c '^NNN-' within the section's line range) AND verify dense numbering: entry numbers within each section run 1..N with no gaps and no duplicates (grep -o the 'NNN-M.' prefixes; check the sequence). Section ranges: ${logSections.map(s => s.id + ':' + s.line_start + '-' + s.line_end).join(' ')}. Report per-section drift only where nonzero.`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-atomic', 'Gates', gatePrompt('atomic', `LOG Invariant 2: entries never reference other entries by number. Grep ${LOG} for entry-id patterns appearing INSIDE entry text (pattern like [0-9]{3}-[0-9]+ occurring anywhere other than at line start as the entry's own id). Classify each hit: own-id (fine) vs cross-reference (violation). Quote violators.`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-xref', 'Gates', gatePrompt('xref', `Every LOG '(SECTION-REF)' must resolve to a real SPEC heading. The unique cited sections are: ${[...citedSet].sort().join(', ')}. The SPEC heading numbers are known-good; here they are: ${specTree.map(s => s.num).join(', ')}. Compute the set difference yourself (cited but not a heading = dangling). For each dangling ref: grep ${LOG} for which entries cite it, quote them. Also spot-check 10 random resolvable refs for aptness (does the section topic plausibly match the entry?) — report blatant mismatches only.`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-direction', 'Gates', gatePrompt('direction', `LOG Invariant 3: SPEC never references LOG entries. Grep ${SPEC} for LOG-entry-id patterns ([0-9]{3}-[0-9]+ that look like decision ids) and for the phrase 'DECISION_LOG'. Classify hits (a code-fence numeric literal is not a violation; a prose reference to a decision id is). Quote violators.`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-neardupe', 'Gates', gatePrompt('near-duplicate', `Hunt near-duplicate LOG entries WITHIN and ACROSS sections: same rule stated 2+ times in variant wordings where the LOG's atomicity design did not require restatement — decision decay. Method: grep for distinctive shared phrases ('single authoritative source', 'no separate grammar', 'takes precedence over any conflicting grammar', and others you derive from reading the identity section ${secText('001')}). Category design_smell, severity LOW, unless the duplicates DISAGREE (then contradiction/MED). Quote all copies.`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-deferred', 'Gates', gatePrompt('deferred-language', `The LOG's identity rules claim NO postponed design decisions — every boundary is explicit (std-delegation or implementation-defined). Grep BOTH docs for deferral language: 'TBD', 'TODO', 'for now', 'will be specified', 'to be determined', 'later revision', 'future work', 'not yet', 'postponed', 'deferred'. Classify each hit: genuine deferral in normative content (finding, category gap, severity HIGH per the spec's own standard — quote the no-postponed-decisions identity rules alongside) vs legitimate (explicit implementation-defined boundary, historical narration, example text).`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-keywords', 'Gates', gatePrompt('keyword-closure', `The lexical section (002, ${secText('002')}) enumerates CLOSED keyword sets (declaration, clause, control-flow, operator-context). Read those entries, extract the complete lists. Then hunt the corpus for tokens used in keyword position but absent from every list: grep both docs for backticked lowercase words presented as keywords/clauses (patterns like 'the X keyword', 'X clause', 'X block') and diff against the lists. Category invariant_violation, severity MED (a keyword outside the closed set breaks the every-keyword-is-reserved rule).`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
      () => run('gate-fence-lex', 'Gates', gatePrompt('fence-lexer', `Mechanical lex check on ${SPEC} code fences: (1) grep -n ';' inside fence line ranges; (2) grep -n for literal tab characters in fence leading whitespace. Fence ranges: ${(((mapFences && mapFences.fences) || []).map(f => f.line_start + '-' + f.line_end).join(',')) || 'none found'}. A ';' in Ductus source is a lex error; a leading tab is a lex error (quote the current lexical rules as evidence). Skip fences whose info string marks a non-Ductus language — attest which. Category stale_example.`), { model: 'opus', effort: 'low', schema: FINDINGS_SCHEMA }),
    ]
    const gateRes = await parallel(gateTasks)
    const gateNames = ['gate-count', 'gate-atomic', 'gate-xref', 'gate-direction', 'gate-neardupe', 'gate-deferred', 'gate-keywords', 'gate-fence-lex']
    gateRes.forEach((r, i) => collect('gate', gateNames[i], r))
  } catch (e) { familyErrors.push('gates: ' + String(e)) }
})()

const [claimsOut] = await Promise.all([claimsWork, shardWork, sweepsWork, gatesWork])

// collision-derived witness probes (after claims merge)
phase('Sweeps & probes')
if (claimsOut.collisionScenarios.length) {
  const colRes = await parallel(claimsOut.collisionScenarios.map((sc, i) => () => run('witness-col-' + (i + 1), 'Sweeps & probes', `${P}

ROLE: witness prober (read-only, constructive). SCENARIO (derived from a claims-index subject spanning 3+ sections): ${sc.scenario}
Method: (1) Read the listed entries (grep their ids in ${LOG}); (2) construct a concrete Ductus program sketch exercising them jointly; (3) follow cited SPEC leaf sections where the LOG is thin. Report contradiction / unsatisfiable / gap / clean-with-notes. Quote every rule relied on.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA })))
  colRes.forEach((r, i) => collect('witness', 'witness-col-' + (i + 1), r))
}
log('Fleet complete: ' + allFindings.length + ' raw findings' + (familyErrors.length ? ' | family errors: ' + familyErrors.join('; ') : ''))

// ============ PHASE: TRIAGE ============
phase('Triage')
function bucketize(findings) {
  const parent = {}
  const find = x => { while (parent[x] !== x) { parent[x] = parent[parent[x]]; x = parent[x] } return x }
  const union = (a, b) => { const ra = find(a), rb = find(b); if (ra !== rb) parent[rb] = ra }
  for (const f of findings) parent[f.finding_id] = f.finding_id
  const byAnchor = {}
  for (const f of findings) {
    const anchors = [...(f.log_ids || []).map(x => 'L:' + x), ...(f.spec_sections || []).map(x => 'S:' + x)]
    for (const a of anchors) { if (byAnchor[a]) union(byAnchor[a], f.finding_id); else byAnchor[a] = f.finding_id }
  }
  const buckets = {}
  for (const f of findings) { const r = find(f.finding_id); if (!buckets[r]) buckets[r] = []; buckets[r].push(f) }
  return Object.values(buckets)
}
const verifiable = allFindings.filter(f => f.category !== 'observation')
const observations = allFindings.filter(f => f.category === 'observation')
const buckets = bucketize(verifiable)
const multiBuckets = buckets.filter(b => b.length > 1)
log('Triage: ' + verifiable.length + ' verifiable findings in ' + buckets.length + ' anchor buckets (' + multiBuckets.length + ' multi-member), ' + observations.length + ' observations')

const findingById = {}
for (const f of allFindings) findingById[f.finding_id] = f
const clusters = []
const dedupBatches = []
{ let cur = [], n = 0
  for (const b of multiBuckets) { if (cur.length >= 8 || n + b.length > 30) { if (cur.length) dedupBatches.push(cur); cur = []; n = 0 } cur.push(b); n += b.length }
  if (cur.length) dedupBatches.push(cur) }
const dedupRes = await parallel(dedupBatches.map((batch, i) => () => run('dedup-' + (i + 1), 'Triage', `${P}

ROLE: dedup adjudicator. Below are buckets of findings sharing anchors. Within each bucket, cluster TRUE duplicates (same defect, possibly different wording/category labels) — cluster, never drop: every member id stays listed; distinct defects sharing an anchor stay separate clusters (a cluster of one is fine). For each cluster output: representative_id (the best-evidenced member), member_ids (all, including representative), merged_one_line (the sharpest statement), severity (the rubric-correct one for the merged defect), category (the truest one). Do NOT judge validity — only sameness.

${batch.map((b, j) => 'BUCKET ' + (j + 1) + ':\n' + b.map(f => '  ' + digestOf(f) + '\n    why: ' + String(f.why_defect).slice(0, 300)).join('\n')).join('\n\n')}`, { model: 'opus', effort: 'medium', schema: DEDUP_SCHEMA })))
const clusteredIds = new Set()
for (const r of dedupRes) if (r && r.clusters) for (const c of r.clusters) { clusters.push(c); for (const m of c.member_ids) clusteredIds.add(m) }
for (const b of buckets) for (const f of b) if (!clusteredIds.has(f.finding_id)) clusters.push({ representative_id: f.finding_id, member_ids: [f.finding_id], merged_one_line: f.one_line, severity: f.severity, category: f.category })
log('Triage: ' + clusters.length + ' unique defect clusters after dedup')

const ledgerSummaryFn = () => ledger.map(l => l.label + ':' + l.status + (l.halt_reason ? '(' + l.halt_reason + ')' : '')).join(', ')
function clusterDigest(c) {
  const rep = findingById[c.representative_id] || {}
  return c.representative_id + ' [' + c.severity + '/' + c.category + '] x' + c.member_ids.length + ' anchors(' + ((rep.log_ids || []).slice(0, 6).join(',')) + ';' + ((rep.spec_sections || []).slice(0, 4).join(',')) + ') src:' + (rep.family || '?') + ' — ' + c.merged_one_line
}
function directorTriagePrompt(roundLabel, extra) {
  return `${P}

${DIRECTOR_ROLE}

THIS CALL: triage (${roundLabel}). Your prior memo:
${JSON.stringify(charter.memo)}

CLUSTER DIGESTS (${clusters.length} unique defects; full records exist and go to verification as-is — you rank and route):
${clusters.map(clusterDigest).join('\n')}

KNOWN-OPEN REGISTER:
${registerText}

AGENT LEDGER (dead/halted agents = coverage holes to consider for deepening):
${ledgerSummaryFn()}

FAMILY ERRORS: ${familyErrors.join('; ') || 'none'}
${extra}

DELIVER:
1. severity_overrides: clusters whose severity violates the rubric (cite why). Only where the digest makes it clear-cut; do not guess.
2. known_open_tags: clusters matching a register entry AT CLAIM LEVEL — same defect (same rule ids or same behavioral question), register entry quoted. Adjacency or same-topic is NOT a match. Tagged items still get verified.
3. drop_as_duplicate: clusters that are duplicates of another cluster the dedup pass missed.
4. deepening_charters (max 14): follow-up probes for hot zones, coverage holes (dead/halted agents, trimmed SPEC ranges, overflow notes), and suspicious silence (a churned section with zero findings deserves a second look). Empty array + done_deepening=true if nothing warrants it.
5. memo: updated threaded state.`
}
let triage = await run('director-triage', 'Triage', directorTriagePrompt('round 1', ''), { model: 'fable', effort: 'xhigh', schema: TRIAGE_SCHEMA })
if (!triage) { triage = { severity_overrides: [], known_open_tags: [], drop_as_duplicate: [], deepening_charters: [], memo: charter.memo, done_deepening: true }; log('Director triage DEAD — proceeding with mechanical defaults') }
function applyTriage(t) {
  for (const o of (t.severity_overrides || [])) { const c = clusters.find(x => x.representative_id === o.finding_id); if (c) c.severity = o.severity }
  for (const d of (t.drop_as_duplicate || [])) { const c = clusters.find(x => x.representative_id === d.finding_id); const into = clusters.find(x => x.representative_id === d.duplicate_of); if (c && into && c !== into) { into.member_ids.push(...c.member_ids); c.dropped = true } }
  for (const k of (t.known_open_tags || [])) { const c = clusters.find(x => x.representative_id === k.finding_id); if (c) { c.known_open = k.register_key; c.known_open_quote = k.register_quote } }
}
applyTriage(triage)

// ============ PHASE: DEEPENING ============
phase('Deepening')
const criticRes = await run('completeness-critic', 'Deepening', `${P}

ROLE: completeness critic (read-only, adversarial). The audit so far: families = claims-index (all entries normalized, subject collisions adjudicated), ${charter.shards.length} shard deep-audits (all ${SECTION_COUNT} LOG sections + cited SPEC leaf ranges), ${charter.cross_pairs.length} cross-pair hunts, terminology sweep (mechanical inventory + adjudication), design-smell + implementer panels over ${charter.clusters.length} clusters, 2 parser adversaries, ${charter.scenario_seeds.length + (claimsOut.collisionScenarios || []).length} witness probes, fence linting, reverse-losslessness over churn+orphan zones, 8 mechanical gates. Ledger: ${ledgerSummaryFn()}.
Question: what did this audit NOT cover? Think in defect classes and in corpus regions. Check the corpus yourself where needed (sample, grep). For each real gap: why the current mechanisms miss it + a concrete charter (title, question, ranges, lens). Also list honest_disclaimers the final report must carry even after deepening. Do not pad — a short list of real gaps beats a long list of theoretical ones.`, { model: 'opus', effort: 'high', schema: CRITIC_SCHEMA })
let round = 0
let charters = [...(triage.deepening_charters || []), ...(((criticRes && criticRes.gaps) || []).map(g => g.suggested_charter))].slice(0, 14)
while (round < 2 && charters.length && !(round > 0 && triage.done_deepening)) {
  round++
  log('Deepening round ' + round + ': ' + charters.length + ' charters')
  const deepRes = await parallel(charters.map((c, i) => () => run('deep-r' + round + '-' + (i + 1), 'Deepening', `${P}

ROLE: deepening auditor (read-only), chartered by the Audit Director. TITLE: ${c.title}
QUESTION: ${c.question}
LENS: ${c.lens}
SCOPE: LOG ranges ${(c.log_ranges || []).join(', ') || 'locate by grepping the relevant section headers'}; SPEC sections ${(c.spec_sections || []).join(', ') || 'follow citations'} (leaf ranges; >2000-line ranges need multiple offset Reads).
Full defect-hunt discipline as the audit mandates: all categories admissible, verbatim evidence, anchors, direction-of-change only.`, { model: 'opus', effort: 'high', schema: FINDINGS_SCHEMA })))
  const before = allFindings.length
  deepRes.forEach((r, i) => collect('deepening', 'deep-r' + round + '-' + (i + 1), r))
  const fresh = allFindings.slice(before).filter(f => f.category !== 'observation')
  for (const f of fresh) clusters.push({ representative_id: f.finding_id, member_ids: [f.finding_id], merged_one_line: f.one_line, severity: f.severity, category: f.category })
  if (!fresh.length) { log('Deepening round ' + round + ' dry'); break }
  triage = await run('director-triage-r' + (round + 1), 'Deepening', directorTriagePrompt('after deepening round ' + round, '\nNEW this round: ' + fresh.map(f => digestOf(f)).join('\n')), { model: 'fable', effort: 'high', schema: TRIAGE_SCHEMA })
  if (!triage) break
  applyTriage(triage)
  charters = (triage.deepening_charters || []).slice(0, 14)
  if (triage.done_deepening) break
}

// ============ PHASE: VERIFICATION ============
phase('Verification')
const liveClusters = clusters.filter(c => !c.dropped)
const himed = liveClusters.filter(c => c.severity === 'HIGH' || c.severity === 'MED')
const lows = liveClusters.filter(c => c.severity === 'LOW')
log('Verification: ' + himed.length + ' HIGH/MED clusters (dual-lens), ' + lows.length + ' LOW (evidence-only)')
function fullRecord(c) {
  const members = c.member_ids.map(id => findingById[id]).filter(Boolean)
  return 'CLUSTER ' + c.representative_id + ' [' + c.severity + '/' + c.category + ']: ' + c.merged_one_line + '\n' + members.map(m => '  member ' + m.finding_id + ' (src ' + m.source + '): ' + m.one_line + '\n  why: ' + m.why_defect + '\n  anchors: ' + (m.log_ids || []).join(',') + ' ; ' + (m.spec_sections || []).join(',') + '\n  evidence:\n' + fmtEvidence(m)).join('\n')
}
function sortKey(c) { const rep = findingById[c.representative_id] || {}; return ((rep.log_ids && rep.log_ids[0]) || (rep.spec_sections && 'z' + rep.spec_sections[0]) || 'zz') }
himed.sort((a, b) => sortKey(a) < sortKey(b) ? -1 : 1)
lows.sort((a, b) => sortKey(a) < sortKey(b) ? -1 : 1)
const CHARITY_STANDARDS = `CATEGORY-CONDITIONAL STANDARDS (mandatory):
- ambiguity / vague_term: the EXISTENCE of a plausible second reading IS the defect. You may refute ONLY by producing a reading FORCED by quoted normative text (quote it) that eliminates the alternative — 'a careful reader can infer' is a SUSTAIN (the inference-required-ness is the finding).
- gap: refute ONLY by citing explicit normative text covering the case (quote it), or an explicit legal boundary (std-delegation or implementation-defined). Inference or 'obviously intended' is a SUSTAIN.
- ALL categories: if you find a passage that dissolves the finding, you MUST check the dissolving passage is consistent with the finding's quoted passage. If they conflict, verdict is refile_divergence (the finding mutates into a divergence between the two passages), never refute.
- Bias check: you are NOT rewarded for refuting. A wrong refutation buries a real defect; a wrong sustain costs one more review. When genuinely torn, sustain and say why in the crux.`
const himedBatches = []
for (let i = 0; i < himed.length; i += 8) himedBatches.push(himed.slice(i, i + 8))
const verdicts = {}
await pipeline(himedBatches, (batch, _orig, bi) => parallel([
  () => run('v-evidence-' + (bi + 1), 'Verification', `${P}

ROLE: evidence verifier (adversarial, per SUBAGENT_PROTOCOL_ADVERSARIAL.md: distrust everything, verify against disk). FIRST: check the corpus pin — git -C ${ROOT} rev-parse HEAD must equal ${PIN.sha} and wc -c must give ${PIN.log_bytes} (LOG) / ${PIN.spec_bytes} (SPEC); mismatch → pin_ok=false and halt.
For EACH cluster below, for EACH member's evidence citations: Read the cited lines fresh. (a) Does the verbatim quote exist at (or within 3 lines of) the citation? (b) Does the quoted text actually say what the finding claims it says? Verdict per cluster: pass (at least one member's evidence fully checks out AND supports the merged claim), partial (quotes exist but support is weaker/narrower than claimed — explain), fail (EVERY member's evidence is fabricated, miscited beyond 3 lines, or does not support the claim). A cluster fails only if ALL members fail.

${batch.map(fullRecord).join('\n\n')}`, { model: 'opus', effort: 'high', schema: VEVIDENCE_SCHEMA }),
  () => run('v-charity-' + (bi + 1), 'Verification', `${P}

ROLE: charitable-reading verifier (adversarial toward the FINDING). For each cluster below: hunt the WHOLE corpus (grep beyond the citations) for text that dissolves the defect — a definition the finder missed, a rule that settles the ambiguity, a boundary declaration covering the gap.
${CHARITY_STANDARDS}
Verdict per cluster: sustain / refute (dissolving_evidence REQUIRED: file:line + verbatim quote) / refile_divergence (dissolving passage conflicts with the finding's passage — quote both).

${batch.map(fullRecord).join('\n\n')}`, { model: 'opus', effort: 'high', schema: VCHARITY_SCHEMA }),
]).then(([ev, ch]) => {
  for (const c of batch) {
    const e = ev && ev.results ? ev.results.find(r => r.finding_id === c.representative_id) : null
    const h = ch && ch.results ? ch.results.find(r => r.finding_id === c.representative_id) : null
    verdicts[c.representative_id] = { evidence: e || { evidence_verdict: 'unverified', crux: ev === null ? 'verifier dead' : 'missing from verifier output' }, charity: h || { verdict: 'unverified', crux: ch === null ? 'verifier dead' : 'missing from verifier output' } }
  }
  return true
}))
const splitHighs = himed.filter(c => { const v = verdicts[c.representative_id]; return c.severity === 'HIGH' && v && v.evidence.evidence_verdict === 'pass' && v.charity.verdict === 'refute' })
if (splitHighs.length) {
  const tbBatches = []
  for (let i = 0; i < splitHighs.length; i += 6) tbBatches.push(splitHighs.slice(i, i + 6))
  const tbRes = await parallel(tbBatches.map((batch, i) => () => run('v-tiebreak-' + (i + 1), 'Verification', `${P}

ROLE: tiebreak verifier. Each HIGH cluster below passed evidence verification but was refuted by the charity verifier. Both cruxes are given. Read the disputed text yourself and rule: sustain or refute.
${CHARITY_STANDARDS}

${batch.map(c => fullRecord(c) + '\nEVIDENCE CRUX: ' + verdicts[c.representative_id].evidence.crux + '\nCHARITY REFUTATION: ' + verdicts[c.representative_id].charity.crux + ' | ' + (verdicts[c.representative_id].charity.dissolving_evidence || '')).join('\n\n')}`, { model: 'opus', effort: 'xhigh', schema: TIEBREAK_SCHEMA })))
  for (const r of tbRes) if (r && r.verdicts) for (const v of r.verdicts) { if (verdicts[v.finding_id]) verdicts[v.finding_id].tiebreak = v }
}
const lowBatches = []
for (let i = 0; i < lows.length; i += 20) lowBatches.push(lows.slice(i, i + 20))
await pipeline(lowBatches, (batch, _o, bi) => run('v-low-' + (bi + 1), 'Verification', `${P}

ROLE: evidence verifier for LOW-severity clusters (lighter pass: quotes + support only). Pin check first (HEAD ${PIN.sha}). For each cluster: verify at least one member's quotes exist as cited (within 3 lines) and support the claim. pass/partial/fail per cluster; fail only if all members fail.

${batch.map(c => fullRecord(c).slice(0, 2500)).join('\n\n')}`, { model: 'opus', effort: 'medium', schema: VEVIDENCE_SCHEMA }).then(ev => {
  for (const c of batch) {
    const e = ev && ev.results ? ev.results.find(r => r.finding_id === c.representative_id) : null
    verdicts[c.representative_id] = { evidence: e || { evidence_verdict: 'unverified', crux: ev === null ? 'verifier dead' : 'missing from output' }, charity: { verdict: 'not_run', crux: 'LOW severity: evidence-only per approved plan' } }
  }
  return true
}))
function finalVerdict(c) {
  const v = verdicts[c.representative_id]
  if (!v || v.evidence.evidence_verdict === 'unverified') return 'UNVERIFIED'
  if (v.evidence.evidence_verdict === 'fail') return 'REFUTED'
  if (v.charity.verdict === 'refile_divergence') { c.category = 'divergence'; return 'CONFIRMED' }
  if (v.charity.verdict === 'refute') {
    if (v.tiebreak) return v.tiebreak.verdict === 'sustain' ? 'CONFIRMED' : 'REFUTED'
    return 'PLAUSIBLE'
  }
  if (v.charity.verdict === 'not_run') return v.evidence.evidence_verdict === 'pass' ? 'CONFIRMED' : 'PLAUSIBLE'
  return v.evidence.evidence_verdict === 'pass' ? 'CONFIRMED' : 'PLAUSIBLE'
}
for (const c of liveClusters) c.final = finalVerdict(c)
const counts = { CONFIRMED: 0, PLAUSIBLE: 0, REFUTED: 0, UNVERIFIED: 0 }
for (const c of liveClusters) counts[c.final]++
log('Verdicts: ' + JSON.stringify(counts))

// ============ PHASE: SYNTHESIS ============
phase('Synthesis')
const confirmed = liveClusters.filter(c => c.final === 'CONFIRMED')
const plausible = liveClusters.filter(c => c.final === 'PLAUSIBLE')
const refuted = liveClusters.filter(c => c.final === 'REFUTED')
const unverified = liveClusters.filter(c => c.final === 'UNVERIFIED')
const synth = await run('director-synthesis', 'Synthesis', `${P}

${DIRECTOR_ROLE}

THIS CALL: final synthesis. Your memo: ${JSON.stringify((triage && triage.memo) || charter.memo)}
The script assembles the findings body mechanically from full records; you write ONLY the bounded narrative. Verdict semantics (repo standard): GREEN = sound, only cosmetic issues; YELLOW = contained defects, design mostly sound, user decides fixes; RED = foundational issues (soundness holes, systemic divergence) that block relying on the docs as-is.

CONFIRMED (${confirmed.length}):
${confirmed.map(clusterDigest).join('\n') || 'none'}

PLAUSIBLE (${plausible.length}) (verifier split or partial evidence; user judgment):
${plausible.map(c => clusterDigest(c) + ' | crux: ' + ((verdicts[c.representative_id] || {}).charity || {}).crux).join('\n') || 'none'}

UNVERIFIED (${unverified.length}): ${unverified.map(c => c.representative_id).join(', ') || 'none'}
REFUTED: ${refuted.length}. Observations (inadmissible smells): ${observations.length}. Known-open tagged: ${liveClusters.filter(c => c.known_open).length}.
Critic disclaimers: ${((criticRes && criticRes.honest_disclaimers) || []).join(' | ') || 'none'}
Ledger: ${ledgerSummaryFn()}

DELIVER: verdict, verdict_one_line, exec_summary (plain language, terse, lead with the worst confirmed problems; the user is the language's designer — no praise, no softening), reading_guide (one short paragraph: how to read the report), themes (group ALL confirmed+plausible cluster ids into 4-10 named themes, ordered most-severe-first; every id appears exactly once), coverage_narrative (what this audit can and cannot claim, from the ledger + disclaimers; name the coverage holes plainly), known_open_note (how known-open overlaps were handled).`, { model: 'fable', effort: 'xhigh', schema: SYNTH_SCHEMA })
if (!synth) throw new Error('Director synthesis dead — cannot assemble report')

// --- script assembles the report ---
function renderCluster(c) {
  const rep = findingById[c.representative_id] || {}
  const v = verdicts[c.representative_id] || {}
  const members = c.member_ids.map(id => findingById[id]).filter(Boolean)
  let s = '#### ' + c.representative_id + ' — ' + c.merged_one_line + '\n\n'
  s += '- **Severity/Category/Verdict:** ' + c.severity + ' / ' + c.category + ' / ' + c.final + (c.member_ids.length > 1 ? ' (x' + c.member_ids.length + ' independent reports)' : '') + '\n'
  s += '- **Anchors:** LOG ' + ((rep.log_ids || []).join(', ') || '—') + ' · SPEC § ' + ((rep.spec_sections || []).join(', ') || '—') + '\n'
  if (c.known_open) s += '- **Known-open match:** [' + c.known_open + '] — "' + (c.known_open_quote || '') + '"\n'
  s += '- **Why it is a defect:** ' + (rep.why_defect || '') + '\n'
  s += '- **Direction of change:** ' + (rep.direction_of_change || '') + '\n'
  if (v.evidence) s += '- **Evidence check:** ' + (v.evidence.evidence_verdict || '?') + ' — ' + (v.evidence.crux || '') + '\n'
  if (v.charity && v.charity.verdict !== 'not_run') s += '- **Charity check:** ' + v.charity.verdict + ' — ' + (v.charity.crux || '') + (v.charity.dissolving_evidence ? ' | ' + v.charity.dissolving_evidence : '') + '\n'
  if (v.tiebreak) s += '- **Tiebreak:** ' + v.tiebreak.verdict + ' — ' + v.tiebreak.crux + '\n'
  s += '- **Evidence:**\n' + members.map(m => fmtEvidence(m)).filter(Boolean).join('\n') + '\n'
  return s
}
const sevRank = { HIGH: 0, MED: 1, LOW: 2 }
const themed = new Set()
let body = ''
body += '# Ductus Design Audit — ' + REPORT_DATE + '\n\n'
body += '**Verdict: ' + synth.verdict + '** — ' + synth.verdict_one_line + '\n\n'
body += '- Corpus pin: `' + PIN.sha + '` (DECISION_LOG.md ' + PIN.log_bytes + ' B, SPEC.md ' + PIN.spec_bytes + ' B)\n'
body += '- Raw findings: ' + allFindings.length + ' → unique defect clusters: ' + liveClusters.length + ' → CONFIRMED ' + confirmed.length + ' · PLAUSIBLE ' + plausible.length + ' · UNVERIFIED ' + unverified.length + ' · REFUTED ' + refuted.length + ' · observations ' + observations.length + '\n\n'
body += '## Executive summary\n\n' + synth.exec_summary + '\n\n'
body += '## How to read this report\n\n' + synth.reading_guide + '\n\n'
body += '## Findings by theme\n\n'
for (const t of synth.themes) {
  body += '### ' + t.title + '\n\n' + (t.note ? t.note + '\n\n' : '')
  const cs = t.finding_ids.map(id => liveClusters.find(c => c.representative_id === id)).filter(Boolean).filter(c => c.final === 'CONFIRMED' || c.final === 'PLAUSIBLE')
  cs.sort((a, b) => (sevRank[a.severity] - sevRank[b.severity]) || (a.final === 'CONFIRMED' ? -1 : 1))
  for (const c of cs) { body += renderCluster(c) + '\n'; themed.add(c.representative_id) }
}
const unthemed = liveClusters.filter(c => (c.final === 'CONFIRMED' || c.final === 'PLAUSIBLE') && !themed.has(c.representative_id))
if (unthemed.length) {
  body += '### Unthemed (Director did not assign; listed for completeness)\n\n'
  unthemed.sort((a, b) => sevRank[a.severity] - sevRank[b.severity])
  for (const c of unthemed) body += renderCluster(c) + '\n'
}
body += '## Known-open handling\n\n' + synth.known_open_note + '\n\n'
if (unverified.length) {
  body += '## UNVERIFIED findings (verifier died or omitted; NOT dropped — treat as unreviewed leads)\n\n'
  for (const c of unverified) body += renderCluster(c) + '\n'
}
body += '## Coverage\n\n' + synth.coverage_narrative + '\n\n'
body += '### Coverage matrix\n\n| Mechanism | Coverage level | Status |\n|---|---|---|\n'
const famStatus = {}
for (const l of ledger) { const fam = l.label.split('-')[0]; if (!famStatus[fam]) famStatus[fam] = { ok: 0, bad: 0 }; famStatus[fam][l.status === 'ok' ? 'ok' : 'bad']++ }
const mechRows = [
  ['Claims index (all LOG entries normalized; subject-collision adjudication)', 'exhaustive at subject-collision fidelity', ['normalize', 'collide']],
  ['Shard deep audits (all sections + cited SPEC leaf ranges)', 'exhaustive per-entry read', ['shard']],
  ['Cross-pair semantic hunts', 'sampled (' + charter.cross_pairs.length + ' pairs)', ['pair']],
  ['Terminology (mechanical inventory + adjudication)', 'heuristic', ['terms']],
  ['Design-smell panel (identity-grounded admissibility)', 'sampled by cluster', ['smell']],
  ['Implementer completeness + unfalsifiability', 'sampled by cluster', ['impl']],
  ['Parser adversaries', 'heuristic', ['parser']],
  ['Witness probes (constructive, 3+ section composition)', 'sampled (' + (charter.scenario_seeds.length + (claimsOut.collisionScenarios || []).length) + ' scenarios)', ['witness']],
  ['Example conformance (all SPEC fences)', 'exhaustive mechanical charter', ['fences']],
  ['Reverse losslessness (churn + orphan zones)', 'sampled (churn+orphan zones only)', ['lossless']],
  ['Mechanical gates (8 invariants)', 'exhaustive mechanical', ['gate']],
  ['Deepening rounds', 'Director-directed', ['deep']],
  ['Adversarial verification (dual-lens HIGH/MED; evidence-only LOW)', 'per-cluster', ['v']],
]
for (const [mech, level, fams] of mechRows) {
  let ok = 0, bad = 0
  for (const f of fams) { if (famStatus[f]) { ok += famStatus[f].ok; bad += famStatus[f].bad } }
  body += '| ' + mech + ' | ' + level + ' | ' + (bad ? ok + ' ok / ' + bad + ' dead-or-halted (coverage hole)' : 'all ' + ok + ' ok') + ' |\n'
}
body += '\n### Disclaimed (this audit does NOT claim)\n\n- Semantic cross-section completeness beyond subject collisions and the sampled pairs.\n- Parse-ambiguity completeness (heuristic adversaries, not a grammar proof).\n- Joint-satisfiability completeness (witness probes sample composition space).\n' + (((criticRes && criticRes.honest_disclaimers) || []).map(d => '- ' + d + '\n').join('')) + '\n'
const deadHalt = ledger.filter(l => l.status !== 'ok')
if (deadHalt.length) body += '### Agent failures/halts\n\n' + deadHalt.map(l => '- ' + l.label + ' (' + l.phase + '): ' + l.status + (l.halt_reason ? ' — ' + l.halt_reason : '') + '\n').join('') + '\n'
if (observations.length) {
  body += '## Appendix A — Observations (inadmissible smells; unverified opinions, kept for honesty)\n\n'
  for (const o of observations) body += '- **' + o.finding_id + '** [' + (o.family || '') + '] ' + o.one_line + ' — ' + String(o.why_defect).slice(0, 400) + '\n'
  body += '\n'
}
if (refuted.length) {
  body += '## Appendix B — Refuted findings (raised and killed; reasons given)\n\n'
  for (const c of refuted) {
    const v = verdicts[c.representative_id] || {}
    body += '- **' + c.representative_id + '** ' + c.merged_one_line + ' — refuted: ' + ((v.evidence && v.evidence.evidence_verdict === 'fail') ? 'evidence failed: ' + v.evidence.crux : (v.tiebreak ? 'tiebreak: ' + v.tiebreak.crux : 'charity: ' + ((v.charity || {}).crux || '') + (v.charity && v.charity.dissolving_evidence ? ' | ' + v.charity.dissolving_evidence : ''))) + '\n'
  }
  body += '\n'
}
body += '## Appendix C — Method\n\nOne Fable Director (charter → triage → deepening → synthesis, memo-threaded), Opus worker fleets, deterministic script plumbing (buckets, ledger, retry, assembly). Read-only throughout; all evidence quoted verbatim from the pinned corpus. Verification: dual-lens (evidence + category-conditional charity) per HIGH/MED cluster, evidence-only for LOW, tiebreak on HIGH splits. Agents: ' + ledger.length + '. Output tokens spent (all fleets): ~' + Math.round(budget.spent() / 1000) + 'k.\n'

// ============ PHASE: PERSIST ============
phase('Persist')
const CHUNK = 60000
const parts = []
for (let i = 0; i < body.length; i += CHUNK) parts.push(body.slice(i, i + CHUNK))
log('Persisting report: ' + body.length + ' chars in ' + parts.length + ' parts')
await pipeline(parts.map((p, i) => ({ p, i })), (item) => run('write-part-' + (item.i + 1), 'Persist', `You are a mechanical file writer. Create the directory ${REPORT_DIR} if missing (mkdir -p via Bash). Then use the Write tool to write the file ${REPORT_DIR}/part-${String(item.i + 1).padStart(2, '0')}.md with EXACTLY the content between the BEGIN/END markers below — byte-exact, no additions, no trailing commentary, do not even fix typos. Then verify with wc -c and report bytes_written.
Schema note: protocol_acknowledged=true is granted for this mechanical role; halt only if Write fails.
===BEGIN CONTENT===
${item.p}
===END CONTENT===`, { model: 'opus', effort: 'low', agentType: 'general-purpose', schema: WRITER_SCHEMA }))
const expectedBytes = utf8len(body)
const concat = await run('concat-verify', 'Persist', `You are a mechanical assembler. Steps:
1. cat ${REPORT_DIR}/part-*.md (in name order) > ${REPORT_PATH}
2. wc -c ${REPORT_PATH} — expect ~${expectedBytes} bytes (report actual).
3. Verify corpus pin unchanged: git -C ${ROOT} rev-parse HEAD equals ${PIN.sha}; wc -c of DECISION_LOG.md and SPEC.md equal ${PIN.log_bytes} / ${PIN.spec_bytes}. pin_ok accordingly.
4. rm the part-*.md files (they are yours; the assembled report stays).
5. head -30 ${REPORT_PATH} to confirm it starts with '# Ductus Design Audit'.
protocol_acknowledged=true is granted for this mechanical role.`, { model: 'opus', effort: 'low', agentType: 'general-purpose', schema: CONCAT_SCHEMA })

return {
  reportPath: REPORT_PATH,
  verdict: synth.verdict,
  verdict_one_line: synth.verdict_one_line,
  exec_summary: synth.exec_summary,
  counts: { raw: allFindings.length, clusters: liveClusters.length, ...counts, observations: observations.length },
  agents: ledger.length,
  dead_or_halted: deadHalt.map(l => l.label + ':' + l.status),
  family_errors: familyErrors,
  persisted_bytes: concat ? concat.bytes : null,
  pin_ok_at_end: concat ? concat.pin_ok : null,
  orchestrator_required_step: 'Read the full report at ' + REPORT_PATH + ' end-to-end personally and spot-check a sample of CONFIRMED findings against disk BEFORE presenting to the user. Do not edit LOG/SPEC.',
}
