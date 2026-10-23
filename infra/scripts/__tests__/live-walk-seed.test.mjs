/**
 * The live walks' accounts derived from WALK_SEED_KEY (CD-278): the derivation,
 * the seed the release runs before its pre-flight (the CD-272 rules: keep what
 * signs in, register on 401 auth.invalid_credentials, skip a 409 email_taken,
 * stop on anything ambiguous). The API is faked; nothing here reaches a real server. No key and
 * no password may appear in any line printed or any error.
 */
import { createHmac } from 'node:crypto';

import { describe, expect, it } from 'vitest';

import { accountFor, walkAccounts, walkAccountsFromKey as appAccounts } from '../../../apps/app/e2e/live/walk-accounts.ts';
import { WALK_EMAILS, main, seedWalkAccounts, timedFetch, walkAccountsFromKey, walkPassword } from '../live-walk-accounts.mjs';

const API = 'https://api.example.dev';
const KEY = 'walk-seed-key-must-never-appear-0123456789abcdef';
const OTHER_KEY = 'a-rotated-walk-seed-key-must-never-appear-000000';
const DERIVED = walkAccountsFromKey(KEY);
const SECRETS = [KEY, OTHER_KEY, ...DERIVED.map((account) => account.password), ...walkAccountsFromKey(OTHER_KEY).map((account) => account.password)];

const json = (status, body) => new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

// A fake auth API: `users` maps email → password; `login`/`register`/`remove`
// override a reply per email.
function fakeApi(users = {}, { login = {}, register = {}, remove = {} } = {}) {
  const calls = [];
  const tokens = new Map();
  const fetch = async (url, init) => {
    const route = url.replace(`${API}/api/v1/`, '');
    if (route === 'me' && init.method === 'DELETE') {
      const email = tokens.get(init.headers.authorization);
      calls.push({ route: 'delete', email, body: JSON.parse(init.body) });
      if (remove[email]) return remove[email]();
      delete users[email];
      return new Response(null, { status: 204 });
    }
    const body = JSON.parse(init.body);
    calls.push({ route: route.replace('auth/', ''), email: body.email, displayName: body.displayName, password: body.password });
    if (route === 'auth/login') {
      if (login[body.email]) return login[body.email]();
      if (users[body.email] !== undefined && users[body.email] === body.password) {
        const token = `Bearer t-${body.email}`;
        tokens.set(token, body.email);
        return json(200, { accessToken: token.slice(7) });
      }
      return json(401, { error: { code: 'auth.invalid_credentials', message: 'Bad credentials' } });
    }
    if (register[body.email]) return register[body.email]();
    if (body.email in users) return json(409, { error: { code: 'auth.email_taken', message: 'That email is already registered' } });
    users[body.email] = body.password;
    return json(201, { accessToken: 'a' });
  };
  return { calls, fetch, users };
}

const derivedUsers = (key = KEY) => Object.fromEntries(walkAccountsFromKey(key).map((account) => [account.email, account.password]));

function noSecrets(text) {
  for (const secret of SECRETS) expect(text).not.toContain(secret);
}

async function seed(api, key = KEY) {
  const lines = [];
  const result = await seedWalkAccounts({ api: `${API}/`, key, fetch: api.fetch, log: (line) => lines.push(line) });
  const printed = lines.join('\n');
  noSecrets(printed);
  return { result, printed };
}

