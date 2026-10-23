import { execFileSync, spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { PNG } from 'pngjs';
import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';

import { importGraph } from '../import-graph.mjs';
import { proofSlugOf } from '../proof-branch.mjs';
import { ADAPTERS, adminSet, appSet, baseJobsOf, caption, carryOver, compareToBase, cropPairs, isNoise, jobsFor, judgedSet, main, pageFiles, pageModel, publishPage, renderPage, summary, summaryLine } from '../proof.mjs';
import { REPO_ROOT } from '../screens.mjs';
import { importersFrom } from '../shot-scope.mjs';

import { FIXTURE, writeTree } from './screens-fixture.mjs';

const ADMIN_SCREENS = [
  { name: 'home', path: '/' },
  { name: 'users', path: '/users' },
  { name: 'user-coins', path: '/users/6f3a/coins' },
];
const MAP = { home: ['/'], users: ['/users'] };
const reasons = (set) => Object.fromEntries(set.map(({ surface, id, reason }) => [`${surface}:${id}`, reason]));

describe('the screen set comes from the diff, each screen with its reason', () => {
  let root;
  let scope;

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'proof-set-'));
    writeTree(root, FIXTURE);
    importGraph(root, 'apps/app');
    const graph = importGraph(root, 'apps/admin');
    scope = { importers: importersFrom(graph, ['apps/admin/src/app/router.tsx']), packages: [], harness: [], unresolved: graph.unresolved };
  }, 120_000);

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it('places a changed route as changed, a shared import as via that file, an unplaceable change on every screen, an admin route component, and a walk with its screen’s reason', () => {
    expect.soft(reasons(appSet(['apps/app/src/app/(app)/store.tsx', 'apps/app/src/ui/card.tsx'], { root })), 'app route and shared import').toEqual({
      'app:/': 'via apps/app/src/ui/card.tsx',
      'app:/store': 'changed',
    });
    const reached = appSet(['apps/app/src/ui/card.tsx', 'apps/app/src/features/store/price.tsx'], { root });
    expect
      .soft(Object.fromEntries(reached.map(({ id, reason, reachers }) => [id, [reason, reachers]])), 'a screen reached by two changed files')
      .toEqual({ '/': ['via apps/app/src/ui/card.tsx', ['apps/app/src/ui/card.tsx']], '/store': ['via apps/app/src/ui/card.tsx', ['apps/app/src/ui/card.tsx', 'apps/app/src/features/store/price.tsx']] });
    const unplaceable = appSet(['apps/app/src/navigation/nav-items.ts'], { root });
    expect.soft(unplaceable.map(({ id }) => id), 'unplaceable').toEqual(['/', '/game/[id]', '/login', '/settings', '/store']);
    expect.soft(new Set(unplaceable.map(({ reason }) => reason)), 'unplaceable').toEqual(new Set(['via apps/app/src/navigation/nav-items.ts']));
    expect.soft(reasons(adminSet(['apps/admin/src/modules/users/UsersPage.tsx'], ADMIN_SCREENS, [], { root, scope, map: MAP })), 'admin route component').toEqual({
      'admin:users': 'changed',
      'admin:user-coins': 'via apps/admin/src/modules/users/UsersPage.tsx',
    });
    expect
      .soft(reasons(adminSet(['apps/admin/src/components/DataTable.tsx', 'apps/admin/src/modules/users/CoinsTab.tsx'], ADMIN_SCREENS, [], { root, scope, map: MAP })), 'admin shared component')
      .toEqual({
        'admin:home': 'via apps/admin/src/components/DataTable.tsx',
        'admin:users': 'via apps/admin/src/components/DataTable.tsx',
        'admin:user-coins': 'changed',
      });
    const walked = adminSet(['apps/admin/src/modules/users/UsersPage.tsx'], ADMIN_SCREENS, [{ name: 'users walk', prefix: 'users' }, { name: 'home walk', prefix: 'home' }], { root, scope, map: MAP });
    expect.soft(reasons(walked), 'walk').toEqual({ 'admin:users': 'changed', 'admin:user-coins': 'via apps/admin/src/modules/users/UsersPage.tsx', 'admin:users walk': 'changed' });
    const twice = adminSet(['apps/admin/src/components/DataTable.tsx', 'apps/admin/src/modules/home/HomePage.tsx'], ADMIN_SCREENS, [{ name: 'users walk', prefix: 'users' }], { root, scope, map: MAP });
    expect
      .soft(Object.fromEntries(twice.map(({ id, reachers }) => [id, reachers])), 'admin screens reached by two changed files')
      .toEqual({
        home: ['apps/admin/src/components/DataTable.tsx', 'apps/admin/src/modules/home/HomePage.tsx'],
        users: ['apps/admin/src/components/DataTable.tsx'],
        'user-coins': ['apps/admin/src/components/DataTable.tsx'],
        'users walk': ['apps/admin/src/components/DataTable.tsx'],
      });
  });
});

const SCREEN = { surface: 'app', id: '/store', family: 'store', reason: 'changed' };
const shot = (fields) => ({
  job: { screen: SCREEN, width: 390, theme: 'dark', state: 'default' },
  state: 'default',
  console: 0,
  requests: 1,
  ms: 812,
  file: 'app-store-390-dark-default.png',
  bytes: 48213,
  hash: '3f2a9c1d02ab',
  ...fields,
});

