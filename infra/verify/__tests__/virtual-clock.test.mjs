import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { FRAME_MS, onClock, pauseClock, resumeClock, stepFrames } from '../../../apps/app/e2e/tools/virtual-clock.ts';
import { browsersPath, launchChromium, useInstalledBrowsers } from '../interaction-check.mjs';

const TIMEOUT_MS = 60_000;
const LATE_MS = 5_000;

const hasChromium = browsersPath() !== undefined;
let browser;

beforeAll(async () => {
  useInstalledBrowsers();
  if (hasChromium) browser = await launchChromium();
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
});

function lateAfterEvery(page, lateMs) {
  const evaluate = async (...args) => {
    const read = await page.evaluate(...args);
    await page.clock.fastForward(lateMs);
    return read;
  };
  return new Proxy(page, {
    get(target, key) {
      if (key === 'evaluate') return evaluate;
      const value = Reflect.get(target, key, target);
      return typeof value === 'function' ? value.bind(target) : value;
    },
  });
}

const now = (page) => page.evaluate(() => Date.now());

describe.skipIf(!hasChromium)('pauseClock', { timeout: TIMEOUT_MS }, () => {
  it('pauses the page clock whether the round trip after reading it lands seconds late or on time at the first try', async () => {
    for (const [timing, wrap] of [
      ['late', (page) => lateAfterEvery(page, LATE_MS)],
      ['on time', (page) => page],
    ]) {
      const context = await browser.newContext();
      try {
        const page = await context.newPage();
        await page.goto('about:blank');
        await pauseClock(wrap(page));
        const first = await now(page);
        const second = await now(page);
        expect.soft(second, `${timing}: the clock kept running after the pause`).toBe(first);
        await stepFrames(page, 1);
        expect.soft((await now(page)) - first, `${timing}: a stepped frame`).toBe(FRAME_MS);
        await resumeClock(page);
      } finally {
        await context.close();
      }
    }
  });

  it('lets any other refusal of the clock through', async () => {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      await page.goto('about:blank');
      const refusing = new Proxy(page, {
        get(target, key) {
          if (key === 'clock') {
            return new Proxy(target.clock, {
              get(clock, name) {
                if (name === 'pauseAt') return () => Promise.reject(new Error('the clock is gone'));
                const value = Reflect.get(clock, name, clock);
                return typeof value === 'function' ? value.bind(clock) : value;
              },
            });
          }
          const value = Reflect.get(target, key, target);
          return typeof value === 'function' ? value.bind(target) : value;
        },
      });
      await expect(pauseClock(refusing)).rejects.toThrow('the clock is gone');
    } finally {
      await context.close();
    }
  });
});

describe.skipIf(!hasChromium)('onClock', { timeout: TIMEOUT_MS }, () => {
  it('hands back what the run returns with the clock running again, and resumes the clock when the run fails on a live page', async () => {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      await page.goto('about:blank');
      const returned = await onClock(page, () => now(page));
      await expect.poll(() => now(page), { message: 'the clock stayed paused after the run' }).toBeGreaterThan(returned);
      let paused = 0;
      await expect(
        onClock(page, async () => {
          paused = await now(page);
          throw new Error('the run stopped here');
        }),
      ).rejects.toThrow('the run stopped here');
      await expect.poll(() => now(page), { message: 'the clock stayed paused after the failed run' }).toBeGreaterThan(paused);
    } finally {
      await context.close();
    }
  });

  it("throws the run's own error when the page is gone before the clock can resume, and lets a refused resume through when the run succeeded", async () => {
    for (const [outcome, run, error] of [
      [
        'the run failed',
        async (context) => {
          await context.close();
          throw new Error('the run stopped here');
        },
        'the run stopped here',
      ],
      ['the run succeeded', (context) => context.close(), /closed/],
    ]) {
      const context = await browser.newContext();
      const page = await context.newPage();
      await page.goto('about:blank');
      await expect.soft(onClock(page, () => run(context)), outcome).rejects.toThrow(error);
    }
  });
});
