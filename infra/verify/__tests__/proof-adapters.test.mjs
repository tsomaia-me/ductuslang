import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { describe, expect, it } from 'vitest';

import { ADAPTERS } from '../proof.mjs';
import { REPO_ROOT } from '../screens.mjs';

const answers = async (origin) => {
  try {
    return (await fetch(origin)).ok;
  } catch {
    return false;
  }
};

describe('each surface’s proof adapter starts its dev server and stops it', () => {
  it('answers once started and is gone once stopped, for every surface', async () => {
    const surfaces = Object.entries(ADAPTERS);
    const answered = await Promise.all(
      surfaces.map(async ([, path]) => {
        const adapter = await import(pathToFileURL(join(REPO_ROOT, path)).href);
        const server = await adapter.serve(REPO_ROOT);
        let started;
        try {
          started = await answers(server.origin);
        } finally {
          await server.stop();
        }
        return { started, stopped: await answers(server.origin) };
      }),
    );
    surfaces.forEach(([surface], index) => expect.soft(answered[index], surface).toEqual({ started: true, stopped: false }));
  }, 240_000);
});
