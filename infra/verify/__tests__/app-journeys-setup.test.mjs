import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { walkAccountsFromKey } from '../../../apps/app/e2e/live/walk-accounts.ts';
import { browsersPath } from '../interaction-check.mjs';

const TIMEOUT_MS = 120_000;
const APP = fileURLToPath(new URL('../../../apps/app/', import.meta.url));
const PLAYWRIGHT = fileURLToPath(new URL('../../../node_modules/@playwright/test/cli.js', import.meta.url));
const CLOSED_PORT = 1;
const ACCESS = { CF_ACCESS_CLIENT_ID: 'dummy-setup-id-cd326.access', CF_ACCESS_CLIENT_SECRET: 'dummy-setup-secret-cd326' };
const KEY = 'dummy-walk-seed-key-cd326-setup-only-0123456789';
const DROPPED = ['NODE_USE_ENV_PROXY', 'LIVE_API_URL', 'CI'];

let out;

beforeAll(() => {
  out = mkdtempSync(join(tmpdir(), 'cd326-setup-'));
});

afterAll(() => rmSync(out, { recursive: true, force: true }));

function runSetup() {
  const env = { ...process.env, ...ACCESS, WALK_SEED_KEY: KEY, LIVE_URL: `http://127.0.0.1:${CLOSED_PORT}`, LIVE_REPORT: join(out, 'report.json'), QA_STATE: join(out, 'state.json') };
  for (const name of DROPPED) delete env[name];
  return new Promise((done) => {
    const child = spawn(process.execPath, [PLAYWRIGHT, 'test', '-c', 'e2e/journeys/playwright.config.ts', '--project', 'setup'], { cwd: APP, env });
    let output = '';
    child.stdout.on('data', (chunk) => (output += chunk));
    child.stderr.on('data', (chunk) => (output += chunk));
    child.on('close', (code) => done({ code, output }));
  });
}

describe.skipIf(browsersPath() === undefined)("the app journeys' setup keeps the Access token and the walk passwords out of its output (CD-326)", { timeout: TIMEOUT_MS }, () => {
  it('a sign-in that fails on the network fails the setup and prints no header value, in the list output or LIVE_REPORT', async () => {
    const { code, output } = await runSetup();
    const report = existsSync(join(out, 'report.json')) ? readFileSync(join(out, 'report.json'), 'utf8') : '';
    expect(code).not.toBe(0);
    expect(output).toMatch(/ECONNREFUSED/);
    expect(report).toMatch(/ECONNREFUSED/);
    const secrets = [...Object.values(ACCESS), KEY, ...walkAccountsFromKey(KEY).map(({ password }) => password)];
    for (const secret of secrets) {
      expect(output.includes(secret), 'a secret in the list output').toBe(false);
      expect(report.includes(secret), 'a secret in LIVE_REPORT').toBe(false);
    }
  });
});
