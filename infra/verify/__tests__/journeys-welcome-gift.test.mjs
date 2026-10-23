import { describe, expect, it } from 'vitest';

import { DEV_URL } from '../../../apps/app/e2e/journeys/journey-env.ts';
import { DEV_ECONOMY_NOTE, owedWelcomeGift } from '../../../apps/app/e2e/journeys/welcome-gift.ts';

const AT = '2026-10-14T12:00:00.000Z';
const LOCAL = 'http://127.0.0.1:24801';
const grant = (reason, ackedAt = null) => ({ id: `grant-${reason}`, source: 'rule', reason, reward: { coins: 100 }, status: 'applied', at: AT, ackedAt });
const WITH_RULE = [{ id: 'winGame' }, { id: 'welcomePack' }];
const WITHOUT_RULE = [{ id: 'winGame' }];

describe('the welcome gift a new guest owes on the local stack, whose economy is the seed', () => {
  it('is the pending welcome pack when the economy, given or shipped, has the rule, and nothing with a note saying why when it has none', () => {
    const owed = grant('welcomePack');
    expect.soft(owedWelcomeGift([grant('winGame'), owed], LOCAL, WITH_RULE), 'with the rule').toEqual({ owed, note: null });
    expect.soft(owedWelcomeGift([grant('welcomePack')], LOCAL).owed, 'the shipped economy').toMatchObject({ reason: 'welcomePack' });
    expect.soft(owedWelcomeGift([], LOCAL, WITHOUT_RULE), 'without the rule').toEqual({ owed: null, note: expect.stringContaining('no welcomePack rule') });
  });

  it('fails the journey when the economy, given or shipped, has the rule and the guest was granted none, or only one already acknowledged', () => {
    for (const [name, grants, rules] of [
      ['no grant', [], WITH_RULE],
      ['another grant', [grant('winGame')], WITH_RULE],
      ['an acknowledged welcome pack', [grant('welcomePack', AT)], WITH_RULE],
      ['no grant under the shipped economy', [], undefined],
    ])
      expect.soft(() => owedWelcomeGift(grants, LOCAL, rules), name).toThrow('welcomePack rule');
  });
});

describe('the welcome gift a new guest owes on dev, whose economy the admin owns', () => {
  it('asserts nothing about the grant on every address but the local stack, still hands the journey a pending grant, and the note says so', () => {
    expect.soft(DEV_ECONOMY_NOTE, 'the note').toBe('welcome grant not asserted: dev economy is admin-owned');
    const owed = grant('welcomePack');
    for (const [name, grants, url, expected] of [
      ['none granted', [], DEV_URL, { owed: null, note: DEV_ECONOMY_NOTE }],
      ['an acknowledged pack', [grant('welcomePack', AT)], DEV_URL, { owed: null, note: DEV_ECONOMY_NOTE }],
      ['a pending pack', [owed], DEV_URL, { owed, note: DEV_ECONOMY_NOTE }],
      ['a preview address', [], 'https://preview.example.com', { owed: null, note: DEV_ECONOMY_NOTE }],
    ])
      expect.soft(owedWelcomeGift(grants, url, WITH_RULE), name).toEqual(expected);
  });
});
