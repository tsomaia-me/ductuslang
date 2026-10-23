/**
 * `transfer-guard.mjs` (CD-285): a real `--env dev` deploy is refused until the production api
 * Worker is at or past production's transfer migration (the 11 Durable Object classes from
 * cd-api-dev; later tags such as CD-291's v2 come after it); dev's v2 would delete them otherwise. The Cloudflare API is a fake `fetch`.
 */
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { readConfig } from '../../../apps/edge/scripts/live-vars.mjs';
import { main, productionTransfer } from '../../../apps/edge/scripts/transfer-guard.mjs';

const EDGE = fileURLToPath(new URL('../../../apps/edge/', import.meta.url));
const API = 'https://cf.test';

const cloudflare = (scripts, { status = 200 } = {}) => {
  const asked = [];
  const fetchImpl = async (url) => {
    asked.push(url);
    const body = url.endsWith('/accounts') ? { success: true, result: [{ id: 'acc-1' }] } : { success: status === 200, result: scripts };
    return { ok: status === 200, status, json: async () => body };
  };
  return { fetchImpl, asked };
};

const guard = async (scripts, { env = { CLOUDFLARE_API_TOKEN: 't', CLOUDFLARE_API_BASE: API }, status } = {}) => {
  const out = [];
  const err = [];
  const { fetchImpl, asked } = cloudflare(scripts, { status });
  const code = await main(['--dir', EDGE], { env, fetchImpl, out: (line) => out.push(line), err: (line) => err.push(line) });
  return { code, out: out.join('\n'), err: err.join('\n'), asked };
};

describe('transfer-guard (CD-285)', () => {
  it('reads the production Worker and its transfer migration from the config: cd-api at v1, and the tags after it', () => {
    expect(productionTransfer(readConfig(`${EDGE}wrangler.api.jsonc`))).toEqual({ worker: 'cd-api', tag: 'v1', since: ['v1', 'v2', 'v3'] });
  });

  it('lets dev go once cd-api is at v1 or past it at a later migration (CD-291 ConfigRegistry), refuses a tag it does not know, no migration (the placeholder) or no such Worker, naming what to deploy first, and refuses when it cannot ask: no token, or Cloudflare refusing the listing', async () => {
    const rows = [
      { label: 'at v1', scripts: [{ id: 'cd-api-dev', migration_tag: 'v1' }, { id: 'cd-api', migration_tag: 'v1' }], code: 0, out: /cd-api is at migration v1/ },
      { label: 'at v2', scripts: [{ id: 'cd-api', migration_tag: 'v2' }], code: 0, out: /cd-api is at migration v2, at or past the transfer v1/ },
      { label: 'at v3', scripts: [{ id: 'cd-api', migration_tag: 'v3' }], code: 0, out: /cd-api is at migration v3, at or past the transfer v1/ },
      { label: 'an unknown tag', scripts: [{ id: 'cd-api', migration_tag: 'v0' }], code: 3, err: /cd-api is at migration "v0", not v1:/ },
      { label: 'the placeholder', scripts: [{ id: 'cd-api' }, { id: 'cd-api-dev', migration_tag: 'v1' }], code: 3, err: /cd-api is at migration "", not v1: .*dev's v2 would delete them\. Deploy --env production first/ },
      { label: 'no such Worker', scripts: [{ id: 'cd-api-dev', migration_tag: 'v1' }], code: 3, err: /"no such Worker"/ },
      { label: 'no token', scripts: [], options: { env: {} }, code: 2 },
      { label: 'the listing refused', scripts: [], options: { status: 403 }, code: 2, err: /Cloudflare answered 403 listing the Workers; nothing was deployed/ },
    ];
    for (const row of rows) {
      const result = await guard(row.scripts, row.options);
      expect.soft(result.code, row.label).toBe(row.code);
      if (row.out) expect.soft(result.out, row.label).toMatch(row.out);
      if (row.err) expect.soft(result.err, row.label).toMatch(row.err);
      if (row.label === 'at v1') expect.soft(result.asked, row.label).toEqual([`${API}/accounts`, `${API}/accounts/acc-1/workers/scripts`]);
    }
  });
});
