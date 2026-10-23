import { createServer } from 'node:http';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { walkContext } from '../../../apps/app/e2e/live/walk-access.ts';
import { HIDDEN_ACCESS, MOST_SAME_ORIGIN_HOPS } from '../../e2e/access.mjs';
import { playwrightBrowser } from '../../scripts/smoke-walk.mjs';
import { browsersPath, launchChromium, runInteractionCheck, useInstalledBrowsers } from '../interaction-check.mjs';

const TIMEOUT_MS = 60_000;
const ACCESS = { 'CF-Access-Client-Id': 'dummy-id-cd353.access', 'CF-Access-Client-Secret': 'dummy-secret-cd353' };
const ACCESS_ENV = { CF_ACCESS_CLIENT_ID: ACCESS['CF-Access-Client-Id'], CF_ACCESS_CLIENT_SECRET: ACCESS['CF-Access-Client-Secret'] };
const NO_ACCESS = [undefined, undefined];
const REDIRECTED = 302;
const MOVED_FOR_GOOD = 308;
const TEMPORARY_REDIRECT = 307;
const CLOSED_PORT = 1;
const LOCAL_NETWORK_CHECKS = 'LocalNetworkAccessChecks,BlockInsecurePrivateNetworkRequests';
const LAUNCH = { args: [`--disable-features=${LOCAL_NETWORK_CHECKS}`] };
const SVG = '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>';

const hasChromium = browsersPath() !== undefined;
let server;
let portAsideServer;
let site;
let portAside;
let elsewhere;
let browser;
const received = [];

const images = () => [`${elsewhere}/thumb.svg`, `/redirect?to=${encodeURIComponent(`${elsewhere}/moved.svg`)}`, '/own.svg'];
const loadedImages = () => `Promise.all(${JSON.stringify(images())}.map((src) => new Promise((done) => { const image = new Image(); image.onload = image.onerror = done; image.src = src; document.body.append(image); })))`;
const html = (body) => `<!doctype html><meta charset="utf-8"><title>site</title>${body}`;
const SCREEN = () => html(`<p>screen</p><script>${loadedImages()}</script>`);
const LOGIN = () =>
  html(`<input data-testid="login-email"><input data-testid="login-password">
<button data-testid="login-submit">Sign in</button>
<script>
document.querySelector('[data-testid="login-submit"]').addEventListener('click', async () => {
  await fetch('/api/v1/auth/login', { method: 'POST', body: '{}' });
  await ${loadedImages()};
  const home = document.createElement('div');
  home.dataset.testid = 'home-screen';
  document.body.append(home);
});
</script>`);

const answer = (request, response) => {
  const url = new URL(request.url, 'http://fixture.test');
  received.push({
    line: `${request.method} ${url.pathname}`,
    origin: `http://${request.headers.host}`,
    access: [request.headers['cf-access-client-id'], request.headers['cf-access-client-secret']],
    check: request.headers['x-check'],
  });
  const pages = { '/login': LOGIN, '/screen': SCREEN };
  if (url.pathname in pages) {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    return response.end(pages[url.pathname]());
  }
  if (url.pathname === '/redirect') {
    response.writeHead(REDIRECTED, { location: url.searchParams.get('to') });
    return response.end();
  }
  if (url.pathname === '/settings') {
    response.writeHead(MOVED_FOR_GOOD, { location: '/settings/' });
    return response.end();
  }
  if (url.pathname === '/settings/') {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    return response.end(html('<p>settings</p>'));
  }
  if (url.pathname === '/post') {
    response.writeHead(TEMPORARY_REDIRECT, { location: '/post2' });
    return response.end();
  }
  if (url.pathname === '/loop') {
    response.writeHead(REDIRECTED, { location: '/loop' });
    return response.end();
  }
  if (url.pathname.startsWith('/api/')) {
    response.writeHead(200, { 'content-type': 'application/json' });
    return response.end('{}');
  }
  response.writeHead(200, { 'content-type': 'image/svg+xml' });
  return response.end(SVG);
};

