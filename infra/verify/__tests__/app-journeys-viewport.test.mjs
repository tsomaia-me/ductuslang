import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { screenFindings } from '../../../apps/app/e2e/journeys/fixtures.ts';
import { DESKTOP, PHONE } from '../../../apps/app/e2e/journeys/viewports.ts';
import { browsersPath, launchChromium } from '../interaction-check.mjs';
import { watchConsole } from '../interaction-checks.mjs';

const TIMEOUT_MS = 60_000;
const ODD_SIZE = { width: 1000, height: 700 };
const FAULT = 'a console error the interaction check must report';

let browser;

beforeAll(async () => {
  if (browsersPath() !== undefined) browser = await launchChromium();
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
});

async function findingsAt(size) {
  const context = await browser.newContext({ viewport: size });
  try {
    const page = await context.newPage();
    const watch = watchConsole(page);
    await page.goto('about:blank');
    await page.evaluate((message) => console.error(message), FAULT);
    return await screenFindings(page, '/x', watch);
  } finally {
    await context.close();
  }
}

describe.skipIf(browsersPath() === undefined)('the app journeys label a finding by the width the page was at (CD-381 F1)', { timeout: TIMEOUT_MS }, () => {
  it('names the phone and the desktop size the way the viewports do, and any other size by its numbers', async () => {
    const seen = {};
    for (const [name, size] of [['phone', PHONE], ['desktop', DESKTOP], ['odd', ODD_SIZE]]) {
      const findings = await findingsAt(size);
      expect(findings.map((finding) => finding.check)).toEqual(['console']);
      seen[name] = findings[0].viewport;
    }
    expect(seen).toEqual({ phone: 'phone-390x844', desktop: 'laptop-1280x800', odd: '1000x700' });
  });

  it('follows a resize on the same page', async () => {
    const context = await browser.newContext({ viewport: PHONE });
    try {
      const page = await context.newPage();
      const watch = watchConsole(page);
      await page.goto('about:blank');
      await page.evaluate((message) => console.error(message), FAULT);
      const before = await screenFindings(page, '/x', watch);
      await page.setViewportSize(DESKTOP);
      await page.evaluate((message) => console.error(message), FAULT);
      const after = await screenFindings(page, '/x', watch);
      expect([before[0].viewport, after[0].viewport]).toEqual(['phone-390x844', 'laptop-1280x800']);
    } finally {
      await context.close();
    }
  });
});
