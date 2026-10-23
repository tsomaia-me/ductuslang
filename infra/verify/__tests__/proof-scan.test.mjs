import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import { SCALED_TEXT } from '../../../apps/app/scripts/proof-adapter.mjs';
import { browsersPath, launchChromium } from '../interaction-check.mjs';
import { scanPage, tokensOf } from '../proof-scan.mjs';

const TIMEOUT_MS = 60_000;
const hasChromium = browsersPath() !== undefined;
const TOKENS = { colors: ['255,255,255', '0,0,0', '238,238,238'], sizes: [16] };
const BODY = 'margin:0;background:#ffffff;color:#000000;font-size:16px;font-family:sans-serif';
const BUTTON = 'width:48px;height:48px;font-size:16px;background:#ffffff;color:#000000;border:none;padding:0';
const FIELD = 'position:absolute;top:300px;width:48px;height:48px;border:1px solid #000000;display:flex;align-items:center;box-sizing:border-box';
const INPUT = 'width:30px;height:16px;border:none;background:transparent;padding:0;margin:0';
const page = (inner) => `<!doctype html><html><body style="${BODY}">${inner}</body></html>`;

const FIXTURES = {
  clean: page(
    [
      '<p>Hello <span aria-hidden="true" style="color:#eeeeee">/</span>, read <a href="#terms" style="color:#000000">the terms</a> first</p>',
      `<button style="${BUTTON}">Go</button><button style="${BUTTON};position:absolute;left:-400px;width:10px">x</button>`,
      `<button style="${BUTTON};position:absolute;top:200px;left:200px;width:20px;height:20px">x</button>`,
      `<button style="${BUTTON};position:absolute;top:400px;left:0;width:30px;height:44px">q</button><button style="${BUTTON};position:absolute;top:400px;left:30px;width:30px;height:44px">w</button>`,
      `<div style="${FIELD};left:0;justify-content:flex-end"><input style="${INPUT}"></div><div style="${FIELD};left:48px"><input style="${INPUT}"></div>`,
      '<div style="position:relative;width:100px;height:20px;overflow:hidden"><span style="position:absolute;left:-10000px">read aloud only</span></div>',
      '<div style="height:4px;background:rgb(128,128,128)"></div><pre>2026-10-10T12:00:00Z</pre>',
      '<div style="position:absolute;top:520px;left:200px;width:100px;background:#000000;height:40px"><div style="position:absolute;inset:0;background:#ffffff"></div><span style="position:relative;color:#000000">On</span></div>',
      '<div data-testid="cell-3-4" style="width:22px;height:22px"><span style="font-size:9px;color:#000000">DL</span></div>',
      `<div style="height:60px;overflow:auto"><div style="height:120px"></div><button style="${BUTTON}">Down</button></div><div style="height:120px;background:#ffffff"></div>`,
    ].join(''),
  ),
  overflow: page('<div style="width:600px">wide</div>'),
  cut: page('<p style="width:40px;overflow:hidden;white-space:nowrap">a long sentence the box clips</p>'),
  covered: page(`<button style="${BUTTON};position:absolute;top:0;left:0">Go</button><div style="position:absolute;top:0;left:0;width:100px;height:100px;background:#ffffff"></div>`),
  contrast: page('<p style="color:#eeeeee">faint</p>'),
  token: page('<p style="color:#000000;font-size:15px">off the scale</p><div style="height:4px;background:#123456"></div><div data-testid="cell-3-4-tile"><span style="font-size:15px">P</span></div>'),
  target: page(`<button style="${BUTTON};width:20px;height:20px">x</button><button style="${BUTTON};width:20px;height:20px">y</button>`),
  image: page('<img alt="" src="data:image/png;base64,AAAA">'),
  text: page('<p>Total: NaN</p><p>65f1c0ffee00000000000a11</p><p>2026-10-10T12:00:00Z</p>'),
};

