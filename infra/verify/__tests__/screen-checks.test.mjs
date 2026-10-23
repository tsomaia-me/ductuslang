import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { browsersPath, launchChromium } from '../interaction-check.mjs';
import { checkCoveredText, checkCutText, settleLayout } from '../interaction-checks.mjs';

const pages = join(dirname(fileURLToPath(import.meta.url)), 'screen-checks');
const TIMEOUT_MS = 60_000;
const BOUNDED_TIMEOUT_MS = 6_000;
const SETTLE_CAP_MS = 300;
const VIEWPORT = { width: 1200, height: 800 };

const NO_TIP = 'text is cut with an ellipsis and there is no tooltip or title with the full value';
const WRONG_TIP = 'text is cut with an ellipsis and its tooltip does not hold the full value';
const LETTER_SWAG = 'p "Letter Swag is moving to the English boa"';

const hasChromium = browsersPath() !== undefined;
let browser;

beforeAll(async () => {
  if (hasChromium) browser = await launchChromium();
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
});

async function onPage(name, check, { scrollTo = 0 } = {}) {
  const context = await browser.newContext({ viewport: VIEWPORT });
  try {
    const tab = await context.newPage();
    await tab.goto(pathToFileURL(join(pages, name)).href, { waitUntil: 'load' });
    await tab.evaluate((top) => globalThis.scrollTo(0, top), scrollTo);
    await settleLayout(tab, SETTLE_CAP_MS);
    const findings = await check(tab);
    return { findings, scrolled: await tab.evaluate(() => globalThis.scrollY) };
  } finally {
    await context.close();
  }
}

const checkThenCountCut = async (tab) => ({
  reported: await checkCutText(tab),
  ...(await tab.evaluate(() => ({
    renders: Number(globalThis.document.body.dataset.rendered ?? '0'),
    left: globalThis.document.querySelectorAll('.cut').length,
  }))),
});

async function expectFindings(cases) {
  const found = await Promise.all(cases.map(([name, check]) => onPage(name, check)));
  cases.forEach(([name, , expected], index) => expect.soft(found[index].findings, name).toEqual(expected));
}

describe.skipIf(!hasChromium)('checkCutText', { timeout: TIMEOUT_MS }, () => {
  it('stays quiet for a tooltip on focus only, cut text a scrim covers, and items whose tooltip before is still closing', async () => {
    await expectFindings([
      ['tooltip-on-focus.html', checkCutText, []],
      ['under-overlay.html', checkCutText, []],
      ['lingering-tooltips.html', checkCutText, []],
    ]);
  });

  it(
    'still checks text that never holds still under the pointer, and checks the element that took the place of one the page rendered again, once, reporting it only when nothing shows its full value',
    { timeout: BOUNDED_TIMEOUT_MS },
    async () => {
      await expectFindings([
        ['restless.html', checkCutText, [{ element: LETTER_SWAG, detail: WRONG_TIP }]],
        ['rerendered-with-tooltip.html', checkCutText, []],
        ['rerendered-no-tooltip.html', checkCutText, [{ element: LETTER_SWAG, detail: NO_TIP }]],
        ['rerendered-titled.html', checkCutText, []],
        ['rendered-anew-on-each-check.html', checkThenCountCut, { reported: [{ element: LETTER_SWAG, detail: NO_TIP }], renders: 2, left: 1 }],
        ['gone-after-check.html', checkThenCountCut, { reported: [], renders: 0, left: 0 }],
        ['rendered-anew-then-gone.html', checkThenCountCut, { reported: [], renders: 1, left: 0 }],
      ]);
    },
  );

  it('hovers one reachable cut text per table, visual column, cell, element inside it and kind of cut, and per repeated list, row within a card, element inside its repeated item and kind of cut, reporting each that shows no full value, and still hovers every cut text with no repeated ancestor', async () => {
    const hoversAndFindings = async (tab) => ({
      reported: await checkCutText(tab),
      hovers: await tab.evaluate(() =>
        [...globalThis.document.querySelectorAll('.cut')].reduce((sum, el) => sum + Number(el.dataset.hovers ?? '0'), 0),
      ),
    });
    const [table, cards] = await Promise.all([onPage('cut-table.html', hoversAndFindings), onPage('cut-cards.html', hoversAndFindings)]);
    expect.soft(table.findings.hovers, 'table hovers').toBe(9);
    expect.soft(table.findings.reported, 'table reported').toEqual([
      { element: LETTER_SWAG, detail: NO_TIP },
      { element: 'p "Anagram Arena stays on the board while i"', detail: NO_TIP },
      { element: 'span "Paused 2 times while the editors rebuild"', detail: NO_TIP },
      { element: 'span "Paused 3 times while the editors rebuild"', detail: NO_TIP },
      { element: 'strong "Board 12 is the long name of a word list"', detail: NO_TIP },
      { element: 'span "Rebuilt list waits for the editors to si"', detail: NO_TIP },
      { element: 'span "Spelling Bee keeps its own board while t"', detail: NO_TIP },
    ]);
    expect.soft(cards.findings.hovers, 'card hovers').toBe(10);
    expect.soft(cards.findings.reported, 'cards reported').toEqual([
      { element: 'p "Maintenance pauses every board of the ap"', detail: NO_TIP },
      { element: 'p "Weekly puzzles wait for the editors to s"', detail: NO_TIP },
      { element: 'p "Note 2 says the app is paused while its "', detail: NO_TIP },
      { element: 'p "Note 3 says the app is paused while its "', detail: NO_TIP },
      { element: 'em "Note 8 says the app is paused while its "', detail: NO_TIP },
      { element: 'p "Board note 1 says the board waits for th"', detail: NO_TIP },
      { element: 'span "Owner 1 is the editor who keeps the week"', detail: NO_TIP },
      { element: 'span "Tile 1 holds a word list waiting for the"', detail: NO_TIP },
    ]);
  });
});

describe.skipIf(!hasChromium)('checkCoveredText', { timeout: TIMEOUT_MS }, () => {
  const COVERED = [
    {
      element: 'p "First line under the header"',
      detail: 'text "First line under the header" is covered by the pinned div "Header" sitting over it',
    },
    {
      element: 'p "Last line under the footer"',
      detail: 'text "Last line under the footer" is covered by the pinned div "Footer" sitting over it',
    },
  ];

  it('reports the text a fixed header covers at the top and a fixed footer at the end of a scrolling page, from the top or the middle, and puts the scroll back where it was', async () => {
    const [fromTop, fromMiddle] = await Promise.all([
      onPage('covered-top-and-bottom.html', checkCoveredText),
      onPage('covered-top-and-bottom.html', checkCoveredText, { scrollTo: 900 }),
    ]);
    expect.soft(fromTop.findings, 'from the top').toEqual(COVERED);
    expect.soft(fromMiddle, 'from the middle').toEqual({ findings: COVERED, scrolled: 900 });
  });
});
