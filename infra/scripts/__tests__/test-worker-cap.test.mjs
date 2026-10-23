import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));
const CAP = /^[1-9][0-9]*$/;
const READERS = [
  { knob: 'TEST_MAX_WORKERS', runs: [/\bgate\.sh\b/, /\bpnpm\s+(run\s+)?gate\b/] },
  { knob: 'VITEST_MAX_WORKERS', runs: [/\bvitest\b/, /\bpnpm\b[^\n;&|]*\btest\b/] },
];

const commandOf = (step) => String(step.run ?? '').replace(/\\\r?\n/g, ' ');
const readersOf = (step) => READERS.filter(({ runs }) => runs.some((pattern) => pattern.test(commandOf(step))));
const runsTests = (step) => readersOf(step).length > 0;

const exportedDefault = (knob) => new RegExp(`\\b${knob}=\\$\\{${knob}:[-=]([^}]*)\\}[^\\n]*>>\\s*"?\\$GITHUB_ENV\\b`);
const defaultsExportedBy = (step) =>
  Object.fromEntries(READERS.flatMap(({ knob }) => (exportedDefault(knob).exec(commandOf(step)) ?? []).slice(1).map((value) => [knob, value])));

const jobsOf = (file, workflow) =>
  Object.entries(workflow.jobs ?? {}).map(([id, job]) => {
    let exported = {};
    return {
      name: `${file}:${id}`,
      steps: (job.steps ?? []).map((step) => {
        const env = { ...exported, ...workflow.env, ...job.env, ...step.env };
        exported = { ...exported, ...defaultsExportedBy(step) };
        return { ...step, env };
      }),
    };
  });

const withoutCap = (jobs) =>
  jobs.flatMap((job) =>
    job.steps.flatMap((step) =>
      readersOf(step)
        .filter(({ knob }) => !CAP.test(String(step.env[knob] ?? '')))
        .map(({ knob }) => `${job.name}: ${step.name ?? step.run} needs ${knob}`),
    ),
  );

const workflowsOnDisk = readdirSync(WORKFLOWS)
  .filter((file) => /\.ya?ml$/.test(file))
  .flatMap((file) => jobsOf(file, parse(readFileSync(join(WORKFLOWS, file), 'utf8'))));

const fixture = (run, { workflow, job, step, before = [], after = [] } = {}) =>
  jobsOf('fixture.yml', { env: workflow, jobs: { test: { env: job, steps: [...before, { name: 'tests', env: step, run }, ...after] } } });
const exporting = (value) => ({ name: 'workers', run: `echo "TEST_MAX_WORKERS=\${TEST_MAX_WORKERS:-${value}}" >>"$GITHUB_ENV"` });

