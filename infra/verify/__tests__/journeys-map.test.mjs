import { describe, expect, it } from 'vitest';

import { checkJourneys, journeysFor, journeysIn, readJourneys, readPending } from '../journeys.mjs';
import { catalogue } from '../screens.mjs';
import { SURFACE_NAMES } from '../surfaces.mjs';

describe('every screen has a journey and every journey names a screen (CD-322)', () => {
  it('keeps the journey map honest on every surface', () => {
    for (const surface of SURFACE_NAMES) {
      const screens = [...catalogue(surface).screens.keys()];
      const problems = checkJourneys({ surface, screens, journeys: readJourneys(surface), pending: readPending()[surface] ?? [] });
      expect.soft(problems, surface).toEqual([]);
    }
  });
});

describe('the journey map check', () => {
  const screens = ['/', '/store', '/settings'];
  const journey = (title, tagged) => ({ file: 'apps/app/e2e/journeys/a.spec.ts', title, screens: tagged });

  it('passes when every screen has a journey or waits on the pending list, and names a screen with no journey, a journey naming no, an unknown or an unreadable screen, and a pending entry that has a journey or is no screen', () => {
    for (const [name, journeys, pending, problems] of [
      ['every screen covered', [journey('home', ['/', '/store'])], ['/settings'], []],
      ['a screen with no journey', [journey('home', ['/'])], ['/store'], ['app: /settings has no journey']],
      [
        'journeys naming no, an unknown or an unreadable screen',
        [journey('bare', []), journey('lost', ['/nowhere']), journey('built', null)],
        screens,
        [
          'app: apps/app/e2e/journeys/a.spec.ts › bare names no screen (tag it @screen:<screen>)',
          'app: apps/app/e2e/journeys/a.spec.ts › lost names /nowhere, which is not a screen',
          'app: apps/app/e2e/journeys/a.spec.ts › built has a screen tag that is not a string literal',
        ],
      ],
      [
        'pending entries with a journey or no screen',
        [journey('home', ['/'])],
        ['/', '/store', '/settings', '/gone'],
        ['app: / has a journey, so it leaves the pending list', 'app: pending /gone is not a screen'],
      ],
    ])
      expect.soft(checkJourneys({ surface: 'app', screens, journeys, pending }), name).toEqual(problems);
  });
});

describe('reading journeys from a spec', () => {
  const file = 'apps/app/e2e/journeys/store.spec.ts';

  it('reads each test with its screen tags and every tag, a single tag string, and test.only, and marks a tag built at run time as unreadable', () => {
    const source = [
      "import { test } from '@playwright/test';",
      "test('store: buy a booster', { tag: ['@screen:/store', '@slow'] }, async () => {});",
      "test.only('settings', { tag: '@screen:/settings' }, async () => {});",
      "test.describe('group', () => {});",
      "test.beforeAll(async () => {});",
      "test('untagged', async () => {});",
    ].join('\n');
    expect.soft(journeysIn(source, file), 'literal tags').toEqual([
      { file, title: 'store: buy a booster', screens: ['/store'], tags: ['@screen:/store', '@slow'] },
      { file, title: 'settings', screens: ['/settings'], tags: ['@screen:/settings'] },
      { file, title: 'untagged', screens: [], tags: [] },
    ]);
    expect
      .soft(journeysIn("const where = '/store';\ntest('x', { tag: [`@screen:${where}`] }, async () => {});", file), 'a tag built at run time')
      .toEqual([{ file, title: 'x', screens: null, tags: null }]);
  });

  it('selects the files whose journeys cover any of the screens', () => {
    const journeys = [
      { file: 'b.spec.ts', title: 'b', screens: ['/store'] },
      { file: 'a.spec.ts', title: 'a', screens: ['/', '/settings'] },
      { file: 'a.spec.ts', title: 'a2', screens: ['/settings'] },
    ];
    expect(journeysFor(journeys, ['/settings', '/store'])).toEqual(['a.spec.ts', 'b.spec.ts']);
    expect(journeysFor(journeys, ['/play'])).toEqual([]);
  });
});
