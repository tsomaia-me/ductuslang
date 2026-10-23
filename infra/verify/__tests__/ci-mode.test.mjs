import { describe, expect, it } from 'vitest';

import { SMOKE_TAG, journeyPlan, smokeFiles, smokeOf } from '../ci-mode.mjs';

const surface = (e2e) => ({ screens: [], unplaceable: [], served: false, e2e, verify: null });
const plan = (app, admin) => ({ screenChange: true, surfaces: { app: surface(app), admin: surface(admin) } });
const SMOKE = { app: ['apps/app/e2e/journeys/sign-in.spec.ts'], admin: ['apps/admin/e2e/journeys/currency-pair-draft.spec.ts'] };

describe('the smoke journeys', () => {
  it('are the files holding a test tagged @smoke', () => {
    const journeys = [
      { file: 'b.spec.ts', title: 'b', screens: [], tags: [SMOKE_TAG, '@screen:/b'] },
      { file: 'a.spec.ts', title: 'a', screens: [], tags: ['@screen:/a'] },
      { file: 'b.spec.ts', title: 'b2', screens: [], tags: [SMOKE_TAG] },
      { file: 'c.spec.ts', title: 'c', screens: null, tags: null },
    ];
    expect(smokeFiles(journeys)).toEqual(['b.spec.ts']);
  });

  it('sign in and play a match to the end on the app, publish a change on the admin', () => {
    expect(smokeOf('app')).toEqual(['apps/app/e2e/journeys/sign-in.spec.ts', 'apps/app/e2e/journeys/wordgrid-game.spec.ts']);
    expect(smokeOf('admin')).toEqual(['apps/admin/e2e/journeys/currency-pair-draft.spec.ts']);
  });
});

describe('the journeys a PR runs', () => {
  const smoke = (name) => SMOKE[name];

  it('are only the smoke journeys, filtered to the smoke tests, on each surface the change reaches, and none on a surface it does not reach', () => {
    const touched = plan({ all: true, files: [] }, { all: false, files: ['apps/admin/e2e/journeys/users.spec.ts'] });
    expect.soft(journeyPlan(touched, smoke), 'both surfaces reached').toEqual({
      ...touched,
      grep: SMOKE_TAG,
      surfaces: { app: surface({ all: false, files: SMOKE.app }), admin: surface({ all: false, files: SMOKE.admin }) },
    });
    const adminOnly = plan({ all: false, files: [] }, { all: false, files: ['apps/admin/e2e/journeys/users.spec.ts'] });
    expect.soft(journeyPlan(adminOnly, smoke).surfaces.app.e2e, 'the app not reached').toEqual({ all: false, files: [] });
  });
});
