import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { readConfig } from '../../../apps/edge/scripts/live-vars.mjs';

import { childEnv } from './hermetic.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../../..');
const read = (path) => readFileSync(join(ROOT, path), 'utf8');
const deploy = parse(read('.github/workflows/deploy.yml'));
const part = parse(read('.github/workflows/deploy-part.yml'));
const stack = parse(read('.github/actions/deploy-stack/action.yml')).runs.steps;
const { plan } = deploy.jobs;
const PARTS = ['api', 'admin', 'app'];

const strip = (expression) =>
  String(expression)
    .replace(/^\$\{\{\s*|\s*\}\}$/g, '')
    .replace(/inputs\.([a-z]+(?:-[a-z]+)+)/g, "inputs['$1']");
const evaluate = (expression, { needs = {}, jobs = {}, inputs = {}, github = { ref: 'refs/heads/main' }, failed = false, cancelled = false }) =>
  new Function('needs', 'jobs', 'inputs', 'github', 'failure', 'cancelled', 'contains', 'fromJSON', `return (${strip(expression)});`)(
    needs,
    jobs,
    inputs,
    github,
    () => failed,
    () => cancelled,
    (list, item) => list.includes(item),
    JSON.parse,
  );

describe('deploy.yml deploys the parts ticked to one environment with the Wrangler action (CD-478)', () => {
  it('starts only by hand or from another workflow, one run per environment at a time, never cancelled, and pushes nothing', () => {
    expect(Object.keys(deploy.on).sort()).toEqual(['workflow_call', 'workflow_dispatch']);
    expect(deploy.concurrency).toEqual({ group: 'deploy-${{ inputs.env }}', 'cancel-in-progress': false });
    expect(JSON.stringify([deploy, part])).not.toMatch(/git push|git tag|dispatches/);
    expect(evaluate(plan.if, { inputs: { env: 'dev' } })).toBe(true);
    expect(evaluate(plan.if, { inputs: { env: 'production' } })).toBe(true);
    expect(evaluate(plan.if, { inputs: { env: 'staging' } })).toBe(false);
    expect(evaluate(plan.if, { inputs: { env: 'dev' }, github: { ref: 'refs/heads/claude/x' } })).toBe(false);
  });

  it('asks for four tick boxes, all of them ticked by default, and nothing names the halves any more', () => {
    for (const trigger of ['workflow_dispatch', 'workflow_call']) {
      const inputs = deploy.on[trigger].inputs;
      expect(inputs.all).toMatchObject({ type: 'boolean', default: true });
      for (const name of PARTS) expect(inputs[name], `${trigger} ${name}`).toMatchObject({ type: 'boolean', default: false });
    }
    expect(read('.github/workflows/deploy.yml') + read('.github/workflows/deploy-part.yml') + read('.github/actions/deploy-stack/action.yml')).not.toMatch(/halves|HALVES/);
    expect(plan.env).toMatchObject({ APP: '${{ inputs.all || inputs.app }}', ADMIN: '${{ inputs.all || inputs.admin }}', API: '${{ inputs.all || inputs.api }}' });
  });

  it("deploys only a commit on main, even forced; resolves a branch or tag by gate-main's filter and a sha only as named; force skips the gate evidence, deployed/dev and the newest-release check alone", () => {
    const sha = plan.steps.find((step) => step.id === 'sha');
    expect(sha.if).toBeUndefined();
    expect(sha.run).toContain('git merge-base --is-ancestor "$named" origin/main');
    expect(sha.run).toContain('[[ "$TARGET" =~ ^[0-9a-f]{7,40}$ ]] || walk=(--walk)');
    expect(sha.run).toContain('node infra/scripts/deploy-ref.mjs --sha "$named" "${walk[@]}"');
    expect(sha.run).toContain('$GITHUB_STEP_SUMMARY');
    expect(sha.env.FORCE).toBe('${{ inputs.force || false }}');
    const forced = plan.steps.filter((step) => String(step.if).includes('inputs.force')).map((step) => step.name);
    expect(forced).toEqual(['Production deploys only the newest release', 'Forced', 'Production gets only what dev served']);
    const devServed = plan.steps.find((step) => step.name === 'Production gets only what dev served');
    expect(evaluate(devServed.if, { inputs: { env: 'production' } })).toBe(true);
    expect(evaluate(devServed.if, { inputs: { env: 'dev' } })).toBe(false);
    expect(evaluate(devServed.if, { inputs: { env: 'production', force: true } })).toBe(false);
    expect(devServed.run).toContain('deployed/dev/$part');
  });

  it("deploys to production only the newest v5.N.0 tag unless forced, so a re-run of an older release's run never goes over a newer one; a read-only check, dev and the owner's forced deploy take any ref", () => {
    const newest = plan.steps.find((step) => step.id === 'newest');
    expect(plan.steps.indexOf(newest)).toBeLessThan(plan.steps.findIndex((step) => step.id === 'sha'));
    expect(evaluate(newest.if, { inputs: { env: 'production', all: true } })).toBe(true);
    expect(evaluate(newest.if, { inputs: { env: 'production', all: true, force: true } })).toBe(false);
    expect(evaluate(newest.if, { inputs: { env: 'production', api: true } })).toBe(true);
    expect(evaluate(newest.if, { inputs: { env: 'production', all: false, app: false, admin: false, api: false } })).toBe(false);
    expect(evaluate(newest.if, { inputs: { env: 'dev', all: true } })).toBe(false);
    expect(newest.env.TARGET).toBe('${{ inputs.ref }}');
    const repo = mkdtempSync(join(tmpdir(), 'deploy-newest-'));
    try {
      const git = (...args) => spawnSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@t', ...args], { cwd: repo, env: childEnv(), encoding: 'utf8' });
      git('init', '-q');
      for (const tag of ['v5.9.0', 'v5.10.0', 'v5.11', 'v5.13']) {
        git('commit', '-q', '--allow-empty', '-m', tag);
        git('tag', tag);
      }
      const run = (target) => spawnSync('bash', ['-c', newest.run], { cwd: repo, env: { ...childEnv(), TARGET: target }, encoding: 'utf8' });
      expect(run('v5.10.0').status).toBe(0);
      for (const target of ['v5.9.0', 'main', 'v5.13', 'v5.11']) {
        const refused = run(target);
        expect(refused.status, target).toBe(1);
        expect(refused.stdout, target).toContain(`::error::production deploys only the newest release tag (v5.10.0), not '${target}'`);
      }
      const malformed = run('v5.10.0;x');
      expect(malformed.status).toBe(1);
      expect(malformed.stdout).toBe('::error::the ref is not a branch, tag or sha; nothing was deployed\n');
      const injected = run('v5.10.0\n::warning::injected');
      expect(injected.status).toBe(1);
      expect(injected.stdout).toBe('::error::the ref is not a branch, tag or sha; nothing was deployed\n');
      git('tag', '-d', 'v5.9.0', 'v5.10.0');
      expect(run('v5.13').stdout).toContain('newest release tag (none)');
    } finally {
      rmSync(repo, { recursive: true, force: true });
    }
  });

  it('reads what each part runs live and holds a part whose migrations or Worker config changed', () => {
    const live = plan.steps.find((step) => step.id === 'live').run;
    expect(live).toContain('read_sha "$WEB_URL/build.json" sha');
    expect(live).toContain('read_sha "$API_URL/api/v1/health" build.sha');
    expect(live).toContain('live-vars.mjs --env "$DEPLOY_ENV" --dir apps/edge --config wrangler.admin.jsonc --build-sha');
    expect(live).toContain('-- apps/edge/migrations "$@"');
    expect(live).toContain('held admin "$live_admin" apps/edge/wrangler.admin.jsonc');
    expect(live).toContain('held api "$live_api" apps/edge/wrangler.auth.jsonc apps/edge/wrangler.api.jsonc');
  });

  it('deploys each ticked part in its own job: the api first, then the admin and the app, neither after a failed or cancelled api', () => {
    for (const name of PARTS) {
      const job = deploy.jobs[name];
      expect(job.uses).toBe('./.github/workflows/deploy-part.yml');
      expect(job.secrets).toBe('inherit');
      expect(job.with).toMatchObject({ part: name, sha: '${{ needs.plan.outputs.sha }}', live: `\${{ needs.plan.outputs.live_${name} }}` });
    }
    expect(deploy.jobs.app.with.hold).toBeUndefined();
    const runs = (name, { ticked = 'true', api = 'success', cancelled = false } = {}) =>
      evaluate(deploy.jobs[name].if, { cancelled, needs: { plan: { result: 'success', outputs: { [name]: ticked } }, api: { result: api } } });
    expect(runs('api')).toBe(true);
    expect(runs('api', { ticked: 'false' })).toBe(false);
    for (const name of ['admin', 'app']) {
      expect(deploy.jobs[name].needs).toEqual(['plan', 'api']);
      expect(runs(name)).toBe(true);
      expect(runs(name, { api: 'skipped' })).toBe(true);
      expect(runs(name, { api: 'failure' })).toBe(false);
      expect(runs(name, { api: 'cancelled' })).toBe(false);
      expect(runs(name, { cancelled: true })).toBe(false);
      expect(runs(name, { ticked: 'false' })).toBe(false);
    }
  });

  it("builds and deploys only the part asked for: migrations with either Worker part, the admin app as the admin Worker's assets", () => {
    const ran = (name) => stack.filter((step) => step.if === undefined || evaluate(step.if, { inputs: { part: name, 'accept-var-change': 'false' } })).map((step) => step.name ?? step.uses);
    const always = stack.filter((step) => step.if === undefined).map((step) => step.name ?? step.uses);
    expect(ran('api')).toEqual([...always, "Build the Workers' packages", 'The configured vars equal what runs live', 'D1 migrations, every database of the environment', 'The auth and api Workers']);
    expect(ran('admin')).toEqual([...always, "Build the Workers' packages", 'Build the admin app', 'The configured vars equal what runs live', 'D1 migrations, every database of the environment', 'The admin Worker']);
    expect(ran('app')).toEqual([...always, 'Export the web app', 'The web app to Pages']);
    const step = (name) => stack.find((entry) => entry.name === name);
    for (const name of ['The auth and api Workers', 'The admin Worker', 'The web app to Pages']) {
      expect(step(name).uses).toMatch(/^cloudflare\/wrangler-action@[0-9a-f]{40}$/);
      expect(step(name).with.wranglerVersion).toBeUndefined();
      expect(step(name).with.command).not.toContain('--dry-run');
    }
    expect(step('The auth and api Workers').with.command.trim().split('\n').map((line) => line.split(' ')[2])).toEqual(['wrangler.auth.jsonc', 'wrangler.api.jsonc']);
    expect(step('The admin Worker').with.command).toMatch(/^deploy -c wrangler\.admin\.jsonc /);
    expect(step('The configured vars equal what runs live').env.CONFIGS).toBe("${{ inputs.part == 'api' && '--config wrangler.auth.jsonc --config wrangler.api.jsonc' || '--config wrangler.admin.jsonc' }}");
    expect(evaluate(step('The configured vars equal what runs live').if, { inputs: { part: 'api', 'accept-var-change': 'true' } })).toBe(false);
    expect(step('Export the web app').env.TMPDIR).toBe('${{ runner.temp }}');
    expect(step('The web app to Pages').with.command).toContain("--branch ${{ inputs.env == 'dev' && 'deploy/web-dev' || 'deploy/web' }} --commit-hash ${{ inputs.sha }}");
    expect(readConfig(join(ROOT, 'apps/edge/wrangler.admin.jsonc')).assets.directory).toBe('../admin/dist');
  });

  it('checks each part its own way, marks it deployed/<env>/<part> only after that, and rolls back only a failed part to what ran live, never on cancel, never when held', () => {
    const { deploy: job, rollback } = part.jobs;
    const names = job.steps.map((step) => step.name ?? step.uses);
    const marked = names.indexOf('deployed/${{ inputs.env }}/${{ inputs.part }} on the commit');
    expect(names.indexOf('Smoke walk')).toBeLessThan(marked);
    expect(names.indexOf('Smoke the admin on dev')).toBeLessThan(marked);
    const step = (name) => job.steps.find((entry) => entry.name === name);
    expect(evaluate(step('Smoke walk').if, { inputs: { part: 'admin' } })).toBe(false);
    expect(evaluate(step('Smoke walk').if, { inputs: { part: 'app' } })).toBe(true);
    expect(evaluate(step('Smoke the admin on dev').if, { inputs: { part: 'admin', env: 'dev' } })).toBe(true);
    expect(evaluate(step('Smoke the admin on dev').if, { inputs: { part: 'admin', env: 'production' } })).toBe(false);
    expect(job.steps[marked].run).toContain('\\"context\\":\\"deployed/$DEPLOY_ENV/$PART\\"');
    const rolls = (result, hold = '0', live = 'a') => evaluate(rollback.if, { failed: result === 'failure', needs: { deploy: { result } }, inputs: { hold, live } });
    expect(rolls('failure')).toBe(true);
    expect(rolls('cancelled')).toBe(false);
    expect(rolls('success')).toBe(false);
    expect(rolls('failure', '1')).toBe(false);
    expect(rolls('failure', '0', '')).toBe(false);
    expect(evaluate(step('Held').if, { failed: true, inputs: { hold: '1' } })).toBe(true);
    expect(evaluate(step('Held').if, { failed: true, inputs: { hold: '0' } })).toBe(false);
  });

  it("tells its caller how each part ended: deployed, held, rolled back or not rolled back, and nothing for a part that never ran (CD-478)", () => {
    expect(deploy.on.workflow_call.outputs).toEqual(Object.fromEntries(PARTS.map((name) => [name, { value: `\${{ jobs.${name}.outputs.ended }}` }])));
    const { deploy: job, rollback } = part.jobs;
    expect(job.outputs.deployed).toBe('${{ steps.mark.outputs.deployed }}');
    expect(job.steps.find((step) => step.id === 'mark').run.trim().split('\n').at(-1)).toBe('echo "deployed=1" >>"$GITHUB_OUTPUT"');
    expect(rollback.outputs.back).toBe('${{ steps.back.outputs.back }}');
    const back = rollback.steps.find((step) => step.id === 'back').run;
    expect(back.indexOf('echo "back=1" >>"$GITHUB_OUTPUT"')).toBeGreaterThan(back.indexOf('esac'));
    const ended = ({ deployed = '', rolledBack = '', hold = '0' }) =>
      evaluate(part.on.workflow_call.outputs.ended.value, { jobs: { deploy: { outputs: { deployed } }, rollback: { outputs: { back: rolledBack } } }, inputs: { hold } });
    expect(ended({ deployed: '1' })).toBe('deployed');
    expect(ended({ rolledBack: '1' })).toBe('rolled back');
    expect(ended({ hold: '1' })).toBe('held');
    expect(ended({})).toBe('not rolled back');
  });

  it('hands the Access service token to the steps that reach dev alone', () => {
    const jobs = [...Object.values(deploy.jobs), ...Object.values(part.jobs)];
    const holders = jobs.flatMap((job) => (job.steps ?? []).filter((step) => step.env?.CF_ACCESS_CLIENT_ID));
    expect(holders.length).toBeGreaterThan(0);
    for (const step of holders) {
      expect(step.env).toMatchObject({
        CF_ACCESS_CLIENT_ID: "${{ inputs.env == 'dev' && secrets.CF_ACCESS_CLIENT_ID || '' }}",
        CF_ACCESS_CLIENT_SECRET: "${{ inputs.env == 'dev' && secrets.CF_ACCESS_CLIENT_SECRET || '' }}",
      });
    }
    expect(JSON.stringify(jobs.map((job) => job.env ?? {}))).not.toContain('CF_ACCESS');
  });
});