describe('the summary is one line per shot and a total', () => {
  it('names each field of a shot, marks same, via and noise shots, faults a scanned shot, and ends with the counts and the page', () => {
    expect.soft(summaryLine(shot({})), 'OK line').toBe('OK app:/store 390 dark "default" console=0 requests=1 bytes=48213 hash=3f2a9c1d02ab ms=812');
    expect.soft(summaryLine(shot({ error: 'the page never settled' })), 'FAIL line').toBe('FAIL app:/store 390 dark "default" console=0 requests=1 bytes=48213 hash=3f2a9c1d02ab ms=812 error="the page never settled"');

    const repeated = summary([shot({}), shot({ state: 'error', file: 'b.png', error: 'no shot within 10000 ms', hash: null, bytes: 0 }), shot({ state: 'empty', file: 'c.png' })], {
      dropped: 2,
      seconds: 41.23,
      page: '.proof/README.md',
    }).split('\n');
    expect.soft(repeated[2], 'same bytes').toMatch(/^OK app:\/store 390 dark "empty" .* same=app-store-390-dark-default\.png$/);
    expect
      .soft(repeated[3], 'same bytes total')
      .toBe('proof: 3 shots, 2 OK, 1 FAIL, 2 states the screens do not have, 0 same=base, 0 differs, 0 noise, 0 judged; scan: none; base: 0 shots, 0 FAIL; 41.2 s; page .proof/README.md');

    const difference = { changed: 1312, box: { x: 4, y: 80, w: 382, h: 40 }, sizes: null };
    const via = summary([shot({ mark: 'same' }), shot({ state: 'error', file: 'b.png', mark: 'differs', difference, judged: true }), shot({ state: 'empty', file: 'c.png', hash: 'aaaaaaaaaaaa', mark: 'noise', difference: { ...difference, changed: 3 } })], {
      base: [shot({}), shot({ error: 'no shot within 10000 ms', hash: null })],
      seconds: 9,
      page: '.proof/README.md',
    }).split('\n');
    expect.soft(via[0], 'same=base').toMatch(/ ms=812 same=base$/);
    expect.soft(via[1], 'differs').toMatch(/ ms=812 same=app-store-390-dark-default\.png differs diff=1312px@4,80,382x40 judged$/);
    expect.soft(via[2], 'noise').toMatch(/ ms=812 noise diff=3px@4,80,382x40$/);
    expect
      .soft(via[3], 'via total')
      .toBe('proof: 3 shots, 3 OK, 0 FAIL, 0 states the screens do not have, 1 same=base, 1 differs, 1 noise, 1 judged; scan: none; base: 2 shots, 1 FAIL; 9.0 s; page .proof/README.md');

    const scan = [
      { check: 'contrast', element: 'td "Pending"', detail: 'text contrast 2.10:1 is under 4.5:1' },
      { check: 'target', element: 'button "x"', detail: 'the control is 20x20px, under the 44px touch size' },
    ];
    const scanned = summary([shot({ scan }), shot({ state: 'error', file: 'b.png', scan: [scan[0]] })], { seconds: 1, page: 'p' }).split('\n');
    expect
      .soft(scanned[0], 'scan line')
      .toBe(
        'FAIL app:/store 390 dark "default" console=0 requests=1 bytes=48213 hash=3f2a9c1d02ab ms=812 scan="contrast td \\"Pending\\": text contrast 2.10:1 is under 4.5:1 | target button \\"x\\": the control is 20x20px, under the 44px touch size"',
      );
    expect.soft(scanned[2], 'scan total').toContain('0 OK, 2 FAIL');
    expect.soft(scanned[2], 'scan total').toContain('; scan: 2 contrast, 1 target; ');
  });
});

describe('a via screen’s shots are compared with the base’s, a changed screen’s are not', () => {
  const VIA_SCREEN = { surface: 'app', id: '/', family: 'home', reason: 'via apps/app/src/ui/card.tsx' };
  const job = (screen, fields = {}) => ({ screen, width: 390, theme: 'dark', state: 'default', ...fields });
  const at = (theJob, fields = {}) => shot({ job: theJob, state: theJob.state, ...fields });

  it('marks a via shot same when its base shot has its hash, differs otherwise, and leaves changed and failed shots unmarked', () => {
    const plain = job(VIA_SCREEN);
    const dark = job(VIA_SCREEN, { theme: 'light' });
    const broken = job(VIA_SCREEN, { state: 'error' });
    const unshot = job(VIA_SCREEN, { state: 'empty' });
    const changed = job(SCREEN);
    const failedHead = job(VIA_SCREEN, { width: 1440 });
    const baseFail = at(broken, { hash: null, file: null, error: 'no shot within 10000 ms' });
    const marked = compareToBase(
      [at(plain), at(dark, { hash: 'bbbbbbbbbbbb' }), at(broken), at(unshot), at(changed, { hash: 'cccccccccccc' }), at(failedHead, { hash: null, error: 'x' })],
      [at(plain), at(dark), baseFail, at(changed)],
    );
    expect(marked.map(({ mark }) => mark ?? null)).toEqual(['same', 'differs', 'differs', 'differs', null, null]);
    expect(marked[1].before.hash).toBe('3f2a9c1d02ab');
    expect(marked[2].before).toBe(baseFail);
    expect(marked[3].before).toBeNull();
  });
});

describe('the scan faults only what the diff touched', () => {
  const VIA_SCREEN = { surface: 'app', id: '/', family: 'home', reason: 'via apps/app/src/ui/card.tsx' };
  const finding = { check: 'contrast', element: 'p "faint"', detail: 'text contrast 2.10:1 is under 4.5:1' };

  it('drops the findings of a same=base shot and keeps those of a differs, a noise and a changed shot', () => {
    const at = (screen, fields) => shot({ job: { screen, width: 390, theme: 'dark', state: 'default' }, scan: [finding], ...fields });
    const same = at(VIA_SCREEN, {});
    const differs = at(VIA_SCREEN, { job: { ...same.job, theme: 'light' }, hash: 'bbbbbbbbbbbb' });
    const noise = at(VIA_SCREEN, { job: { ...same.job, width: 1440 }, hash: 'cccccccccccc' });
    const changed = at(SCREEN, {});
    const base = [at(VIA_SCREEN, {}), at(VIA_SCREEN, { job: differs.job }), at(VIA_SCREEN, { job: noise.job })];
    const measure = (head) => (head.hash === 'cccccccccccc' ? { changed: 0, box: null, sizes: null } : { changed: 900, box: null, sizes: null });
    const marked = compareToBase([same, differs, noise, changed], base, measure);
    expect(marked.map(({ mark }) => mark ?? null)).toEqual(['same', 'differs', 'noise', null]);
    expect(marked.map((one) => one.scan?.length ?? 0)).toEqual([0, 1, 1, 1]);
    expect(summaryLine(marked[0])).toMatch(/^OK .* same=base$/);
  });
});

