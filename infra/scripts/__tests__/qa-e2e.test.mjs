import { describe, expect, it } from 'vitest';

import { readReport, run } from '../qa-e2e.mjs';

const spec = (title, tags, tests, file = 'store.spec.ts') => ({ title, file, tags, tests });
const test = (projectName, status, message) => ({ projectName, status, results: [{ status: status === 'unexpected' ? 'failed' : 'passed', ...(message ? { errors: [{ message }] } : {}) }] });

describe("the journeys' report (CD-322)", () => {
  it('makes each failed journey one blocking finding on the screens it declares', () => {
    const report = {
      suites: [
        {
          title: 'store.spec.ts',
          specs: [spec('store: buy', ['@screen:/store'], [test('journeys', 'unexpected', '\u001b[31mError: expect(locator).toBeVisible() failed\u001b[39m\n\nCall log:')])],
          suites: [{ specs: [spec('home: cards', ['@screen:/', '@screen:/game/[id]', '@slow'], [test('journeys', 'expected')], 'home.spec.ts')] }],
        },
        { specs: [spec('sign in', [], [test('setup', 'expected')], 'auth.setup.ts')] },
      ],
    };
    expect(readReport(report, 'app')).toEqual({
      ran: 2,
      infrastructure: [],
      findings: [{ severity: 'blocking', screen: '/store', what: 'Error: expect(locator).toBeVisible() failed', source: 'e2e apps/app/e2e/journeys/store.spec.ts › store: buy' }],
    });
  });

  it("treats a failed sign-in or an error outside any test as the run's own failure and files nothing", () => {
    const report = { errors: [{ message: 'Error: browserType.launch: Executable does not exist' }], suites: [{ specs: [spec('sign in', [], [test('setup', 'unexpected', 'Error: 403 from Access')], 'auth.setup.ts'), spec('store: buy', ['@screen:/store'], [test('journeys', 'skipped')])] }] };
    const written = {};
    const out = [];
    const err = [];
    expect(run(['--surface', 'app', '--report', 'r.json', '--out', 'f.json'], { read: () => JSON.stringify(report), write: (path, text) => (written[path] = text), out: (line) => out.push(line), err: (line) => err.push(line) })).toBe(0);
    expect(JSON.parse(written['f.json'])).toEqual({ ran: 0, findings: [], infrastructure: ['Error: browserType.launch: Executable does not exist', 'sign-in setup failed: Error: 403 from Access'] });
    expect(out).toEqual(['qa-e2e: app: 0 journey(s) ran, 0 failed, and the run itself failed']);
    expect(err).toEqual(['qa-e2e: app: Error: browserType.launch: Executable does not exist', 'qa-e2e: app: sign-in setup failed: Error: 403 from Access']);
  });

  it('refuses a missing report', () => {
    expect(run(['--surface', 'app', '--report', 'gone.json', '--out', 'f.json'], { read: () => { throw new Error('ENOENT'); }, err: () => {} })).toBe(2);
  });
});
