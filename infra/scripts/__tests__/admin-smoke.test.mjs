import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { adminOrigin } from '../admin-hosts.mjs';
import { smoke } from '../admin-smoke.mjs';

const SHA = 'a'.repeat(40);
const SCRIPT = fileURLToPath(new URL('../admin-smoke.mjs', import.meta.url));

function admin({ build = SHA, openWithoutToken = false, kind = 'service', failFirst = 0 } = {}) {
  let calls = 0;
  return async (_url, init) => {
    calls += 1;
    const token = init?.headers?.['cf-access-client-id'];
    if (!token) return new Response('', { status: openWithoutToken ? 200 : 302 });
    if (calls <= failFirst * 2) return new Response('', { status: 530 });
    return Response.json({ identity: { id: token, kind }, environment: 'dev', build });
  };
}

const run = (fetchImpl) => smoke({ url: 'https://admin.test/', sha: SHA, id: 'id.access', secret: 's', tries: 3, every: 0, fetchImpl, log: () => {} });

describe('admin-smoke.mjs (CD-291)', () => {
  it('passes when Access refuses no token and /api/me names the service token and the released build, and waits out a deploy that is not live yet', async () => {
    expect.soft(await run(admin()), 'live').toMatchObject({ ok: true });
    expect.soft(await run(admin({ failFirst: 2 })), 'not live yet').toMatchObject({ ok: true });
  });

  it('fails on another build, a person identity, or an admin open without a token', async () => {
    expect(await run(admin({ build: 'b'.repeat(40) }))).toMatchObject({ ok: false, why: expect.stringContaining('not aaaaaaa') });
    expect(await run(admin({ kind: 'person' }))).toMatchObject({ ok: false, why: expect.stringContaining('not the service token') });
    expect(await run(admin({ openWithoutToken: true }))).toMatchObject({ ok: false, why: expect.stringContaining('without any Access token') });
  });

  it('checks nothing and says so when the secrets are not set, on the dev admin\'s own host unless --url names another (CD-377), and refuses a run without its arguments', () => {
    const env = { PATH: process.env.PATH };
    const skipped = spawnSync('node', [SCRIPT, '--url', 'https://admin.test', '--sha', SHA], { encoding: 'utf8', env });
    expect.soft([skipped.status, skipped.stdout], '--url').toEqual([0, expect.stringContaining('::warning::admin-smoke')]);
    expect.soft(skipped.stdout, '--url').toContain('the admin at https://admin.test was not checked');
    const dev = spawnSync('node', [SCRIPT, '--sha', SHA], { encoding: 'utf8', env });
    expect.soft([dev.status, dev.stdout], 'dev').toEqual([0, expect.stringContaining(`the admin at ${adminOrigin('dev')} was not checked`)]);
    const refused = spawnSync('node', [SCRIPT, '--url', 'https://admin.test'], { encoding: 'utf8', env });
    expect.soft(refused.status, 'no --sha').toBe(1);
  });
});
