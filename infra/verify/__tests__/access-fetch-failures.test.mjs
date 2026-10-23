import { afterEach, describe, expect, it, vi } from 'vitest';

import { HIDDEN_ACCESS, routeAccessHeaders } from '../../e2e/access.mjs';

const ORIGIN = 'http://fixture.test';
const SECRET = 'dummy-fetch-failure-secret';
const ACCESS = { 'CF-Access-Client-Id': 'dummy-fetch-failure-id', 'CF-Access-Client-Secret': SECRET };
const FONT = `${ORIGIN}/fonts/inter-700.woff2?v=3`;
const RESET = 'route.fetch: read ECONNRESET';
const HUNG_UP = 'route.fetch: socket hang up';
const CALL_LOG = `\nCall log:\n  - → GET ${FONT}\n  - CF-Access-Client-Secret: ${SECRET}`;
const CLOSED = [
  'route.fetch: Target page, context or browser has been closed',
  'route.fetch: Request context disposed.',
];

async function routed() {
  let handler;
  const context = {
    setOffline: async () => {},
    route: async (_match, routedBy) => {
      handler = routedBy;
    },
    routeWebSocket: async () => {},
  };
  const access = await routeAccessHeaders(context, [ORIGIN], ACCESS);
  return { handler, access };
}

function routeWhoseFetchRejects(message, url = FONT) {
  const aborted = [];
  return {
    aborted,
    route: {
      request: () => ({ headers: () => ({}), method: () => 'GET', url: () => url }),
      fetch: async () => {
        throw new Error(message);
      },
      fulfill: async () => {},
      abort: async (code) => {
        aborted.push(code ?? 'failed');
      },
    },
  };
}

function routeResettingBeforeAnswering(resets, method = 'GET', message = RESET) {
  const calls = { fetched: 0, fulfilled: 0, aborted: 0 };
  return {
    calls,
    route: {
      request: () => ({ headers: () => ({}), method: () => method, url: () => FONT }),
      fetch: async () => {
        calls.fetched += 1;
        if (calls.fetched <= resets) throw new Error(`${message}${CALL_LOG}`);
        return { status: () => 200 };
      },
      fulfill: async () => {
        calls.fulfilled += 1;
      },
      abort: async () => {
        calls.aborted += 1;
      },
    },
  };
}

describe('a request the Access route could not fetch (CD-381 R9)', () => {
  it('is recorded with its method, its address without the query and the cause, and the page gets the failure', async () => {
    const { handler, access } = await routed();
    const { route, aborted } = routeWhoseFetchRejects(`${RESET}${CALL_LOG}`);
    await handler(route);
    expect(access.fetchFailures()).toEqual([`GET ${ORIGIN}/fonts/inter-700.woff2: ${RESET}`]);
    expect(aborted).toEqual(['failed']);
  });

  it('never records a header value, even one the cause repeats', async () => {
    const { handler, access } = await routed();
    await handler(routeWhoseFetchRejects(`route.fetch: refused ${SECRET}${CALL_LOG}`).route);
    expect(access.fetchFailures()).toEqual([`GET ${ORIGIN}/fonts/inter-700.woff2: route.fetch: refused ${HIDDEN_ACCESS}`]);
    expect(access.fetchFailures().join('\n')).not.toContain(SECRET);
  });

  it('stays quiet when the page, context or browser closed under it', async () => {
    for (const message of CLOSED) {
      const { handler, access } = await routed();
      const { route, aborted } = routeWhoseFetchRejects(message);
      await handler(route);
      expect.soft(access.fetchFailures(), message).toEqual([]);
      expect.soft(aborted, message).toEqual(['failed']);
    }
  });

  it('retries a GET once on a reset or a hang-up and fulfils the page with the second answer, records one that fails twice, and never retries a POST or another cause', async () => {
    const REFUSED = 'route.fetch: connect ECONNREFUSED';
    const failure = (method, cause) => [`${method} ${ORIGIN}/fonts/inter-700.woff2: ${cause}`];
    for (const [resets, method, cause, calls, failures] of [
      [0, 'GET', RESET, { fetched: 1, fulfilled: 1, aborted: 0 }, []],
      [1, 'GET', RESET, { fetched: 2, fulfilled: 1, aborted: 0 }, []],
      [2, 'GET', RESET, { fetched: 2, fulfilled: 0, aborted: 1 }, failure('GET', RESET)],
      [1, 'POST', RESET, { fetched: 1, fulfilled: 0, aborted: 1 }, failure('POST', RESET)],
      [1, 'GET', HUNG_UP, { fetched: 2, fulfilled: 1, aborted: 0 }, []],
      [2, 'GET', HUNG_UP, { fetched: 2, fulfilled: 0, aborted: 1 }, failure('GET', HUNG_UP)],
      [1, 'POST', HUNG_UP, { fetched: 1, fulfilled: 0, aborted: 1 }, failure('POST', HUNG_UP)],
      [1, 'GET', REFUSED, { fetched: 1, fulfilled: 0, aborted: 1 }, failure('GET', REFUSED)],
    ]) {
      const named = `${method} failing ${resets} times with ${cause}`;
      const { handler, access } = await routed();
      const routing = routeResettingBeforeAnswering(resets, method, cause);
      await handler(routing.route);
      expect.soft(routing.calls, named).toEqual(calls);
      expect.soft(access.fetchFailures(), named).toEqual(failures);
    }
  });

  it('has nothing to record when no Access header is sent', async () => {
    const access = await routeAccessHeaders({}, [ORIGIN], {});
    expect(access.fetchFailures()).toEqual([]);
  });
});

describe('a retry the Access route fires (CD-448)', () => {
  afterEach(() => vi.restoreAllMocks());

  it('logs one line with the method, the address without the query and the cause for each retry, and nothing when no retry fires', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    for (const message of [RESET, HUNG_UP]) {
      warn.mockClear();
      const { handler } = await routed();
      await handler(routeResettingBeforeAnswering(1, 'GET', message).route);
      expect.soft(warn.mock.calls, message).toEqual([[`access retry: GET ${ORIGIN}/fonts/inter-700.woff2: ${message}`]]);
    }
    warn.mockClear();
    const { handler } = await routed();
    await handler(routeResettingBeforeAnswering(1, 'POST').route);
    await handler(routeResettingBeforeAnswering(0).route);
    expect.soft(warn, 'no retry').not.toHaveBeenCalled();
  });
});