describe('every CI job that runs tests caps its test workers (CD-309: several runners share each host)', () => {
  it('finds the jobs that run tests, and reaches the code that reads the cap in every step that runs tests', () => {
    expect.soft(workflowsOnDisk.filter((job) => job.steps.some(runsTests)).map((job) => job.name)).toContain('checks.yml:test');
    expect.soft(withoutCap(workflowsOnDisk), 'set the knob the step reads in its env, or default it to GITHUB_ENV in an earlier step').toEqual([]);
  });

  it('counts the knob wherever the workflow, job or step sets it, or a default an earlier step of the job exports to GITHUB_ENV, and only as a whole number of at least 1, the nearer setting winning', () => {
    for (const [label, run, options, missing] of [
      ['a default an earlier step exports', 'pnpm gate', { before: [exporting('3')] }, 0],
      ...['0', 'three', ''].map((value) => [`an exported default of ${JSON.stringify(value)}`, 'pnpm gate', { before: [exporting(value)] }, 1]),
      ['a default a later step exports', 'pnpm gate', { after: [exporting('3')] }, 1],
      ['an exported default the workflow overrides with 0', 'pnpm gate', { before: [exporting('3')], workflow: { TEST_MAX_WORKERS: '0' } }, 1],
      ['an exported TEST_MAX_WORKERS for vitest', 'npx vitest run', { before: [exporting('3')] }, 1],
      ['the job env', 'pnpm test', { job: { VITEST_MAX_WORKERS: '2' } }, 0],
      ['the step env', 'pnpm test', { step: { VITEST_MAX_WORKERS: '2' } }, 0],
      ...['0', '-1', '2.5', 'three', ''].map((cap) => [`a workflow cap of ${JSON.stringify(cap)}`, 'pnpm test', { workflow: { VITEST_MAX_WORKERS: cap } }, 1]),
      ['a step 0 over a workflow 3', 'pnpm test', { workflow: { VITEST_MAX_WORKERS: '3' }, step: { VITEST_MAX_WORKERS: '0' } }, 1],
    ]) {
      if (missing === 0) expect.soft(withoutCap(fixture(run, options)), label).toEqual([]);
      else expect.soft(withoutCap(fixture(run, options)), label).toHaveLength(missing);
    }
  });

  it('asks of a step the knob the code it runs reads: TEST_MAX_WORKERS of the gate, VITEST_MAX_WORKERS of vitest run directly, which neither stands in for, both of a step that runs both, and nothing of a step that runs no tests', () => {
    for (const [label, run] of [
      ['gate.sh', 'bash "$RUNNER_TEMP/gate.sh" --base origin/main'],
      ['pnpm gate', 'pnpm gate'],
    ]) {
      expect.soft(withoutCap(fixture(run)), `${label}, no knob`).toHaveLength(1);
      expect.soft(withoutCap(fixture(run, { workflow: { VITEST_MAX_WORKERS: '3' } })), `${label}, VITEST_MAX_WORKERS`).toHaveLength(1);
      expect.soft(withoutCap(fixture(run, { workflow: { TEST_MAX_WORKERS: '3' } })), `${label}, TEST_MAX_WORKERS`).toEqual([]);
    }
    for (const [label, run] of [
      ['pnpm test', 'pnpm test'],
      ['pnpm -r run test', 'pnpm -r --if-present run test'],
      ['vitest', 'npx vitest run'],
    ]) {
      expect.soft(withoutCap(fixture(run, { workflow: { TEST_MAX_WORKERS: '3' } })), `${label}, TEST_MAX_WORKERS`).toHaveLength(1);
      expect.soft(withoutCap(fixture(run, { workflow: { VITEST_MAX_WORKERS: '3' } })), `${label}, VITEST_MAX_WORKERS`).toEqual([]);
    }
    expect.soft(withoutCap(fixture('pnpm gate && npx vitest run')).sort(), 'the gate and vitest').toEqual([
      'fixture.yml:test: tests needs TEST_MAX_WORKERS',
      'fixture.yml:test: tests needs VITEST_MAX_WORKERS',
    ]);
    expect.soft(withoutCap(fixture('pnpm install --frozen-lockfile')), 'no tests').toEqual([]);
  });

  it('knows a step that runs tests when it sees one (fixtures)', () => {
    const caught = [
      'bash "$RUNNER_TEMP/gate.sh" "${args[@]}"',
      'bash infra/scripts/gate.sh --full',
      'pnpm gate',
      'pnpm run gate',
      'pnpm test',
      'pnpm -r --if-present run test',
      'pnpm --filter @wg/game-wordgrid run test:balance',
      'pnpm --filter ./apps/app \\\n  exec vitest related --run src/x.ts',
      'npx vitest run',
    ];
    for (const run of caught) expect(runsTests({ run }), run).toBe(true);
    for (const run of ['pnpm install --frozen-lockfile', 'pnpm --filter @wg/app run export:web', 'pnpm baseline:update --screens "$LIST"', 'echo testing', undefined]) {
      expect(runsTests({ run }), String(run)).toBe(false);
    }
  });
});
