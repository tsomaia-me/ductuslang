import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { importGraph } from '../import-graph.mjs';
import { journeySelection, planRelease, planView } from '../qa-plan.mjs';
import { catalogue, navigation, placeOnSurface } from '../screens.mjs';

import { FIXTURE, writeTree } from './screens-fixture.mjs';

describe('which screens a release touches, over a fixture repo (CD-322)', () => {
  let root;
  const app = (paths) => placeOnSurface('app', paths, { root });
  const admin = (paths) => placeOnSurface('admin', paths, { root });
  const plan = (paths) => planRelease(paths, { root });

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'cd322-screens-'));
    writeTree(root, FIXTURE);
    importGraph(root, 'apps/app');
    importGraph(root, 'apps/admin');
  }, 120_000);

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it('lists the screens: app routes by URL without layouts or dev routes, admin leaf routes by full path', () => {
    expect([...catalogue('app', root).screens.keys()].sort()).toEqual(['/', '/game/[id]', '/login', '/settings', '/store']);
    expect([...catalogue('admin', root).screens.keys()].sort()).toEqual(['/', '/users', '/users/$id/coins', '/users/$id/notes']);
    expect(navigation('app', root)).toEqual(['/', '/settings']);
    expect(navigation('admin', root)).toEqual(['/', '/users']);
  });

  it('places a screen change on its screen, a shared component, package or game file on the screens that reach it, and the shell, sign-in, routing and unreached files nowhere', () => {
    expect.soft(app(['apps/app/src/app/(app)/store.tsx']), 'app screen').toMatchObject({ screens: ['/store'], unplaceable: [] });
    expect.soft(admin(['apps/admin/src/modules/users/NotesTab.tsx']), 'admin screen').toMatchObject({ screens: ['/users/$id/notes'], unplaceable: [] });
    expect.soft(app(['apps/app/src/ui/card.tsx']).screens, 'app shared component').toEqual(['/', '/store']);
    expect.soft(admin(['apps/admin/src/components/DataTable.tsx']).screens, 'admin shared component').toEqual(['/users', '/users/$id/coins']);
    expect.soft(admin(['apps/admin/src/modules/users/UserLayout.tsx']).screens, 'admin layout').toEqual(['/users/$id/coins', '/users/$id/notes']);
    expect.soft(app(['packages/shared/src/money.ts']).screens, 'package on app').toEqual(['/store']);
    expect.soft(admin(['packages/shared/src/money.ts']).screens, 'package on admin').toEqual(['/users/$id/coins']);
    expect.soft(app(['apps/app/src/games/wordgrid/board.tsx']), "a game's code").toMatchObject({ screens: ['/game/[id]'], unplaceable: [] });
    expect.soft(app(['apps/app/src/ui/button.tsx']), 'a file deeper in the shell').toMatchObject({ screens: ['/', '/store'], unplaceable: [] });
    for (const path of [
      'apps/app/src/app/_layout.tsx',
      'apps/app/src/ui/tab-bar.tsx',
      'apps/app/src/features/gift-sheet.tsx',
      'apps/app/src/app/(auth)/login.tsx',
      'apps/app/src/navigation/nav-items.ts',
      'apps/app/app.config.ts',
      'apps/app/src/features/gone.tsx',
    ])
      expect.soft(app([path]).unplaceable, path).toEqual([path]);
    for (const path of ['apps/admin/src/app/router.tsx', 'apps/admin/src/modules/users/routes.ts', 'apps/admin/src/main.tsx', 'apps/admin/src/app/Rail.tsx'])
      expect.soft(admin([path]).unplaceable, path).toEqual([path]);
  });

  it('plans the whole suite and the navigation for an unplaceable change, the declaring journeys and their screens for a placed one, a changed journey alone, no journey for a served (owner, 11 Oct) or non-product change, and a surface whole for a file its journeys import (CD-326)', () => {
    const unplaceable = plan(['apps/admin/src/app/router.tsx']);
    expect.soft(unplaceable.screenChange, 'unplaceable').toBe(true);
    expect.soft(unplaceable.surfaces.admin.e2e.all, 'unplaceable').toBe(true);
    expect.soft(unplaceable.surfaces.admin.verify, 'unplaceable').toEqual({ scope: 'navigation', screens: ['/', '/users'] });
    expect.soft(unplaceable.surfaces.app.verify, 'unplaceable').toBeNull();
    expect.soft(unplaceable.surfaces.app.e2e, 'unplaceable').toEqual({ all: false, files: [] });

    const placed = plan(['apps/app/src/ui/card.tsx']);
    expect.soft(placed.screenChange, 'placed').toBe(true);
    expect.soft(placed.surfaces.app.e2e, 'placed').toEqual({ all: false, files: ['apps/app/e2e/journeys/home.spec.ts', 'apps/app/e2e/journeys/store.spec.ts'] });
    expect.soft(placed.surfaces.app.verify, 'placed').toEqual({ scope: 'screens', screens: ['/', '/store'] });
    expect.soft(plan(['apps/app/src/app/(app)/settings/index.tsx']).surfaces.app.e2e, 'placed with no journey').toEqual({ all: false, files: [] });

    const journey = plan(['apps/app/e2e/journeys/store.spec.ts']);
    expect.soft(journey.screenChange, 'changed journey').toBe(false);
    expect.soft(journey.surfaces.app.e2e, 'changed journey').toEqual({ all: false, files: ['apps/app/e2e/journeys/store.spec.ts'] });
    expect.soft(journey.surfaces.app.verify, 'changed journey').toBeNull();

    const api = plan(['apps/edge/src/api.ts']);
    expect.soft(api.screenChange, 'API').toBe(false);
    expect.soft(api.surfaces.app, 'API').toMatchObject({ served: true, e2e: { all: false, files: [] }, verify: null });
    expect.soft(api.surfaces.admin, 'API').toMatchObject({ served: false, e2e: { all: false, files: [] } });
    const worker = plan(['apps/edge/src/admin/app.ts']);
    expect.soft(worker.surfaces.admin.served, 'admin Worker').toBe(true);
    expect.soft(worker.surfaces.app.served, 'admin Worker').toBe(false);
    const shared = plan(['apps/edge/src/wallet/credit.ts', 'apps/edge/migrations/0042_x.sql']);
    expect.soft([shared.surfaces.app.served, shared.surfaces.admin.served, shared.screenChange], 'shared edge code').toEqual([true, true, false]);
    expect.soft(shared.surfaces.app.e2e, 'shared edge code').toEqual({ all: false, files: [] });
    const edgeOnlyPackage = plan(['packages/shared/src/edge-only.ts']);
    expect.soft([edgeOnlyPackage.surfaces.app.served, edgeOnlyPackage.surfaces.admin.served, edgeOnlyPackage.screenChange], 'edge-only package').toEqual([true, true, false]);
    for (const name of ['app', 'admin']) expect.soft(edgeOnlyPackage.surfaces[name], `edge-only package ${name}`).toMatchObject({ e2e: { all: false, files: [] }, verify: null });

    const ridesAlong = plan(['apps/edge/src/api.ts', 'apps/app/src/app/(app)/store.tsx']).surfaces;
    expect.soft(ridesAlong.app.e2e, 'served beside placed').toEqual({ all: false, files: ['apps/app/e2e/journeys/store.spec.ts'] });
    expect.soft(ridesAlong.app.verify, 'served beside placed').toEqual({ scope: 'screens', screens: ['/store'] });

    const checks = plan(['infra/verify/interaction-checks.mjs']);
    expect.soft(checks.screenChange, 'interaction checks').toBe(false);
    expect.soft(checks.surfaces.admin.e2e, 'interaction checks').toEqual({ all: true, files: [] });
    expect.soft(checks.surfaces.app.e2e, 'interaction checks').toEqual({ all: false, files: [] });
    for (const both of ['infra/e2e/access.mjs', 'infra/scripts/access-headers.mjs']) {
      const { surfaces } = plan([both]);
      expect.soft([surfaces.app.e2e, surfaces.admin.e2e], both).toEqual([{ all: true, files: [] }, { all: true, files: [] }]);
    }
    expect.soft(plan(['apps/app/e2e/fixtures.ts', 'infra/e2e/fixtures.ts']).surfaces.app.e2e, 'app fixtures').toEqual({ all: true, files: [] });
    expect.soft(plan(['infra/e2e/fixtures.ts']).surfaces.admin.e2e, 'infra fixtures on admin').toEqual({ all: false, files: [] });

    const nonProduct = plan(['infra/scripts/edge-deploy.sh', '.github/workflows/qa.yml', 'docs/x.md', 'apps/app/src/ui/card.test.tsx', 'apps/edge/test/a.test.ts', 'apps/app/e2e/smoke.spec.ts']);
    expect.soft(nonProduct.screenChange, 'non-product').toBe(false);
    for (const name of ['app', 'admin'])
      expect.soft(nonProduct.surfaces[name], `non-product ${name}`).toMatchObject({ screens: [], unplaceable: [], served: false, e2e: { all: false, files: [] }, verify: null });

    const lockfile = plan(['pnpm-lock.yaml']).surfaces;
    expect.soft(lockfile.app.unplaceable, 'lockfile').toEqual(['pnpm-lock.yaml']);
    expect.soft(lockfile.admin.unplaceable, 'lockfile').toEqual(['pnpm-lock.yaml']);
  });

  it('runs everything with an import graph that could not be built, every product file unplaceable, and with no QA-passed release to diff from', () => {
    const broken = { modules: [], e2e: [], unresolved: ['the import graph could not be built: timed out'] };
    const place = (name, paths, options) => placeOnSurface(name, paths, { ...options, graph: broken });
    for (const name of ['app', 'admin']) {
      const placed = place(name, ['packages/shared/src/money.ts'], { root });
      expect.soft(placed, name).toMatchObject({ screens: [], unplaceable: ['packages/shared/src/money.ts'] });
    }
    expect.soft(place('app', ['apps/app/src/app/(app)/store.tsx'], { root }).unplaceable, 'broken graph').toEqual(['apps/app/src/app/(app)/store.tsx']);
    for (const [label, planned] of [
      ['broken graph', planRelease(['packages/shared/src/money.ts'], { root, place })],
      ['no QA-passed release', planRelease([], { root, everything: true })],
    ]) {
      expect.soft(planned.screenChange, label).toBe(true);
      expect.soft(planned.surfaces.app.e2e.all && planned.surfaces.admin.e2e.all, label).toBe(true);
      expect.soft(planned.surfaces.app.verify.scope, label).toBe('navigation');
    }
  });
});