describe('the derivation (CD-278)', () => {
  it('is HMAC-SHA256(key, email) in base64url for walk-cf+1..3@crookedduke.com, the last the smoke walk\'s, and fits the register schema (8..128 characters)', () => {
    expect(WALK_EMAILS).toEqual(['walk-cf+1@crookedduke.com', 'walk-cf+2@crookedduke.com', 'walk-cf+3@crookedduke.com']);
    expect(DERIVED.map((account) => account.email)).toEqual(WALK_EMAILS);
    for (const account of DERIVED) {
      expect(account.password).toBe(createHmac('sha256', KEY).update(account.email).digest('base64url'));
      expect(account.password).toMatch(/^[A-Za-z0-9_-]{43}$/);
    }
    expect(new Set(DERIVED.map((account) => account.password)).size).toBe(3);
    expect(walkPassword(OTHER_KEY, WALK_EMAILS[0])).not.toBe(DERIVED[0].password);
    expect(walkAccountsFromKey(KEY)).toEqual(DERIVED);
    for (const account of DERIVED) expect(account.password.length).toBeGreaterThanOrEqual(8);
    for (const account of DERIVED) expect(account.password.length).toBeLessThanOrEqual(128);
  });

  it('refuses no key or a key under 32 characters, without quoting it', () => {
    expect(() => walkAccountsFromKey(undefined)).toThrow(/WALK_SEED_KEY is not set/);
    expect(() => walkAccountsFromKey('')).toThrow(/WALK_SEED_KEY is not set/);
    const short = 'short-key-must-not-appear';
    expect(() => walkAccountsFromKey(short)).toThrow(/shorter than 32 characters/);
    try {
      walkAccountsFromKey(short);
    } catch (error) {
      expect(error.message).not.toContain(short);
    }
  });

  it('is the same in the live walks (apps/app/e2e/live/walk-accounts.ts), whose pool is the first two', () => {
    expect(appAccounts(KEY)).toEqual(DERIVED);
    expect(walkAccounts({ WALK_SEED_KEY: KEY })).toEqual(DERIVED);
    expect(walkAccounts({})).toEqual([]);
    expect(() => appAccounts('short-key-must-not-appear')).toThrow(/WALK_SEED_KEY is shorter than 32 characters/);
    expect([0, 1, 2, 3].map((look) => accountFor(DERIVED, look)?.email)).toEqual([WALK_EMAILS[0], WALK_EMAILS[1], WALK_EMAILS[0], WALK_EMAILS[1]]);
    expect(accountFor(DERIVED.slice(0, 1), 0)).toBeNull();
    expect(accountFor([], 0)).toBeNull();
  });
});