describe('a difference too small to see is noise, not judged', () => {
  const VIA_SCREEN = { surface: 'app', id: '/', family: 'home', reason: 'via apps/app/src/ui/card.tsx' };
  const theJob = { screen: VIA_SCREEN, width: 390, theme: 'dark', state: 'default' };
  const box = { x: 0, y: 0, w: 1, h: 1 };

  it('classes a difference of at most 16 pixels past the tolerance, at the same size, as noise and every other as differs, measuring each pair whose hashes differ', () => {
    expect.soft(isNoise({ changed: 16, box, sizes: null }), '16 pixels').toBe(true);
    expect.soft(isNoise({ changed: 17, box, sizes: null }), '17 pixels').toBe(false);
    expect.soft(isNoise({ changed: 0, box: null, sizes: '390×900 → 390×901' }), 'another size').toBe(false);

    const measured = [];
    const measure = (head, base) => {
      measured.push([head.hash, base.hash]);
      return head.hash === 'aaaaaaaaaaaa' ? { changed: 0, box: null, sizes: null } : { changed: 40, box, sizes: null };
    };
    const marked = compareToBase([shot({ job: theJob, hash: 'aaaaaaaaaaaa' }), shot({ job: { ...theJob, theme: 'light' }, hash: 'bbbbbbbbbbbb' })], [shot({ job: theJob }), shot({ job: { ...theJob, theme: 'light' } })], measure);
    expect.soft(marked.map(({ mark }) => mark), 'marks').toEqual(['noise', 'differs']);
    expect.soft(marked[1].difference, 'difference').toEqual({ changed: 40, box, sizes: null });
    expect.soft(measured, 'measured pairs').toEqual([['aaaaaaaaaaaa', '3f2a9c1d02ab'], ['bbbbbbbbbbbb', '3f2a9c1d02ab']]);
  });
});

describe('the judged set is the default state of each new or changed screen and one differs pair per changed shared file, all at the judged views', () => {
  const screen = (id, reason, surface = 'app') => ({ surface, id, family: id, reason, reachers: reason.startsWith('via ') ? [reason.slice('via '.length)] : [] });
  const at = (theScreen, fields = {}) => {
    const { width = 390, theme = 'light', state = 'default', ...rest } = fields;
    return shot({ job: { screen: theScreen, width, theme, state }, state, ...rest });
  };
  const grid = (theScreen, states) => states.flatMap((state) => [390, 1440].flatMap((width) => ['light', 'dark'].map((theme) => at(theScreen, { width, theme, state }))));
  const views = (shots) => judgedSet(shots).filter((one) => one.judged).map((one) => [one.job.screen.id, one.state, one.job.width, one.job.theme]);
  const pair = (theScreen, changed, fields = {}) => at(theScreen, { mark: 'differs', difference: { changed, box: null, sizes: null }, ...fields });

  it('judges the default state alone of a new and a changed screen, the app at 390 and 1440 in light and the admin at 1440 dark, else its first state or every width it was shot at, never a via screen for itself, and for each shared file only the pair at those views it reaches with the most changed pixels, the first on a tie', () => {
    expect.soft(views(grid(screen('/store', 'new'), ['default', 'empty', 'error'])), 'new app screen').toEqual([
      ['/store', 'default', 390, 'light'],
      ['/store', 'default', 1440, 'light'],
    ]);
    expect.soft(views(grid(screen('/settings', 'changed'), ['default', 'error'])), 'changed app screen').toEqual([
      ['/settings', 'default', 390, 'light'],
      ['/settings', 'default', 1440, 'light'],
    ]);
    expect.soft(views(grid(screen('users', 'changed', 'admin'), ['default', 'empty'])), 'changed admin screen').toEqual([['users', 'default', 1440, 'dark']]);
    expect.soft(views([at(screen('/new', 'new'), { file: null, error: 'x' })]), 'fileless').toEqual([]);
    const walk = screen('users walk', 'changed', 'admin');
    expect
      .soft(views(['walk-row-opened', 'walk-saved'].flatMap((state) => [1440, 390].map((width) => at(walk, { width, theme: 'dark', state })))), 'a walk, with no default state')
      .toEqual([['users walk', 'walk-row-opened', 1440, 'dark']]);
    const phone = screen('coins-phone', 'new', 'admin');
    expect.soft(views(['default', 'empty'].map((state) => at(phone, { width: 390, theme: 'dark', state }))), 'a phone-only admin entry').toEqual([['coins-phone', 'default', 390, 'dark']]);

    const home = screen('/', 'via apps/app/src/ui/card.tsx');
    const game = screen('/game/[id]', 'via apps/app/src/ui/card.tsx');
    const chat = screen('/chat', 'via apps/app/src/ui/theme/tokens.ts');
    const quiet = screen('/profile', 'via apps/app/src/ui/badge.tsx');
    const spend = screen('spend', 'via apps/admin/src/components/Money.tsx', 'admin');
    const both = { ...screen('/both', 'via apps/app/src/ui/card.tsx'), reachers: ['apps/app/src/ui/card.tsx', 'apps/app/src/ui/badge.tsx'] };
    expect.soft(views(grid(home, ['default', 'error'])), 'via screen with no differs').toEqual([]);
    expect
      .soft(
        views([
          pair(game, 5000, { width: 1440, theme: 'dark' }),
          pair(home, 900),
          pair(game, 1312, { width: 1440, state: 'error' }),
          pair(home, 1312, { state: 'error' }),
          pair(chat, 40, { theme: 'dark' }),
          at(chat, { mark: 'noise', difference: { changed: 3, box: null, sizes: null } }),
          at(quiet, { mark: 'noise', difference: { changed: 5000, box: null, sizes: null } }),
          at(quiet, { mark: 'same' }),
          pair(spend, 9000, { theme: 'dark' }),
          pair(spend, 300, { width: 1440, theme: 'dark' }),
          pair(spend, 8000, { width: 1440, theme: 'light' }),
          pair(both, 700),
        ]),
        'one pair per shared file, at the judged views',
      )
      .toEqual([
        ['/game/[id]', 'error', 1440, 'light'],
        ['spend', 'default', 1440, 'dark'],
        ['/both', 'default', 390, 'light'],
      ]);
  });
});