describe('the real repo (CD-322)', () => {
  it('places a shared admin component on the screens that use it and a page on its own screen, and runs the whole suite of each surface whose journeys import a changed infra file (CD-326)', () => {
    expect.soft(placeOnSurface('admin', ['apps/admin/src/modules/users/CoinsTab.tsx']).screens, 'admin page').toEqual(['/users/$id/coins']);
    expect.soft(placeOnSurface('admin', ['apps/admin/src/components/DataTable.tsx']).screens.length, 'admin shared component').toBeGreaterThan(10);
    expect.soft(placeOnSurface('app', ['apps/app/src/app/(app)/store.tsx']).screens, 'app page').toEqual(['/store']);
    const selected = (path) => Object.fromEntries(Object.entries(planRelease([path]).surfaces).map(([name, surface]) => [name, surface.e2e.all]));
    expect.soft(selected('infra/verify/interaction-checks.mjs').admin, 'interaction checks').toBe(true);
    expect.soft(selected('infra/e2e/access.mjs'), 'access').toEqual({ app: true, admin: true });
    expect.soft(selected('infra/scripts/access-headers.mjs'), 'access headers').toEqual({ app: true, admin: true });
  }, 180_000);
});

describe("the plan's views for the QA job (CD-322)", () => {
  const surface = (e2e, verify = null) => ({ screens: [], unplaceable: [], served: false, e2e, verify });
  const plan = { screenChange: true, surfaces: { app: surface({ all: false, files: ['apps/app/e2e/journeys/store.spec.ts'] }, { scope: 'screens', screens: ['/store'] }), admin: surface({ all: true, files: [] }) } };

  it("says per surface whether journeys run (none, some, all) and whether a verifier does, and lists a surface's selected journey files from its own folder, none for the whole suite", () => {
    expect.soft(planView(plan), 'view').toEqual(['app={"e2e":"some","verify":true}', 'admin={"e2e":"all","verify":false}']);
    expect
      .soft(planView({ surfaces: { app: surface({ all: false, files: [] }), admin: surface({ all: false, files: [] }) } }), 'view of nothing')
      .toEqual(['app={"e2e":"none","verify":false}', 'admin={"e2e":"none","verify":false}']);
    expect.soft(journeySelection(plan, 'app'), 'app selection').toEqual({ run: 'some', files: ['e2e/journeys/store.spec.ts'] });
    expect.soft(journeySelection(plan, 'admin'), 'admin selection').toEqual({ run: 'all', files: [] });
    expect.soft(journeySelection({ surfaces: { app: surface({ all: false, files: [] }) } }, 'app'), 'no selection').toEqual({ run: 'none', files: [] });
  });
});
