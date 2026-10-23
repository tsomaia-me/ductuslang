import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const STACK_ENV = ['E2E_WORKERS', 'E2E_FULL_API_PORT'];
const CONFIGS = ['app journeys', 'app journeys on the local stack'];

let saved;
let configs;

beforeAll(async () => {
  saved = Object.fromEntries(STACK_ENV.map((name) => [name, process.env[name]]));
  configs = {
    'app journeys': (await import('../../../apps/app/e2e/journeys/playwright.config.ts')).default,
    'app journeys on the local stack': (await import('../../../apps/app/e2e/journeys/local.config.ts')).default,
  };
});

afterAll(() => {
  for (const [name, value] of Object.entries(saved)) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
  }
});

describe('the app journeys run one at a time, since they share walk accounts 1 and 2 and one stack (CD-381)', () => {
  it('runs one worker in every app journeys config and one Workers stack on the local stack', async () => {
    for (const name of CONFIGS) expect.soft(configs[name].workers, name).toBe(1);
    const { FULL_WORKERS } = await import('../../../apps/app/e2e/full/stack.ts');
    expect.soft(FULL_WORKERS, 'the local stack').toBe(1);
  });
});
