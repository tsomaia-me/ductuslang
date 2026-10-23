/**
 * The Cloudflare Access service token on every check of a dev host (CD-285): `access-headers.mjs`
 * for the node scripts. Both secrets set: both headers; one or none: no header.
 */
import { describe, expect, it } from 'vitest';

import { accessHeaders } from '../access-headers.mjs';

describe('accessHeaders', () => {
  it('names both headers only when both secrets are set', () => {
    expect(accessHeaders({ CF_ACCESS_CLIENT_ID: 'id.access', CF_ACCESS_CLIENT_SECRET: 'secret' })).toEqual({ 'CF-Access-Client-Id': 'id.access', 'CF-Access-Client-Secret': 'secret' });
    expect(accessHeaders({ CF_ACCESS_CLIENT_ID: 'id.access' })).toEqual({});
    expect(accessHeaders({ CF_ACCESS_CLIENT_ID: ' ', CF_ACCESS_CLIENT_SECRET: 'secret' })).toEqual({});
    expect(accessHeaders({})).toEqual({});
  });
});
