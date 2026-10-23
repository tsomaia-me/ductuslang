/**
 * `cd-watch` (CD-203; stress test round 2, finding 6): the production probe and
 * the urgent page live on Cloudflare, not on wb. What has to hold: two failed
 * probes in a row page once (Linear Urgent, and the webhook when set), not on
 * every tick; a recovery says so once; a page no channel took is
 * retried. KV is a Map, `fetch` a fake.
 */

import { describe, expect, it } from 'vitest';

import { PRODUCTION_TITLE, handle, tick } from '../src/watch.mjs';

const T0 = new Date('2026-10-04T05:00:00Z');
const minutes = (n) => new Date(T0.getTime() + n * 60_000);

const kv = () => {
  const map = new Map();
  return { map, get: async (key) => map.get(key) ?? null, put: async (key, value) => void map.set(key, value) };
};

const world = ({ apiUp = true, webUp = true, linearUp = true, open = [] } = {}) => {
  const state = { apiUp, webUp, linearUp, open, pages: [], webhook: [] };
  const json = (body, status = 200) => ({ ok: status < 300, status, json: async () => body, text: async () => JSON.stringify(body) });
  state.fetchImpl = async (url, init = {}) => {
    if (url === 'https://api.test/api/v1/health') return state.apiUp ? json({ status: 'ok' }) : json({ error: 'x' }, 503);
    if (url === 'https://web.test/build.json') return state.webUp ? json({ sha: 'abc1234' }) : json({}, 502);
    if (url === 'https://page.test/hook') {
      state.webhook.push(JSON.parse(init.body).text);
      return json({});
    }
    if (url === 'https://api.linear.app/graphql') {
      if (!state.linearUp) return json({ errors: [{ message: 'down' }] }, 500);
      const { query, variables } = JSON.parse(init.body);
      if (query.includes('WatchOpen')) return json({ data: { issues: { nodes: state.open.filter((i) => i.title === variables.title) } } });
      if (query.includes('WatchTeam')) return json({ data: { teams: { nodes: [{ id: 'team-cd' }] } } });
      if (query.includes('issueCreate')) {
        const made = { id: `uuid-${state.pages.length}`, identifier: `CD-${990 + state.pages.length}`, title: variables.input.title };
        state.open.push(made);
        state.pages.push({ create: variables.input });
        return json({ data: { issueCreate: { issue: made } } });
      }
      state.pages.push({ comment: variables.input });
      return json({ data: { commentCreate: { success: true } } });
    }
    return json({}, 404);
  };
  return state;
};

const ENV = (store, extra = {}) => ({
  WATCH: store,
  API_HEALTH_URL: 'https://api.test/api/v1/health',
  WEB_BUILD_URL: 'https://web.test/build.json',
  LINEAR_TEAM: 'CD',
  LINEAR_API_KEY: 'lin',
  PAGE_WEBHOOK_URL: 'https://page.test/hook',
  ...extra,
});
const quiet = { log() {}, error() {} };
const at = (store, w, when, extra) => tick({ env: ENV(store, extra), fetchImpl: w.fetchImpl, now: () => when, log: quiet });

describe('production probe', () => {
  it('pages once after two failed probes, keeps quiet while it stays down, and says once that it recovered', async () => {
    const store = kv();
    const w = world({ apiUp: false });
    await at(store, w, minutes(0));
    expect(w.pages).toEqual([]);
    await at(store, w, minutes(5));
    expect(w.pages).toEqual([{ create: { teamId: 'team-cd', title: PRODUCTION_TITLE, description: expect.stringMatching(/API health answered 503 — 2 probes in a row/), priority: 1 } }]);
    expect(w.webhook).toHaveLength(1);
    await at(store, w, minutes(10));
    expect(w.pages).toHaveLength(1);
    w.apiUp = true;
    await at(store, w, minutes(15));
    expect(w.pages[1]).toEqual({ comment: { issueId: 'uuid-0', body: expect.stringMatching(/^Resolved at 2026-10-04T05:15:00\.000Z: production answers again/) } });
    await at(store, w, minutes(20));
    expect(w.pages).toHaveLength(2);
  });

  it('counts the web too, and one good probe resets the count', async () => {
    const store = kv();
    const w = world({ webUp: false });
    await at(store, w, minutes(0));
    w.webUp = true;
    await at(store, w, minutes(5));
    w.webUp = false;
    await at(store, w, minutes(10));
    expect(w.pages).toEqual([]);
    await at(store, w, minutes(15));
    expect(w.pages[0].create.description).toMatch(/web build\.json answered 502/);
  });

  it('retries a page no channel took, and comments on an open twin instead of filing another', async () => {
    const store = kv();
    const w = world({ apiUp: false, linearUp: false });
    await at(store, w, minutes(0), { PAGE_WEBHOOK_URL: '' });
    await at(store, w, minutes(5), { PAGE_WEBHOOK_URL: '' });
    expect(JSON.parse(store.map.get('state')).productionPaged).toBeNull();
    w.linearUp = true;
    w.open.push({ id: 'uuid-old', identifier: 'CD-900', title: PRODUCTION_TITLE });
    await at(store, w, minutes(10), { PAGE_WEBHOOK_URL: '' });
    expect(w.pages).toEqual([{ comment: { issueId: 'uuid-old', body: expect.stringMatching(/3 probes in a row/) } }]);
  });
});

describe('the endpoints', () => {
  const request = (path, init) => new Request(`https://cd-watch.test${path}`, init);

  it('answers /status and 404s the rest', async () => {
    const store = kv();
    const env = ENV(store);
    const status = await handle(request('/status'), env);
    expect(await status.json()).toMatchObject({ failures: 0 });
    expect((await handle(request('/'), env)).status).toBe(404);
  });
});
