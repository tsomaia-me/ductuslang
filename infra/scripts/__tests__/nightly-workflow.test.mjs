import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { CATALOGUE_ARTIFACT, JOURNEYS_ARTIFACT } from '../nightly-findings.mjs';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));
const text = readFileSync(join(WORKFLOWS, 'nightly.yml'), 'utf8');
const workflow = parse(text);
const journeys = parse(readFileSync(join(WORKFLOWS, 'journeys.yml'), 'utf8'));
const { run, file } = workflow.jobs ?? {};
const ON_MAIN = "github.ref == 'refs/heads/main'";
const secretsOf = (entry) => [...JSON.stringify(entry ?? {}).matchAll(/secrets\.([A-Z_]+)/g)].map((match) => match[1]);

describe('nightly.yml: the full journeys on main at 02:00 UTC, filed into the one MVP debt issue, blocking nothing (owner, 19 Oct)', () => {
  it("starts at 02:00 UTC or by hand, never on a pull request, a push or the merge queue, and runs every journey through journeys.yml, on main only, reading the repository alone, in a concurrency group of the caller's own, so a manual journeys run on main never cancels the nightly", () => {
    expect.soft(workflow.on).toEqual({ schedule: [{ cron: '0 2 * * *' }], workflow_dispatch: null });
    expect.soft(workflow.concurrency).toEqual({ group: 'nightly', 'cancel-in-progress': false });
    expect.soft(workflow.permissions).toEqual({});
    expect.soft(Object.keys(workflow.jobs)).toEqual(['run', 'file']);
    expect.soft(run.uses).toBe('./.github/workflows/journeys.yml');
    expect.soft(run.with).toEqual({ scope: 'all', catalogue: 'on' });
    expect.soft(run.if).toBe(ON_MAIN);
    expect.soft(run.permissions).toEqual({ contents: 'read' });
    expect.soft(secretsOf(run)).toEqual([]);
    expect.soft(run.secrets).toBeUndefined();
    expect.soft(journeys.concurrency.group).toBe('journeys-${{ github.workflow }}-${{ github.ref }}');
    expect.soft(workflow.name).not.toBe(journeys.name);
  });

  it('files only when the run failed, only on main, on wb-release, from main, through the QA filer into the MVP debt batch, and posts no commit status and installs nothing, so nothing waits on it', () => {
    expect.soft(file.needs).toBe('run');
    expect.soft(file.if).toBe(`\${{ !cancelled() && ${ON_MAIN} && needs.run.result == 'failure' }}`);
    expect.soft(file['runs-on']).toEqual(['self-hosted', 'wb-release']);
    expect.soft(file.permissions).toEqual({ contents: 'read' });
    const checkout = file.steps.find((step) => String(step.uses).startsWith('actions/checkout@'));
    expect.soft(checkout.with).toMatchObject({ ref: 'main', 'persist-credentials': false });
    const downloads = file.steps.filter((step) => String(step.uses).startsWith('actions/download-artifact@'));
    expect.soft(downloads.map((step) => step.with.name)).toEqual([JOURNEYS_ARTIFACT, CATALOGUE_ARTIFACT]);
    for (const step of downloads) expect.soft(step['continue-on-error'], step.with.name).toBe(true);
    const filer = file.steps.find((step) => step.id === 'linear');
    expect.soft(filer.run).toContain('node infra/scripts/nightly-findings.mjs --dir "$OUT" --run "$RUN_URL"');
    expect.soft(filer.run).toContain('NODE_USE_ENV_PROXY=1 node infra/scripts/qa-linear.mjs --batch mvp-debt --surface "$surface" --version "${GITHUB_SHA:0:8}" --findings "$OUT/$surface-file.json"');
    expect.soft(secretsOf(file)).toEqual(['LINEAR_API_KEY']);
    expect.soft(secretsOf(filer)).toEqual(['LINEAR_API_KEY']);
    expect.soft(text, 'no status, no install').not.toMatch(/statuses|gate-status|pnpm install/);
  });
});
