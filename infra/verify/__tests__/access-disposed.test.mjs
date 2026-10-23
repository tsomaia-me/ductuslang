import { describe, expect, it } from 'vitest';

import { routeAccessHeaders } from '../../e2e/access.mjs';

const ORIGIN = 'http://fixture.test';
const ACCESS = { 'CF-Access-Client-Id': 'dummy-disposed-id', 'CF-Access-Client-Secret': 'dummy-disposed-secret' };
const DISPOSED = 'route.fulfill: Fetch response has been disposed';
const REFUSED_BY_BROWSER = 'route.fulfill: Invalid status code';

async function handlerOf() {
  let handler;
  const context = {
    setOffline: async () => {},
    route: async (_match, routed) => {
      handler = routed;
    },
    routeWebSocket: async () => {},
  };
  await routeAccessHeaders(context, [ORIGIN], ACCESS);
  return handler;
}

function routeFailingToFulfil(message) {
  return {
    request: () => ({ headers: () => ({}) }),
    fetch: async () => ({ status: () => 200 }),
    fulfill: async () => {
      throw new Error(message);
    },
    abort: async () => {},
  };
}

describe('a request in flight when its page closes (CD-381)', () => {
  it('ends quietly when the request is gone with nothing left to fulfil, and still surfaces any other error of the fulfil', async () => {
    const handler = await handlerOf();
    await expect.soft(handler(routeFailingToFulfil(DISPOSED)), 'disposed').resolves.toBeUndefined();
    await expect.soft(handler(routeFailingToFulfil(REFUSED_BY_BROWSER)), 'refused by the browser').rejects.toThrow(REFUSED_BY_BROWSER);
  });
});
