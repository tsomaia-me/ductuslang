import { describe, expect, it } from 'vitest';

import { resetWalkAccount } from '../../../apps/app/e2e/live/walk-reset.ts';

describe('resetWalkAccount (CD-153)', () => {
  const reply = (status, body) => ({ ok: () => status < 400, status: () => status, json: async () => body });

  it('pages through every active game and resigns each with {confirm:true}', async () => {
    const calls = [];
    const pages = {
      '/games?status=active': { items: [{ id: 'g1' }, { id: 'g2' }], nextCursor: 'c 1' },
      '/games?status=active&cursor=c%201': { items: [{ id: 'g3' }], nextCursor: null },
    };
    const problems = await resetWalkAccount(async (path, method, body) => {
      calls.push([method, path, body]);
      return method === 'GET' ? reply(200, pages[path]) : reply(200, {});
    });
    expect(problems).toEqual([]);
    expect(calls.filter(([method]) => method === 'POST')).toEqual([
      ['POST', '/games/g1/resign', { confirm: true }],
      ['POST', '/games/g2/resign', { confirm: true }],
      ['POST', '/games/g3/resign', { confirm: true }],
    ]);
  });

  it('reports a failed list and a failed resign', async () => {
    expect(
      await resetWalkAccount(async (path) => (path.startsWith('/games') ? reply(500, {}) : reply(200, {}))),
    ).toEqual(['active games → 500']);
    expect(
      await resetWalkAccount(async (path, method) =>
        method === 'GET' ? reply(200, { items: [{ id: 'g1' }], nextCursor: null }) : path.includes('resign') ? reply(409, {}) : reply(200, {}),
      ),
    ).toEqual(['resign g1 → 409']);
  });
});

describe('resetWalkAccount paging limits (CD-153)', () => {
  const reply = (status, body) => ({ ok: () => status < 400, status: () => status, json: async () => body });

  it('stops, saying so, when the cursor repeats or after 50 pages', async () => {
    let repeated = 0;
    const same = await resetWalkAccount(async (path, method) => {
      if (method !== 'GET') return reply(200, {});
      repeated += 1;
      return reply(200, { items: [{ id: `g${String(repeated)}` }], nextCursor: 'same' });
    });
    expect.soft(repeated, 'a repeated cursor').toBe(2);
    expect.soft(same.join(' '), 'a repeated cursor').toMatch(/paging did not end/);
    let paged = 0;
    const endless = await resetWalkAccount(async (path, method) => {
      if (method !== 'GET') return reply(200, {});
      paged += 1;
      return reply(200, { items: [], nextCursor: `c${String(paged)}` });
    });
    expect.soft(paged, '50 pages').toBeLessThanOrEqual(51);
    expect.soft(endless.join(' '), '50 pages').toMatch(/paging did not end/);
  });
});