describe('the page is a README of the judged set, each judged differs pair side by side, every other differs pair as text, and the push carries only what it shows', () => {
  const via = { surface: 'app', id: '/', family: 'home', reason: 'via apps/app/src/ui/card.tsx' };
  const of = (fields) => shot({ job: { screen: via, width: 390, theme: 'dark', state: fields.state }, ...fields });
  const before = of({ state: 'error', file: 'app-home-390-dark-error.png', hash: '000000000000' });
  const tokens = { surface: 'app', id: '/settings', family: 'settings', reason: 'via apps/app/src/ui/theme/tokens.ts' };
  const chat = { ...via, id: '/chat', family: 'chat' };
  const profile = { ...via, id: '/profile', family: 'profile' };
  const scan = [{ check: 'contrast', element: 'p "a"', detail: 'under 4.5:1' }, { check: 'text', element: 'p "b"', detail: 'shows "NaN"' }];
  const shots = [
    shot({ judged: true }),
    shot({ state: 'empty', file: 'app-store-390-dark-empty.png' }),
    of({ state: 'default', file: 'app-home-390-dark-default.png', mark: 'same' }),
    of({ state: 'error', file: 'app-home-390-dark-error.png', mark: 'differs', before, judged: true, scan, crop: { x: 0, y: 202, w: 78, h: 98 } }),
    ...[chat, chat, profile, tokens].map((screen, index) => shot({ job: { screen, width: 390, theme: 'dark', state: 'default' }, state: ['default', 'error', 'default', 'default'][index], mark: 'same' })),
    of({ state: 'empty', file: 'app-home-390-dark-empty.png', mark: 'differs', before: of({ state: 'empty', file: 'app-home-390-dark-empty.png' }), difference: { changed: 900, box: { x: 1, y: 2, w: 3, h: 4 }, sizes: null } }),
    of({ state: 'loading', file: 'app-home-390-dark-loading.png', mark: 'differs', before: null, difference: null }),
  ];
  const model = pageModel({ head: 'abcdef0123456789', base: '0123456789abcdef', dirty: true, round: 2, fixtures: { app: 'the API mock' }, set: [SCREEN, via, chat, profile, tokens], shots });
  const page = renderPage(model);

  it('heads the page with the sha, base, round, fixture and the set with reasons, shows each judged shot by its relative path under its caption, pairs a judged differs shot with its base shot in a two-column table, and lists every other differs pair as text with its difference', () => {
    expect.soft(page).toContain('# Proof round 2 at abcdef0\n');
    expect.soft(page).toContain('- **Head**: abcdef0123456789 with uncommitted changes\n');
    expect.soft(page).toContain('- **Base**: 0123456789abcdef\n');
    expect.soft(page).toContain('- **Fixture, app**: the API mock\n');
    expect.soft(page).toContain('- **Shots**: 10, 1 failed, 5 same as the base and 0 noise, 3 differ from the base, 2 judged and shown\n');
    expect
      .soft(page)
      .toContain('- app:/store: changed · round 2\n- app:/: via apps/app/src/ui/card.tsx; 1 same=base, 3 differs, 0 noise · round 2\n- via apps/app/src/ui/card.tsx: 2 screens same as base\n- via apps/app/src/ui/theme/tokens.ts: 1 screen same as base\n');
    expect.soft(page).not.toContain('- app:/chat');
    expect.soft(model.sameAsBase).toEqual({ 'apps/app/src/ui/card.tsx': 2, 'apps/app/src/ui/theme/tokens.ts': 1 });
    expect.soft(page).toContain('## Differs, not judged\n\n- app:/ · 390 · dark · empty · diff=900px@1,2,3x4\n- app:/ · 390 · dark · loading · not measured');
    expect.soft(page).not.toContain('app-home-390-dark-empty.png');
    expect.soft(page).not.toContain('app-home-390-dark-loading.png');
    expect.soft(caption(shot({}), 2)).toBe('app:/store · 390 · dark · default · round 2');
    expect.soft(page).toContain('## app · store\n\n**judged · app:/store · 390 · dark · default · round 2**\n\n<img src="shots/app-store-390-dark-default.png" alt="judged · app:/store · 390 · dark · default · round 2">\n');
    const pair = page.split('\n').filter((line) => line.startsWith('| '));
    expect.soft(pair).toHaveLength(3);
    expect.soft(pair.map((line) => line.split(/(?<!\\)\|/).length - 2)).toEqual([2, 2, 2]);
    expect.soft(pair[0]).toContain('FAIL: contrast p &quot;a&quot;: under 4.5:1 &#124; text p &quot;b&quot;: shows &quot;NaN&quot; · judged · differs · app:/ · 390 · dark · error · round 2 · crop 0,202,78x98');
    expect.soft(pair[0]).toContain('| base 0123456 · app:/ · 390 · dark · error · round 2 · crop 0,202,78x98 |');
    expect.soft(pair[2]).toMatch(/^\| <img src="shots\/app-home-390-dark-error\.png" alt="[^"]*"> \| <img src="base\/app-home-390-dark-error\.png" alt="[^"]*"> \|$/);
    expect.soft(page).not.toContain('app-store-390-dark-empty.png');
    expect.soft(page).not.toContain('app-home-390-dark-default.png');
  });

  it('pushes the judged shots and the base shot of each differs pair, and no other', () => {
    expect(pageFiles(model)).toEqual(['shots/app-store-390-dark-default.png', 'shots/app-home-390-dark-error.png', 'base/app-home-390-dark-error.png']);
  });

  it('keeps on a fix round each screen the round did not shoot, with its files and its entry marked with the round that shot it, and recounts the screens same as base', () => {
    const settings = { surface: 'app', id: '/settings', family: 'settings', reason: 'changed' };
    const at = (screen, round) => shot({ job: { screen, width: 390, theme: 'light', state: 'default' }, file: `${slug(screen)}-${round}.png`, judged: true });
    const slug = (screen) => screen.id.slice(1);
    const fields = { base: 'b', dirty: false, fixtures: {} };
    const home = { surface: 'app', id: '/', family: 'home', reason: 'via apps/app/src/ui/card.tsx' };
    const before = pageModel({ ...fields, head: 'aaaaaaa', round: 0, set: [SCREEN, settings, home], shots: [at(SCREEN, 0), at(settings, 0), shot({ job: { screen: home, width: 390, theme: 'light', state: 'default' }, mark: 'same' })] });
    const fix = pageModel({ ...fields, head: 'bbbbbbb', round: 1, set: [SCREEN], shots: [at(SCREEN, 1)] });
    const merged = carryOver(before, fix);
    const text = renderPage(merged);
    expect.soft(text).toContain('# Proof round 1 at bbbbbbb\n');
    expect.soft(text).toContain('- app:/store: changed · round 1\n- app:/settings: changed · round 0\n- via apps/app/src/ui/card.tsx: 1 screen same as base');
    expect.soft(merged.sameAsBase).toEqual({ 'apps/app/src/ui/card.tsx': 1 });
    expect.soft(text).toContain('**judged · app:/settings · 390 · light · default · round 0**');
    expect.soft(text).not.toContain('store-0.png');
    expect.soft(pageFiles(merged)).toEqual(['shots/store-1.png', 'shots/settings-0.png']);
  });
});

