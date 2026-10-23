/**
 * The workflows around the WALK_SEED_KEY-derived walk accounts (CD-278): the
 * release's seed on dev and its smoke walk reading the key, and the retired
 * `walk-accounts` workflow (CD-272) gone.
 */
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));
const read = (name) => readFileSync(join(WORKFLOWS, name), 'utf8');
const partSteps = parse(read('deploy-part.yml')).jobs.deploy.steps;
const partSeed = partSteps.find((candidate) => candidate.name === 'Seed the walk accounts (never fails the deploy)');

describe('the release and the retired walk-accounts workflow (CD-278)', () => {
  const release = read('release.yml');
  const jobs = parse(release).jobs;

  it("seeds the walk accounts in deploy.yml's api and app parts, after the deploy and before their walks, never failing the deploy, bounded in time; the release seeds nothing itself (owner, 7 Oct; CD-285, CD-478)", () => {
    const at = (name) => partSteps.findIndex((candidate) => candidate.name === name);
    expect.soft(at('Seed the walk accounts (never fails the deploy)')).toBeGreaterThan(partSteps.findIndex((candidate) => String(candidate.uses ?? '').includes('deploy-stack')));
    expect.soft(at('Seed the walk accounts (never fails the deploy)')).toBeLessThan(at('Smoke walk'));
    expect.soft(partSeed.if).toBe("inputs.part != 'admin'");
    expect.soft(partSeed['continue-on-error']).toBe(true);
    expect.soft(partSeed['timeout-minutes']).toBeLessThanOrEqual(3);
    expect.soft(partSeed.run).toContain('timeout 150 node infra/scripts/live-walk-accounts.mjs --seed --api "$API_URL"');
    expect.soft(partSeed.env.WALK_SEED_KEY).toBe('${{ secrets.WALK_SEED_KEY }}');
    expect.soft(release).not.toContain('live-walk-accounts.mjs');
    expect.soft(jobs.smoke.needs).toContain('dev');
  });

  it('hands WALK_SEED_KEY to the smoke walk step alone (no job or workflow env, no other step, no accounts secret), and walk-accounts.yml (CD-272) is retired with no workflow upserting a file', () => {
    for (const job of Object.values(jobs)) expect.soft(JSON.stringify(job.env ?? {})).not.toContain('WALK_SEED_KEY');
    expect.soft(JSON.stringify(parse(release).env ?? {})).not.toContain('WALK_SEED_KEY');
    const holders = Object.values(jobs).flatMap((job) => job.steps ?? []).filter((step) => JSON.stringify(step).includes('WALK_SEED_KEY') && step.env?.WALK_SEED_KEY).map((step) => step.id);
    expect.soft(holders).toEqual(['smoke']);
    expect.soft(release.match(/secrets\.WALK_SEED_KEY/g)).toHaveLength(1);
    const step = jobs.smoke.steps.find((candidate) => candidate.id === 'smoke');
    expect.soft(step.env.WALK_SEED_KEY).toBe('${{ secrets.WALK_SEED_KEY }}');
    expect.soft(step.env.LIVE_WALK_ACCOUNTS).toBeUndefined();
    expect.soft(release).not.toContain('LIVE_WALK_ACCOUNTS');
    expect.soft(release).not.toContain('--accounts-file');
    expect.soft(existsSync(join(WORKFLOWS, 'walk-accounts.yml'))).toBe(false);
    expect.soft(release).not.toContain('--upsert');
  });
});
