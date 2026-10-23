import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { nightlyFindings, run } from '../nightly-findings.mjs';
import { parseFindings } from '../qa-linear.mjs';

const RUN = 'https://github.com/parlornights/CrookedDuke/actions/runs/1';
const journeyFailure = (surface, file, title, project) => ({ surface, file, title, project, message: ['Error: timed out at a different line each night'], trace: null, screenshots: [], attachments: [] });

describe('the nightly failures as findings for the MVP debt issue', () => {
  it('files each failing journey test as blocking under its surface, keyed by its file, title and project, never by its message; each catalogue shot the scan faulted or that failed, once per screen and finding across widths and themes; one blocking item when the run failed and left no failure to read; each surface as findings the QA filer accepts', () => {
    const faint = { check: 'contrast', element: 'p "Insights"', detail: 'text contrast 4.01:1 is under 4.5:1' };
    const rows = [
      {
        label: 'journeys',
        input: { summary: { failures: [journeyFailure('app', 'apps/app/e2e/journeys/store.spec.ts', 'store: buy', 'journeys'), journeyFailure('admin', 'apps/admin/e2e/journeys/users.spec.ts', 'users: list', 'phone')] }, run: RUN },
        filed: {
          app: [{ severity: 'blocking', screen: 'apps/app/e2e/journeys/store.spec.ts', what: 'store: buy (journeys)', source: 'nightly journeys', link: RUN }],
          admin: [{ severity: 'blocking', screen: 'apps/admin/e2e/journeys/users.spec.ts', what: 'users: list (phone)', source: 'nightly journeys', link: RUN }],
        },
      },
      {
        label: 'catalogue scan',
        input: {
          summary: { failures: [] },
          scanned: [
            { surface: 'admin', screen: 'overview', width: 1440, theme: 'dark', state: 'default', error: null, scan: [faint] },
            { surface: 'admin', screen: 'overview', width: 390, theme: 'dark', state: 'default', error: null, scan: [faint] },
            { surface: 'app', screen: '/store', width: 390, theme: 'light', state: 'error', error: 'no shot within 10000 ms', scan: [] },
          ],
          run: RUN,
        },
        filed: {
          app: [{ severity: 'blocking', screen: 'app:/store', what: 'the shot failed: no shot within 10000 ms', source: 'nightly scan', link: RUN }],
          admin: [{ severity: 'blocking', screen: 'admin:overview', what: 'contrast p "Insights": text contrast 4.01:1 is under 4.5:1', source: 'nightly scan', link: RUN }],
        },
      },
      {
        label: 'a broken run',
        input: { summary: undefined, run: RUN },
        filed: { app: [{ severity: 'blocking', screen: 'nightly', what: 'the nightly run failed without a failure it could read', source: 'nightly', link: RUN }], admin: [] },
      },
    ];
    for (const row of rows) {
      const filed = nightlyFindings(row.input);
      expect.soft(filed, row.label).toEqual(row.filed);
      for (const [surface, findings] of Object.entries(filed)) expect.soft(parseFindings(JSON.stringify(findings)), `${row.label} ${surface} for the filer`).toEqual(findings);
    }
  });
});

describe('nightly-findings.mjs', () => {
  let dir;
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'nightly-findings-'));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it('reads the downloaded journeys summary and writes <surface>-file.json beside it, and needs the folder and the run address', () => {
    mkdirSync(join(dir, 'journeys-summary'));
    writeFileSync(join(dir, 'journeys-summary', 'journeys-summary.json'), JSON.stringify({ failures: [journeyFailure('app', 'apps/app/e2e/journeys/store.spec.ts', 'store: buy', 'journeys'), journeyFailure('admin', 'apps/admin/e2e/journeys/users.spec.ts', 'users: list', 'phone')] }));
    expect.soft(run(['--dir', dir, '--run', RUN])).toBe(0);
    expect.soft(JSON.parse(readFileSync(join(dir, 'app-file.json'), 'utf8'))).toHaveLength(1);
    expect.soft(JSON.parse(readFileSync(join(dir, 'admin-file.json'), 'utf8'))).toHaveLength(1);
    expect.soft(run(['--dir', dir], { err: () => {} }), 'no run address').toBe(2);
  });
});