const CUT_LONG = 'a long sentence the box cuts';
const TOOLTIP = page(
  [
    `<div class="cut" style="width:40px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis">${CUT_LONG}</div><div style="height:900px"></div><div data-portal="true"></div>`,
    '<script>',
    "const cut = document.querySelector('.cut');",
    "const portal = document.querySelector('[data-portal]');",
    `cut.addEventListener('mouseenter', () => { const tip = document.createElement('div'); tip.setAttribute('role', 'tooltip'); tip.style.cssText = 'position:fixed;top:40px;left:0'; tip.textContent = '${CUT_LONG}'; portal.append(tip); });`,
    "cut.addEventListener('mouseleave', () => portal.replaceChildren());",
    '</script>',
  ].join(''),
);
const BAR = 'position:fixed;top:0;left:0;right:0;height:60px;background:#ffffff';
const SCROLLED_UNDER_BAR = page(`<div style="${BAR}"></div><div style="height:400px"></div><button style="${BUTTON}">Go</button><div style="height:1200px"></div><script>scrollTo(0, 380)</script>`);
const BOXED = 'width:48px;height:48px;border:1px solid #000000;box-sizing:border-box;display:block';
const BODY_SCROLL = 'height:200px;overflow:auto';
const FOOTER_OVER_LAST = page(
  `<div style="${BODY_SCROLL}"><div style="height:400px"></div><input style="${BOXED}"><div style="position:sticky;bottom:0;height:60px;margin-top:-50px;background:#ffffff"></div></div><script>const body = document.querySelector('div'); body.scrollTop = body.scrollHeight;</script>`,
);
const HEADER_AT_TOP = page(`<div style="${BODY_SCROLL}"><div style="position:sticky;top:0;height:60px;background:#ffffff"></div><input style="${BOXED};margin-top:-40px"><div style="height:400px"></div></div>`);
const FIXED_FOOTER_AT_END = page(
  `<div style="height:1200px"></div><input style="${BOXED}"><div style="position:fixed;bottom:0;left:0;right:0;height:60px;background:#ffffff"></div><script>scrollTo(0, document.documentElement.scrollHeight)</script>`,
);
const SHEET = 'position:fixed;top:0;left:0;width:300px;height:300px;background:#ffffff';
const SHEET_ABOVE = 'position:fixed;top:0;left:0;width:200px;height:200px;background:#ffffff';
const NESTED_MODALS = page(
  `<div data-portal="true"><div><section role="dialog" aria-modal="true" style="${SHEET}"><button style="${BUTTON}">Go</button></section></div><div><section role="dialog" aria-modal="true" style="${SHEET_ABOVE}"></section></div></div>`,
);
const EARLIER_LAYER_ON_TOP = page(
  `<div data-portal="true"><div><section role="dialog" aria-modal="true" style="${SHEET_ABOVE};z-index:2"></section></div><div><section role="dialog" aria-modal="true" style="${SHEET}"><button style="${BUTTON}">Go</button></section></div></div>`,
);
const DROPDOWN_OVER_DIALOG = page(
  `<section role="dialog" aria-modal="true" style="${SHEET}"><textarea style="${BOXED}"></textarea></section><div data-portal="true"><div role="listbox" style="position:fixed;top:0;left:0;width:100px;height:100px;background:#ffffff"></div></div>`,
);
const PHONE_NAV = (expanded) =>
  page(
    `<button aria-expanded="${expanded}" aria-label="Navigation" style="${BUTTON}">N</button><p style="position:absolute;top:100px;margin:0">Page text</p><nav style="position:fixed;top:60px;left:0;right:0;bottom:0;background:#ffffff"></nav>`,
  );
const NOTICE = 'position:fixed;bottom:0;left:0;right:0;height:80px;background:#ffffff';
const TOAST = page(`<p style="position:absolute;top:780px;margin:0">Reward</p><div data-portal="true"><div style="${NOTICE}"><div role="alert">SPRING25 was already deactivated</div></div></div>`);
const BANNER = page(`<p style="position:absolute;top:780px;margin:0">Reward</p><div style="${NOTICE}"><div role="alert">SPRING25 was already deactivated</div></div>`);
const CONTAINER_SIZED = page('<div style="width:150px;container-type:inline-size"><div style="font-size:calc(100cqw / 15 * 0.5)">DL</div></div>');
const TILE_TEXT = page('<div data-testid="rack-tile-0"><div style="font-size:10px">4</div></div><div data-testid="peep-emblem-row-0"><div style="font-size:11px">B</div></div>');
const UNHITTABLE_PILL = page('<style>.unhittable { pointer-events: none !important; }</style><div style="position:relative;width:200px;height:40px;background:#000000"><div class="unhittable" style="position:absolute;inset:0;background:#ffffff"></div><span style="position:relative;color:#000000">On</span></div>');
const UNDER_BAR = page(`<button style="${BUTTON};position:absolute;top:0;left:0">Go</button><div style="${BAR}"></div>`);
const OVERLAY = 'position:absolute;top:0;left:0;width:100px;height:100px;background:#ffffff';
const LOADING = (clears) =>
  page(
    `<button style="${BUTTON}">Go</button><div class="mantine-LoadingOverlay-root" style="${OVERLAY}"></div>` +
      (clears ? "<script>setTimeout(() => document.querySelector('.mantine-LoadingOverlay-root').remove(), 300)</script>" : ''),
  );

