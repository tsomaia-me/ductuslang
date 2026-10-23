// CD-284: which routes an e2e spec opens, read from its goto() calls and tab clicks.
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import {
  gotoTargets,
  navClickTargets,
  navItems,
  routeMatches,
  routeSegments,
  routeVisitors,
  specTargets,
  linkTargets,
} from '../spec-routes.mjs';

const R = 'apps/app/src/app/';
const EVERY = { prefix: '/' };

describe('route files as URL segments', () => {
  it('drops groups and index, keeps dynamic segments, and has no URL for layouts', () => {
    for (const [file, segments] of [
      [`${R}(app)/chat/[id].tsx`, ['chat', '[id]']],
      [`${R}(app)/index.tsx`, []],
      [`${R}(app)/play/index.tsx`, ['play']],
      [`${R}(auth)/login.tsx`, ['login']],
      [`${R}privacy.web.tsx`, ['privacy']],
      [`${R}_layout.tsx`, null],
      [`${R}(app)/_layout.tsx`, null],
      [`${R}+html.tsx`, null],
      [`${R}+not-found.tsx`, null],
    ])
      expect.soft(routeSegments(file), file).toEqual(segments);
  });
});

describe('matching a target to a route', () => {
  const chat = ['chat', '[id]'];
  const home = [];
  const play = ['play'];
  const wordgridNew = ['play', 'wordgrid', 'new'];

  it('matches an exact path to its route, dynamic segments matching any value, and a prefix to every route that can start with it', () => {
    for (const [segments, target, matches] of [
      [chat, { path: '/chat/u_ada' }, true],
      [home, { path: '/' }, true],
      [play, { path: '/play/' }, true],
      [play, { path: '/play/new' }, false],
      [chat, { path: '/chat' }, false],
      [chat, { prefix: '/chat/' }, true],
      [home, { prefix: '/chat/' }, false],
      [['data-deletion'], { prefix: '/data-del' }, true],
      [wordgridNew, { prefix: '/play/' }, true],
      [play, { prefix: '/play/' }, true],
      ...[chat, home, play, wordgridNew].map((segments) => [segments, EVERY, true]),
    ])
      expect.soft(routeMatches(segments, target), `/${segments.join('/')} ${JSON.stringify(target)}`).toBe(matches);
  });
});

