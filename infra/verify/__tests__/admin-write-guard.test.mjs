import { createServer } from 'node:http';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { guardJourney } from '../../../apps/admin/e2e/journeys/journey-guard.ts';
import { guardRequests } from '../../../apps/admin/e2e/journeys/request-guard.ts';
import { browsersPath, launchChromium, useInstalledBrowsers } from '../interaction-check.mjs';

const TIMEOUT_MS = 60_000;
const PREVIEW = '/api/config/preview';
const REFUSED = 405;
const FAILED = 500;
const REDIRECTED = 302;
const ACCESS = { 'CF-Access-Client-Id': 'dummy-id.access', 'CF-Access-Client-Secret': 'dummy-secret-zz9' };
const NO_ACCESS = [undefined, undefined];
const JOURNEY_PROJECT = 'desktop';
const CLOSED_PORT = 1;
const LOCAL_NETWORK_CHECKS = 'LocalNetworkAccessChecks,BlockInsecurePrivateNetworkRequests';

const hasChromium = browsersPath() !== undefined;
let server;
let admin;
let elsewhere;
let browser;
let apiRequest;
let playwrightExpect;
let journeysConfig;
const received = [];

const html = (body) => `<!doctype html><title>admin</title>${body}`;

beforeAll(async () => {
  server = createServer((request, response) => {
    const url = new URL(request.url, 'http://fixture.test');
    received.push({
      line: `${request.method} ${url.pathname}`,
      origin: `http://${request.headers.host}`,
      access: [request.headers['cf-access-client-id'], request.headers['cf-access-client-secret']],
    });
    if (url.pathname === '/') {
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
      return response.end(html(url.searchParams.getAll('img').map((src) => `<img src="${src}">`).join('')));
    }
    if (url.pathname === '/redirect') {
      response.writeHead(REDIRECTED, { location: url.searchParams.get('to') });
      return response.end();
    }
    if (url.pathname === '/failing') {
      response.writeHead(FAILED, { 'content-type': 'text/plain' });
      return response.end('failed');
    }
    response.writeHead(200, { 'content-type': 'application/json' });
    return response.end('{}');
  });
  await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
  const { port } = server.address();
  admin = `http://127.0.0.1:${port}`;
  elsewhere = `http://localhost:${port}`;
  useInstalledBrowsers();
  if (hasChromium) browser = await launchChromium({ args: [`--disable-features=${LOCAL_NETWORK_CHECKS}`] });
  ({ request: apiRequest } = await import('playwright'));
  ({ expect: playwrightExpect } = await import('@playwright/test'));
  ({ journeysConfig } = await import('../../../apps/admin/e2e/journeys/journeys-config.ts'));
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
  await new Promise((done) => server.close(done));
});

const writesSent = () => received.filter(({ line }) => !line.startsWith('GET ')).map(({ line }) => line);
const linesTo = (origin) => received.filter((entry) => entry.origin === origin).map(({ line }) => line);
const accessTo = (origin) => received.filter((entry) => entry.origin === origin).map(({ access }) => access);
const failure = (sending) => sending.then(() => null, (error) => String(error.message));

async function journeyContext() {
  const config = journeysConfig({ baseURL: admin, accessHeaders: ACCESS });
  const project = config.projects.find(({ name }) => name === JOURNEY_PROJECT);
  const use = { ...config.use, ...project.use };
  const context = await browser.newContext({
    baseURL: use.baseURL,
    viewport: use.viewport,
    colorScheme: use.colorScheme,
    extraHTTPHeaders: use.extraHTTPHeaders,
  });
  const guard = await guardJourney(context, { baseURL: use.baseURL, headers: use.accessHeaders });
  return { context, guard };
}

async function inJourney(walk) {
  received.length = 0;
  const { context, guard } = await journeyContext();
  try {
    const page = await context.newPage();
    await page.goto(`${admin}/`);
    const result = await walk(page);
    return { result, refused: await guard.refused(), sent: writesSent() };
  } finally {
    await context.close();
  }
}

