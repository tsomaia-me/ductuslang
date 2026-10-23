import { createServer } from 'node:http';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { journeyAccounts } from '../../../apps/app/e2e/journeys/journey-env.ts';
import { guardJourney } from '../../../apps/app/e2e/journeys/journey-guard.ts';
import { guardRequests } from '../../../apps/app/e2e/journeys/request-guard.ts';
import { UNMINTABLE_TOKEN } from '../../../apps/app/e2e/journeys/write-guard.ts';
import { WALK_EMAILS } from '../../../apps/app/e2e/live/walk-accounts.ts';
import { browsersPath, launchChromium, useInstalledBrowsers } from '../interaction-check.mjs';

const TIMEOUT_MS = 60_000;
const REFUSED = 405;
const ACCESS = { 'CF-Access-Client-Id': 'dummy-app-id.access', 'CF-Access-Client-Secret': 'dummy-app-secret-k4' };
const KEY = 'dummy-walk-seed-key-for-the-guard-test-only';
const PAIR = [
  { email: WALK_EMAILS[0], id: 'u1', username: 'walker1' },
  { email: WALK_EMAILS[1], id: 'u2', username: 'walker2' },
];
const STRANGER = 'u3';
const BEARER = { authorization: 'Bearer walker' };
const LOCAL_NETWORK_CHECKS = 'LocalNetworkAccessChecks,BlockInsecurePrivateNetworkRequests';
const LISTS = {
  '/api/v1/conversations': [{ id: 'c-pair', user: { id: 'u2' } }, { id: 'c-stranger', user: { id: STRANGER } }],
  '/api/v1/friends/requests': [{ id: 'r-pair', user: { id: 'u1' } }, { id: 'r-stranger', user: { id: STRANGER } }],
};

const hasChromium = browsersPath() !== undefined;
let server;
let site;
let elsewhere;
let browser;
let apiRequest;
const received = [];

beforeAll(async () => {
  server = createServer((request, response) => {
    const url = new URL(request.url, 'http://fixture.test');
    received.push({ line: `${request.method} ${url.pathname}`, origin: `http://${request.headers.host}` });
    if (url.pathname === '/') {
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
      return response.end('<!doctype html><title>app</title>');
    }
    response.writeHead(200, { 'content-type': 'application/json', 'access-control-allow-origin': '*' });
    return response.end(JSON.stringify(LISTS[url.pathname] ?? {}));
  });
  await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
  const { port } = server.address();
  site = `http://127.0.0.1:${port}`;
  elsewhere = `http://localhost:${port}`;
  useInstalledBrowsers();
  if (hasChromium) browser = await launchChromium({ args: [`--disable-features=${LOCAL_NETWORK_CHECKS}`] });
  ({ request: apiRequest } = await import('playwright'));
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
  await new Promise((done) => server.close(done));
});