describe('reading goto targets', () => {
  it('reads literals, follows names, parameters, calls and loops to their values, opens both branches of a conditional, and counts what it cannot read as every route', () => {
    for (const [rule, source, targets] of [
      [
        'string literals, a template up to its first interpolation, the query cut',
        [
          "await page.goto('/play');",
          'await page.goto("/store", { waitUntil: "load" });',
          'await page.goto(`/game/${id}`);',
          'await page.goto(`/dev/kit?theme=${theme}`);',
          'await page.goto(`/settings`);',
        ].join('\n'),
        [{ path: '/play' }, { path: '/store' }, { prefix: '/game/' }, { path: '/dev/kit' }, { path: '/settings' }],
      ],
      ['a variable to its declaration', "const FRESH = '/game/g_fresh';\nawait page.goto(FRESH);", [{ path: '/game/g_fresh' }]],
      ['a list a loop walks', "for (const path of ['/play', '/friends'] as const) await page.goto(path);", [{ path: '/play' }, { path: '/friends' }]],
      [
        'a list of pairs a loop destructures',
        "const PAIRS = [\n  ['/store', 'store-screen'],\n  ['/leaderboards', 'x'],\n];\nfor (const [path, screen] of PAIRS) await page.goto(path);",
        [{ path: '/store' }, { path: '/leaderboards' }],
      ],
      [
        'a member of the objects a loop walks',
        "const SCREENS = [{ name: 'a', path: '/profile/u_1' }, { name: 'b', path: '/chat/u_2' }];\nfor (const screen of SCREENS) await page.goto(screen.path);",
        [{ path: '/profile/u_1' }, { path: '/chat/u_2' }],
      ],
      [
        "a parameter to its function's call sites, and to its default",
        [
          "async function leave(page, to = '/') { await page.goto(to); }",
          'async function open(page: Page, path: string): Promise<void> {\n  await page.goto(path);\n}',
          "await open(page, '/settings');",
          'await open(page, `/game/${id}`);',
          'await leave(page);',
        ].join('\n'),
        [{ path: '/' }, { path: '/settings' }, { prefix: '/game/' }],
      ],
      ['a member call', "const nav = { open(page, to) { return page.goto(to); } };\nawait nav.open(page, '/store');", [{ path: '/store' }]],
      ['an alias', "const open = (page, to) => page.goto(to);\nconst go = open;\nawait go(page, '/friends');", [{ path: '/friends' }]],
      ['an array-iterator callback', "['/play', '/settings'].forEach((path) => page.goto(path));", [{ path: '/play' }, { path: '/settings' }]],
      ['a member passed on as a callback', "const nav = { open: (page, to) => page.goto(to) };\nrunAll(nav.open);\nawait nav.open(page, '/store');", [EVERY, { path: '/store' }]],
      ['a function passed on as a callback', 'async function visit(to) { await page.goto(to); }\nawait each(routes, visit);', [EVERY]],
      ['a function nobody calls', 'export const open = (page, to) => page.goto(to);', []],
      ['a concatenation', "page.goto('/game/' + id)", [EVERY]],
      ['a base built at run time', 'page.goto(`${base}/x`)', [EVERY]],
      ['a host built at run time', 'page.goto(`https://${host}/x`)', [EVERY]],
      ['an unknown name', 'page.goto(fromSomewhere)', [EVERY]],
      ['an unknown member', 'page.goto(entry.path)', [EVERY]],
      ['a conditional of two literals', "page.goto(theme === 'a' ? '/' : `/dev/welcome?theme=${theme}`)", [{ path: '/' }, { path: '/dev/welcome' }]],
      ['a conditional with an unknown branch', 'page.goto(a ? "/" : b)', [{ path: '/' }, EVERY]],
      ['a file:// page', 'page.goto(`file://${html}`)', []],
      ['an http URL on the app host', "page.goto('http://localhost:8081/play?x=1')", [{ path: '/play' }]],
      ['an http URL on the app address', "page.goto('http://127.0.0.1:8081/store')", [{ path: '/store' }]],
      [
        'an http URL the spec fulfils',
        "const PLANTED = 'http://planted.test/';\nawait page.route(PLANTED, (route) => route.fulfill({ body: markup }));\nawait page.goto(PLANTED);",
        [{ path: '/' }],
      ],
      ['an http URL on an unknown host', "page.goto('http://planted.test/')", [EVERY]],
      ['an https URL on another host', "page.goto('https://example.com/play')", [EVERY]],
      ['an http URL the spec aborts', "await page.route('http://other.test/', (route) => route.abort());\nawait page.goto('http://other.test/');", [EVERY]],
      [
        'Object.entries of an object literal',
        "const BOARDS = { wordgrid: '/game/g_1', peep: '/play/peep' };\nfor (const [type, route] of Object.entries(BOARDS)) await page.goto(route);",
        [{ path: '/game/g_1' }, { path: '/play/peep' }],
      ],
      ['Object.values of an object literal', "const PATHS = { a: '/store' };\nfor (const path of Object.values(PATHS)) await page.goto(path);", [{ path: '/store' }]],
      ['Object.values of an unknown name', 'for (const path of Object.values(fromSomewhere)) await page.goto(path);', [EVERY]],
    ])
      expect.soft(gotoTargets(source), rule).toEqual(targets);
  });
});

describe('reading tab-bar clicks', () => {
  const nav = navItems(
    "export const NAV_ITEMS = [\n  { key: 'home', href: '/', icon: 'home' },\n  { key: 'play', href: '/play', icon: 'play' },\n  { key: 'leaders', href: '/leaderboards', icon: 'trophy' },\n];",
  );

  it('reads the destinations from NAV_ITEMS; a nav-<key> click opens its destination and what is below it, a key built at run time every destination, an unknown one every route', () => {
    expect.soft(Object.fromEntries(nav), 'NAV_ITEMS').toEqual({ home: '/', play: '/play', leaders: '/leaderboards' });
    expect.soft(navClickTargets("await page.getByTestId('nav-leaders').click();", nav), 'nav-leaders').toEqual([{ path: '/leaderboards' }, { prefix: '/leaderboards/' }]);
    expect.soft(navClickTargets("page.getByTestId('nav-home-label')", nav), 'nav-home-label').toEqual([{ path: '/' }]);
    expect.soft(navClickTargets('page.getByTestId(`nav-${key}`).click()', nav), 'built at run time').toHaveLength(5);
    expect.soft(navClickTargets("page.getByTestId('nav-nowhere').click()", nav), 'unknown').toEqual([EVERY]);
  });
});

