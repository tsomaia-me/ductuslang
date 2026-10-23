import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { childEnv } from './hermetic.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../../..');
const workflow = (name) => readFileSync(join(ROOT, '.github/workflows', name), 'utf8');

function job(text, name) {
  const start = text.indexOf(`\n  ${name}:\n`);
  if (start === -1) throw new Error(`no job ${name}`);
  const next = text.slice(start + 1).search(/\n {2}[a-z][\w-]*:\n/);
  return next === -1 ? text.slice(start) : text.slice(start, start + 1 + next);
}

describe('the review workflows (CD-204)', () => {
  it('runs gate-main and checks on the wb runners and checks-run on the heavy runners (CD-269: no GitHub-hosted minutes; owner, 12 Oct, CD-354), and takes no verify lock in checks, so it never waits for a gate (CD-210)', () => {
    expect.soft(job(workflow('gate-main.yml'), 'gate-main')).toContain('runs-on: [self-hosted, wb]');
    expect.soft(job(workflow('checks.yml'), 'test')).toContain('runs-on: [self-hosted, heavy]');
    for (const name of ['lane', 'checks']) expect.soft(job(workflow('checks.yml'), name), name).toContain('runs-on: [self-hosted, wb]');
    for (const name of ['test', 'checks']) {
      const checks = job(workflow('checks.yml'), name);
      expect.soft(checks, name).not.toContain('Hold the verify lock');
      expect.soft(checks, name).not.toContain('gate-locked.sh');
    }
    expect.soft(job(workflow('checks.yml'), 'test')).toMatch(/\n {4}timeout-minutes: 60/);
    expect.soft(job(workflow('checks.yml'), 'checks')).toMatch(/\n {4}timeout-minutes: 10/);
  });

  it('runs every release job on the wb-release runner, dev through deploy.yml; dev waits for the runner check, which has no needs of its own, so nothing is released while the runner is offline, and the walk waits on dev (CD-259, CD-285, CD-322, CD-478)', () => {
    const release = parse(workflow('release.yml')).jobs;
    expect.soft(Object.keys(release)).toEqual(['plan', 'runner-check', 'dev', 'smoke']);
    for (const id of ['plan', 'runner-check', 'smoke']) expect.soft(release[id]['runs-on'], id).toEqual(['self-hosted', 'wb-release']);
    expect.soft(release.dev.uses).toBe('./.github/workflows/deploy.yml');
    expect.soft(release.plan.needs).toBeUndefined();
    expect.soft(release['runner-check'].needs).toBeUndefined();
    expect.soft(release['runner-check'].steps).toHaveLength(1);
    expect.soft(release['runner-check'].if).toBe(release.plan.if);
    expect.soft(release.dev.needs).toEqual(['plan', 'runner-check']);
    expect.soft(release.smoke.needs).toEqual(['plan', 'dev']);
    expect.soft(release.smoke.if).toBe("needs.dev.result == 'success'");
    expect.soft(Object.fromEntries(Object.entries(release).map(([id, entry]) => [id, entry['timeout-minutes']]))).toEqual({ plan: 30, 'runner-check': 5, dev: undefined, smoke: 30 });
    const text = workflow('release.yml');
    expect.soft(text).not.toContain('playwright install');
    expect.soft(job(text, 'plan')).not.toMatch(/pnpm|wg-verify|playwright/i);
    expect.soft(job(text, 'smoke')).toContain('. infra/scripts/qa-browser-env.sh');
  });

  it("finds wb's Chromium, its libraries and its temp folder in one script the smoke walk and the QA share (N8), and counts a headless-shell folder as a Chromium install", () => {
    const home = mkdtempSync(join(tmpdir(), 'cd322-browser-'));
    const shell = mkdtempSync(join(tmpdir(), 'cd405-browser-'));
    try {
      for (const dir of ['wg-verify/tmp', 'wg-verify/libs/nss/usr/lib', '.cache/ms-playwright/chromium-1']) mkdirSync(join(home, dir), { recursive: true });
      writeFileSync(join(home, 'wg-verify/libs/nss/usr/lib/libnss3.so'), '');
      const shown = spawnSync('bash', ['-c', '. infra/scripts/qa-browser-env.sh && printf "%s\\n" "$TMPDIR" "$PLAYWRIGHT_BROWSERS_PATH" "$LD_LIBRARY_PATH"'], {
        cwd: ROOT,
        encoding: 'utf8',
        env: { ...childEnv(), HOME: home, PLAYWRIGHT_BROWSERS_PATH: '', LD_LIBRARY_PATH: '' },
      }).stdout.split('\n');
      expect.soft(shown.slice(0, 3), 'chromium').toEqual([join(home, 'wg-verify/tmp'), join(home, '.cache/ms-playwright'), join(home, 'wg-verify/libs/nss/usr/lib')]);
      mkdirSync(join(shell, '.cache/ms-playwright/chromium_headless_shell-1'), { recursive: true });
      const headless = spawnSync('bash', ['-c', '. infra/scripts/qa-browser-env.sh && printf "%s" "$PLAYWRIGHT_BROWSERS_PATH"'], {
        cwd: ROOT,
        encoding: 'utf8',
        env: { ...childEnv(), HOME: shell, PLAYWRIGHT_BROWSERS_PATH: '', LD_LIBRARY_PATH: '' },
      }).stdout;
      expect.soft(headless, 'headless shell').toBe(join(shell, '.cache/ms-playwright'));
    } finally {
      rmSync(home, { recursive: true, force: true });
      rmSync(shell, { recursive: true, force: true });
    }
  });

  it('deploys every part of the released commit to dev through deploy.yml, walks dev and ends there (CD-285, CD-322, CD-478)', () => {
    const release = parse(workflow('release.yml'));
    const steps = (id) => release.jobs[id].steps;
    const run = (id, stepId) => String(steps(id).find((entry) => entry.id === stepId).run);
    expect.soft(run('smoke', 'smoke')).toContain('node infra/scripts/smoke-walk.mjs --env dev --browser --sha "$TIP" --web-sha "$TIP"');
    expect.soft(release.jobs.dev).toMatchObject({ uses: './.github/workflows/deploy.yml', secrets: 'inherit', with: { env: 'dev', ref: '${{ needs.plan.outputs.sha }}' } });
    expect.soft(Object.keys(release.jobs.dev.with)).toEqual(['env', 'ref']);
    expect.soft(run('plan', 'tip')).not.toMatch(/release-live|deploy\/|HALVES/);
    expect.soft(release.on.workflow_dispatch).toBeNull();
    expect.soft(workflow('release.yml')).not.toMatch(/preflight|warn-only|--quick/);
    expect.soft(steps('plan').map((entry) => entry.id).filter(Boolean)).toEqual(['tip', 'gate']);
  });

  it("hands the Access service token to the steps that reach dev alone (CD-285)", () => {
    const jobs = parse(workflow('release.yml')).jobs;
    const holders = Object.entries(jobs).flatMap(([id, entry]) => (entry.steps ?? []).filter((step) => step.env?.CF_ACCESS_CLIENT_ID).map((step) => `${id}:${step.id}`));
    expect.soft(holders).toEqual(['smoke:smoke']);
    for (const holder of holders) {
      const [id, stepId] = holder.split(':');
      expect.soft(jobs[id].steps.find((step) => step.id === stepId).env).toMatchObject({
        CF_ACCESS_CLIENT_ID: '${{ secrets.CF_ACCESS_CLIENT_ID }}',
        CF_ACCESS_CLIENT_SECRET: '${{ secrets.CF_ACCESS_CLIENT_SECRET }}',
      });
    }
    for (const entry of Object.values(jobs)) expect.soft(JSON.stringify(entry.env ?? {})).not.toContain('CF_ACCESS');
  });

  it('puts the production environment on every release job that reads a secret (CD-259)', () => {
    const release = parse(workflow('release.yml')).jobs;
    expect.soft(Object.entries(release).filter(([, entry]) => entry.environment === 'production').map(([id]) => id)).toEqual(['plan', 'smoke']);
    expect.soft(release['runner-check'].environment).toBeUndefined();
    const text = workflow('release.yml');
    expect.soft(job(text, 'smoke')).not.toMatch(/RELEASE_APP|CLOUDFLARE|LINEAR_API_KEY/);
    expect.soft(job(text, 'plan')).not.toMatch(/RELEASE_APP|CLOUDFLARE|LINEAR_API_KEY/);
  });

  const strip = (expression) => expression.replace(/^\$\{\{\s*|\s*\}\}$/g, '');

  it('queues only a run that releases: a failed or cancelled main gate, or a dispatch off main, never replaces the release waiting (CD-478)', () => {
    const { concurrency } = parse(workflow('release.yml'));
    const format = (text, ...values) => text.replace(/\{(\d+)\}/g, (_, at) => String(values[at]));
    const group = (github) => new Function('github', 'format', `return (${strip(concurrency.group)});`)(github, format);
    const main = { ref: 'refs/heads/main', run_id: 7 };
    expect.soft(group({ ...main, event_name: 'workflow_run', event: { workflow_run: { conclusion: 'success' } } })).toBe('release');
    expect.soft(group({ ...main, event_name: 'workflow_dispatch', event: {} })).toBe('release');
    for (const conclusion of ['failure', 'cancelled', 'skipped']) {
      expect.soft(group({ ...main, event_name: 'workflow_run', event: { workflow_run: { conclusion } } }), conclusion).toBe('release-not-run-7');
    }
    expect.soft(group({ ref: 'refs/heads/claude/x', run_id: 7, event_name: 'workflow_dispatch', event: {} })).toBe('release-not-run-7');
    expect.soft(concurrency['cancel-in-progress']).toBe(false);
  });

  it("rolls nothing back, tags nothing and starts no production deploy itself: deploy.yml rolls back a failed part, and the version is the release PR's (CD-478)", () => {
    const text = workflow('release.yml');
    expect(text).not.toMatch(/release-rollback\.sh|release-linear\.mjs|release-live\.sh|release-version\.mjs|git-as-app\.sh|promote|dispatch-workflow|create-github-app-token/);
    expect(parse(text).jobs.dev.with.env).toBe('dev');
  });
});
