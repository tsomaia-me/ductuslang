/**
 * `smoke-walk.mjs` (CD-203, CD-285): the walk `release.yml` runs on dev after
 * promoting to it, and the quick check of production after promoting there; a
 * failure of either rolls that stack back. What has to hold: every check runs in order and the first
 * failure stops the walk with exit 1; a build other than the released one is a
 * failure; the bot game the move opens is resigned whatever happened; no walk
 * account (no WALK_SEED_KEY) is exit 2, never a pass; and no password, nor the
 * key, is printed (CD-278: the account is derived from it). The API and the
 * web are a fake `fetch`; the browser is a stub.
 */
import { describe, expect, it } from 'vitest';

import { WALK_EMAILS, walkPassword } from '../live-walk-accounts.mjs';
import { BOT_GAME, parseArgs, playwrightBrowser, run } from '../smoke-walk.mjs';

const SHA = 'a'.repeat(40);
const TIP = 'b'.repeat(40);
const KEY = 'smoke-walk-seed-key-must-not-appear-0123456789';
const PASSWORD = walkPassword(KEY, WALK_EMAILS[2]);
const OTHER_KEY = 'another-seed-key-that-signs-into-nothing-000';
const ENV = { WALK_SEED_KEY: KEY, SMOKE_API_URL: 'https://api.test', SMOKE_WEB_URL: 'https://web.test' };

const fakeProduction = (overrides = {}) => {
  const calls = [];
  const game = { id: 'g1', seq: 0, myTurn: false };
  let polls = 0;
  const reply = (status, body, type = 'application/json', headers = {}) => ({
    ok: status >= 200 && status < 300,
    status,
    headers: { get: (name) => (name.toLowerCase() === 'content-type' ? type : (headers[name.toLowerCase()] ?? null)) },
    text: async () => (typeof body === 'string' ? body : JSON.stringify(body)),
  });
  const fetchImpl = async (url, init) => {
    const { pathname, host } = new URL(url);
    const key = `${init.method} ${host}${pathname}`;
    const body = init.body ? JSON.parse(init.body) : undefined;
    calls.push({ key, body, auth: init.headers.authorization, cache: init.headers['cache-control'], access: [init.headers['CF-Access-Client-Id'], init.headers['CF-Access-Client-Secret']] });
    if (overrides[key]) return overrides[key](body);
    switch (key) {
      case 'GET api.test/api/v1/health':
        return reply(200, { status: 'ok', build: { sha: SHA, short: SHA.slice(0, 7) } });
      case 'GET web.test/':
        return reply(200, '<!doctype html><html><div id="root"></div></html>', 'text/html; charset=utf-8');
      case 'GET web.test/build.json':
        return reply(200, { sha: TIP });
      case 'POST api.test/api/v1/auth/login':
        return body.password === PASSWORD ? reply(200, { accessToken: 'tok', refreshToken: 'r', user: { id: 'u1' } }) : reply(401, { error: { code: 'auth.invalid' } });
      case 'GET api.test/api/v1/me':
        return reply(200, { id: 'u1' });
      case 'GET api.test/api/v1/games':
        return reply(200, { items: [] });
      case 'POST api.test/api/v1/games':
        return reply(201, game);
      case 'GET api.test/api/v1/games/g1':
        polls += 1;
        return reply(200, { ...game, seq: 1, myTurn: polls >= 2 });
      case 'POST api.test/api/v1/games/g1/moves':
        return reply(200, { game: { ...game, seq: body.seq + 1 } });
      case 'POST api.test/api/v1/games/g1/resign':
        return reply(200, { ...game, status: 'finished' });
      default:
        return reply(404, { error: { code: 'not_found' } });
    }
  };
  return { calls, fetchImpl, reply };
};

const walk = async (argv, { env = ENV, production = fakeProduction(), openBrowser, waited = [] } = {}) => {
  const out = [];
  const err = [];
  const code = await run(argv, env, {
    wait: async (ms) => {
      waited.push(ms);
    },
    fetchImpl: production.fetchImpl,
    openBrowser: openBrowser ?? (async () => 'Home rendered at 390×844'),
    out: (line) => out.push(line),
    err: (line) => err.push(line),
    pollEveryMs: 0,
    pollTries: 5,
    buildEveryMs: 0,
    buildTries: 4,
  });
  return { code, out: out.join('\n'), err: err.join('\n'), calls: production.calls };
};