beforeAll(async () => {
  server = createServer(answer);
  portAsideServer = createServer(answer);
  await Promise.all([server, portAsideServer].map((listener) => new Promise((ready) => listener.listen(0, '127.0.0.1', ready))));
  const { port } = server.address();
  site = `http://127.0.0.1:${port}`;
  elsewhere = `http://localhost:${port}`;
  portAside = `http://127.0.0.1:${portAsideServer.address().port}`;
  useInstalledBrowsers();
  if (hasChromium) browser = await launchChromium(LAUNCH);
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
  await Promise.all([server, portAsideServer].map((listener) => new Promise((done) => listener.close(done))));
});

const linesTo = (origin) => received.filter((entry) => entry.origin === origin).map(({ line }) => line);
const accessTo = (origin) => received.filter((entry) => entry.origin === origin).map(({ access }) => access);
const failure = (sending) => sending.then(() => null, (error) => String(error.message));

function expectTokenOnTheSiteAlone(siteLines) {
  expect(linesTo(elsewhere).sort()).toEqual(['GET /moved.svg', 'GET /thumb.svg']);
  expect(accessTo(elsewhere)).toEqual([NO_ACCESS, NO_ACCESS]);
  expect(linesTo(site)).toEqual(expect.arrayContaining(siteLines));
  expect(accessTo(site)).toEqual(accessTo(site).map(() => Object.values(ACCESS)));
}