const fetchAll = (page, requests) =>
  page.evaluate(
    (calls) =>
      Promise.all(calls.map(({ method, url, mode }) => fetch(url, { method, mode }).then((response) => response.status))),
    requests.map((call) => ({ ...call, mode: call.url.startsWith(admin) ? 'same-origin' : 'no-cors' })),
  );

async function withApi(walk) {
  received.length = 0;
  const api = await apiRequest.newContext({ baseURL: admin });
  try {
    const guard = guardRequests(api, { baseURL: admin, headers: ACCESS });
    const result = await walk(api);
    return { result, refused: guard.refused(), sent: writesSent() };
  } finally {
    await api.dispose();
  }
}

describe('the admin journeys write guard (CD-352)', () => {
  describe.skipIf(!hasChromium)('in the browser', { timeout: TIMEOUT_MS }, () => {
    it('refuses every other POST, PUT, PATCH and DELETE before it leaves the browser, naming each route', async () => {
      const writes = [
        { method: 'POST', path: '/api/users/u1/ban' },
        { method: 'PUT', path: '/api/settings' },
        { method: 'PATCH', path: '/api/config/drafts/d1' },
        { method: 'DELETE', path: '/api/media/m1' },
        { method: 'PUT', path: PREVIEW },
      ];
      const { result, refused, sent } = await inJourney((page) =>
        fetchAll(page, writes.map(({ method, path }) => ({ method, url: `${admin}${path}` }))),
      );
      expect(result).toEqual(writes.map(() => REFUSED));
      expect(refused).toEqual(writes.map(({ method, path }) => `${method} ${path}`));
      expect(sent).toEqual([]);
    });

    it('lets the config preview and reads through on the admin origin, recording nothing, and refuses the preview on another origin, naming that origin', async () => {
      const { result, refused, sent } = await inJourney((page) =>
        fetchAll(page, [
          { method: 'POST', url: `${admin}${PREVIEW}` },
          { method: 'GET', url: `${admin}/api/me` },
          { method: 'POST', url: `${elsewhere}${PREVIEW}` },
        ]),
      );
      expect.soft(result.slice(0, 2), 'admin origin').toEqual([200, 200]);
      expect.soft(refused).toEqual([`POST ${elsewhere}${PREVIEW}`]);
      expect.soft(sent).toEqual([`POST ${PREVIEW}`]);
    });

    it('sends the Access headers to the admin origin only, never to another origin a page loads or is redirected to', async () => {
      await inJourney(async (page) => {
        const images = [`${elsewhere}/thumb.svg`, `/redirect?to=${encodeURIComponent(`${elsewhere}/moved.svg`)}`, '/own.svg'];
        await page.goto(`${admin}/?${images.map((src) => `img=${encodeURIComponent(src)}`).join('&')}`);
        await fetchAll(page, [{ method: 'POST', url: `${admin}${PREVIEW}` }]);
      });
      expect(linesTo(elsewhere).sort()).toEqual(['GET /moved.svg', 'GET /thumb.svg']);
      expect(accessTo(elsewhere)).toEqual([NO_ACCESS, NO_ACCESS]);
      expect(linesTo(admin)).toEqual(expect.arrayContaining(['GET /', 'GET /redirect', 'GET /own.svg', `POST ${PREVIEW}`]));
      expect(accessTo(admin)).toEqual(accessTo(admin).map(() => Object.values(ACCESS)));
    });

    it('fails the journey on a write its own route continued to the server, naming the route, and lets a write the journey fulfils itself pass', async () => {
      for (const [answer, sentToServer, findings] of [
        ['continue', ['DELETE /api/media/m1'], ['DELETE /api/media/m1 reached the server']],
        ['fulfill', [], []],
      ]) {
        const { refused, sent } = await inJourney(async (page) => {
          await page.route('**/api/media/m1', (route) => (answer === 'continue' ? route.continue() : route.fulfill({ status: 204 })));
          return fetchAll(page, [{ method: 'DELETE', url: `${admin}/api/media/m1` }]);
        });
        expect.soft(sent, answer).toEqual(sentToServer);
        expect.soft(refused, answer).toEqual(findings);
      }
    });

    it("refuses writes through the page's request context and a route's fetch, naming each route", async () => {
      const { result, refused, sent } = await inJourney(async (page) => {
        const direct = await failure(page.request.post('/api/users/u1/ban'));
        await page.route('**/api/settings', async (route) => {
          const fetched = await failure(route.fetch());
          await route.abort();
          return fetched;
        });
        await failure(fetchAll(page, [{ method: 'PUT', url: `${admin}/api/settings` }]));
        return direct;
      });
      expect(result).toContain('POST /api/users/u1/ban');
      expect([...refused].sort()).toEqual(['POST /api/users/u1/ban', 'PUT /api/settings']);
      expect(sent).toEqual([]);
    });
  });

  describe('through the request fixture', { timeout: TIMEOUT_MS }, () => {
    it('refuses every write but the preview before it is sent, naming the route', async () => {
      const writes = ['POST /api/users/u1/ban', 'PUT /api/settings', 'PATCH /api/config/drafts/d1', 'DELETE /api/media/m1', 'DELETE /api/media/m2', `POST ${elsewhere}${PREVIEW}`];
      const { result, refused, sent } = await withApi(async (api) => [
        await failure(api.post('/api/users/u1/ban')),
        await failure(api.put('/api/settings')),
        await failure(api.patch('/api/config/drafts/d1')),
        await failure(api.delete('/api/media/m1')),
        await failure(api.fetch('/api/media/m2', { method: 'delete' })),
        await failure(api.post(`${elsewhere}${PREVIEW}`)),
      ]);
      expect(result).toEqual(writes.map((write) => expect.stringContaining(write)));
      expect(refused).toEqual(writes);
      expect(sent).toEqual([]);
    });

    it('sends the preview and reads, with the Access headers on the admin origin only, and never follows a redirect with them', async () => {
      const { result, refused, sent } = await withApi(async (api) => [
        (await api.post(PREVIEW)).status(),
        (await api.get('/api/me')).status(),
        (await api.get(`${elsewhere}/api/me`)).status(),
        (await api.get(`/redirect?to=${encodeURIComponent(`${elsewhere}/moved`)}`)).status(),
      ]);
      expect.soft(result).toEqual([200, 200, 200, REDIRECTED]);
      expect.soft(refused).toEqual([]);
      expect.soft(sent).toEqual([`POST ${PREVIEW}`]);
      expect.soft(accessTo(admin)).toEqual([Object.values(ACCESS), Object.values(ACCESS), Object.values(ACCESS)]);
      expect.soft(accessTo(elsewhere)).toEqual([NO_ACCESS]);
      expect.soft(linesTo(elsewhere), 'the redirect').toEqual(['GET /api/me']);
    });

    it('keeps the Access values out of failure messages and logs', async () => {
      const { result } = await withApi(async (api) => {
        const response = await api.get('/failing');
        const notOk = await failure(Promise.resolve().then(() => playwrightExpect(response).toBeOK()));
        const unreachableAdmin = `http://127.0.0.1:${CLOSED_PORT}`;
        const closed = await apiRequest.newContext();
        guardRequests(closed, { baseURL: unreachableAdmin, headers: ACCESS });
        const unreachable = await failure(closed.get(`${unreachableAdmin}/api/me`)).finally(() => closed.dispose());
        return [notOk, unreachable];
      });
      expect(result).toEqual([expect.stringContaining(String(FAILED)), expect.stringContaining('ECONNREFUSED')]);
      for (const value of Object.values(ACCESS)) expect(result.join('\n')).not.toContain(value);
    });
  });

  it('leaves every journey Playwright’s default per-test time budget', () => {
    expect(journeysConfig({ baseURL: admin, accessHeaders: ACCESS })).not.toHaveProperty('timeout');
  });
});
