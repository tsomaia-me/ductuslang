import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { namedPlan } from '../journeys-named.mjs';
import { journeySelection } from '../qa-plan.mjs';

const CLI = fileURLToPath(new URL('../journeys-named.mjs', import.meta.url));

const FIXTURE = ['apps/app/e2e/journeys/home.spec.ts', 'apps/admin/e2e/journeys/home.spec.ts', 'apps/admin/e2e/journeys/users.spec.ts', 'apps/admin/e2e/journeys/fixtures.ts', 'apps/admin/src/app/nav.ts'];

const picked = (plan) => ({ app: journeySelection(plan, 'app'), admin: journeySelection(plan, 'admin') });

describe('journeys-named: a plan of the journey specs a dispatch names (CD-405)', () => {
  let root;

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'cd405-named-'));
    for (const path of FIXTURE) {
      mkdirSync(dirname(join(root, path)), { recursive: true });
      writeFileSync(join(root, path), '');
    }
  });

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it('runs exactly the named specs, split by spaces, commas or lines, each on its own surface, as paths from that surface, and no journey of a surface it names none of', () => {
    const plan = namedPlan('./apps/admin/e2e/journeys/home.spec.ts, apps/app/e2e/journeys/home.spec.ts\napps/admin/e2e/journeys/home.spec.ts', root);
    expect.soft(picked(plan), 'both surfaces').toEqual({ app: { run: 'some', files: ['e2e/journeys/home.spec.ts'] }, admin: { run: 'some', files: ['e2e/journeys/home.spec.ts'] } });
    expect.soft(picked(namedPlan('apps/admin/e2e/journeys/users.spec.ts', root)).app, 'a surface named none of').toEqual({ run: 'none', files: [] });
  });

  it('refuses a path that is not a journey spec of a surface, or is missing at the ref, naming it, and an empty list', () => {
    for (const bad of ['apps/admin/e2e/journeys/fixtures.ts', 'apps/admin/src/app/nav.ts', 'apps/admin/e2e/journeys/gone.spec.ts', '../elsewhere/apps/admin/e2e/journeys/home.spec.ts', '/apps/admin/e2e/journeys/home.spec.ts']) {
      expect.soft(() => namedPlan(bad, root), bad).toThrow(bad);
    }
    expect.soft(() => namedPlan(' , ', root), 'an empty list').toThrow(/usage/);
  });

  it('prints the plan qa-plan.mjs reads, and exits 2 with the reason on a bad path', () => {
    const ok = spawnSync(process.execPath, [CLI, '--files', 'apps/admin/e2e/journeys/users.spec.ts', '--root', root], { encoding: 'utf8' });
    expect(ok.status, ok.stderr).toBe(0);
    expect(picked(JSON.parse(ok.stdout)).admin).toEqual({ run: 'some', files: ['e2e/journeys/users.spec.ts'] });
    const bad = spawnSync(process.execPath, [CLI, '--files', 'apps/admin/src/app/nav.ts', '--root', root], { encoding: 'utf8' });
    expect(bad.status).toBe(2);
    expect(bad.stdout).toBe('');
    expect(bad.stderr).toContain('apps/admin/src/app/nav.ts');
  });
});
