import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { MESSAGE_LINES, summarise } from '../journeys-summary.mjs';

const CLI = fileURLToPath(new URL('../journeys-summary.mjs', import.meta.url));
const ROOT = '/work/repo';
const RESULTS = `${ROOT}/apps/admin/test-results`;

const LONG = ['\u001b[31mError: expect(locator).toBeVisible() failed\u001b[39m', '', 'Locator: getByRole(\'row\')', 'Expected: visible', 'Received: hidden', 'Timeout: 5000ms', 'Call log:', '  - waiting for row', '  - a seventh line'].join('\n');

const adminReport = {
  suites: [
    {
      title: 'users.spec.ts',
      specs: [],
      suites: [
        {
          title: 'users',
          specs: [
            {
              title: 'users: list',
              file: 'users.spec.ts',
              tests: [
                {
                  projectName: 'desktop',
                  status: 'unexpected',
                  results: [
                    {
                      errors: [{ message: LONG }],
                      attachments: [
                        { name: 'screenshot', contentType: 'image/png', path: `${RESULTS}/users-list-desktop/test-failed-1.png` },
                        { name: 'trace', contentType: 'application/zip', path: `${RESULTS}/users-list-desktop/trace.zip` },
                        { name: 'dom-excerpt', contentType: 'text/plain', path: `${RESULTS}/users-list-desktop/dom-excerpt.txt` },
                        { name: 'inline', contentType: 'text/plain', body: 'aGk=' },
                        { name: 'elsewhere', contentType: 'text/plain', path: '/home/runner/.secret/file' },
                      ],
                    },
                  ],
                },
                { projectName: 'phone', status: 'expected', results: [{ errors: [], attachments: [] }] },
              ],
            },
            { title: 'users: later', file: 'users.spec.ts', tests: [{ projectName: 'phone', status: 'skipped', results: [] }] },
          ],
        },
      ],
    },
    { title: 'access.setup.ts', specs: [{ title: 'service identity', file: 'access.setup.ts', tests: [{ projectName: 'setup', status: 'expected', results: [{}] }] }] },
  ],
  errors: [{ message: 'Error: webServer exited early\nmore' }],
};

describe('journeys-summary: what a triage agent reads first about a journeys run (CD-405)', () => {
  it('totals every test and lists each failure with its spec, title, project, first message lines and artifact paths, naming no path outside the uploaded results and carrying no attachment body', () => {
    const summary = summarise({ root: ROOT, reports: { admin: adminReport }, notes: { app: 'no journeys yet' } });
    expect.soft(summary.totals).toEqual({ passed: 2, failed: 1, skipped: 1 });
    expect.soft(summary.surfaces).toEqual({ app: { ran: false, note: 'no journeys yet' }, admin: { ran: true, totals: { passed: 2, failed: 1, skipped: 1 }, errors: ['Error: webServer exited early'] } });
    expect.soft(summary.failures).toEqual([
      {
        surface: 'admin',
        file: 'apps/admin/e2e/journeys/users.spec.ts',
        title: 'users › users: list',
        project: 'desktop',
        message: ['Error: expect(locator).toBeVisible() failed', "Locator: getByRole('row')", 'Expected: visible', 'Received: hidden', 'Timeout: 5000ms', 'Call log:'],
        trace: 'admin-test-results/users-list-desktop/trace.zip',
        screenshots: ['admin-test-results/users-list-desktop/test-failed-1.png'],
        attachments: [{ name: 'dom-excerpt', path: 'admin-test-results/users-list-desktop/dom-excerpt.txt' }],
      },
    ]);
    expect.soft(summary.failures[0].message).toHaveLength(MESSAGE_LINES);
    const text = JSON.stringify(summary);
    expect.soft(text, 'outside the results').not.toContain('/home/runner');
    expect.soft(text, 'the root').not.toContain(ROOT);
    expect.soft(text, 'an attachment body').not.toContain('aGk=');
  });

  it('keeps the failing step that follows a timeout in the message', () => {
    const report = { suites: [{ title: 'apps.spec.ts', specs: [{ title: 'apps: list', file: 'apps.spec.ts', tests: [{ projectName: 'desktop', status: 'unexpected', results: [{ errors: [{ message: 'Test timeout of 30000ms exceeded.' }, { message: 'Error: locator.fill: Test timeout of 30000ms exceeded.\nCall log:' }] }] }] }] }] };
    expect(summarise({ root: ROOT, reports: { admin: report }, notes: {} }).failures[0].message).toEqual(['Test timeout of 30000ms exceeded.', 'Error: locator.fill: Test timeout of 30000ms exceeded.', 'Call log:']);
  });

  describe('the command', () => {
    let dir;
    beforeEach(() => {
      dir = mkdtempSync(join(tmpdir(), 'cd405-summary-'));
    });
    afterEach(() => rmSync(dir, { recursive: true, force: true }));

    it("writes journeys-summary.json from the run folder's reports and notes, prints the totals, and says a surface whose report cannot be read did not report", () => {
      writeFileSync(join(dir, 'admin-report.json'), JSON.stringify(adminReport));
      writeFileSync(join(dir, 'app-e2e.json'), JSON.stringify({ ran: 0, findings: [], infrastructure: [], note: 'no journeys selected' }));
      const result = spawnSync(process.execPath, [CLI, '--dir', dir, '--root', ROOT], { encoding: 'utf8' });
      expect.soft(result.status, result.stderr).toBe(0);
      expect.soft(result.stdout).toBe('journeys: 2 passed, 1 failed, 1 skipped\n');
      const written = JSON.parse(readFileSync(join(dir, 'journeys-summary.json'), 'utf8'));
      expect.soft(written.surfaces.app).toEqual({ ran: false, note: 'no journeys selected' });
      expect.soft(written.failures.map((failure) => failure.title)).toEqual(['users › users: list']);
      writeFileSync(join(dir, 'admin-report.json'), '{ not json');
      const unread = spawnSync(process.execPath, [CLI, '--dir', dir, '--root', ROOT], { encoding: 'utf8' });
      expect.soft(unread.status, unread.stderr).toBe(0);
      expect.soft(JSON.parse(readFileSync(join(dir, 'journeys-summary.json'), 'utf8')).surfaces.admin, 'unreadable').toEqual({ ran: false, note: 'the journeys did not report' });
    });
  });
});