describe('a release that can be played', () => {
  it('passes every check in order, moves on its turn, and resigns the game it opened', async () => {
    const result = await walk(['--sha', SHA, '--web-sha', TIP, '--browser']);
    expect(result.code).toBe(0);
    expect(result.out.split('\n').filter((line) => line.startsWith('smoke: ok')).map((line) => line.split(' ')[2])).toEqual([
      'health',
      'web',
      'sign-in',
      'home',
      'browser',
      'move',
    ]);
    const keys = result.calls.map((call) => call.key);
    expect(keys.at(-1)).toBe('POST api.test/api/v1/games/g1/resign');
    expect(result.calls.find((call) => call.key === 'POST api.test/api/v1/games').body).toEqual(BOT_GAME);
    expect(result.calls.find((call) => call.key === 'POST api.test/api/v1/games/g1/moves').body).toEqual({ move: { type: 'pass' }, seq: 1 });
    expect(result.calls.filter((call) => call.key.includes('/games')).every((call) => call.auth === 'Bearer tok')).toBe(true);
    expect(result.out).toMatch(/smoke: PASSED/);
  });

  it('signs in as the last account derived from WALK_SEED_KEY (CD-278), never prints a password or the key, and reads /build.json once without --web-sha', async () => {
    const result = await walk([]);
    expect.soft(result.code).toBe(0);
    expect.soft(result.out).toContain('walk-cf+3@crookedduke.com');
    expect.soft(result.calls.find((call) => call.key === 'POST api.test/api/v1/auth/login')?.body).toEqual({ email: 'walk-cf+3@crookedduke.com', password: PASSWORD });
    const printed = `${result.out}${result.err}`;
    for (const secret of [KEY, PASSWORD, walkPassword(KEY, WALK_EMAILS[0])]) expect.soft(printed).not.toContain(secret);
    expect.soft(result.calls.filter((call) => call.key === 'GET web.test/build.json')).toHaveLength(1);
  });
});