describe('seedWalkAccounts (CD-278, the CD-272 rules)', () => {
  it('registers all three on an empty API, the smoke walk\'s first, printing emails only; keeps every account that signs in (a second run is a no-op); registers only the missing one; skips an email taken by an account the key does not sign into (409), reports it, and is not ok', async () => {
    const empty = fakeApi();
    const added = await seed(empty);
    expect.soft(added.result, 'empty').toEqual({ kept: 0, added: 3, skipped: 0, ok: true });
    const order = [WALK_EMAILS[2], WALK_EMAILS[0], WALK_EMAILS[1]];
    expect.soft(empty.calls.map((call) => `${call.route} ${call.email}`), 'empty').toEqual(order.flatMap((email) => [`login ${email}`, `register ${email}`]));
    expect.soft(empty.calls.filter((call) => call.route === 'register').map((call) => call.displayName), 'empty').toEqual(['Walk CF 3', 'Walk CF 1', 'Walk CF 2']);
    expect.soft(empty.users, 'empty').toEqual(derivedUsers());
    expect.soft(added.printed, 'empty').toContain(`walk account added: ${WALK_EMAILS[2]}`);
    expect.soft(added.printed, 'empty').toContain('3 of 3 sign in at https://api.example.dev (kept 0, added 3, skipped 0)');

    const seeded = fakeApi(derivedUsers());
    const kept = await seed(seeded);
    expect.soft(kept.result, 'seeded').toEqual({ kept: 3, added: 0, skipped: 0, ok: true });
    expect.soft(seeded.calls.map((call) => call.route), 'seeded').toEqual(['login', 'login', 'login']);
    expect.soft(kept.printed, 'seeded').toContain('kept 3, added 0, skipped 0');

    const users = derivedUsers();
    delete users[WALK_EMAILS[1]];
    const partial = fakeApi(users);
    expect.soft((await seed(partial)).result, 'one missing').toEqual({ kept: 2, added: 1, skipped: 0, ok: true });
    expect.soft(partial.calls.filter((call) => call.route === 'register').map((call) => call.email), 'one missing').toEqual([WALK_EMAILS[1]]);

    const taken = fakeApi(derivedUsers(OTHER_KEY));
    const skipped = await seed(taken);
    expect.soft(skipped.result, 'taken').toEqual({ kept: 0, added: 0, skipped: 3, ok: false });
    expect.soft(skipped.printed, 'taken').toContain(`walk account skipped: ${WALK_EMAILS[2]} is registered, and WALK_SEED_KEY does not sign into it`);
    expect.soft(taken.users, 'taken').toEqual(derivedUsers(OTHER_KEY));
  });

  it('stops on anything ambiguous at sign-in (429, a 401 without the wrong-credentials code, a 400 that names wrong credentials, 500, a network error), registering nothing more, and on a failed registration other than 409 email_taken; not ok', async () => {
    for (const [label, reply] of [
      ['429', () => json(429, { error: { code: 'rate_limited', message: 'Slow down' } })],
      ['a 401 without the wrong-credentials code', () => json(401, {})],
      ['a 400 that names wrong credentials', () => json(400, { error: { code: 'auth.invalid_credentials', message: 'Bad credentials' } })],
      ['500', () => json(500, { error: { code: 'internal_error', message: 'Internal error' } })],
      ['a network error', () => Promise.reject(new TypeError('fetch failed'))],
    ]) {
      const api = fakeApi({}, { login: { [WALK_EMAILS[0]]: reply } });
      const { result, printed } = await seed(api);
      expect.soft(result.ok, label).toBe(false);
      expect.soft(api.calls.map((call) => `${call.route} ${call.email}`), label).toEqual([`login ${WALK_EMAILS[2]}`, `register ${WALK_EMAILS[2]}`, `login ${WALK_EMAILS[0]}`]);
      expect.soft(printed, label).toMatch(/1 of 3 sign in .*; stopped: /);
    }
    const api = fakeApi({}, { register: { [WALK_EMAILS[2]]: () => json(409, { error: { code: 'conflict' } }) } });
    const { result, printed } = await seed(api);
    expect.soft(result.ok, 'register 409 conflict').toBe(false);
    expect.soft(printed, 'register 409 conflict').toContain(`stopped: register ${WALK_EMAILS[2]} → 409`);
    expect.soft(api.calls, 'register 409 conflict').toHaveLength(2);
  });

  it('runs from the command line with the key from the environment only: 0 seeded, 1 not, 2 no key', async () => {
    const lines = [];
    const out = { log: (line) => lines.push(line), err: (line) => lines.push(line) };
    expect(await main(['--seed', '--api', API], { WALK_SEED_KEY: KEY }, { fetch: fakeApi().fetch, ...out })).toBe(0);
    expect(await main(['--seed', '--api', API], { WALK_SEED_KEY: KEY }, { fetch: fakeApi(derivedUsers(OTHER_KEY)).fetch, ...out })).toBe(1);
    expect(await main(['--seed', '--api', API], { WALK_SEED_KEY: KEY }, { fetch: async () => json(503, {}), ...out })).toBe(1);
    expect(await main(['--seed', '--api', API], {}, { fetch: fakeApi().fetch, ...out })).toBe(2);
    expect(await main(['--seed'], { WALK_SEED_KEY: KEY }, { fetch: fakeApi().fetch, ...out })).toBe(2);
    expect(await main(['--upsert', '--api', API], { WALK_SEED_KEY: KEY }, { fetch: fakeApi().fetch, ...out })).toBe(2);
    noSecrets(lines.join('\n'));
  });

  it('gives every request a time limit, so a hung API never holds the release', async () => {
    let signal;
    await timedFetch(async (_, init) => {
      signal = init.signal;
      return json(200, {});
    }, 50)(`${API}/x`, { method: 'POST' });
    expect(signal).toBeInstanceOf(AbortSignal);
    const LIMIT_BOUND_MS = 200;
    const first = await Promise.race([
      new Promise((resolve) => (signal.aborted ? resolve('aborted') : signal.addEventListener('abort', () => resolve('aborted'), { once: true }))),
      new Promise((resolve) => setTimeout(() => resolve('still open'), LIMIT_BOUND_MS)),
    ]);
    expect(first).toBe('aborted');
  });

  it('sends the Access service token\'s headers with every request it is given them for, beside its own (CD-285)', async () => {
    let headers;
    const access = { 'CF-Access-Client-Id': 'id.access', 'CF-Access-Client-Secret': 'secret' };
    await timedFetch(async (_, init) => {
      headers = init.headers;
      return json(200, {});
    }, 1000, access)(`${API}/x`, { method: 'POST', headers: { 'content-type': 'application/json' } });
    expect(headers).toEqual({ ...access, 'content-type': 'application/json' });
    await timedFetch(async (_, init) => {
      headers = init.headers;
      return json(200, {});
    }, 1000)(`${API}/x`, { method: 'POST' });
    expect(headers).toEqual({});
  });
});
