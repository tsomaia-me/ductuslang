import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { CONFIGS, RETIRED, chosenConfigs, readConfig, varsDrift } from '../../../apps/edge/scripts/live-vars.mjs';

const EDGE = fileURLToPath(new URL('../../../apps/edge/', import.meta.url));

describe('live-vars (CD-142)', () => {
  it('names every var that differs or is missing on either side, ignores BUILD_SHA, and lets a retired var leave a Worker, and only a retired one, only by its absence from the config (CD-291)', () => {
    for (const [label, config, live, drift] of [
      ['a differing and a missing var, BUILD_SHA ignored', { A: '1', B: '2', BUILD_SHA: '' }, { A: '1', B: '3', C: 'x', BUILD_SHA: 'abc' }, ['B', 'C']],
      ['the same', { A: '1' }, { A: '1' }, []],
      ['missing live', { A: '' }, {}, ['A']],
      ['retired vars left live', { A: '1' }, { A: '1', ADMIN_USER_IDS: '', ACCESS_AUD: '' }, []],
      ['a retired var still in the config', { A: '1', ACCESS_AUD: 'new' }, { A: '1', ACCESS_AUD: 'old' }, ['ACCESS_AUD']],
      ['a var that is not retired', { A: '1' }, { A: '1', OTHER: '' }, ['OTHER']],
    ]) {
      expect.soft(varsDrift(config, live), label).toEqual(drift);
    }
    expect.soft([...RETIRED]).toEqual(['ADMIN_USER_IDS', 'ADMIN_EMAILS', 'ACCESS_TEAM_DOMAIN', 'ACCESS_AUD']);
    for (const env of ['production', 'dev']) {
      const api = readConfig(`${EDGE}wrangler.api.jsonc`).env[env].vars;
      for (const name of RETIRED) expect.soft([env, name, name in api]).toEqual([env, name, false]);
    }
  });

  it('compares every Worker the deploy ships, the admin included (CD-291)', () => {
    expect(CONFIGS).toEqual(['wrangler.auth.jsonc', 'wrangler.api.jsonc', 'wrangler.admin.jsonc']);
    for (const file of CONFIGS) for (const env of ['production', 'dev']) expect(readConfig(`${EDGE}${file}`).env[env].vars.ENVIRONMENT).toBe(env);
  });

  it('compares only the Workers --config names, every one without it, and refuses a file it does not know (CD-478)', () => {
    expect(chosenConfigs(['--env', 'dev'])).toEqual(CONFIGS);
    expect(chosenConfigs(['--env', 'dev', '--config', 'wrangler.auth.jsonc', '--config', 'wrangler.api.jsonc'])).toEqual(['wrangler.auth.jsonc', 'wrangler.api.jsonc']);
    expect(() => chosenConfigs(['--config', 'wrangler.jsonc'])).toThrow(/--config takes/);
  });

  it('holds each stack’s own addresses: production at crookedduke.com, dev at dev.crookedduke.com behind Access (CD-285)', () => {
    const addresses = {
      production: { web: 'https://crookedduke.com', api: 'https://api.crookedduke.com' },
      dev: { web: 'https://dev.crookedduke.com', api: 'https://dev.crookedduke.com' },
    };
    for (const [env, want] of Object.entries(addresses)) {
      const api = readConfig(`${EDGE}wrangler.api.jsonc`).env[env].vars;
      const auth = readConfig(`${EDGE}wrangler.auth.jsonc`).env[env].vars;
      expect(api.ENVIRONMENT, env).toBe(env);
      expect(auth.ENVIRONMENT, env).toBe(env);
      expect(api.APP_WEB_URL, env).toBe(want.web);
      expect(api.APP_WEB_URL_ALIASES, env).toBe('');
      expect(auth.APP_WEB_URL, env).toBe(want.web);
      expect(auth.API_PUBLIC_URL, env).toBe(want.api);
      for (const vars of [api, auth]) {
        expect(vars.MAIL_FROM, env).toMatch(/@mail\.crookedduke\.com>$/);
        expect(JSON.stringify(vars), env).not.toMatch(/beta\.crookedduke|api-next/);
      }
      expect(api.VAPID_PUBLIC_KEY, env).toMatch(/^[A-Za-z0-9_-]{87}$/);
    }
    // Production signs push with its own key pair (owner, CD-285 Q8): its private half was made fresh for cd-api.
    expect(readConfig(`${EDGE}wrangler.api.jsonc`).env.production.vars.VAPID_PUBLIC_KEY).not.toBe(readConfig(`${EDGE}wrangler.api.jsonc`).env.dev.vars.VAPID_PUBLIC_KEY);
  });
});