const scope = () => ({ baseURL: site, api: site, headers: ACCESS, pair: PAIR });
const writesSent = () => received.filter(({ line }) => !line.startsWith('GET ')).map(({ line }) => line);
const failure = (sending) => sending.then(() => null, (error) => String(error.message));
const writes = {
  unexpected: { method: 'POST', path: '/api/v1/reports', body: { userId: 'u2' } },
  botGame: { method: 'POST', path: '/api/v1/games', body: { gameType: 'wordgrid', seats: [{ kind: 'bot', botProfileId: 'bot-1' }] } },
  openGame: { method: 'POST', path: '/api/v1/games', body: { gameType: 'wordgrid', seats: [{ kind: 'open' }, { kind: 'open' }] } },
  pairGame: { method: 'POST', path: '/api/v1/games', body: { gameType: 'wordgrid', seats: [{ kind: 'user', userId: 'u2' }] } },
  strangerGame: { method: 'POST', path: '/api/v1/games', body: { gameType: 'wordgrid', seats: [{ kind: 'user', userId: STRANGER }] } },
  mixedGame: { method: 'POST', path: '/api/v1/games', body: { gameType: 'wordgrid', seats: [{ kind: 'bot', botProfileId: 'bot-1' }, { kind: 'user', userId: STRANGER }] } },
  seatlessGame: { method: 'POST', path: '/api/v1/games', body: { gameType: 'wordgrid' } },
  leaveGame: { method: 'POST', path: '/api/v1/games/g-open/leave' },
  settings: { method: 'PATCH', path: '/api/v1/me/settings', body: { themeId: 'marmalade' } },
  rewardsAck: { method: 'POST', path: '/api/v1/me/rewards/ack', body: { ids: ['rule:firstGameOfDay:2026-10-14'] } },
  pairRequest: { method: 'POST', path: '/api/v1/friends/requests', body: { userId: 'u2' } },
  strangerRequest: { method: 'POST', path: '/api/v1/friends/requests', body: { userId: STRANGER } },
  strangerByName: { method: 'POST', path: '/api/v1/friends/requests', body: { username: 'someone-else' } },
  strangerUnfriend: { method: 'DELETE', path: `/api/v1/friends/${STRANGER}` },
  pairAccept: { method: 'POST', path: '/api/v1/friends/requests/r-pair/accept' },
  strangerAccept: { method: 'POST', path: '/api/v1/friends/requests/r-stranger/accept' },
  pairMessage: { method: 'POST', path: '/api/v1/conversations/c-pair/messages', body: { text: 'hi' } },
  strangerMessage: { method: 'POST', path: '/api/v1/conversations/c-stranger/messages', body: { text: 'hi' } },
  smokeLogin: { method: 'POST', path: '/api/v1/auth/login', body: { email: WALK_EMAILS[2], password: 'x' } },
  pairLogin: { method: 'POST', path: '/api/v1/auth/login', body: { email: WALK_EMAILS[0], password: 'x' } },
  resetUnmintable: { method: 'POST', path: '/api/v1/auth/password/reset', body: { token: UNMINTABLE_TOKEN, password: 'longenough' } },
  verifyUnmintable: { method: 'POST', path: '/api/v1/auth/email/verify', body: { token: UNMINTABLE_TOKEN } },
  resetOtherToken: { method: 'POST', path: '/api/v1/auth/password/reset', body: { token: 'aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789-_aBcDe', password: 'longenough' } },
  verifyOtherToken: { method: 'POST', path: '/api/v1/auth/email/verify', body: { token: 'aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789-_aBcDe' } },
  resetNoToken: { method: 'POST', path: '/api/v1/auth/password/reset', body: { password: 'longenough' } },
  registerPair: { method: 'POST', path: '/api/v1/auth/register', body: { email: WALK_EMAILS[0], password: 'longenough', displayName: 'Taken' } },
  forgotPair: { method: 'POST', path: '/api/v1/auth/password/forgot', body: { email: WALK_EMAILS[0] } },
};
const OTHER_BODY = 'with a body other than the one the journeys send';
const THIRD = 'naming an account other than walk accounts 1 and 2';

async function inJourney(walk) {
  received.length = 0;
  const context = await browser.newContext();
  const guard = await guardJourney(context, scope());
  try {
    const page = await context.newPage();
    await page.goto(`${site}/`);
    const result = await walk(page);
    return { result, refused: await guard.refused(), sent: writesSent() };
  } finally {
    await context.close();
  }
}

const fetchAll = (page, calls) =>
  page.evaluate(
    (all) =>
      Promise.all(
        all.map(({ method, url, body, headers, mode }) =>
          fetch(url, { method, mode, headers: { 'content-type': 'application/json', ...headers }, body: body === undefined ? undefined : JSON.stringify(body) }).then((response) => response.status),
        ),
      ),
    calls.map(({ method, path, body, origin = site }) => ({ method, url: `${origin}${path}`, body, headers: BEARER, mode: origin === site ? 'same-origin' : 'no-cors' })),
  );

