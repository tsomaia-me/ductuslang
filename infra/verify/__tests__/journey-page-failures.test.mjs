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
const JOURNEY_PROJECT = 'desktop';
const ADMIN = fileURLToPath(new URL('../../../apps/admin/', import.meta.url));
const FIXTURES = join(ADMIN, 'e2e/journeys/fixtures.ts');
const JOURNEYS_CONFIG = join(ADMIN, 'e2e/journeys/journeys-config.ts');
const PLAYWRIGHT_CLI = createRequire(join(ADMIN, 'package.json')).resolve('@playwright/test/cli');
const ACCESS = { 'CF-Access-Client-Id': 'dummy-fixture-id.access', 'CF-Access-Client-Secret': 'dummy-fixture-secret-zz9' };
const LOCAL_ONLY = [`--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE ${LOCAL_HOST}`, '--no-proxy-server'];
const UNSET = ['CF_ACCESS_CLIENT_ID', 'CF_ACCESS_CLIENT_SECRET', 'NODE_USE_ENV_PROXY'];
const LOGGED = 'a console error the page logs';

const JOURNEYS = {
  clean: 'a journey on a page that loads cleanly',
  logs: 'a journey on a page that logs a console error',
  thrown: 'a journey on a page that throws',
  undeclared: 'a journey whose page gets a 404 it did not expect',
  declared: 'a journey whose page gets the 404 it expected',
  serverError: 'a journey whose page gets the 500 it expected',
  otherStatus: 'a journey whose page gets a 500 where it expected a 404',
  routed: 'a journey whose page keeps asking for an address the journey answered',
};

const page = (script) => `<!doctype html><html><head><meta charset="utf-8"></head><body><main>journey</main><script>${script}</script></body></html>`;
const PAGES = {
  '/clean': page(''),
  '/logs': page(`console.error(${JSON.stringify(LOGGED)});`),
  '/throws': page("setTimeout(() => { throw new Error('the page threw'); });"),
  '/gets-404': page("fetch('/answer/404');"),
  '/gets-500': page("fetch('/answer/500');"),
};

const spec = `import { test } from ${JSON.stringify(FIXTURES)};

const open = async (page, path, answer) => {
  const answered = answer ? page.waitForResponse((response) => response.url().endsWith(answer)) : null;
  await page.goto(path);
  await answered;
  await page.waitForTimeout(200);
};

test(${JSON.stringify(JOURNEYS.clean)}, async ({ page }) => {
  await open(page, '/clean');
});

test(${JSON.stringify(JOURNEYS.logs)}, async ({ page }) => {
  await open(page, '/logs');
});

test(${JSON.stringify(JOURNEYS.thrown)}, async ({ page }) => {
  await open(page, '/throws');
});

test(${JSON.stringify(JOURNEYS.undeclared)}, async ({ page }) => {
  await open(page, '/gets-404', '/answer/404');
});

test(${JSON.stringify(JOURNEYS.declared)}, async ({ page, pageFailures }) => {
  pageFailures.expect(/\\/answer\\/404$/, 404);
  await open(page, '/gets-404', '/answer/404');
});

test(${JSON.stringify(JOURNEYS.serverError)}, async ({ page, pageFailures }) => {
  pageFailures.expect(/\\/answer\\/500$/, 500);
  await open(page, '/gets-500', '/answer/500');
});

test(${JSON.stringify(JOURNEYS.otherStatus)}, async ({ page, pageFailures }) => {
  pageFailures.expect(/\\/answer\\/500$/, 404);
  await open(page, '/gets-500', '/answer/500');
});

test(${JSON.stringify(JOURNEYS.routed)}, async ({ page }) => {
  await page.route('**/answer/404?polled', (route) => route.fulfill({ json: {} }));
  await open(page, '/clean');
  const asking = setInterval(() => page.evaluate(() => fetch('/answer/404?polled').then(() => null)).catch(() => null), 5);
  setTimeout(() => clearInterval(asking), 3000);
  await page.waitForTimeout(100);
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
let origin;

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

const journey = (title) => {
  const found = JSON.parse(readFileSync(report, 'utf8'))
    .suites.flatMap((suite) => suite.specs)
    .find((each) => each.title === title);
  return { status: found.tests[0].status, errors: found.tests[0].results.flatMap((result) => result.errors.map((error) => error.message)).join('\n') };
};

beforeAll(async () => {
  if (!hasChromium) return;
  server = createServer((request, response) => {
    const path = new URL(request.url, `http://${LOCAL_HOST}`).pathname;
    const status = /^\/answer\/(\d{3})$/.exec(path)?.[1];
    if (status) {
      response.writeHead(Number(status), { 'content-type': 'application/json' });
      response.end('{}');
      return;
    }
    response.writeHead(200, { 'content-type': PAGES[path] ? 'text/html' : 'application/json' });
    response.end(PAGES[path] ?? '{}');
  });
  await new Promise((ready) => server.listen(0, LOCAL_HOST, ready));
  origin = `http://${LOCAL_HOST}:${server.address().port}`;
  dir = mkdtempSync(join(tmpdir(), 'page-failures-'));
  report = join(dir, 'report.json');
  writeFileSync(join(dir, 'package.json'), JSON.stringify({ type: 'module' }));
  writeFileSync(join(dir, 'failures.spec.ts'), spec);
  writeFileSync(join(dir, 'playwright.config.ts'), config(origin));
  const env = childEnv({ LIVE_REPORT: report });
  for (const name of UNSET) delete env[name];
  run = await playwright(['test', '-c', 'playwright.config.ts', '--project', JOURNEY_PROJECT, '--no-deps'], env);
}, TIMEOUT_MS);

afterAll(async () => {
  if (server) await new Promise((done) => server.close(done));
  if (dir) rmSync(dir, { recursive: true, force: true });
});

describe.skipIf(!hasChromium)('a journey fails on a console error or a failed response its page did not expect, in a real playwright run', () => {
  it('fails the run on a journey whose page logs, throws or gets a failed response it did not expect, naming it, and passes one whose page loads cleanly or gets the failure it expected', () => {
    expect.soft(run.status, run.output).toBe(1);
    for (const [title, outcome] of [
      [JOURNEYS.clean, { status: 'expected', errors: '' }],
      [JOURNEYS.logs, { status: 'unexpected', errors: expect.stringContaining(LOGGED) }],
      [JOURNEYS.thrown, { status: 'unexpected', errors: expect.stringContaining('the page threw') }],
      [JOURNEYS.undeclared, { status: 'unexpected', errors: expect.stringContaining(`404 GET ${origin}/answer/404`) }],
      [JOURNEYS.declared, { status: 'expected', errors: '' }],
      [JOURNEYS.serverError, { status: 'expected', errors: '' }],
      [JOURNEYS.routed, { status: 'expected', errors: '' }],
      [JOURNEYS.otherStatus, { status: 'unexpected', errors: expect.stringContaining(`500 GET ${origin}/answer/500`) }],
    ])
      expect.soft(journey(title), title).toEqual(outcome);
  });
});
