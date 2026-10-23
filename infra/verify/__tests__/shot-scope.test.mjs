import { readdirSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { SCREENS as SHOT_LIST } from '../../../apps/admin/scripts/shots.mjs';
import { REPO_ROOT, catalogue } from '../screens.mjs';
import { MODULE_ROUTES, harnessFiles, importersFrom, screensFor, unmappedRoutes } from '../shot-scope.mjs';

const SCREENS = [
  { name: 'overview', path: '/' },
  { name: 'overview-7-days', path: '/?days=7' },
  { name: 'users', path: '/users' },
  { name: 'user-coins', path: '/users/p1/coins' },
  { name: 'wallet', path: '/economy/wallets/p1' },
  { name: 'spend', path: '/spend' },
  { name: 'releases states', path: '/releases' },
];
const ALL = SCREENS.map(({ name }) => name);
const MODULE = 'apps/admin/src/modules';
const IMPORTERS = {
  [`${MODULE}/spend/BudgetCard.tsx`]: [`${MODULE}/spend/SpendPage.tsx`],
  [`${MODULE}/spend/lines.ts`]: [`${MODULE}/spend/SpendPage.tsx`, `${MODULE}/home/SpendPanel.tsx`, `${MODULE}/spend/spend.test.ts`],
  [`${MODULE}/users/search.ts`]: [`${MODULE}/users/UsersPage.tsx`, 'apps/admin/src/components/player/PlayerPicker.tsx'],
  [`${MODULE}/economy/dates.ts`]: [`${MODULE}/economy/WalletPage.tsx`, 'apps/admin/src/test/economy.ts'],
};
const scope = (files, options = {}) =>
  screensFor(files, SCREENS, {
    map: MODULE_ROUTES,
    importers: (file) => (file in IMPORTERS ? [file, ...IMPORTERS[file]] : null),
    packages: ['packages/shared/', 'packages/i18n/', 'packages/config/'],
    harness: ['apps/admin/scripts/shots.mjs', 'apps/admin/scripts/shots-kit.mjs', 'infra/verify/interaction-checks.mjs'],
    ...options,
  });

describe('screensFor: the screens a diff can affect (owner, 17 Oct)', () => {
  it("selects a module's screens for a file only its module reaches, every importing module's screens, every screen for a file shared code reaches, and nothing for a file no screen renders", () => {
    const everything = { everything: true, screens: ALL };
    for (const [file, selected, options] of [
      [`${MODULE}/spend/BudgetCard.tsx`, { everything: false, modules: ['spend'], screens: ['spend'] }],
      [`${MODULE}/spend/lines.ts`, { everything: false, modules: ['home', 'spend'], screens: ['overview', 'overview-7-days', 'spend'] }],
      [`${MODULE}/economy/dates.ts`, { everything: false, modules: ['economy'], screens: ['wallet'] }],
      [`${MODULE}/users/gone.ts`, { everything: false, modules: ['users'], screens: ['users', 'user-coins'] }],
      [`${MODULE}/users/search.ts`, everything],
      ...[
        'apps/admin/src/app/theme.ts',
        'apps/admin/src/components/DataTable.tsx',
        'apps/admin/src/lib/format.ts',
        'apps/admin/src/api/client.ts',
        'apps/admin/src/i18n/en/releases.json',
        'apps/admin/src/main.tsx',
        'apps/admin/vite.config.ts',
        'apps/admin/package.json',
        'packages/shared/src/admin/csv.ts',
        'packages/config/tsconfig.json',
        'pnpm-lock.yaml',
        'apps/admin/scripts/shots-kit.mjs',
        'infra/verify/interaction-checks.mjs',
      ].map((shared) => [shared, everything]),
      [`${MODULE}/spend/BudgetCard.tsx`, everything, { unresolved: ['apps/admin/src/x.ts -> ./y'] }],
    ])
      expect.soft(scope([file], options), `${file}${options ? ' with unresolved imports' : ''}`).toMatchObject(selected);
    for (const file of [
      'apps/edge/src/admin.ts',
      'apps/app/src/app/index.tsx',
      'packages/game-peep/src/rules.ts',
      'infra/verify/qa-plan.mjs',
      '.github/workflows/journeys.yml',
      'apps/admin/e2e/journeys/apps.spec.ts',
      'apps/admin/src/test/render.tsx',
      `${MODULE}/spend/spend.test.ts`,
      'apps/admin/scripts/fixture-server.mjs',
      'apps/admin/README.md',
    ])
      expect.soft(scope([file]), file).toEqual({ everything: false, modules: [], screens: [] });
  });

  it('refuses a screen whose path no module owns, so no screen escapes a scoped run', () => {
    expect(() => scope([`${MODULE}/spend/BudgetCard.tsx`], { map: { ...MODULE_ROUTES, spend: [] } })).toThrow(/spend: \/spend/);
  });
});

describe('MODULE_ROUTES: every admin route belongs to exactly one module', () => {
  const routes = [...catalogue('admin').screens.keys()];

  it('names a route no module owns, and a route two modules own', () => {
    expect(unmappedRoutes([...routes, '/reports/$id'], MODULE_ROUTES)).toEqual(['/reports/$id: no module']);
    expect(unmappedRoutes(routes, { ...MODULE_ROUTES, audit: ['/audit', '/spend'] })).toEqual(['/spend: audit, spend']);
  });

  it('maps every route registered in router.tsx to exactly one module, and has one entry per module folder', () => {
    expect.soft(routes.length, 'routes').toBeGreaterThan(20);
    expect.soft(unmappedRoutes(routes, MODULE_ROUTES), 'routes').toEqual([]);
    const folders = readdirSync(join(REPO_ROOT, MODULE), { withFileTypes: true }).filter((entry) => entry.isDirectory()).map((entry) => entry.name);
    expect.soft(Object.keys(MODULE_ROUTES).sort(), 'module folders').toEqual(folders.sort());
  });

  it("gives releases '/releases' alone, and every screen of the shot list a module of its own", () => {
    expect.soft(MODULE_ROUTES.releases, 'releases').toEqual(['/releases']);
    const paths = SHOT_LIST.map(({ path }) => path);
    expect.soft(paths.length, 'shot list').toBeGreaterThan(20);
    expect.soft(unmappedRoutes(paths, MODULE_ROUTES), 'shot list').toEqual([]);
  }, 30_000);
});

describe('importersFrom: the files that import a file, directly or through others', () => {
  it("follows imports back to every file that reaches the changed one, and stops at the router, so a page renders on its own module's routes", () => {
    const graph = {
      modules: [
        { source: 'a.ts', dependencies: [{ resolved: 'b.ts' }] },
        { source: 'b.ts', dependencies: [{ resolved: 'c.ts' }] },
        { source: 'c.ts', dependencies: [] },
        { source: 'd.ts', dependencies: [{ resolved: 'a.ts' }] },
      ],
    };
    const importers = importersFrom(graph);
    expect.soft(importers('c.ts').sort(), 'c.ts').toEqual(['a.ts', 'b.ts', 'c.ts', 'd.ts']);
    expect.soft(importers('d.ts'), 'd.ts').toEqual(['d.ts']);
    expect.soft(importers('e.ts'), 'e.ts').toBeNull();
    const routed = {
      modules: [
        { source: 'main.tsx', dependencies: [{ resolved: 'router.tsx' }] },
        { source: 'router.tsx', dependencies: [{ resolved: 'spend/SpendPage.tsx' }] },
        { source: 'spend/SpendPage.tsx', dependencies: [{ resolved: 'spend/BudgetCard.tsx' }] },
        { source: 'spend/BudgetCard.tsx', dependencies: [] },
      ],
    };
    expect.soft(importersFrom(routed, ['router.tsx'])('spend/BudgetCard.tsx').sort(), 'the router').toEqual(['spend/BudgetCard.tsx', 'spend/SpendPage.tsx']);
  });
});

describe('harnessFiles: the harness and every file it imports', () => {
  it('reaches the shots kit, the page checks, the proof driver and its admin adapter, and the scope itself', () => {
    const harness = harnessFiles(REPO_ROOT);
    for (const file of [
      'apps/admin/scripts/shots.mjs',
      'apps/admin/scripts/proof-adapter.mjs',
      'infra/verify/proof.mjs',
      'infra/verify/proof-serve.mjs',
      'apps/admin/scripts/shots-kit.mjs',
      'apps/admin/scripts/shots-fixtures.mjs',
      'infra/verify/interaction-checks.mjs',
      'infra/verify/interaction-scan.mjs',
      'infra/verify/shot-scope.mjs',
    ])
      expect(harness, file).toContain(file);
    expect(harness).not.toContain('apps/admin/scripts/fixture-server.mjs');
  });
});