describe('a judged differs pair is cropped to its difference and a margin, the same box on both shots, every pixel inside kept', () => {
  let dirs;
  const image = (width, height) => {
    const png = new PNG({ width, height });
    randomBytes(width * height * 4).copy(png.data);
    return PNG.sync.write(png, { colorType: 2 });
  };
  const region = (buffer, { x, y, w, h }) => {
    const png = PNG.sync.read(buffer);
    return Array.from({ length: h }, (_, row) => png.data.subarray(((y + row) * png.width + x) * 4, ((y + row) * png.width + x + w) * 4).toString('hex')).join('');
  };

  beforeAll(() => {
    const root = mkdtempSync(join(tmpdir(), 'proof-crop-'));
    dirs = { root, head: join(root, 'shots'), base: join(root, 'base') };
    for (const folder of [dirs.head, dirs.base]) mkdirSync(folder);
    for (const [file, height] of [['a.png', 300], ['b.png', 300], ['c.png', 300], ['d.png', 300], ['e.png', 300]]) writeFileSync(join(dirs.head, file), image(200, height));
    for (const [file, height] of [['a.png', 300], ['b.png', 300], ['c.png', 320], ['d.png', 300], ['e.png', 300]]) writeFileSync(join(dirs.base, file), image(200, height));
  });

  afterAll(() => rmSync(dirs.root, { recursive: true, force: true }));

  it('crops both shots of a judged pair to the difference box plus 48 px clamped to the image, keeps the file of a crop that spans the whole image, and leaves an unjudged pair and a pair of two sizes whole', () => {
    const original = Object.fromEntries(['a.png', 'b.png', 'c.png', 'd.png', 'e.png'].flatMap((file) => [[`shots/${file}`, readFileSync(join(dirs.head, file))], [`base/${file}`, readFileSync(join(dirs.base, file))]]));
    const pairOf = (file, fields) => shot({ file, mark: 'differs', before: shot({ file }), difference: { changed: 600, box: { x: 10, y: 250, w: 20, h: 30 }, sizes: null }, ...fields });
    const cropped = cropPairs([pairOf('a.png', { judged: true }), pairOf('b.png', { judged: false }), pairOf('c.png', { judged: true, difference: { changed: 6000, box: { x: 0, y: 0, w: 200, h: 320 }, sizes: '200×320 → 200×300' } }), pairOf('d.png', { judged: true, difference: { changed: 600, box: { x: 170, y: 10, w: 20, h: 30 }, sizes: null } }), pairOf('e.png', { judged: true, difference: { changed: 600, box: { x: 40, y: 40, w: 120, h: 220 }, sizes: null } })], dirs);
    const boxes = { 'a.png': { x: 0, y: 202, w: 78, h: 98 }, 'd.png': { x: 122, y: 0, w: 78, h: 88 } };
    expect.soft(cropped.map((one) => one.crop ?? null), 'boxes').toEqual([boxes['a.png'], null, null, boxes['d.png'], { x: 0, y: 0, w: 200, h: 300 }]);
    for (const [file, box] of Object.entries(boxes)) for (const side of ['shots', 'base']) {
      const now = readFileSync(join(dirs.root, side, file));
      expect.soft([PNG.sync.read(now).width, PNG.sync.read(now).height], `${side}/${file} size`).toEqual([box.w, box.h]);
      expect.soft(region(now, { x: 0, y: 0, w: box.w, h: box.h }) === region(original[`${side}/${file}`], box), `${side}/${file} pixels`).toBe(true);
    }
    for (const side of ['shots', 'base']) {
      for (const file of ['b.png', 'c.png', 'e.png']) expect.soft(readFileSync(join(dirs.root, side, file)).equals(original[`${side}/${file}`]), `${side}/${file} whole`).toBe(true);
    }
  });
});

describe('the proof shoots each screen’s states and operates nothing', () => {
  it('shoots each screen’s states at both widths in each theme, every default first, and a sample theme once', () => {
    const adapters = { app: { themes: ['light', 'dark'], states: () => ['default', 'error'] }, admin: { themes: ['dark'], sample: 'light', states: () => ['default'] } };
    const admin = { surface: 'admin', id: 'spend', family: 'spend', reason: 'changed' };
    const label = ({ screen, width, theme, state }) => `${screen.surface} ${width} ${theme} ${state}`;
    expect(jobsFor([SCREEN, admin], adapters).map(label)).toEqual([
      'app 1440 light default',
      'app 1440 dark default',
      'app 390 light default',
      'app 390 dark default',
      'admin 1440 dark default',
      'admin 390 dark default',
      'admin 1440 light default',
      'app 1440 light error',
      'app 1440 dark error',
      'app 390 light error',
      'app 390 dark error',
    ]);
  });
});

describe('the proof branch is the PR head branch without claude/', () => {
  it.each([
    ['claude/cd-440-admin-restructure', 'cd-440-admin-restructure'],
    ['claude/proof-rework', 'proof-rework'],
    ['claude/tic-020-hotfix-payout-retry', 'tic-020-hotfix-payout-retry'],
    ['claude/', null],
    ['claude/a/b', null],
    ['claude/-x', null],
    ['feature/claude/x', null],
    ['main', null],
    ['HEAD', null],
  ])('%s gives %s', (branch, slug) => {
    expect(proofSlugOf(branch)).toBe(slug);
  });
});

function fakeBrowser() {
  const page = { on: () => {} };
  const context = { setDefaultTimeout: () => {}, route: async () => {}, newPage: async () => page, close: async () => {} };
  return { newContext: async () => context, close: async () => {} };
}

const fakeAdmin = (show) => ({
  themes: ['dark'],
  FIXTURE: 'fake fixtures',
  SCREENS: [{ name: 'spend', path: '/spend' }],
  WALKS: [],
  states: () => ['default'],
  serve: vi.fn(async () => ({ origin: 'http://localhost:1', stop: vi.fn(async () => {}) })),
  open: async () => ({ contextOptions: () => ({}), prepare: async () => {}, show, capture: async () => Buffer.from('png') }),
});

const repo = (prefix, files, change) => {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  writeTree(dir, files);
  const git = (...args) => execFileSync('git', args, { cwd: dir, stdio: 'ignore' });
  const commit = (message) => git('-c', 'user.email=a@b', '-c', 'user.name=a', 'commit', '-qam', message);
  git('init', '-q');
  git('checkout', '-qb', BRANCH);
  git('add', '.');
  commit('one');
  if (change) {
    writeFileSync(join(dir, change), `${readFileSync(join(dir, change), 'utf8')}\n`);
    commit('two');
  }
  return dir;
};
const worktreesOf = (dir) => execFileSync('git', ['worktree', 'list', '--porcelain'], { cwd: dir, encoding: 'utf8' }).match(/^worktree /gm).length;
const shootAll = async (_page, job, take) => {
  if (take) await take(job.state);
};
const fakeApp = (show) => ({ ...fakeAdmin(show), themes: ['light'], SCREENS: undefined });
const BRANCH = 'claude/tic-1-fixture';
const recorder = () => {
  const pushed = [];
  return { pushed, publish: async (page) => (pushed.push(page), `https://github.com/o/r/tree/${page.branch}`) };
};

