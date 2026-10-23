import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it, vi } from 'vitest';
import { parse } from 'yaml';

import { liveReporters } from '../../e2e/reporters.ts';
import { failureScreenshots } from '../../e2e/screenshots.ts';
import { CATALOGUE_ARTIFACT, CATALOGUE_FILE } from '../nightly-findings.mjs';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));
const workflow = parse(readFileSync(join(WORKFLOWS, 'journeys.yml'), 'utf8'));
const { journeys } = workflow.jobs;
const step = (id) => journeys.steps.find((entry) => entry.id === id);

describe('journeys.yml: the one run of the journeys, once per PR, targeted (owner, 15 Oct)', () => {
  it('is dispatched by hand, targeted by default to what the ref changes against origin/main, with named files overriding it, and called by the nightly run with the same inputs, typed as strings and defaulted alike', () => {
    expect.soft(Object.keys(workflow.on)).toEqual(['workflow_dispatch', 'workflow_call']);
    expect.soft(workflow.on.workflow_dispatch.inputs.scope).toMatchObject({ type: 'choice', options: ['touched', 'all'], default: 'touched' });
    expect.soft(workflow.on.workflow_dispatch.inputs.files).toMatchObject({ type: 'string', default: '' });
    expect.soft(step('plan').run).toContain('qa-plan.mjs --base "$(git merge-base origin/main "$head")" --head "$head"');
    expect.soft(step('plan').run).toContain('journeys-named.mjs --files "$FILES"');
    const dispatched = workflow.on.workflow_dispatch.inputs;
    const called = workflow.on.workflow_call.inputs;
    expect.soft(Object.keys(called)).toEqual(Object.keys(dispatched));
    for (const [name, input] of Object.entries(called)) expect.soft(input, name).toEqual({ description: dispatched[name].description, type: 'string', default: dispatched[name].default });
  });

  it('cuts a touched run to the smoke journeys only; named files and scope all run as asked', () => {
    const lines = step('plan').run.split('\n').map((line) => line.trim());
    const touched = lines.indexOf('else');
    expect(lines[touched + 1]).toBe('node infra/verify/qa-plan.mjs --base "$(git merge-base origin/main "$head")" --head "$head" >"$OUT/plan.json"');
    expect(lines[touched + 2]).toBe('node infra/verify/ci-mode.mjs journeys --plan "$OUT/plan.json" | tee -a "$GITHUB_OUTPUT"');
    expect(lines[touched + 3]).toBe('fi');
    expect(step('plan').run.match(/ci-mode\.mjs/g)).toHaveLength(1);
  });
});

describe('journeys.yml: the whole catalogue shot and scanned, on main only when the nightly asks', () => {
  it('runs pnpm proof --catalogue only when the catalogue input is on and the ref is main, so a PR never runs it, and uploads the findings file the nightly filer reads, under the name it downloads', () => {
    expect.soft(workflow.on.workflow_dispatch.inputs.catalogue).toMatchObject({ type: 'choice', options: ['off', 'on'], default: 'off' });
    expect.soft(step('catalogue').if).toContain("inputs.catalogue == 'on' && github.ref == 'refs/heads/main'");
    expect.soft(step('catalogue').run).toBe('pnpm proof --catalogue --out "$OUT/catalogue"');
    const upload = journeys.steps.find((entry) => entry.with?.name === CATALOGUE_ARTIFACT);
    expect.soft(upload.if).toContain("steps.catalogue.outcome != 'skipped'");
    expect.soft(upload.with.path).toBe(`\${{ runner.temp }}/journeys/catalogue/${CATALOGUE_FILE}`);
  });
});

describe('the journeys Playwright configs', () => {
  const configs = async () => {
    vi.resetModules();
    const { default: app } = await import('../../../apps/app/e2e/journeys/playwright.config.ts');
    const { default: admin } = await import('../../../apps/admin/e2e/journeys/playwright.config.ts');
    return [app, admin];
  };

  it('screenshot failing tests only in a journeys.yml run, which sets JOURNEYS_SCREENSHOTS, a local run keeping them off (head16), and report to the list, and to the JSON file journeys-local.sh reads when LIVE_REPORT names one', async () => {
    expect.soft(failureScreenshots({})).toBe('off');
    expect.soft(failureScreenshots({ JOURNEYS_SCREENSHOTS: ' ' })).toBe('off');
    expect.soft(failureScreenshots({ JOURNEYS_SCREENSHOTS: 'on' })).toBe('off');
    expect.soft(failureScreenshots({ QA_RESULTS: '/tmp/app-results' })).toBe('off');
    expect.soft(failureScreenshots({ JOURNEYS_SCREENSHOTS: 'only-on-failure' })).toBe('only-on-failure');
    expect.soft(liveReporters({})).toEqual([['list']]);
    expect.soft(liveReporters({ LIVE_REPORT: '/tmp/r.json' })).toEqual([['list'], ['json', { outputFile: '/tmp/r.json' }]]);
    const shown = async () => (await configs()).map(({ use, reporter }) => ({ screenshot: use.screenshot, trace: use.trace, video: use.video, reporter }));
    const reporter = [['list'], ['json', { outputFile: '/tmp/journeys/report.json' }]];
    try {
      vi.stubEnv('JOURNEYS_SCREENSHOTS', '');
      vi.stubEnv('QA_RESULTS', '/tmp/app-results');
      vi.stubEnv('LIVE_REPORT', '/tmp/journeys/report.json');
      expect.soft(await shown(), 'a local run').toEqual(Array(2).fill({ screenshot: 'off', trace: 'off', video: 'off', reporter }));
      vi.stubEnv('JOURNEYS_SCREENSHOTS', 'only-on-failure');
      expect.soft(await shown(), 'a journeys.yml run').toEqual(Array(2).fill({ screenshot: 'only-on-failure', trace: 'off', video: 'off', reporter }));
    } finally {
      vi.unstubAllEnvs();
      vi.resetModules();
    }
  });
});