describe.skipIf(!hasChromium)('the release tools send the Access token only to the origins behind Access (CD-353)', { timeout: TIMEOUT_MS }, () => {
  it('the smoke walk: the walked origin gets it, another origin a page loads or a redirect leads to does not', async () => {
    received.length = 0;
    const rendered = await playwrightBrowser({
      web: site,
      api: site,
      sameOrigin: true,
      account: { email: 'walk@example.test', password: 'walk-password' },
      headers: ACCESS,
      launch: () => launchChromium(LAUNCH),
    }).catch((error) => String(error.message));
    expect(rendered).toBe('Home rendered at 390×844');
    expectTokenOnTheSiteAlone(['GET /login', 'POST /api/v1/auth/login', 'GET /redirect', 'GET /own.svg']);
  });

  it('the interaction check: headers, the token and --header alike, go to the --base origin alone', async () => {
    received.length = 0;
    const { findings } = await runInteractionCheck({
      base: site,
      paths: ['/screen'],
      headers: { ...ACCESS, 'X-Check': 'yes' },
      viewports: ['desktop'],
      only: ['console'],
      browser,
    });
    expect(findings).toEqual([]);
    expectTokenOnTheSiteAlone(['GET /screen', 'GET /redirect', 'GET /own.svg']);
    expect(received.filter((entry) => entry.origin === elsewhere).map(({ check }) => check)).toEqual([undefined, undefined]);
    expect(received.filter((entry) => entry.origin === site).every(({ check }) => check === 'yes')).toBe(true);
  });

  describe('the live walk', () => {
    async function inWalk(walk, extra = {}) {
      received.length = 0;
      const context = await walkContext(browser, { baseURL: site }, { ...ACCESS_ENV, LIVE_URL: site, ...extra });
      try {
        return await walk(await context.newPage());
      } finally {
        await context.close();
      }
    }

    it("the browser: the walked origin gets it, another origin a page loads or a redirect leads to does not", async () => {
      await inWalk((page) => page.goto(`${site}/screen`, { waitUntil: 'networkidle' }));
      expectTokenOnTheSiteAlone(['GET /screen', 'GET /redirect', 'GET /own.svg']);
    });

    it('a redirect that stays on the walked origin is followed with the token, and the browser gets the page it leads to', async () => {
      const shown = await inWalk(async (page) => {
        await page.goto(`${site}/settings`);
        return page.locator('p').textContent();
      });
      expect(shown).toBe('settings');
      expect(linesTo(site)).toEqual(['GET /settings', 'GET /settings/']);
      expect(accessTo(site)).toEqual([Object.values(ACCESS), Object.values(ACCESS)]);
    });

    it('a redirect to another origin or to the same host on another port goes back to the browser unfollowed, and the browser follows it without the token', async () => {
      await inWalk((page) => page.goto(`${site}/redirect?to=${encodeURIComponent(`${elsewhere}/screen`)}`, { waitUntil: 'networkidle' }));
      expect.soft(linesTo(site), 'another origin').toEqual(expect.arrayContaining(['GET /redirect']));
      expect.soft(accessTo(site), 'another origin').toEqual(accessTo(site).map(() => Object.values(ACCESS)));
      expect.soft(linesTo(elsewhere), 'another origin').toEqual(expect.arrayContaining(['GET /screen']));
      expect.soft(accessTo(elsewhere).every((access) => access.every((value) => value === undefined)), 'another origin').toBe(true);

      await inWalk((page) => page.goto(`${site}/redirect?to=${encodeURIComponent(`${portAside}/plain.svg`)}`, { waitUntil: 'networkidle' }));
      expect.soft(linesTo(site), 'another port').toEqual(['GET /redirect']);
      expect.soft(accessTo(site), 'another port').toEqual([Object.values(ACCESS)]);
      expect.soft(linesTo(portAside), 'another port').toEqual(['GET /plain.svg']);
      expect.soft(accessTo(portAside), 'another port').toEqual([NO_ACCESS]);
    });

    it('a write answered with a redirect is not followed by the route, and no request to its Location carries the token', async () => {
      await inWalk(async (page) => {
        await page.goto(`${site}/settings/`);
        received.length = 0;
        await page.evaluate(() => fetch('/post', { method: 'POST', body: '{"write":true}' }).then((response) => response.status));
      });
      expect(received.filter(({ line }) => line === 'POST /post').map(({ access }) => access)).toEqual([Object.values(ACCESS)]);
      expect(received.filter(({ line }) => line === 'POST /post2').map(({ access }) => access)).toEqual([NO_ACCESS]);
    });

    it('follows a same-origin redirect only a bounded number of hops with the token', async () => {
      await inWalk((page) => failure(page.goto(`${site}/loop`)));
      const withToken = received.filter((entry) => entry.line === 'GET /loop' && entry.access[0] !== undefined);
      expect(withToken).toHaveLength(MOST_SAME_ORIGIN_HOPS + 1);
    });

    it("the page's API requests: the walked origin gets it, another origin does not, and no redirect is followed with it", async () => {
      const statuses = await inWalk(async (page) => [
        (await page.request.get('/api/v1/me')).status(),
        (await page.request.get(`${elsewhere}/api/v1/me`)).status(),
        (await page.request.get(`/redirect?to=${encodeURIComponent(`${elsewhere}/moved.svg`)}`)).status(),
      ]);
      expect(statuses).toEqual([200, 200, REDIRECTED]);
      expect(linesTo(site)).toEqual(['GET /api/v1/me', 'GET /redirect']);
      expect(accessTo(site)).toEqual([Object.values(ACCESS), Object.values(ACCESS)]);
      expect(linesTo(elsewhere)).toEqual(['GET /api/v1/me']);
      expect(accessTo(elsewhere)).toEqual([NO_ACCESS]);
    });

    it('stays offline while the context is offline: the route sends nothing on its behalf', async () => {
      const [offline, online] = await inWalk(async (page) => {
        await page.goto(`${site}/screen`, { waitUntil: 'networkidle' });
        received.length = 0;
        const load = () => page.evaluate((url) => fetch(url).then((response) => response.status, () => 'failed'), `${site}/api/v1/me`);
        await page.context().setOffline(true);
        const whileOffline = await load();
        await page.context().setOffline(false);
        return [whileOffline, await load()];
      });
      expect([offline, online]).toEqual(['failed', 200]);
      expect(linesTo(site)).toEqual(['GET /api/v1/me']);
    });

    it("keeps the token out of a failed API request's error", async () => {
      const closedApi = `http://127.0.0.1:${CLOSED_PORT}`;
      const error = await inWalk((page) => failure(page.request.get(`${closedApi}/api/v1/me`)), { LIVE_API_URL: closedApi });
      expect(error).toContain('ECONNREFUSED');
      expect(error).toContain(HIDDEN_ACCESS);
      for (const value of Object.values(ACCESS)) expect(error).not.toContain(value);
    });
  });
});