describe('a release that cannot', () => {
  it('fails on another build than the one released, and stops there', async () => {
    const result = await walk(['--sha', 'c'.repeat(40)]);
    expect(result.code).toBe(1);
    expect(result.err).toMatch(/FAILED health — health reports build a{40}, not c{40}/);
    expect(result.calls.map((call) => call.key)).toEqual(['GET api.test/api/v1/health']);
  });

  it('waits past caches for /build.json to name the wanted build through a stale build, a 503, an HTML fallback and a dropped read (CD-270), and fails naming what it last saw: another tip, a 503, or an error page', async () => {
    const fresh = fakeProduction();
    let staleReads = 0;
    const stale = fakeProduction({ 'GET web.test/build.json': () => fresh.reply(200, { sha: staleReads++ < 2 ? 'a'.repeat(40) : TIP }) });
    const waited = await walk(['--web-sha', TIP], { production: stale });
    expect.soft(waited.code, 'stale').toBe(0);
    expect.soft(staleReads, 'stale').toBe(3);
    expect.soft(waited.calls.filter((call) => call.key === 'GET web.test/build.json').every((call) => call.cache === 'no-cache'), 'stale').toBe(true);

    const answers = [() => fresh.reply(503, 'down', 'text/plain'), () => fresh.reply(200, '<html></html>', 'text/html'), () => { throw Object.assign(new Error('x'), { name: 'TimeoutError' }); }, () => fresh.reply(200, { sha: TIP })];
    let flakyReads = 0;
    const flaky = fakeProduction({ 'GET web.test/build.json': () => answers[flakyReads++]() });
    expect.soft((await walk(['--web-sha', TIP], { production: flaky })).code, 'flaky').toBe(0);
    expect.soft(flakyReads, 'flaky').toBe(4);

    const down = await walk(['--web-sha', TIP], { production: fakeProduction({ 'GET web.test/build.json': () => fresh.reply(503, 'down', 'text/plain') }) });
    expect.soft(down.err, 'never recovers').toMatch(/FAILED web — \/build\.json answered 503/);
    expect.soft(down.calls.filter((call) => call.key === 'GET web.test/build.json'), 'never recovers').toHaveLength(4);

    const stuck = await walk(['--web-sha', 'd'.repeat(40)]);
    expect.soft(stuck.err, 'another tip').toMatch(/FAILED web — \/build\.json names b{40}, not d{40} after 0s/);
    expect.soft(stuck.calls.filter((call) => call.key === 'GET web.test/build.json'), 'another tip').toHaveLength(4);
    const broken = fakeProduction({ 'GET web.test/': () => fresh.reply(502, 'bad gateway', 'text/plain') });
    expect.soft((await walk([], { production: broken })).err, 'an error page').toMatch(/FAILED web — \/ answered 502/);
  });

  it('fails on a refused sign-in, and on Home not rendering in the browser', async () => {
    const wrong = { ...ENV, WALK_SEED_KEY: OTHER_KEY };
    expect((await walk([], { env: wrong })).err).toMatch(/FAILED sign-in — sign-in answered 401 \(auth\.invalid\)/);
    const result = await walk(['--browser'], {
      openBrowser: async () => {
        const { CheckFailed } = await import('../smoke-walk.mjs');
        throw new CheckFailed('Home did not render: timeout');
      },
    });
    expect(result.code).toBe(1);
    expect(result.err).toMatch(/FAILED browser — Home did not render/);
    expect(result.calls.some((call) => call.key === 'POST api.test/api/v1/games')).toBe(false);
  });

  it('fails a move the engine refuses, and still resigns the game', async () => {
    const production = fakeProduction();
    const refusing = fakeProduction({ 'POST api.test/api/v1/games/g1/moves': () => production.reply(400, { error: { code: 'game.not_your_turn' } }) });
    const result = await walk([], { production: refusing });
    expect(result.code).toBe(1);
    expect(result.err).toMatch(/FAILED move — the pass answered 400 \(game\.not_your_turn\)/);
    expect(result.calls.at(-1).key).toBe('POST api.test/api/v1/games/g1/resign');
    expect(result.out).toMatch(/resigned g1/);
  });

  it('fails when the turn never comes, and when the network does not answer', async () => {
    const production = fakeProduction();
    const stuck = fakeProduction({ 'GET api.test/api/v1/games/g1': () => production.reply(200, { id: 'g1', seq: 1, myTurn: false }) });
    expect((await walk([], { production: stuck })).err).toMatch(/never became the walk account's turn/);
    const offline = { fetchImpl: async () => Promise.reject(Object.assign(new Error('x'), { name: 'TimeoutError' })), calls: [] };
    expect((await walk([], { production: offline })).err).toMatch(/FAILED health — GET \/api\/v1\/health did not answer \(TimeoutError\)/);
  });
});

describe('a "too many requests" on the walk (CD-360)', () => {
  const NEW_GAME = 'POST api.test/api/v1/games';
  const refusing = (answers) => {
    const production = fakeProduction();
    let sent = 0;
    const refused = (retryAfter) => production.reply(429, { error: { code: 'rate_limited' } }, 'application/json', retryAfter === undefined ? {} : { 'retry-after': retryAfter });
    return fakeProduction({ [NEW_GAME]: () => (answers[sent++] ?? (() => production.reply(201, { id: 'g1', seq: 0, myTurn: false })))(refused) });
  };
  const newGames = (result) => result.calls.filter((call) => call.key === NEW_GAME);

  it('retries a new bot game once after the 5 seconds a 429 asks, printing each 429, and retries nothing else: a second 429, a real ceiling hit, a 429 with no or a dated retry-after, or any other status', async () => {
    const waited = [];
    const result = await walk([], { production: refusing([(refused) => refused('5')]), waited });
    expect.soft(result.code, 'one 429').toBe(0);
    expect.soft(waited, 'one 429').toEqual([5_000]);
    expect.soft(newGames(result), 'one 429').toHaveLength(2);
    expect.soft(newGames(result)[1]?.body, 'one 429').toEqual(BOT_GAME);
    expect.soft(result.err, 'one 429').toBe('smoke: 429 move — POST /api/v1/games, retry-after 5, attempt 1, retrying once');
    expect.soft(result.out, 'one 429').toMatch(/smoke: ok move — passed in g1/);
    expect.soft(result.out, 'one 429').toMatch(/resigned g1/);

    const FAILED = 'smoke: FAILED move — new bot game answered 429 (rate_limited)';
    for (const [label, answers, tries, err] of [
      ['a second 429', [(refused) => refused('5'), (refused) => refused('5')], 2, ['smoke: 429 move — POST /api/v1/games, retry-after 5, attempt 1, retrying once', 'smoke: 429 move — POST /api/v1/games, retry-after 5, attempt 2', FAILED]],
      ['a real ceiling hit (retry-after past 5 seconds)', [(refused) => refused('86400')], 1, ['smoke: 429 move — POST /api/v1/games, retry-after 86400, attempt 1', FAILED]],
      ['no retry-after', [(refused) => refused(undefined)], 1, ['smoke: 429 move — POST /api/v1/games, retry-after none, attempt 1', FAILED]],
      ['a retry-after that is not a number of seconds', [(refused) => refused('Wed, 21 Oct 2026 07:28:00 GMT')], 1],
    ]) {
      const slept = [];
      const refused = await walk([], { production: refusing(answers), waited: slept });
      expect.soft(refused.code, label).toBe(1);
      expect.soft(newGames(refused), label).toHaveLength(tries);
      if (tries === 1) expect.soft(slept, label).toEqual([]);
      if (err) expect.soft(refused.err.split('\n'), label).toEqual(err);
    }

    const production = fakeProduction();
    let sent = 0;
    const failing = fakeProduction({ [NEW_GAME]: () => (sent++, production.reply(500, { error: { code: 'internal' } }, 'application/json', { 'retry-after': '1' })) });
    const other = await walk([], { production: failing });
    expect.soft(other.code, 'a 500').toBe(1);
    expect.soft(sent, 'a 500').toBe(1);
    expect.soft(other.err, 'a 500').toBe('smoke: FAILED move — new bot game answered 500 (internal)');
  });

  it('prints a 429 on any other call too, without retrying it or printing a token or secret', async () => {
    const production = fakeProduction();
    const limited = fakeProduction({ 'GET api.test/api/v1/me': () => production.reply(429, { error: { code: 'rate_limited' } }, 'application/json', { 'retry-after': '5' }) });
    const result = await walk([], { env: { ...ENV, CF_ACCESS_CLIENT_ID: 'access-id.access', CF_ACCESS_CLIENT_SECRET: 'access-secret-must-not-appear' }, production: limited });
    expect(result.code).toBe(1);
    expect(result.calls.filter((call) => call.key === 'GET api.test/api/v1/me')).toHaveLength(1);
    expect(result.err.split('\n')).toEqual(['smoke: 429 home — GET /api/v1/me, retry-after 5, attempt 1', 'smoke: FAILED home — /me answered 429 (rate_limited)']);
    for (const secret of ['tok', 'access-secret-must-not-appear', KEY, PASSWORD]) expect(result.err).not.toContain(secret);
  });
});

describe('the stacks, the quick check and Cloudflare Access (CD-285)', () => {
  const ACCESS = { CF_ACCESS_CLIENT_ID: 'access-id.access', CF_ACCESS_CLIENT_SECRET: 'access-secret-must-not-appear' };

  it('walks dev at its own addresses with --env dev and production by default, takes its targets from flags, else the environment, and refuses a flag without its value and the retired --preflight and --accounts-file', async () => {
    expect.soft(parseArgs(['--env', 'dev'], {})).toMatchObject({ env: 'dev', api: 'https://dev.crookedduke.com', web: 'https://dev.crookedduke.com' });
    expect.soft(parseArgs([], {})).toMatchObject({ env: 'production', api: 'https://api.crookedduke.com', web: 'https://crookedduke.com', browser: false, move: true });
    expect.soft(parseArgs(['--env', 'dev', '--api', 'https://a/'], { SMOKE_WEB_URL: 'https://w' })).toMatchObject({ api: 'https://a', web: 'https://w' });
    expect.soft(parseArgs(['--api', 'https://a/', '--no-move'], {})).toMatchObject({ api: 'https://a', move: false });
    for (const [argv, refusal] of [
      [['--env', 'staging'], /production or dev/],
      [['--sha'], /needs a value/],
      [['--preflight'], /unknown flag --preflight/],
      [['--accounts-file', '/tmp/x'], /unknown flag --accounts-file/],
    ]) {
      expect.soft(() => parseArgs(argv, {}), argv.join(' ')).toThrow(refusal);
    }
    expect.soft((await walk(['--preflight'])).code).toBe(2);
  });

  it('sends the service token on every request and to the browser only when both secrets are set, and never prints them', async () => {
    let given;
    const result = await walk(['--browser'], {
      env: { ...ENV, ...ACCESS },
      openBrowser: async ({ headers }) => {
        given = headers;
        return 'Home rendered at 390×844';
      },
    });
    expect.soft(result.code).toBe(0);
    expect.soft(result.calls.length).toBeGreaterThan(5);
    expect.soft(result.calls.every((call) => call.access[0] === 'access-id.access' && call.access[1] === 'access-secret-must-not-appear')).toBe(true);
    expect.soft(given).toEqual({ 'CF-Access-Client-Id': 'access-id.access', 'CF-Access-Client-Secret': 'access-secret-must-not-appear' });
    expect.soft(result.out).toMatch(/with the Access service token/);
    expect.soft(`${result.out}${result.err}`).not.toContain('access-secret-must-not-appear');
    for (const [label, env] of [
      ['no secret', ENV],
      ['one secret', { ...ENV, CF_ACCESS_CLIENT_ID: 'access-id.access' }],
    ]) {
      const plain = await walk([], { env });
      expect.soft(plain.code, label).toBe(0);
      expect.soft(plain.calls.every((call) => call.access[0] === undefined && call.access[1] === undefined), label).toBe(true);
    }
  });

  it('--quick checks health with the build and the web with its build only: no account, no browser, no game', async () => {
    let browsers = 0;
    const result = await walk(['--quick', '--browser', '--sha', SHA, '--web-sha', TIP], {
      env: { SMOKE_API_URL: 'https://api.test', SMOKE_WEB_URL: 'https://web.test' },
      openBrowser: async () => {
        browsers += 1;
        return 'x';
      },
    });
    expect(result.code).toBe(0);
    expect(result.calls.map((call) => call.key)).toEqual(['GET api.test/api/v1/health', 'GET web.test/', 'GET web.test/build.json']);
    expect(browsers).toBe(0);
    expect(result.out).toMatch(/^smoke: quick check of production: /m);
    const wrong = await walk(['--quick', '--sha', 'c'.repeat(40)], { env: { SMOKE_API_URL: 'https://api.test', SMOKE_WEB_URL: 'https://web.test' } });
    expect(wrong.code).toBe(1);
    expect(wrong.err).toMatch(/FAILED health — health reports build a{40}, not c{40}/);
  });
});

describe('the browser check signs in through the stack it walks (CD-285 review)', () => {
  const response = (url, status = 200) => ({ url: () => url, request: () => ({ method: () => 'POST' }), ok: () => status >= 200 && status < 300, status: () => status });
  const fakeBrowser = (responses) => {
    const seen = { context: null, routes: [], sockets: [], closed: false };
    const page = {
      on: () => undefined,
      goto: async () => undefined,
      getByTestId: () => ({ fill: async () => undefined, click: async () => undefined, waitFor: async () => undefined }),
      waitForResponse: async (matches) => {
        const found = responses.find((candidate) => matches(candidate));
        if (!found) throw Object.assign(new Error('Timeout 45000ms exceeded'), { name: 'TimeoutError' });
        return found;
      },
    };
    const launch = async () => ({
      newContext: async (options) => {
        seen.context = options;
        return {
          route: async (matches) => {
            seen.routes.push(matches);
          },
          routeWebSocket: async (matches) => {
            seen.sockets.push(matches);
          },
          setOffline: async () => undefined,
          newPage: async () => page,
        };
      },
      close: async () => {
        seen.closed = true;
      },
    });
    return { launch, seen };
  };
  const DEV = { web: 'https://dev.test', api: 'https://dev.test', sameOrigin: true };
  const PROD = { web: 'https://web.test', api: 'https://api.test', sameOrigin: false };
  const open = (responses, { stack = DEV, headers = {} } = {}) => {
    const { launch, seen } = fakeBrowser(responses);
    return { seen, done: playwrightBrowser({ ...stack, account: { email: 'a@x.test', password: 'p' }, headers, launch }) };
  };

  it('renders Home once the page signed in at the stack\'s own API, with the Access headers routed to the walked origin alone (CD-353)', async () => {
    const headers = { 'CF-Access-Client-Id': 'id.access', 'CF-Access-Client-Secret': 'secret' };
    const { seen, done } = open([response('https://dev.test/api/v1/auth/login')], { headers });
    await expect(done).resolves.toBe('Home rendered at 390×844');
    expect(seen.context.extraHTTPHeaders).toBeUndefined();
    expect(seen.routes).toHaveLength(1);
    const routed = (address) => seen.routes[0](new URL(address));
    expect([routed('https://dev.test/api/v1/me'), routed('https://dev.test/'), routed('https://media-dev.test/a.png'), routed('https://fonts.test/a.woff2')]).toEqual([true, true, false, false]);
    expect(seen.sockets).toHaveLength(1);
    const bridged = (address) => seen.sockets[0](new URL(address));
    expect([bridged('wss://dev.test/ws/hub?ticket=t'), bridged('wss://media-dev.test/ws/hub'), bridged('ws://dev.test/ws/hub')]).toEqual([true, false, false]);
    expect(seen.closed).toBe(true);
    const production = open([response('https://api.test/api/v1/auth/login')], { stack: PROD });
    await expect(production.done).resolves.toBe('Home rendered at 390×844');
    expect(production.seen.routes).toEqual([]);
    expect(production.seen.sockets).toEqual([]);
  });

  it('fails a sign-in anywhere but the stack\'s own API: on dev another origin than the web app, even one the walk was pointed at, on production the web host or another host; and fails when the page never asks or the sign-in is refused', async () => {
    const crossed = { ...DEV, api: 'https://api.dev.test' };
    for (const [label, responses, stack, refusal] of [
      ['dev, the API the walk was pointed at', [response('https://api.dev.test/api/v1/auth/login')], crossed, /the page signed in at https:\/\/api\.dev\.test, not at its own origin https:\/\/dev\.test: dev serves its API, \/api and \/ws, on the web app's own origin/],
      ['dev, the production API', [response('https://api.crookedduke.com/api/v1/auth/login')], DEV, /signed in at https:\/\/api\.crookedduke\.com, not at its own origin/],
      ['production, the web host', [response('https://web.test/api/v1/auth/login')], PROD, /the page signed in at https:\/\/web\.test, not at https:\/\/api\.test/],
      ['production, another host', [response('https://other.test/api/v1/auth/login')], PROD, /signed in at https:\/\/other\.test, not at https:\/\/api\.test/],
      ['no sign-in asked', [], DEV, /the page never asked https:\/\/dev\.test\/api\/v1\/auth\/login/],
      ['a refused sign-in', [response('https://dev.test/api/v1/auth/login', 403)], DEV, /sign-in at https:\/\/dev\.test\/api\/v1\/auth\/login answered 403/],
    ]) {
      await expect.soft(open(responses, { stack }).done, label).rejects.toThrow(refusal);
    }
  });

  it('hands the browser check the walked stack\'s API and web, distinct, and holds dev to one origin', async () => {
    const given = [];
    for (const argv of [['--browser'], ['--env', 'dev', '--browser']]) {
      await walk(argv, {
        openBrowser: async (args) => {
          given.push(args);
          return 'ok';
        },
      });
    }
    expect(given[0]).toMatchObject({ web: 'https://web.test', api: 'https://api.test', sameOrigin: false });
    expect(given[1]).toMatchObject({ sameOrigin: true });
  });
});

describe('the walk account', () => {
  it('is exit 2, never a pass, asking nothing, without WALK_SEED_KEY or with a key too short to derive from, which it never quotes', async () => {
    for (const [label, env, refusal] of [
      ['no key', { HOME: '/nowhere', SMOKE_API_URL: 'https://api.test' }, /no walk account: WALK_SEED_KEY is not set/],
      ['a short key', { ...ENV, WALK_SEED_KEY: 'short-key-must-not-appear' }, /WALK_SEED_KEY is shorter than 32 characters/],
    ]) {
      const result = await walk([], { env });
      expect.soft(result.code, label).toBe(2);
      expect.soft(result.err, label).toMatch(refusal);
      expect.soft(result.err, label).not.toContain('short-key-must-not-appear');
      expect.soft(result.calls, label).toEqual([]);
    }
  });
});
