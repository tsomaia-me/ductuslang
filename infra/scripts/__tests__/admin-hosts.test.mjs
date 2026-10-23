import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { SURFACES } from '../../verify/surfaces.mjs';
import { ADMIN_HOSTS, adminOrigin } from '../admin-hosts.mjs';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const HOME = 'packages/shared/src/admin/hosts.json';
const OWN_TEST = 'infra/scripts/__tests__/admin-hosts.test.mjs';
const DOCS = /(^|\/)[^/]+\.md$|^\.claude\//;
const BINARY = /\.(png|jpe?g|webp|gif|ico|woff2?|ttf|otf|pdf|zip|gz|wasm)$/i;

describe('the admin’s hosts (CD-377)', () => {
  it('are admin.parlornights.com and its dev twin, each replacing its crookedduke.com host', () => {
    expect(ADMIN_HOSTS).toEqual({
      production: { host: 'admin.parlornights.com', legacyHost: 'admin.crookedduke.com' },
      dev: { host: 'admin-dev.parlornights.com', legacyHost: 'admin-dev.crookedduke.com' },
    });
    expect(adminOrigin('dev')).toBe('https://admin-dev.parlornights.com');
    expect(SURFACES.admin.address).toBe(adminOrigin('dev'));
  });

  it('are written in one place: no code, config or workflow outside the shared admin hosts names one', () => {
    const hosts = Object.values(ADMIN_HOSTS).flatMap(({ host, legacyHost }) => [host, legacyHost]);
    const files = execFileSync('git', ['ls-files', '-z'], { cwd: ROOT, encoding: 'utf8' })
      .split('\0')
      .filter((file) => file && file !== HOME && file !== OWN_TEST && !DOCS.test(file) && !BINARY.test(file));
    const copies = files.flatMap((file) => {
      let text;
      try {
        text = readFileSync(join(ROOT, file), 'utf8');
      } catch {
        return [];
      }
      return hosts.filter((host) => text.includes(host)).map((host) => `${file}: ${host}`);
    });
    expect(copies).toEqual([]);
  });
});
