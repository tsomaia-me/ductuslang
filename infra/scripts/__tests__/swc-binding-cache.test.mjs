/**
 * @swc/core unpacks its native binding on first load, into a cache folder that
 * defaults to the user's (CD-347: a job on wb-release failed there).
 * `SWC_NATIVE_BINDING_CACHE` names an absolute folder instead. Every
 * job that loads swc, directly or through a script it runs, makes one under
 * RUNNER_TEMP before the first step that loads it and exports the variable to
 * the steps after it. The loaders are found in the source, so a new script that
 * reaches swc is held to the same rule.
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const WORKFLOWS = join(ROOT, '.github/workflows');
const VARIABLE = 'SWC_NATIVE_BINDING_CACHE';
const SCRIPT_DIRS = ['infra/scripts', 'infra/verify', 'apps/admin/scripts', 'apps/app/scripts', 'apps/edge/scripts'];
const RUNS_THE_INFRA_SUITE = 'infra/scripts/gate.sh';

const code = (text) =>
  text
    .split('\n')
    .filter((line) => !/^\s*(\/\/|\*|\/\*|#)/.test(line))
    .join('\n');

const scripts = SCRIPT_DIRS.flatMap((dir) =>
  readdirSync(join(ROOT, dir), { withFileTypes: true })
    .filter((entry) => entry.isFile() && /\.(mjs|sh)$/.test(entry.name))
    .map((entry) => join(ROOT, dir, entry.name)),
).map((path) => ({ path, text: code(readFileSync(path, 'utf8')) }));

const mentions = (text, path) => new RegExp(`(^|[^\\w.-])${basename(path).replace(/[.]/g, '\\.')}(?![\\w-])`).test(text);

const reachingSwc = () => {
  const loaders = new Set(scripts.filter(({ text }) => text.includes('@swc/core')).map(({ path }) => path));
  for (let grew = true; grew; ) {
    grew = false;
    for (const { path, text } of scripts) {
      if (loaders.has(path)) continue;
      const imports = [...text.matchAll(/(?:from\s+|import\s*\(\s*)['"](\.[^'"]+)['"]/g)].map((match) => resolve(dirname(path), match[1]));
      if (imports.some((target) => loaders.has(target)) || [...loaders].some((loader) => mentions(text, loader))) {
        loaders.add(path);
        grew = true;
      }
    }
  }
  return [...loaders].map((path) => relative(ROOT, path)).sort();
};

const LOADERS = reachingSwc();

const runsLoader = (step) => {
  const text = code(String(step.run ?? ''));
  return [...LOADERS, RUNS_THE_INFRA_SUITE].some((loader) => mentions(text, loader));
};

const workflows = readdirSync(WORKFLOWS)
  .filter((name) => /\.ya?ml$/.test(name))
  .map((name) => ({ name, jobs: parse(readFileSync(join(WORKFLOWS, name), 'utf8')).jobs }));

const loadingJobs = workflows.flatMap(({ name, jobs }) =>
  Object.entries(jobs)
    .filter(([, job]) => (job.steps ?? []).some(runsLoader))
    .map(([id, job]) => ({ id: `${name}:${id}`, job })),
);

const cacheStep = (step) => {
  const run = String(step.run ?? '');
  const made = /mkdir -p "\$RUNNER_TEMP\/([\w-]+)"/.exec(run)?.[1];
  return made !== undefined && run.includes(`${VARIABLE}=$RUNNER_TEMP/${made}" >>"$GITHUB_ENV"`) ? made : null;
};

describe('the jobs that load @swc/core (CD-347)', () => {
  it('are found: swc is required by screens.mjs, spec-routes.mjs and import-graph.mjs, and reached through the journeys\' selection', () => {
    expect(LOADERS).toEqual(
      expect.arrayContaining([
        'infra/verify/screens.mjs',
        'infra/verify/spec-routes.mjs',
        'infra/verify/import-graph.mjs',
        'infra/verify/qa-plan.mjs',
        'infra/scripts/journeys-local.sh',
      ]),
    );
    expect(loadingJobs.map(({ id }) => id)).toEqual(expect.arrayContaining(['checks.yml:test']));
  });

  it('are the only jobs with that setup step, and no job sets the variable to anything but a folder it made', () => {
    for (const { name, jobs } of workflows) {
      for (const [id, job] of Object.entries(jobs)) {
        const text = JSON.stringify(job);
        if (!text.includes(VARIABLE)) continue;
        expect(loadingJobs.map((loading) => loading.id), `${name}:${id}`).toContain(`${name}:${id}`);
        expect((job.steps ?? []).filter((step) => cacheStep(step) !== null), `${name}:${id}`).toHaveLength(1);
        expect(text.match(new RegExp(`${VARIABLE}=`, 'g')), `${name}:${id}`).toHaveLength(1);
      }
    }
  });

  it('each make a folder under RUNNER_TEMP and export it as SWC_NATIVE_BINDING_CACHE before the first step that loads swc, and the setup step run as written creates the folder and writes the variable to GITHUB_ENV', () => {
    for (const { id, job } of loadingJobs) {
      const first = job.steps.findIndex(runsLoader);
      const setup = job.steps.findIndex((step) => cacheStep(step) !== null);
      expect.soft(setup, `${id}: a step that makes the folder and exports it`).toBeGreaterThanOrEqual(0);
      expect.soft(setup, id).toBeLessThan(first);
      expect.soft(job.env?.[VARIABLE], id).toBeUndefined();
      for (const step of job.steps.slice(0, setup)) expect.soft(String(step.run ?? ''), id).not.toContain(VARIABLE);
      if (setup < 0) continue;
      const temp = mkdtempSync(join(tmpdir(), 'swc-cache-'));
      const runner = join(temp, 'runner');
      mkdirSync(runner);
      const envFile = join(temp, 'github-env');
      writeFileSync(envFile, '');
      const step = job.steps[setup];
      const ran = spawnSync('bash', ['-euo', 'pipefail', '-c', step.run], { env: { PATH: process.env.PATH, RUNNER_TEMP: runner, GITHUB_ENV: envFile }, encoding: 'utf8' });
      expect.soft(ran.status, `${id}: ${ran.stderr}`).toBe(0);
      const folder = join(runner, cacheStep(step));
      expect.soft(existsSync(folder), id).toBe(true);
      expect.soft(readFileSync(envFile, 'utf8'), id).toBe(`${VARIABLE}=${folder}\n`);
      rmSync(temp, { recursive: true, force: true });
    }
  });

  it('is the variable the locked @swc/core reads: its native binding is unpacked under that folder', () => {
    const cache = mkdtempSync(join(tmpdir(), 'swc-binding-'));
    const ran = spawnSync(process.execPath, ['--input-type=commonjs', '-e', "require('@swc/core').parseSync('1', { syntax: 'ecmascript' })"], {
      cwd: join(ROOT, 'infra'),
      env: { PATH: process.env.PATH, [VARIABLE]: join(cache, 'binding') },
      encoding: 'utf8',
    });
    expect(ran.status, ran.stderr).toBe(0);
    expect(readdirSync(join(cache, 'binding'), { recursive: true }).some((entry) => String(entry).endsWith('.node'))).toBe(true);
    rmSync(cache, { recursive: true, force: true });
  });
});