function writeTree(root, files) {
  for (const [path, body] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), body);
  }
}

describe('the routes a spec opens, through the e2e files it imports', () => {
  let root;
  const E = 'apps/app/e2e/';
  const e2e = [
    { source: `${E}a.spec.ts`, dependencies: [`${E}helpers.ts`] },
    { source: `${E}b.spec.ts`, dependencies: [`${E}helpers.ts`] },
    { source: `${E}c.spec.ts`, dependencies: [`${E}helpers.ts`] },
    { source: `${E}d.spec.ts`, dependencies: [`${E}helpers.ts`] },
    { source: `${E}helpers.ts`, dependencies: [] },
  ];

  beforeAll(() => {
    root = mkdtempSync(join(tmpdir(), 'cd284-spec-routes-'));
    writeTree(root, {
      [`${E}helpers.ts`]:
        "export const open = (page, to) => page.goto(to);\nexport async function signIn(page) { await page.goto('/login'); }\n",
      [`${E}a.spec.ts`]: "import { open } from './helpers';\nawait open(page, '/store');\n",
      [`${E}b.spec.ts`]: "import { signIn } from './helpers';\nawait signIn(page);\n",
      [`${E}c.spec.ts`]:
        "import { open } from './helpers';\nimport { HOME } from '../src/routes';\nawait open(page, HOME);\n",
      [`${E}d.spec.ts`]: "import { open } from './helpers';\nawait open(page, '/game/' + id);\n",
      'apps/app/src/navigation/nav-items.ts': "[{ key: 'leaders', href: '/leaderboards' }]\n",
    });
  });

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it("counts a helper's goto(variable) only for the specs that call it, with their arguments, and a value from ../src or built by concatenation as every route", () => {
    expect.soft(specTargets(`${E}a.spec.ts`, e2e, root), 'a').toEqual([{ path: '/store' }, { path: '/login' }]);
    expect.soft(specTargets(`${E}b.spec.ts`, e2e, root), 'b').toEqual([{ path: '/login' }]);
    expect.soft(specTargets(`${E}c.spec.ts`, e2e, root), 'c').toContainEqual(EVERY);
    expect.soft(specTargets(`${E}d.spec.ts`, e2e, root), 'd').toContainEqual(EVERY);
  });

  it("reads an app file's literal links: href, router.push and router.replace", () => {
    writeTree(root, {
      'apps/app/src/features/x/links.tsx': [
        "import { Link, router } from 'expo-router';",
        'export const A = () => <Link href="/store">s</Link>;',
        'export const B = () => <Link href={`/chat/${id}`}>c</Link>;',
        "export const items = [{ key: 'friends', href: '/friends' }];",
        "router.push('/play');",
        'router.replace(`/game/${id}?x`);',
        'router.push(somewhere);',
      ].join('\n'),
    });
    expect(linkTargets(root, 'apps/app/src/features/x/links.tsx')).toEqual([
      { path: '/store' },
      { prefix: '/chat/' },
      { path: '/friends' },
      { path: '/play' },
      { prefix: '/game/' },
    ]);
  });

  it('maps each route to the projects whose specs open it', () => {
    const routes = [`${R}(app)/store.tsx`, `${R}(auth)/login.tsx`, `${R}(app)/_layout.tsx`];
    const specs = new Map([
      ['a', 'economy'],
      ['b', 'settings-onboarding'],
    ]);
    const visitors = routeVisitors({ e2e }, specs, routes, root);
    expect(
      Object.fromEntries([...visitors].map(([route, projects]) => [route, [...projects].sort()])),
    ).toEqual({
      [`${R}(app)/store.tsx`]: ['economy'],
      [`${R}(auth)/login.tsx`]: ['economy', 'settings-onboarding'],
    });
  });
});
