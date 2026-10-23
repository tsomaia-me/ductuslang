import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { childEnv } from '../../scripts/__tests__/hermetic.mjs';
import { browsersPath } from '../interaction-check.mjs';

const TIMEOUT_MS = 180_000;
const LOCAL_HOST = '127.0.0.1';
const PREVIEW = '/api/config/preview';
const JOURNEY_PROJECT = 'desktop';
const ADMIN = fileURLToPath(new URL('../../../apps/admin/', import.meta.url));
const FIXTURES = join(ADMIN, 'e2e/journeys/fixtures.ts');
const JOURNEYS_CONFIG = join(ADMIN, 'e2e/journeys/journeys-config.ts');
const PLAYWRIGHT_CLI = createRequire(join(ADMIN, 'package.json')).resolve('@playwright/test/cli');
const ACCESS = { 'CF-Access-Client-Id': 'dummy-fixture-id.access', 'CF-Access-Client-Secret': 'dummy-fixture-secret-zz9' };
const LOCAL_ONLY = [`--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE ${LOCAL_HOST}`, '--no-proxy-server'];
const UNSET = ['CF_ACCESS_CLIENT_ID', 'CF_ACCESS_CLIENT_SECRET', 'NODE_USE_ENV_PROXY'];

const PLAYWRIGHT_TEST_TIMEOUT_MS = 30_000;
const SLOW_TEST_TIMEOUT_MS = 3 * PLAYWRIGHT_TEST_TIMEOUT_MS;
const ACTION_TIMEOUT_MS = 10_000;
const MISSING_BUTTON = 'A button the page never renders';

const JOURNEYS = {
  request: 'a journey writes through the request fixture',
  pageRequest: 'a journey writes through page.request',
  allowed: 'a journey sends the preview and reads',
  budget: 'a journey reads its own time budget',
  slowBudget: 'a slow journey reads its own time budget',
  missing: 'a journey clicks a button the page never renders',
};

const spec = `import { expect, test } from ${JSON.stringify(FIXTURES)};

test(${JSON.stringify(JOURNEYS.request)}, async ({ request }) => {
  await request.post('/api/users/u1/ban').catch(() => null);
});

test(${JSON.stringify(JOURNEYS.pageRequest)}, async ({ page }) => {
  await page.request.delete('/api/media/m1').catch(() => null);
});

test(${JSON.stringify(JOURNEYS.allowed)}, async ({ request, page }) => {
  await expect(await request.post(${JSON.stringify(PREVIEW)})).toBeOK();
  await expect(await page.request.get('/api/me')).toBeOK();
});

const budget = () => test.info().annotations.push({ type: 'timeout', description: String(test.info().timeout) });

test(${JSON.stringify(JOURNEYS.budget)}, budget);

test(${JSON.stringify(JOURNEYS.slowBudget)}, () => {
  test.slow();
  budget();
});

test(${JSON.stringify(JOURNEYS.missing)}, async ({ page }) => {
  await page.setContent('<main><button>Another button</button></main>');
  await page.getByRole('button', { name: ${JSON.stringify(MISSING_BUTTON)} }).click();
});
`;

const config = (baseURL) => `import { journeysConfig } from ${JSON.stringify(JOURNEYS_CONFIG)};

const journeys = journeysConfig({ baseURL: ${JSON.stringify(baseURL)}, accessHeaders: ${JSON.stringify(ACCESS)} });

export default { ...journeys, use: { ...journeys.use, launchOptions: { args: ${JSON.stringify(LOCAL_ONLY)} } } };
`;

const hasChromium = browsersPath() !== undefined;
let server;
let dir;
let report;
let run;
const received = [];

function playwright(args, env) {
  return new Promise((done, failed) => {
    const child = spawn(process.execPath, [PLAYWRIGHT_CLI, ...args], { cwd: dir, env, stdio: ['ignore', 'pipe', 'pipe'] });
    let output = '';
    child.stdout.on('data', (chunk) => (output += chunk));
    child.stderr.on('data', (chunk) => (output += chunk));
    child.on('error', failed);
    child.on('close', (status) => done({ status, output }));
  });
}

const results = () =>
  JSON.parse(readFileSync(report, 'utf8')).suites.flatMap((suite) =>
    suite.specs.map(({ title, tests }) => ({
      title,
      status: tests[0].status,
      errors: tests[0].results.flatMap((result) => result.errors.map((error) => error.message)).join('\n'),
    })),
  );
const journey = (title) => results().find((result) => result.title === title);
const annotated = (title, type) =>
  JSON.parse(readFileSync(report, 'utf8'))
    .suites.flatMap((suite) => suite.specs)
    .find((found) => found.title === title)
    .tests[0].annotations.filter((annotation) => annotation.type === type)
    .map((annotation) => annotation.description);

beforeAll(async () => {
  if (!hasChromium) return;
  server = createServer((request, response) => {
    received.push(`${request.method} ${new URL(request.url, `http://${LOCAL_HOST}`).pathname}`);
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end('{}');
  });
  await new Promise((ready) => server.listen(0, LOCAL_HOST, ready));
  dir = mkdtempSync(join(tmpdir(), 'cd352-fixtures-'));
  report = join(dir, 'report.json');
  writeFileSync(join(dir, 'package.json'), JSON.stringify({ type: 'module' }));
  writeFileSync(join(dir, 'writes.spec.ts'), spec);
  writeFileSync(join(dir, 'playwright.config.ts'), config(`http://${LOCAL_HOST}:${server.address().port}`));
  const env = childEnv({ LIVE_REPORT: report });
  for (const name of UNSET) delete env[name];
  run = await playwright(['test', '-c', 'playwright.config.ts', '--project', JOURNEY_PROJECT, '--no-deps'], env);
}, TIMEOUT_MS);

afterAll(async () => {
  if (server) await new Promise((done) => server.close(done));
  if (dir) rmSync(dir, { recursive: true, force: true });
});

describe.skipIf(!hasChromium)('the admin journey fixtures in a real playwright run (CD-352)', () => {
  it('fails the run on a journey that writes through the request fixture or page.request, naming the route, and on a click past the action timeout, passes one that sends only the preview and reads, and gives each the default budget, tripled under test.slow', () => {
    expect.soft(run.status, run.output).toBe(1);
    expect.soft(received.filter((line) => !line.startsWith('GET ')), 'writes the server saw').toEqual([`POST ${PREVIEW}`]);
    for (const [title, status, errors] of [
      [JOURNEYS.request, 'unexpected', expect.stringContaining('POST /api/users/u1/ban')],
      [JOURNEYS.pageRequest, 'unexpected', expect.stringContaining('DELETE /api/media/m1')],
      [JOURNEYS.allowed, 'expected', ''],
      [JOURNEYS.missing, 'unexpected', expect.stringContaining(`Timeout ${ACTION_TIMEOUT_MS}ms exceeded`)],
    ])
      expect.soft(journey(title), title).toEqual({ title, status, errors });
    expect
      .soft([annotated(JOURNEYS.budget, 'timeout'), annotated(JOURNEYS.slowBudget, 'timeout')], 'time budgets')
      .toEqual([[String(PLAYWRIGHT_TEST_TIMEOUT_MS)], [String(SLOW_TEST_TIMEOUT_MS)]]);
  });
});