async function withApi(walk) {
  received.length = 0;
  const api = await apiRequest.newContext({ baseURL: site });
  try {
    const guard = guardRequests(api, scope());
    const result = await walk(api);
    return { result, refused: guard.refused(), sent: writesSent() };
  } finally {
    await api.dispose();
  }
}

const send = (api, { method, path, body }) => failure(api.fetch(path, { method, headers: BEARER, ...(body === undefined ? {} : { data: body }) }));

describe('the app journeys write guard (CD-326)', () => {
  it('signs in walk accounts 1 and 2 only: account 3, the smoke walk\'s, is never a journey account', () => {
    expect(journeyAccounts({ WALK_SEED_KEY: KEY }).map(({ email }) => email)).toEqual(WALK_EMAILS.slice(0, 2));
  });

  describe.skipIf(!hasChromium)('in the browser', { timeout: TIMEOUT_MS }, () => {
    it('refuses an unexpected write before it leaves the browser and fails the journey naming its route', async () => {
      const { result, refused, sent } = await inJourney(async (page) => {
        const statuses = await fetchAll(page, [writes.unexpected]);
        await fetchAll(page, [{ ...writes.botGame, origin: elsewhere }]);
        return statuses;
      });
      expect(result).toEqual([REFUSED]);
      expect(refused).toEqual(['POST /api/v1/reports', `POST ${elsewhere}/api/v1/games`]);
      expect(sent).toEqual([]);
    });

    it('lets the allowed writes through, recording nothing', async () => {
      const { result, refused, sent } = await inJourney((page) =>
        fetchAll(page, [writes.botGame, writes.openGame, writes.pairGame, writes.settings, writes.pairRequest, writes.pairAccept, writes.pairMessage, writes.rewardsAck, writes.leaveGame]),
      );
      expect(result).toEqual([200, 200, 200, 200, 200, 200, 200, 200, 200]);
      expect(refused).toEqual([]);
      expect([...sent].sort()).toEqual([
        'PATCH /api/v1/me/settings',
        'POST /api/v1/conversations/c-pair/messages',
        'POST /api/v1/friends/requests',
        'POST /api/v1/friends/requests/r-pair/accept',
        'POST /api/v1/games',
        'POST /api/v1/games',
        'POST /api/v1/games',
        'POST /api/v1/games/g-open/leave',
        'POST /api/v1/me/rewards/ack',
      ]);
    });

    it('refuses a social write naming a third account, by body, path or the listed request or conversation', async () => {
      const social = [writes.strangerRequest, writes.strangerByName, writes.strangerUnfriend, writes.strangerAccept, writes.strangerMessage];
      const { result, refused, sent } = await inJourney((page) => fetchAll(page, social));
      expect(result).toEqual(social.map(() => REFUSED));
      expect([...refused].sort()).toEqual(
        [
          `POST /api/v1/friends/requests ${THIRD}`,
          `POST /api/v1/friends/requests ${THIRD}`,
          `DELETE /api/v1/friends/${STRANGER} ${THIRD}`,
          `POST /api/v1/friends/requests/r-stranger/accept ${THIRD}`,
          `POST /api/v1/conversations/c-stranger/messages ${THIRD}`,
        ].sort(),
      );
      expect(sent).toEqual([]);
    });
  });

  describe.skipIf(!hasChromium)('the auth refusals in the browser', { timeout: TIMEOUT_MS }, () => {
    it('lets a reset and a verify through only with the one token no reset or verify email can carry', async () => {
      const { result, refused, sent } = await inJourney((page) => fetchAll(page, [writes.resetUnmintable, writes.verifyUnmintable]));
      expect(result).toEqual([200, 200]);
      expect(refused).toEqual([]);
      expect([...sent].sort()).toEqual(['POST /api/v1/auth/email/verify', 'POST /api/v1/auth/password/reset']);
    });

    it('refuses a reset or verify of any other token, a reset with no token, a register and a forgot: each could write or mail', async () => {
      const others = [writes.resetOtherToken, writes.verifyOtherToken, writes.resetNoToken, writes.registerPair, writes.forgotPair];
      const { result, refused, sent } = await inJourney((page) => fetchAll(page, others));
      expect(result).toEqual(others.map(() => REFUSED));
      expect([...refused].sort()).toEqual(
        [
          `POST /api/v1/auth/password/reset ${OTHER_BODY}`,
          `POST /api/v1/auth/email/verify ${OTHER_BODY}`,
          `POST /api/v1/auth/password/reset ${OTHER_BODY}`,
          'POST /api/v1/auth/register',
          'POST /api/v1/auth/password/forgot',
        ].sort(),
      );
      expect(sent).toEqual([]);
    });
  });

  describe.skipIf(!hasChromium)('a game in the browser', { timeout: TIMEOUT_MS }, () => {
    it('refuses a game seating a third account or naming no seats, naming the route, and sends nothing', async () => {
      const seatings = [writes.strangerGame, writes.mixedGame, writes.seatlessGame];
      const { result, refused, sent } = await inJourney((page) => fetchAll(page, seatings));
      expect(result).toEqual(seatings.map(() => REFUSED));
      expect(refused).toEqual([`POST /api/v1/games ${THIRD}`, `POST /api/v1/games ${THIRD}`, `POST /api/v1/games ${THIRD}`]);
      expect(sent).toEqual([]);
    });
  });

  describe('through the request fixture', { timeout: TIMEOUT_MS }, () => {
    it('refuses the same: an unexpected write, a third account, and signing in account 3, each before it is sent and named', async () => {
      const { result, refused, sent } = await withApi(async (api) => [
        await send(api, writes.unexpected),
        await send(api, writes.strangerRequest),
        await send(api, writes.strangerMessage),
        await send(api, writes.smokeLogin),
        await send(api, writes.strangerGame),
        await send(api, writes.mixedGame),
      ]);
      expect(result).toEqual([
        expect.stringContaining('POST /api/v1/reports'),
        expect.stringContaining(`POST /api/v1/friends/requests ${THIRD}`),
        expect.stringContaining(`POST /api/v1/conversations/c-stranger/messages ${THIRD}`),
        expect.stringContaining(`POST /api/v1/auth/login ${THIRD}`),
        expect.stringContaining(`POST /api/v1/games ${THIRD}`),
        expect.stringContaining(`POST /api/v1/games ${THIRD}`),
      ]);
      expect(refused).toHaveLength(6);
      expect(sent).toEqual([]);
    });

    it('refuses a reset or verify of another token before it is sent, and sends the one with the unmintable token', async () => {
      const { result, refused, sent } = await withApi(async (api) => [
        await send(api, writes.resetOtherToken),
        await send(api, writes.verifyOtherToken),
        await send(api, writes.resetUnmintable),
        await send(api, writes.verifyUnmintable),
      ]);
      expect(result).toEqual([expect.stringContaining(`POST /api/v1/auth/password/reset ${OTHER_BODY}`), expect.stringContaining(`POST /api/v1/auth/email/verify ${OTHER_BODY}`), null, null]);
      expect(refused).toHaveLength(2);
      expect(sent).toEqual(['POST /api/v1/auth/password/reset', 'POST /api/v1/auth/email/verify']);
    });

    it('sends the allowed writes', async () => {
      const { result, refused, sent } = await withApi(async (api) => [await send(api, writes.pairLogin), await send(api, writes.pairMessage), await send(api, writes.botGame), await send(api, writes.pairGame), await send(api, writes.rewardsAck), await send(api, writes.leaveGame)]);
      expect(result).toEqual([null, null, null, null, null, null]);
      expect(refused).toEqual([]);
      expect(sent).toEqual(['POST /api/v1/auth/login', 'POST /api/v1/conversations/c-pair/messages', 'POST /api/v1/games', 'POST /api/v1/games', 'POST /api/v1/me/rewards/ack', 'POST /api/v1/games/g-open/leave']);
    });
  });
});
