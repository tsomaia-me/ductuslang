import { createServer } from 'node:http';

import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';

import { fixtureAnswers } from '../../../apps/admin/scripts/fixture-answers.mjs';
import { browsersPath, launchChromium, useInstalledBrowsers } from '../interaction-check.mjs';

const TIMEOUT_MS = 60_000;
const ENVIRONMENT_ACCESS = { CF_ACCESS_CLIENT_ID: 'from-the-environment.access', CF_ACCESS_CLIENT_SECRET: 'from-the-environment-secret' };
const FIXTURE_VARIANT = 'long-service-dev';
const MEDIA_LIST = '/api/media?page=1&pageSize=25';

const hasChromium = browsersPath() !== undefined;
let configs;
let server;
let local;
let other;
const received = [];

const effectiveUse = (config) => config.projects.map((project) => ({ name: project.name, use: { ...config.use, ...project.use } }));

beforeAll(async () => {
  useInstalledBrowsers();
  for (const [name, value] of Object.entries(ENVIRONMENT_ACCESS)) vi.stubEnv(name, value);
  configs = {
    'admin journeys': (await import('../../../apps/admin/e2e/journeys/playwright.config.ts')).default,
    'admin local': (await import('../../../apps/admin/e2e/journeys/local.config.ts')).default,
    'app journeys': (await import('../../../apps/app/e2e/journeys/playwright.config.ts')).default,
  };
  server = createServer((request, response) => {
    received.push(`${request.headers.host} ${request.url}`);
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(`<!doctype html><title>local</title><img src="${new URL(request.url, 'http://local.test').searchParams.get('img') ?? ''}">`);
  });
  await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
  const { port } = server.address();
  local = `http://127.0.0.1:${port}`;
  other = `http://localhost:${port}`;
}, TIMEOUT_MS);

afterAll(async () => {
  vi.unstubAllEnvs();
  await new Promise((done) => server.close(done));
});

describe('the journeys configs keep the Access service token to its origin (CD-352)', () => {
  it('records no request in a trace, video or HAR, sends headers on no project (CD-326: the app setup no longer does), and sends the Access token by route from the environment in the journeys, never in the admin local config', () => {
    for (const name of ['admin journeys', 'admin local', 'app journeys']) {
      for (const { name: project, use } of effectiveUse(configs[name])) {
        expect.soft(use.trace, `${name} ${project} trace`).toBe('off');
        expect.soft(use.video, `${name} ${project} video`).toBe('off');
        expect.soft(use.contextOptions?.recordHar, `${name} ${project} HAR`).toBeUndefined();
      }
      const sending = effectiveUse(configs[name]).filter(({ use }) => Object.keys(use.extraHTTPHeaders ?? {}).length > 0);
      expect.soft(sending.map(({ name: project }) => project), `${name} headers on every request`).toEqual([]);
    }
    for (const name of ['admin journeys', 'app journeys'])
      for (const { name: project, use } of effectiveUse(configs[name])) expect.soft(Object.values(use.accessHeaders), `${name} ${project} Access token`).toEqual(Object.values(ENVIRONMENT_ACCESS));
    const sent = Object.values(configs['admin local'].use.accessHeaders);
    expect.soft(sent, 'admin local Access token').toHaveLength(Object.keys(ENVIRONMENT_ACCESS).length);
    for (const value of Object.values(ENVIRONMENT_ACCESS)) expect.soft(sent, 'admin local Access token').not.toContain(value);
  });

  describe.skipIf(!hasChromium)('the admin local browser', { timeout: TIMEOUT_MS }, () => {
    async function loadsFrom(launchOptions) {
      received.length = 0;
      const browser = await launchChromium(launchOptions);
      try {
        const page = await browser.newPage();
        await page.goto(`${local}/?img=${encodeURIComponent(`${other}/media.svg`)}`);
        return [...received];
      } finally {
        await browser.close();
      }
    }

    it('reaches nothing but 127.0.0.1, where a browser without its options reaches another host', async () => {
      const host = new URL(local).host;
      expect(await loadsFrom({})).toContain(`${new URL(other).host} /media.svg`);
      expect((await loadsFrom(configs['admin local'].use.launchOptions)).every((line) => line.startsWith(`${host} `))).toBe(true);
    });
  });
});

describe('the admin fixture server answers the fixtures media itself (CD-352)', () => {
  let fixtures;
  let origin;

  beforeAll(async () => {
    fixtures = createServer((request, response) =>
      fixtureAnswers(origin, FIXTURE_VARIANT)(request, response, () => {
        response.writeHead(404);
        response.end();
      }),
    );
    await new Promise((ready) => fixtures.listen(0, '127.0.0.1', ready));
    origin = `http://127.0.0.1:${fixtures.address().port}`;
  });

  afterAll(() => new Promise((done) => fixtures.close(done)));

  it('names only its own origin for media, answers those addresses with an image, and answers the variant a request names, its own variant otherwise', async () => {
    const body = await (await fetch(`${origin}${MEDIA_LIST}`)).text();
    const addresses = body.match(/https?:\/\/[^"]+/g) ?? [];
    expect.soft(addresses.length, 'media addresses').toBeGreaterThan(0);
    expect.soft(addresses.filter((address) => !address.startsWith(origin)), 'media addresses').toEqual([]);
    const media = await fetch(JSON.parse(body).items[0].url);
    expect.soft(media.status, 'media').toBe(200);
    expect.soft(media.headers.get('content-type'), 'media').toMatch(/^image\//);
    const settings = async (headers) => (await (await fetch(`${origin}/api/settings`, { headers })).json()).settings?.rounds;
    expect.soft(await settings({}), 'own variant').toBeUndefined();
    expect.soft(await settings({ 'x-fixture-variant': 'settings-declared-service' }), 'named variant').toBeDefined();
  });
});