describe('a run exits non-zero on any FAIL', () => {
  let root;
  const checkouts = [];
  const install = async (dir) => {
    checkouts.push(dir);
  };

  beforeAll(() => {
    root = repo('proof-run-', FIXTURE, 'apps/app/src/app/(app)/store.tsx');
  }, 120_000);

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const clean = async () => async () => [];
  const run = async (show, app = fakeApp(show), { extra = [], scanner = clean } = {}) => {
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { pushed, publish } = recorder();
    try {
      const code = await main(['--base', 'HEAD~1', '--head', 'HEAD', ...extra], { root, load: async () => ({ app }), launch: async () => fakeBrowser(), install, scanner, publish });
      return { code, printed: log.mock.calls.flat().join('\n'), app, pushed };
    } finally {
      log.mockRestore();
      error.mockRestore();
    }
  };

  it('returns 1 when a shot fails and pushes nothing, 0 when every shot is taken, stops the server either way, writes the page and the shots, pushes the page to the branch’s proof branch as its last step, and makes no base checkout, install or server when the set has no via screen', async () => {
    const failed = await run(async () => {
      throw new Error('the page never settled');
    });
    expect.soft(failed.code, 'failed').toBe(1);
    expect.soft(failed.printed, 'failed').toContain('FAIL app:/store 1440 light "default"');
    expect.soft(failed.printed, 'failed').toContain('error="the page never settled"');
    const stopped = await failed.app.serve.mock.results[0].value;
    expect.soft(stopped.stop, 'failed').toHaveBeenCalledOnce();
    expect.soft(readFileSync(join(root, '.proof', 'summary.txt'), 'utf8'), 'failed').toContain('proof: 2 shots, 0 OK, 2 FAIL');
    expect.soft(failed.pushed, 'failed').toEqual([]);

    checkouts.length = 0;
    const passed = await run(shootAll);
    expect.soft(passed.code, 'passed').toBe(0);
    expect.soft(passed.printed, 'passed').toContain('proof: 2 shots, 2 OK, 0 FAIL');
    expect.soft(readFileSync(join(root, '.proof', 'README.md'), 'utf8'), 'passed').toMatch(/^- \*\*Base\*\*: [0-9a-f]{40}$/m);
    expect.soft(existsSync(join(root, '.proof', 'shots', 'app-store-1440-light-default.png')), 'passed').toBe(true);
    expect.soft(passed.pushed, 'pushed').toEqual([{ root, out: join(root, '.proof'), branch: 'proof/tic-1-fixture', round: 0, head: expect.stringMatching(/^[0-9a-f]{40}$/), carry: false }]);
    expect.soft(pageFiles(JSON.parse(readFileSync(join(root, '.proof', 'page.json'), 'utf8'))), 'pushed').toEqual(['shots/app-store-1440-light-default.png', 'shots/app-store-390-light-default.png']);
    expect.soft(passed.printed.split('\n').at(-1), 'pushed').toBe('proof: page https://github.com/o/r/tree/proof/tic-1-fixture');
    expect.soft(passed.printed, 'no via screen').toContain('; base: 0 shots, 0 FAIL;');
    expect.soft(checkouts, 'no via screen').toHaveLength(0);
    expect.soft(passed.app.serve, 'no via screen').toHaveBeenCalledOnce();
    expect.soft(worktreesOf(root), 'no via screen').toBe(1);
  });

  it('fails a shot the scan faults and pushes nothing, and refuses a branch outside claude/ before it shoots', async () => {
    const faulted = await run(shootAll, fakeApp(shootAll), { scanner: async () => async () => [{ check: 'text', element: 'p "NaN"', detail: 'shows "NaN"' }] });
    expect.soft(faulted.code).toBe(1);
    expect.soft(faulted.pushed).toEqual([]);
    expect.soft(faulted.printed).toContain('scan="text p \\"NaN\\": shows \\"NaN\\""');
    execFileSync('git', ['checkout', '-qb', 'feature-x'], { cwd: root });
    try {
      const outside = await run(shootAll, fakeApp(shootAll));
      expect.soft(outside.code).toBe(2);
      expect.soft(outside.app.serve).not.toHaveBeenCalled();
      expect.soft(outside.pushed).toEqual([]);
    } finally {
      execFileSync('git', ['checkout', '-q', BRANCH], { cwd: root });
    }
  });

  it('in catalogue mode shoots every catalogue entry, the nightly ones too, with no base run, and writes each failing shot to findings.json', async () => {
    const admin = { ...fakeAdmin(shootAll), CATALOGUE: [{ name: 'spend', path: '/spend' }, { name: 'spend-unreadable-one', path: '/spend', nightly: true }] };
    checkouts.length = 0;
    const faulted = { check: 'contrast', element: 'p "faint"', detail: 'text contrast 2.10:1 is under 4.5:1' };
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});
    const { pushed, publish } = recorder();
    let code;
    try {
      code = await main(['--catalogue'], { root, load: async () => ({ admin }), launch: async () => fakeBrowser(), install, scanner: async () => async () => [faulted], publish });
    } finally {
      log.mockRestore();
    }
    expect(code).toBe(1);
    expect(checkouts).toHaveLength(0);
    expect(pushed).toEqual([]);
    const findings = JSON.parse(readFileSync(join(root, '.proof', 'findings.json'), 'utf8'));
    expect(findings.map(({ screen, width }) => `${screen} ${width}`)).toEqual(['spend 1440', 'spend 390', 'spend-unreadable-one 1440', 'spend-unreadable-one 390']);
    expect(findings[0]).toMatchObject({ surface: 'admin', theme: 'dark', state: 'default', error: null, scan: [faulted] });
  });

});

