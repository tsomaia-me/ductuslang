/**
 * `DEV_MODE` is a hatch into other people's games (ARCHITECTURE §21), and the
 * only thing standing between it and production is that nothing deployed sets it.
 *
 * So every deployed environment of both Workers configs is read and asserted on,
 * in the suite that runs even when a commit's whole diff is infra — which is
 * exactly the commit that could add it. `dev` is a deployed stack too (CD-285:
 * dev.crookedduke.com), so it is held to the same rule as `production`; only
 * the top level (the LOCAL definition `wrangler dev` and the suites use) may say
 * otherwise, and the full-stack suite turns it on through its own env files.
 */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const EDGE = fileURLToPath(new URL('../../../apps/edge/', import.meta.url));
const CONFIGS = ['wrangler.api.jsonc', 'wrangler.auth.jsonc'];
const DEPLOYED = ['production', 'dev'];

function config(name) {
  return JSON.parse(
    readFileSync(`${EDGE}${name}`, 'utf8')
      .split('\n')
      .filter((line) => !line.trimStart().startsWith('//'))
      .join('\n'),
  );
}

describe('the deployed Workers environments', () => {
  it('never turn DEV_MODE on in any deployed environment of either Workers config', () => {
    for (const name of CONFIGS) {
      for (const env of DEPLOYED) {
        const scope = config(name).env?.[env];
        expect.soft(scope, `${name} has no env "${env}"`).toBeDefined();
        const value = scope?.vars?.DEV_MODE;
        expect.soft(value === undefined || value === 'false', `${name} → ${env} sets DEV_MODE=${String(value)}`).toBe(true);
      }
    }
  });
});
