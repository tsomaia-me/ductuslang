import { describe, expect, it } from 'vitest';

import { DEV_URL, serviceWorkersOn } from '../../../apps/app/e2e/journeys/journey-env.ts';

const LOCAL = 'http://127.0.0.1:24801';

describe('service workers in the app journeys (CD-363): the browser fetches the worker script itself, outside the Access route', () => {
  it('are blocked on dev, where Access redirects that unauthenticated fetch, and stay allowed on the local stack, where the cached reload is proven', async () => {
    const dev = (await import('../../../apps/app/e2e/journeys/playwright.config.ts')).default;
    const local = (await import('../../../apps/app/e2e/journeys/local.config.ts')).default;
    expect.soft(serviceWorkersOn(DEV_URL), 'dev').toBe('block');
    expect.soft(dev.use.serviceWorkers, 'dev config').toBe('block');
    expect.soft(serviceWorkersOn(LOCAL), 'local').toBe('allow');
    expect.soft(local.use.serviceWorkers, 'local config').toBe('allow');
  });
});
