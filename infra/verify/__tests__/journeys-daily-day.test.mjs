import { describe, it } from 'vitest';

import { CLOCK_SKEW_MS, ROLLOVER_MARGIN_MS, dayWithRoom } from '../../../apps/app/e2e/journeys/daily-day.ts';

const ROOM_MS = 90_000;
const ENDS_AT = Date.parse('2026-10-15T00:00:00.000Z');
const TODAY = { dayKey: '2026-10-14', endsAt: '2026-10-15T00:00:00.000Z' };
const TOMORROW = { dayKey: '2026-10-15', endsAt: '2026-10-16T00:00:00.000Z' };

function serverRollingOverAfter(reads) {
  let seen = 0;
  return async () => {
    seen += 1;
    return seen > reads ? TOMORROW : TODAY;
  };
}

function serverRollingOverAt(endsAt) {
  const today = { dayKey: TODAY.dayKey, endsAt: new Date(endsAt).toISOString() };
  return async () => (Date.now() >= endsAt ? TOMORROW : today);
}

describe.concurrent('the daily journey runs within one server day (CD-381)', () => {
  it('starts on the day it read, raising no caller timeout, when the day outlasts the room plus the allowed clock skew', async ({ expect }) => {
    const now = () => ENDS_AT - ROOM_MS - CLOCK_SKEW_MS;
    let raised = false;
    const day = await dayWithRoom(serverRollingOverAfter(1), ROOM_MS, {
      now,
      beforeWaiting: () => {
        raised = true;
      },
    });
    expect.soft(day).toEqual(TODAY);
    expect.soft(raised).toBe(false);
  });

  it('waits for the next day when the day ends within the room, and within the room plus the allowed clock skew, because the server clock may run ahead', async ({ expect }) => {
    let waited = 0;
    const [withinRoom, withinSkew] = await Promise.all([
      dayWithRoom(serverRollingOverAfter(3), ROOM_MS, { now: () => ENDS_AT - ROOM_MS + 1 }),
      dayWithRoom(serverRollingOverAfter(3), ROOM_MS, {
        now: () => ENDS_AT - ROOM_MS - CLOCK_SKEW_MS / 2,
        beforeWaiting: (wait) => {
          waited = wait;
        },
      }),
    ]);
    expect.soft(withinRoom, 'within the room').toEqual(TOMORROW);
    expect.soft(withinSkew, 'within the clock skew').toEqual(TOMORROW);
    expect.soft(waited, 'within the clock skew').toBe(ROOM_MS + CLOCK_SKEW_MS / 2 + ROLLOVER_MARGIN_MS);
  });

  it('fails when the server keeps a day that ends within the room past the wait and its margin', async ({ expect }) => {
    const now = () => ENDS_AT - 1;
    const wait = 1 + ROLLOVER_MARGIN_MS;
    await expect(dayWithRoom(async () => TODAY, ROOM_MS, { now })).rejects.toThrow(`Timeout ${String(wait)}ms exceeded`);
  });

  it('still catches a roll-over that comes 10 ms before the room ends, between two polls', async ({ expect }) => {
    const room = 1_000;
    const read = serverRollingOverAt(Date.now() + room - 10);
    expect(await dayWithRoom(read, room)).toEqual(TOMORROW);
  });

  it('raises the caller timeout before it waits, so a test that already spent time on setup does not time out', async ({ expect }) => {
    const room = 3_000;
    const setupSpent = 1_000;
    const dayEndsIn = 2_500;
    const testStarted = Date.now() - setupSpent;
    let deadline = testStarted + room;
    const server = serverRollingOverAt(Date.now() + dayEndsIn);
    const read = async () => {
      if (Date.now() > deadline) throw new Error('Test timeout exceeded');
      return server();
    };
    const day = await dayWithRoom(read, room, {
      beforeWaiting: (wait) => {
        deadline = testStarted + room + wait;
      },
    });
    expect(day).toEqual(TOMORROW);
  });
});