describe('the base run shoots the via screens at the base and removes its checkout', () => {
  let root;
  const checkouts = [];
  const install = async (dir) => {
    checkouts.push(dir);
  };

  beforeAll(() => {
    root = repo('proof-via-', FIXTURE, 'apps/app/src/ui/card.tsx');
  }, 120_000);

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const run = async (app = fakeApp(shootAll), scanner = async () => async () => []) => {
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});
    try {
      const code = await main(['--base', 'HEAD~1', '--head', 'HEAD'], { root, load: async () => ({ app }), launch: async () => fakeBrowser(), install, scanner, publish: recorder().publish });
      return { code, printed: log.mock.calls.flat().join('\n'), app };
    } finally {
      log.mockRestore();
    }
  };

  it('queues only the jobs of via screens for the base', () => {
    const via = { surface: 'app', id: '/', family: 'home', reason: 'via apps/app/src/ui/card.tsx' };
    const jobs = jobsFor([SCREEN, via], { app: { themes: ['light'], states: () => ['default'] } });
    expect(baseJobsOf(jobs).map(({ screen, width }) => `${screen.id} ${width}`)).toEqual(['/ 1440', '/ 390']);
  });

  it('shoots the via screen at the base, marks it, scans the head’s shots and never the base’s, and removes the checkout when the run ends', async () => {
    checkouts.length = 0;
    let scanned = 0;
    const counting = async () => async () => {
      scanned += 1;
      return [];
    };
    const ended = await run(fakeApp(shootAll), counting);
    expect.soft(ended.code).toBe(0);
    expect.soft(ended.printed).toMatch(/^OK app:\/ 1440 light "default" .* same=base$/m);
    expect.soft(ended.printed).toContain('proof: 4 shots, 4 OK, 0 FAIL');
    expect.soft(ended.printed).toContain('; base: 4 shots, 0 FAIL;');
    expect.soft(scanned, 'scans').toBe(4);
    expect.soft(existsSync(join(root, '.proof', 'base', 'app-home-1440-light-default.png'))).toBe(true);
    expect.soft(checkouts).toHaveLength(1);
    expect.soft(existsSync(checkouts[0])).toBe(false);
    expect.soft(worktreesOf(root)).toBe(1);
  });

  it('removes the base checkout when the base cannot start and when the head cannot', async () => {
    const serving = (fails) => {
      const app = fakeApp(shootAll);
      app.serve = vi.fn(async (dir) => {
        if (fails(dir)) throw new Error('nothing answered');
        return { origin: 'http://localhost:1', stop: vi.fn(async () => {}) };
      });
      return app;
    };
    checkouts.length = 0;
    const baseFailed = await run(serving((dir) => dir !== root));
    await expect(run(serving((dir) => dir === root))).rejects.toThrow('nothing answered');
    expect(baseFailed.code).toBe(0);
    expect(baseFailed.printed).toContain('; base: 4 shots, 4 FAIL;');
    expect(checkouts).toHaveLength(2);
    for (const dir of checkouts) expect(existsSync(dir)).toBe(false);
    expect(worktreesOf(root)).toBe(1);
  });

  const killedRun = `
    import { main } from ${JSON.stringify(fileURLToPath(new URL('../proof.mjs', import.meta.url)))};
    const page = { on: () => {} };
    const context = { setDefaultTimeout: () => {}, route: async () => {}, newPage: async () => page, close: async () => {} };
    const app = {
      themes: ['light'], FIXTURE: 'fake', states: () => ['default'],
      serve: async () => ({ origin: 'http://localhost:1', stop: async () => {} }),
      open: async () => ({ contextOptions: () => ({}), prepare: async () => {}, show: async (_page, job, shoot) => shoot && shoot(job.state), capture: async () => Buffer.from('png') }),
    };
    const install = (dir) => {
      console.log(dir);
      setInterval(() => {}, 1000);
      return new Promise(() => {});
    };
    await main(['--base', 'HEAD~1', '--head', 'HEAD'], { root: process.cwd(), load: async () => ({ app }), launch: async () => ({ newContext: async () => context, close: async () => {} }), install, scanner: async () => async () => [] });
  `;

  it('removes the base checkout when the run is killed by SIGINT or SIGTERM', async () => {
    for (const [signal, code] of [
      ['SIGINT', 130],
      ['SIGTERM', 143],
    ]) {
      const child = spawn(process.execPath, ['--input-type=module', '-e', killedRun], { cwd: root, stdio: ['ignore', 'pipe', 'inherit'] });
      const dir = await new Promise((done) => child.stdout.once('data', (chunk) => done(String(chunk).trim())));
      expect.soft(existsSync(dir), `${signal} running`).toBe(true);
      expect.soft(worktreesOf(root), `${signal} running`).toBe(2);
      const exited = new Promise((done) => child.on('exit', (status) => done(status)));
      child.kill(signal);
      expect.soft(await exited, signal).toBe(code);
      expect.soft(existsSync(dir), signal).toBe(false);
      expect.soft(worktreesOf(root), signal).toBe(1);
    }
  }, 60_000);
});

describe('--since scopes the set to what changed since that sha, minus what merges of main brought', () => {
  let root;
  let roundTip;
  const sh = (...args) => execFileSync('git', ['-c', 'user.email=a@b', '-c', 'user.name=a', ...args], { cwd: root, encoding: 'utf8' }).trim();
  const touch = (path) => writeFileSync(join(root, path), `${readFileSync(join(root, path), 'utf8')}\n`);
  const commit = (message, ...paths) => {
    for (const path of paths) touch(path);
    sh('commit', '-qam', message);
    return sh('rev-parse', 'HEAD');
  };

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'proof-since-'));
    writeTree(root, { ...FIXTURE, '.gitignore': 'node_modules\n.proof\n' });
    sh('init', '-q', '-b', 'main');
    sh('add', '.');
    sh('commit', '-qm', 'one');
    sh('update-ref', 'refs/remotes/origin/main', 'HEAD');
    sh('checkout', '-qb', BRANCH);
    roundTip = commit('round one', 'apps/app/src/app/(app)/settings/index.tsx');
    commit('fix', 'apps/app/src/ui/card.tsx');
    sh('checkout', '-q', 'main');
    commit('main moves', 'apps/app/src/app/(auth)/login.tsx');
    sh('update-ref', 'refs/remotes/origin/main', 'HEAD');
    sh('checkout', '-q', BRANCH);
    sh('merge', '-q', '--no-edit', 'main');
  }, 120_000);

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  const pushes = recorder();
  const screensOf = async (...extra) => {
    const log = vi.spyOn(console, 'log').mockImplementation(() => {});
    try {
      const code = await main(extra, { root, load: async () => ({ app: fakeApp(shootAll) }), launch: async () => fakeBrowser(), install: async () => {}, scanner: async () => async () => [], publish: pushes.publish });
      expect(code).toBe(0);
      return [...new Set([...log.mock.calls.flat().join('\n').matchAll(/^OK (\S+) /gm)].map(([, screen]) => screen))].sort();
    } finally {
      log.mockRestore();
    }
  };

  it('gives only the screens a fix commit reaches, shoots its via screens at that sha, pushes keeping the screens it did not shoot, and without --since every screen the branch changed', async () => {
    expect.soft(await screensOf('--since', roundTip), '--since').toEqual(['app:/', 'app:/store']);
    expect.soft(pushes.pushed.at(-1), '--since push').toMatchObject({ branch: 'proof/tic-1-fixture', carry: true });
    expect.soft(existsSync(join(root, '.proof', 'base', 'app-home-1440-light-default.png')), '--since base shot').toBe(true);
    expect.soft(JSON.parse(readFileSync(join(root, '.proof', 'findings.json'), 'utf8')), '--since findings').toEqual([]);

    expect.soft(await screensOf(), 'without --since').toEqual(['app:/', 'app:/settings', 'app:/store']);
  });

  it('refuses --since beside --base, --head or --catalogue', async () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      for (const extra of [['--base', 'HEAD'], ['--head', 'HEAD'], ['--catalogue']]) expect(await main(['--since', roundTip, ...extra], { root, load: async () => ({}) })).toBe(2);
    } finally {
      error.mockRestore();
    }
  });
});

