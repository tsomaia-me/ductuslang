import { existsSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { request as playwrightRequest } from '@playwright/test';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';

import { playerApi } from '../../../apps/app/e2e/journeys/player.ts';
import { TOKEN_KEY } from '../../../apps/app/e2e/live/mode.ts';

const SETTINGS = '/api/v1/me/settings';
const REFRESH = '/api/v1/auth/refresh';
const ORIGIN = 'http://127.0.0.1:1234';
const HOUR_SEC = 3600;
const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
const tokenExpiring = (inSec, name) => `${encode({ alg: 'none' })}.${encode({ exp: Math.floor(Date.now() / 1000) + inSec, name })}.signature`;
const sessionOf = (accessToken, refreshToken) => ({
  cookies: [],
  origins: [{ origin: ORIGIN, localStorage: [{ name: 'other', value: 'kept' }, { name: TOKEN_KEY, value: JSON.stringify({ accessToken, refreshToken }) }] }],
});

let server;
let api;
let folder;
let seen;
let refreshAnswer;
let context;
const was = process.env['QA_STATE'];

beforeAll(async () => {
  folder = mkdtempSync(join(tmpdir(), 'cd381-player-'));
  process.env['QA_STATE'] = join(folder, 'state.json');
  server = createServer((incoming, outgoing) => {
    const chunks = [];
    incoming.on('data', (chunk) => chunks.push(chunk));
    incoming.on('end', () => {
      const body = Buffer.concat(chunks).toString('utf8');
      seen.push({ method: incoming.method, path: incoming.url, authorization: incoming.headers.authorization, body });
      if (incoming.url === REFRESH) {
        outgoing.writeHead(refreshAnswer.status, { 'content-type': 'application/json' });
        return outgoing.end(JSON.stringify(refreshAnswer.json));
      }
      outgoing.writeHead(200, { 'content-type': 'application/json' });
      return outgoing.end('{}');
    });
  });
  await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
  api = `http://127.0.0.1:${server.address().port}`;
  context = await playwrightRequest.newContext();
});

afterAll(async () => {
  await context.dispose();
  await new Promise((done) => server.close(done));
  rmSync(folder, { recursive: true, force: true });
  if (was === undefined) delete process.env['QA_STATE'];
  else process.env['QA_STATE'] = was;
});

beforeEach(() => {
  rmSync(lockFile(), { force: true });
  seen = [];
  refreshAnswer = { status: 200, json: { accessToken: tokenExpiring(HOUR_SEC, 'fresh'), refreshToken: 'rotated-refresh', user: { id: 'u2' } } };
});

const partnerFile = () => join(folder, 'state.partner.json');
const lockFile = () => `${partnerFile()}.lock`;
const stored = () => JSON.parse(readFileSync(partnerFile(), 'utf8'));
const storedTokens = () => JSON.parse(stored().origins[0].localStorage.find((item) => item.name === TOKEN_KEY).value);
const seatApi = () => playerApi(context, { api }, 'partner');

describe('the access token a seat sends', () => {
  it('is the stored one while it is valid, and one refreshed through POST /auth/refresh when the stored one is past or about to reach its expiry, kept in the state file with the rotated refresh token so the next reader refreshes nothing', async () => {
    const valid = tokenExpiring(HOUR_SEC, 'stored');
    writeFileSync(partnerFile(), JSON.stringify(sessionOf(valid, 'stored-refresh')));
    await seatApi().get('/me/settings');
    expect.soft(seen.map((call) => call.path), 'valid').toEqual([SETTINGS]);
    expect.soft(seen[0].authorization, 'valid').toBe(`Bearer ${valid}`);

    seen = [];
    writeFileSync(partnerFile(), JSON.stringify(sessionOf(tokenExpiring(1, 'dying'), 'stored-refresh')));
    await seatApi().get('/me/settings');
    expect.soft(seen[0], 'about to expire').toMatchObject({ method: 'POST', path: REFRESH });

    seen = [];
    const expired = tokenExpiring(-HOUR_SEC, 'expired');
    writeFileSync(partnerFile(), JSON.stringify(sessionOf(expired, 'stored-refresh')));
    await seatApi().get('/me/settings');
    expect.soft(seen.map((call) => `${call.method} ${call.path}`), 'expired').toEqual([`POST ${REFRESH}`, `GET ${SETTINGS}`]);
    expect.soft(JSON.parse(seen[0].body), 'expired').toEqual({ refreshToken: 'stored-refresh' });
    expect.soft(seen[1].authorization, 'expired').toBe(`Bearer ${refreshAnswer.json.accessToken}`);
    expect.soft(seen.map((call) => call.authorization), 'expired').not.toContain(`Bearer ${expired}`);
    expect.soft(storedTokens(), 'state file').toEqual({ accessToken: refreshAnswer.json.accessToken, refreshToken: 'rotated-refresh' });
    expect.soft(stored().origins[0].localStorage.find((item) => item.name === 'other'), 'state file').toEqual({ name: 'other', value: 'kept' });

    seen = [];
    await seatApi().get('/me/settings');
    expect.soft(seen.map((call) => call.path), 'the next reader').toEqual([SETTINGS]);
    expect.soft(seen[0].authorization, 'the next reader').toBe(`Bearer ${refreshAnswer.json.accessToken}`);
  });

  it('fails naming the refresh when the server refuses it, and leaves the stored session as it was', async () => {
    const expired = tokenExpiring(-HOUR_SEC, 'expired');
    writeFileSync(partnerFile(), JSON.stringify(sessionOf(expired, 'stored-refresh')));
    refreshAnswer = { status: 401, json: { error: { code: 'auth.refresh_reused' } } };
    const failure = await seatApi().get('/me/settings').then(() => null, (error) => String(error.message));
    expect(failure).toContain(`POST ${api}${REFRESH}`);
    expect(failure).toContain('401 Unauthorized');
    expect(failure).not.toContain('stored-refresh');
    expect(storedTokens()).toEqual({ accessToken: expired, refreshToken: 'stored-refresh' });
    expect(seen.map((call) => call.path)).toEqual([REFRESH]);
    expect(existsSync(lockFile()), 'the lock outlived the refused refresh').toBe(false);
  });
});

describe('two readers of one seat at the same time', () => {
  it('refresh it once: the second waits for the first, re-reads the state file and sends the rotated token', async () => {
    writeFileSync(partnerFile(), JSON.stringify(sessionOf(tokenExpiring(-HOUR_SEC, 'expired'), 'stored-refresh')));
    await Promise.all([seatApi().get('/me/settings'), seatApi().get('/me/settings')]);
    expect(seen.filter((call) => call.path === REFRESH)).toHaveLength(1);
    const fresh = `Bearer ${refreshAnswer.json.accessToken}`;
    expect(seen.filter((call) => call.path === SETTINGS).map((call) => call.authorization)).toEqual([fresh, fresh]);
    expect(storedTokens()).toEqual({ accessToken: refreshAnswer.json.accessToken, refreshToken: 'rotated-refresh' });
    expect(existsSync(lockFile()), 'the lock outlived the refresh').toBe(false);
  });

  it('take over a lock left by a reader that died, once it is past the stale bound', async () => {
    writeFileSync(partnerFile(), JSON.stringify(sessionOf(tokenExpiring(-HOUR_SEC, 'expired'), 'stored-refresh')));
    writeFileSync(lockFile(), 'a-reader-that-died');
    const longAgo = new Date(Date.now() - HOUR_SEC * 1000);
    utimesSync(lockFile(), longAgo, longAgo);
    await seatApi().get('/me/settings');
    expect(seen.map((call) => `${call.method} ${call.path}`)).toEqual([`POST ${REFRESH}`, `GET ${SETTINGS}`]);
    expect(storedTokens().refreshToken).toBe('rotated-refresh');
    expect(existsSync(lockFile()), 'the lock outlived the refresh').toBe(false);
  });
});