describe('the theme tokens are every colour and font size the theme modules hold', () => {
  it('takes colours from any string and sizes only under a font size or type key', () => {
    const tokens = tokensOf({
      PALETTE: { text: '#eceef3', glow: '0 8px 22px -10px #3a8dff', scrim: 'rgba(59, 42, 30, 0.45)' },
      TYPE: { td: 13.5 },
      RADIUS: { card: 12 },
      theme: { components: { NavLink: { styles: { label: { fontSize: 14 } } } }, fontSizes: { sm: 'calc(0.875rem * var(--mantine-scale))' } },
    });
    expect(tokens.colors).toEqual(['236,238,243', '58,141,255', '59,42,30']);
    expect(tokens.sizes).toEqual([13.5, 14]);
  });
});

describe.skipIf(!hasChromium)('the proof scans a page before it shoots it', { timeout: TIMEOUT_MS }, () => {
  let browser;

  beforeAll(async () => {
    browser = await launchChromium();
  }, TIMEOUT_MS);

  afterAll(async () => {
    await browser?.close();
  });

  const scan = async (html) => {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
    try {
      const tab = await context.newPage();
      await tab.setContent(html);
      return await scanPage(tab, { tokens: TOKENS, target: 44, phone: true, scaled: SCALED_TEXT });
    } finally {
      await context.close();
    }
  };

  const PAGES_AT_ONCE = 4;
  const scanAll = async (fixtures) => {
    const found = [];
    let next = 0;
    const lane = async () => {
      while (next < fixtures.length) {
        const index = next++;
        found[index] = await scan(fixtures[index]);
      }
    };
    await Promise.all(Array.from({ length: Math.min(PAGES_AT_ONCE, fixtures.length) }, lane));
    return found;
  };
  const checksOf = (found) => new Set(found.map(({ check }) => check));
  const CHECKS = ['overflow', 'cut', 'covered', 'contrast', 'token', 'target', 'image', 'text'];

  it('finds nothing on a page that keeps every rule, each check alone on a page that breaks it, and names what it found', async () => {
    const [clean, ...broken] = await scanAll([FIXTURES.clean, ...CHECKS.map((check) => FIXTURES[check])]);
    expect.soft(clean, 'clean').toEqual([]);
    CHECKS.forEach((check, index) => {
      const found = broken[index];
      expect.soft(found.length, check).toBeGreaterThan(0);
      expect.soft(checksOf(found), check).toEqual(new Set([check]));
      for (const finding of found) expect.soft(finding.element && finding.detail, check).toBeTruthy();
    });
    const details = (check) => broken[CHECKS.indexOf(check)].map(({ detail }) => detail);
    expect.soft(details('token'), 'token').toEqual(['font size 15px is not a theme token (and 1 more)', 'background rgb(18,52,86) is not a theme token']);
    expect
      .soft(details('target'), 'target')
      .toEqual(['the control is 20x20px, under the 44px touch size, and another target sits within 12px of its centre (and 1 more)']);
    expect.soft(details('text'), 'text').toEqual(['shows "NaN": "Total: NaN"', 'shows a raw id: "65f1c0ffee00000000000a11"', 'shows a raw ISO date: "2026-10-10T12:00:00Z"']);
  });

  it('passes what a tooltip portal, a scroll, a later layer, an open nav, a toast, a cleared loader, scaled text or an unhittable pill explains, and flags a cover nothing explains', async () => {
    const cases = [
      ['a tooltip in a portal node of no height at the page bottom', TOOLTIP, []],
      ['a control the page scrolled under a fixed bar', SCROLLED_UNDER_BAR, []],
      ['a loading overlay that clears', LOADING(true), []],
      ['nested modals', NESTED_MODALS, []],
      ['a dropdown over a dialog', DROPDOWN_OVER_DIALOG, []],
      ['page text under the open phone nav', PHONE_NAV(true), []],
      ['text under a toast in a portal', TOAST, []],
      ['text sized by a container-query unit', CONTAINER_SIZED, []],
      ['rack tile and emblem text sized from their box', TILE_TEXT, []],
      ['text against a pill that takes no pointer events', UNHITTABLE_PILL, []],
      ['a control a fixed bar covers on a page that does not scroll', UNDER_BAR, ['covered']],
      ['a control under a sticky footer over the last field', FOOTER_OVER_LAST, ['covered']],
      ['a control under a sticky header at the top', HEADER_AT_TOP, ['covered']],
      ['a control under a fixed footer at the end', FIXED_FOOTER_AT_END, ['covered']],
      ['a loading overlay that never clears', LOADING(false), ['covered']],
      ['what an earlier layer covers', EARLIER_LAYER_ON_TOP, ['covered']],
      ['page text under a pinned nav nothing opened', PHONE_NAV(false), ['covered']],
      ['text under an in-page pinned banner', BANNER, ['covered']],
    ];
    const found = await scanAll(cases.map(([, fixture]) => fixture));
    cases.forEach(([name, , checks], index) => {
      if (checks.length === 0) expect.soft(found[index], name).toEqual([]);
      else expect.soft(checksOf(found[index]), name).toEqual(new Set(checks));
    });
  });
});