describe('the push commits the page and its shots, byte for byte, to its proof branch on origin, one commit per round that changes the page', () => {
  let root;
  let remote;
  let out;
  const BRANCH_ON_ORIGIN = 'proof/tic-1-x';
  const sh = (cwd, ...args) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
  const entry = (name, round, files) => ({ name, family: 'app · x', reason: 'changed', via: null, same: false, round, cards: files.map((file) => `<img src="${file}">`), unjudged: [], files });
  const write = (round, screens) => writeFileSync(join(out, 'page.json'), JSON.stringify({ head: 'abcdef0', base: 'b', dirty: false, round, fixtures: {}, shots: '1', screens, sameAsBase: {} }));
  const tree = () => sh(remote, 'ls-tree', '-r', '--name-only', BRANCH_ON_ORIGIN).split('\n');

  beforeAll(() => {
    root = repo('proof-push-', { README: 'fixture\n' });
    sh(root, 'config', 'user.name', 'a');
    sh(root, 'config', 'user.email', 'a@b');
    remote = mkdtempSync(join(tmpdir(), 'proof-remote-'));
    sh(remote, 'init', '-q', '--bare');
    sh(root, 'remote', 'add', 'origin', remote);
    out = mkdtempSync(join(tmpdir(), 'proof-out-'));
    for (const folder of ['shots', 'base']) mkdirSync(join(out, folder));
    for (const path of ['shots/a.png', 'shots/b.png', 'base/a.png']) writeFileSync(join(out, path), randomBytes(4096));
  });

  afterAll(() => {
    for (const dir of [root, remote, out]) rmSync(dir, { recursive: true, force: true });
  });

  it('pushes the README, the page data and the shown shots alone, each byte-equal to its shot, nothing for an unchanged page, and keeps on a fix round the screens it did not shoot', async () => {
    const head = sh(root, 'rev-parse', 'HEAD');
    const page = { root, out, branch: BRANCH_ON_ORIGIN, round: 0, head, carry: false };
    write(0, [entry('app:/a', 0, ['shots/a.png', 'base/a.png']), entry('app:/b', 0, ['shots/b.png'])]);
    expect.soft(await publishPage(page)).toContain(BRANCH_ON_ORIGIN);
    expect.soft(tree()).toEqual(['README.md', 'base/a.png', 'page.json', 'shots/a.png', 'shots/b.png']);
    for (const file of ['shots/a.png', 'base/a.png']) expect.soft(execFileSync('git', ['cat-file', 'blob', `${BRANCH_ON_ORIGIN}:${file}`], { cwd: remote }).equals(readFileSync(join(out, file))), file).toBe(true);
    expect.soft(sh(remote, 'log', '--format=%s', BRANCH_ON_ORIGIN)).toBe(`tic-1-x round 0 at ${head.slice(0, 7)}`);
    await publishPage(page);
    expect.soft(sh(remote, 'rev-list', '--count', BRANCH_ON_ORIGIN)).toBe('1');
    write(1, [entry('app:/a', 1, ['shots/a.png'])]);
    await publishPage({ ...page, round: 1, carry: true });
    expect.soft(tree()).toEqual(['README.md', 'page.json', 'shots/a.png', 'shots/b.png']);
    expect.soft(sh(remote, 'show', `${BRANCH_ON_ORIGIN}:README.md`)).toContain('- app:/a: changed · round 1\n- app:/b: changed · round 0\n');
    write(2, [entry('app:/b', 2, ['shots/b.png'])]);
    await publishPage({ ...page, round: 2 });
    expect.soft(tree()).toEqual(['README.md', 'page.json', 'shots/b.png']);
    expect.soft(sh(remote, 'log', '--format=%s', BRANCH_ON_ORIGIN).split('\n')).toEqual([2, 1, 0].map((round) => `tic-1-x round ${round} at ${head.slice(0, 7)}`));
    expect.soft(worktreesOf(root)).toBe(1);
  });
});

describe('the proof shoots a phone at device scale 1 and leaves the catalogue’s nightly entries out', () => {
  const adapterOf = (surface) => import(pathToFileURL(join(REPO_ROOT, ADAPTERS[surface])).href);

  it('shoots a phone at scale 1, as a desktop shot is, on every surface', async () => {
    for (const surface of Object.keys(ADAPTERS)) {
      const { contextOptions } = await adapterOf(surface);
      expect.soft(contextOptions({ width: 390, height: 844, label: 'phone', theme: 'dark' }).deviceScaleFactor ?? 1, surface).toBe(1);
    }
  });

  it('admin: the set the proof places from holds no nightly entry and keeps each scene’s state no screen shows', async () => {
    const { SCREENS } = await adapterOf('admin');
    const catalogue = await import(pathToFileURL(join(REPO_ROOT, 'apps/admin/scripts/shots.mjs')).href);
    const names = new Set(catalogue.SCREENS.map(({ name }) => name));
    expect([...catalogue.DATA_VARIANTS, ...catalogue.SCENE_STATES].filter((name) => !names.has(name))).toEqual([]);
    const nightly = catalogue.SCREENS.filter((entry) => entry.nightly).map(({ name }) => name);
    expect(nightly).toEqual(expect.arrayContaining(catalogue.DATA_VARIANTS));
    expect(SCREENS.map(({ name }) => name)).toEqual(catalogue.SCREENS.filter((entry) => !entry.nightly).map(({ name }) => name));
    expect(SCREENS.map(({ name }) => name)).toEqual(expect.arrayContaining(catalogue.SCENE_STATES));
  });
});
