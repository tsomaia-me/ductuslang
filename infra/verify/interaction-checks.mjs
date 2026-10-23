import {
  CONTROLS_SELECTOR,
  FOCUSABLE_SELECTOR,
  OVERFLOW_TOLERANCE_PX,
  isReached,
  layoutHasSettled,
  listOutsideButtons,
  listTriggers,
  markClickableRows,
  markCutText,
  markOpenedByTrigger,
  markOverlays,
  markTapTargets,
  neutralPoint,
  overlayState,
  overrunAfterTap,
  scanCellOverflow,
  scanClippedText,
  scanCoveredText,
  scanTabStopsInControls,
  stillCut,
  tooltipTexts,
} from './interaction-scan.mjs';

const COVERING_POSITIONS = ['sticky', 'fixed'];

export const CHECKS = ['overflow', 'truncation', 'covered', 'overlay', 'row-click', 'console'];

const SETTLE_MS = 400;
const OPEN_MS = 600;
const CLOSE_MS = 600;
const POLL_MS = 50;
const HOVER_RESET_MS = 80;
const TOOLTIP_MS = 700;
const ACTION_TIMEOUT_MS = 3000;
const ITEM_WAIT_MS = 300;
const ACTIONABLE_WAIT_MS = 500;
const CUT_ATTEMPTS = 2;
const CLIPPED_LINE_SHARE = 0.25;
const MAX_TRIGGERS = 3;
const MAX_TAP_TARGETS = 6;
const TAP_MS = 400;
const MAX_OUTSIDE_BUTTONS = 1;
const MAX_ROW_TABLES = 3;
const MARK_TRIGGER = 'data-interaction-trigger';
const MARK_PRESS = 'data-interaction-press';
const MARK_SEEN = 'data-interaction-seen';
const MARK_SHOWN = 'data-interaction-shown';
const MARK_OPENED = 'data-interaction-opened';
const MARK_ROW = 'data-interaction-row';
const MARK_CUT = 'data-interaction-cut';
const MARK_TAP = 'data-interaction-tap';
const MARK_LAYOUT = '__interactionLayout';
const QUIET_POLLS = 3;
const RESOURCE_FAILURE = /^Failed to load resource: the server responded with a status of (\d{3})/;

const squash = (text) => text.replace(/\s+/g, ' ').trim();

const onlyTimeout = (error) => {
  if (error?.name !== 'TimeoutError') throw error;
  return null;
};

async function settle(page, { probe, probeArg = null, until, untilArg = null, ms }) {
  const met = await page
    .waitForFunction(
      `(() => { const seen = (${probe})(${JSON.stringify(probeArg)}); return (${until})(seen, ${JSON.stringify(untilArg)}) ? { seen } : false; })()`,
      undefined,
      { polling: POLL_MS, timeout: ms },
    )
    .catch(onlyTimeout);
  if (met) return (await met.jsonValue()).seen;
  return page.evaluate(probe, probeArg);
}

export async function settleLayout(page, ms = SETTLE_MS) {
  await page.evaluate((markName) => delete globalThis[markName], MARK_LAYOUT);
  const settled = await page
    .waitForFunction(layoutHasSettled, { markName: MARK_LAYOUT, quietPolls: QUIET_POLLS }, { polling: POLL_MS, timeout: ms })
    .catch(onlyTimeout);
  return settled !== null;
}

const call = (fn, arg) => `(${fn})(${JSON.stringify(arg)})`;

const isUncut = (state) => state.gone || !state.cut;
const holdsText = (shown, wanted) => shown.some((tip) => tip.includes(wanted));
const noneShown = (shown) => shown.length === 0;
const atLeast = (found, count) => found.length >= count;

export function watchConsole(page) {
  const seen = [];
  page.on('console', (message) => {
    if (message.type() !== 'error') return;
    const text = message.text();
    const status = RESOURCE_FAILURE.exec(text)?.[1];
    if (status?.startsWith('4')) return;
    seen.push({ element: message.location().url || 'console', detail: text });
  });
  page.on('pageerror', (error) => {
    seen.push({ element: 'page', detail: String(error?.message ?? error) });
  });
  return { take: () => seen.splice(0) };
}

export async function checkCellOverflow(page) {
  return page.evaluate(scanCellOverflow, { tolerance: OVERFLOW_TOLERANCE_PX });
}

async function tooltipHolds(page, text) {
  const wanted = squash(text);
  const shown = await settle(page, { probe: tooltipTexts, until: holdsText, untilArg: wanted, ms: TOOLTIP_MS });
  return { holds: holdsText(shown, wanted), shown };
}

const ITEM_WAIT = { timeout: ITEM_WAIT_MS };

const isReachable = (locator) => locator.evaluate(isReached, undefined, ITEM_WAIT).catch(() => false);

const isFocusable = (locator) => locator.evaluate((el) => el.tabIndex >= 0, undefined, ITEM_WAIT).catch(() => false);

const isPresent = async (locator) => (await locator.count()) > 0;

async function hoverCentre(locator, page) {
  const hovered = await locator.hover({ timeout: ACTIONABLE_WAIT_MS }).then(
    () => true,
    () => false,
  );
  if (hovered) return true;
  const box = await locator.boundingBox(ITEM_WAIT).catch(() => null);
  if (!box) return false;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  return true;
}

const cutKey = (item) => `${item.element}|${item.text}`;

function placeCut(cut, fresh) {
  const ids = Map.groupBy(fresh, cutKey);
  const seen = new Map();
  for (const item of cut) {
    const key = cutKey(item);
    const nth = seen.get(key) ?? 0;
    seen.set(key, nth + 1);
    const placed = ids.get(key)?.[nth];
    item.id = placed?.id ?? null;
    item.titled = placed?.titled ?? item.titled;
  }
}

async function locateCut(page, cut, item) {
  const marked = () => page.locator(`[${MARK_CUT}="${item.id}"]`);
  if (item.id !== null && (await isPresent(marked()))) return marked();
  placeCut(cut, await page.evaluate(markCutText));
  return item.id === null ? null : marked();
}

async function checkCutItem(page, locator, text) {
  await page.mouse.move(0, 0);
  await Promise.all([
    settle(page, { probe: tooltipTexts, until: noneShown, ms: HOVER_RESET_MS }),
    locator.scrollIntoViewIfNeeded({ timeout: ACTIONABLE_WAIT_MS }).catch(() => undefined),
  ]);
  if (!(await isReachable(locator))) return null;
  let result = { holds: false, shown: [] };
  if (await hoverCentre(locator, page)) result = await tooltipHolds(page, text);
  if (!result.holds && (await isFocusable(locator))) {
    await locator.focus(ITEM_WAIT).catch(() => undefined);
    result = await tooltipHolds(page, text);
  }
  return result;
}

export async function checkCutText(page) {
  const cut = await page.evaluate(markCutText);
  const findings = [];
  const judged = new Set();
  for (const item of cut) {
    if (item.titled || judged.has(item.case)) continue;
    let result = null;
    for (let attempt = 0; attempt <= CUT_ATTEMPTS; attempt += 1) {
      const locator = await locateCut(page, cut, item);
      if (locator === null || item.titled) {
        result = null;
        break;
      }
      if (attempt === CUT_ATTEMPTS) break;
      result = await checkCutItem(page, locator, item.text);
      if (result?.holds || (await isPresent(locator))) break;
    }
    if (result === null) continue;
    judged.add(item.case);
    if (result.holds) continue;
    findings.push({
      element: item.element,
      detail:
        result.shown.length > 0
          ? 'text is cut with an ellipsis and its tooltip does not hold the full value'
          : 'text is cut with an ellipsis and there is no tooltip or title with the full value',
    });
  }
  await page.mouse.move(0, 0);
  return findings;
}

export async function checkClippedText(page) {
  return page.evaluate(scanClippedText, {
    tolerance: OVERFLOW_TOLERANCE_PX,
    lineShare: CLIPPED_LINE_SHARE,
  });
}

const PAGE_TOP = 0;

const scrollExtent = () => ({
  scrolled: globalThis.scrollY,
  last: Math.max(0, globalThis.document.documentElement.scrollHeight - globalThis.innerHeight),
});

const scanCoveredAt = (top, options) => `(globalThis.scrollTo(0, ${top}), ${call(scanCoveredText, options)})`;

const coveredScans = `(() => {
  const { scrolled, last } = (${scrollExtent})();
  if (scrolled === ${PAGE_TOP} && last === ${PAGE_TOP})
    return { atTop: ${call(scanCoveredText, { coveringPositions: COVERING_POSITIONS })}, here: [], atEnd: [], moved: false };
  const here = scrolled === ${PAGE_TOP} ? [] : ${call(scanCoveredText, { coveringPositions: ['sticky'] })};
  const atTop = ${scanCoveredAt(PAGE_TOP, { coveringPositions: COVERING_POSITIONS })};
  const atEnd = last === ${PAGE_TOP} ? [] : ${scanCoveredAt('last', { coveringPositions: ['fixed'], onlyStuck: true })};
  globalThis.scrollTo(0, scrolled);
  return { atTop, here, atEnd, moved: true };
})()`;

export async function checkCoveredText(page) {
  const { atTop, here, atEnd, moved } = await page.evaluate(coveredScans);
  if (!moved) return atTop;
  await settleLayout(page);
  const seen = new Set();
  const fresh = (finding) => {
    const key = `${finding.element}|${finding.detail}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  };
  return [...atTop, ...here, ...atEnd].filter(fresh);
}

export async function checkTabStopsInControls(page) {
  return page.evaluate(scanTabStopsInControls, { controls: CONTROLS_SELECTOR, focusable: FOCUSABLE_SELECTOR });
}

const touchTap = async (locator) => {
  await locator.dispatchEvent('pointerdown', { pointerType: 'touch', bubbles: true });
  await locator.dispatchEvent('click');
};

export async function checkTapExpansion(page, url) {
  const targets = await page.evaluate(markTapTargets, {
    markName: MARK_TAP,
    limit: MAX_TAP_TARGETS,
    controls: CONTROLS_SELECTOR,
  });
  const findings = [];
  for (const target of targets) {
    const locator = page.locator(`[${MARK_TAP}="${target.id}"]`);
    const probe = { markName: MARK_TAP, id: target.id };
    if (!(await locator.evaluate((el) => el.isConnected).catch(() => false))) continue;
    await locator.scrollIntoViewIfNeeded({ timeout: ACTION_TIMEOUT_MS }).catch(() => undefined);
    try {
      await touchTap(locator);
    } catch {
      continue;
    }
    const after = await settle(page, { probe: stillCut, probeArg: probe, until: isUncut, ms: TAP_MS });
    if (isUncut(after)) {
      const over = await page.evaluate(overrunAfterTap, { ...probe, tolerance: OVERFLOW_TOLERANCE_PX });
      if (over)
        findings.push({
          element: target.element,
          detail: `after a tap the text opens up and runs ${Math.round(over.past)}px out of ${over.surface}`,
        });
      await touchTap(locator).catch(() => undefined);
    }
    if (page.url() !== url) {
      await page.goto(url, { waitUntil: 'networkidle' });
      break;
    }
  }
  return findings;
}

const isOpen = (state) => state.triggerExpanded || state.open.some((overlay) => overlay.fresh);
const isShownInFull = (state) => {
  const fresh = state.open.filter((overlay) => overlay.fresh);
  return fresh.length > 0 && fresh.every((overlay) => overlay.opaque);
};
const isSettled = (state) => !(state.triggerExpanded || state.open.some((overlay) => overlay.fresh));

const settleOverlay = (page, state, until, ms) => settle(page, { probe: overlayState, probeArg: state, until, ms });

async function loadQuietly(page, url) {
  const inFlight = new Set();
  let traffic = 0;
  const started = (request) => {
    inFlight.add(request);
    traffic += 1;
  };
  const ended = (request) => {
    inFlight.delete(request);
    traffic += 1;
  };
  page.on('request', started);
  page.on('requestfinished', ended);
  page.on('requestfailed', ended);
  try {
    await page.goto(url, { waitUntil: 'load' });
    const idle = { reached: false, error: null };
    page.waitForLoadState('networkidle').then(
      () => {
        idle.reached = true;
      },
      (error) => {
        idle.error = error;
      },
    );
    while (!idle.reached) {
      if (idle.error) throw idle.error;
      const before = traffic;
      const settled = await settleLayout(page);
      if (settled && inFlight.size === 0 && traffic === before) return;
    }
  } finally {
    page.off('request', started);
    page.off('requestfinished', ended);
    page.off('requestfailed', ended);
  }
}

async function reopen(page, url, { probe, arg, count }) {
  await loadQuietly(page, url);
  await settle(page, { probe, probeArg: arg, until: atLeast, untilArg: count, ms: SETTLE_MS });
}

async function waitForAnotherUrl(page, url) {
  await page.waitForURL((now) => now.href !== url, { timeout: CLOSE_MS }).catch(onlyTimeout);
}

const markForPress = `(() => {
  const triggers = ${call(listTriggers, { markName: MARK_TRIGGER, limit: MAX_TRIGGERS })};
  const buttons = ${call(listOutsideButtons, { markName: MARK_PRESS, triggerMark: MARK_TRIGGER, limit: MAX_OUTSIDE_BUTTONS })};
  ${call(markOverlays, { markName: MARK_SEEN, shownName: MARK_SHOWN, openedName: MARK_OPENED })};
  return { triggers, buttons };
})()`;

export async function checkOverlays(page, url) {
  const findings = [];
  const triggerScan = { markName: MARK_TRIGGER, limit: MAX_TRIGGERS };
  const triggers = await page.evaluate(listTriggers, triggerScan);
  const rendered = { probe: listTriggers, arg: triggerScan, count: triggers.length };
  for (const trigger of triggers) {
    const presses = [{ name: 'a press outside it', neutral: true }];
    const buttons = await page.evaluate(listOutsideButtons, {
      markName: MARK_PRESS,
      triggerMark: MARK_TRIGGER,
      limit: MAX_OUTSIDE_BUTTONS,
    });
    for (const button of buttons)
      presses.push({ name: `a press on the button "${button.name}"`, id: button.id });
    for (const press of presses) {
      const marked = await page.evaluate(markForPress);
      if (!marked.triggers.some((shown) => shown.id === trigger.id)) break;
      if (!press.neutral && !marked.buttons.some((shown) => shown.id === press.id)) {
        await reopen(page, url, rendered);
        continue;
      }
      const opener = page.locator(`[${MARK_TRIGGER}="${trigger.id}"]`);
      try {
        await opener.click({ timeout: ACTION_TIMEOUT_MS });
      } catch {
        break;
      }
      const state = {
        markName: MARK_SEEN,
        shownName: MARK_SHOWN,
        openedName: MARK_OPENED,
        triggerMark: `${MARK_TRIGGER}="${trigger.id}"`,
      };
      const opened = await settleOverlay(page, state, isOpen, OPEN_MS);
      if (!isOpen(opened)) break;
      await settleOverlay(page, state, isShownInFull, OPEN_MS);
      await page.evaluate(markOpenedByTrigger, { markName: MARK_SEEN, openedName: MARK_OPENED });
      let pressed = true;
      if (press.neutral) {
        const point = await page.evaluate(neutralPoint);
        if (point) await page.mouse.click(point.x, point.y);
        else pressed = false;
      } else {
        try {
          await page.locator(`[${MARK_PRESS}="${press.id}"]`).click({ timeout: ACTION_TIMEOUT_MS });
        } catch {
          pressed = false;
        }
      }
      if (!pressed) {
        await reopen(page, url, rendered);
        continue;
      }
      await settleLayout(page);
      const after = await settleOverlay(page, state, isSettled, CLOSE_MS);
      const left = after.open.filter((overlay) => overlay.fresh && overlay.openedByTrigger);
      if (after.triggerExpanded || left.length > 0)
        findings.push({
          element: trigger.element,
          detail: `still open after ${press.name}${left[0] ? `: ${left[0].element}` : ''}`,
        });
      if (after.appeared.length > 0 || left.length > 0 || page.url() !== url) await reopen(page, url, rendered);
    }
  }
  return findings;
}

const moduleOf = (path) => path.split('/')[1] ?? '';
const staysInModule = (before, after) => moduleOf(before) !== '' && moduleOf(before) === moduleOf(after);

export async function checkRowClicks(page, url) {
  const findings = [];
  const rowScan = { markName: MARK_ROW, limit: MAX_ROW_TABLES };
  const rows = await page.evaluate(markClickableRows, rowScan);
  const rendered = { probe: markClickableRows, arg: rowScan, count: rows.length };
  const before = new URL(url).pathname;
  for (const row of rows) {
    await page.evaluate(markClickableRows, { markName: MARK_ROW, limit: MAX_ROW_TABLES });
    try {
      await page.locator(`[${MARK_ROW}="${row.id}"]`).click({ timeout: ACTION_TIMEOUT_MS });
    } catch {
      continue;
    }
    await waitForAnotherUrl(page, url);
    const after = new URL(page.url()).pathname;
    if (after !== before && !staysInModule(before, after))
      findings.push({
        element: row.element,
        detail: `a click on the row changed the page from ${before} to ${after}`,
      });
    if (page.url() !== url) await reopen(page, url, rendered);
  }
  return findings;
}

export async function checkPage(page, { screen, viewport = '', watch = null, only = CHECKS }) {
  const url = page.url();
  const found = [];
  const collect = (check, items) => {
    for (const item of items) found.push({ screen, viewport, check, ...item });
  };
  if (only.includes('overflow')) collect('overflow', await checkCellOverflow(page));
  if (only.includes('truncation')) {
    collect('truncation', await checkClippedText(page));
    collect('truncation', await checkCutText(page));
    collect('truncation', await checkTabStopsInControls(page));
    collect('truncation', await checkTapExpansion(page, url));
  }
  if (only.includes('covered')) collect('covered', await checkCoveredText(page));
  if (only.includes('overlay')) collect('overlay', await checkOverlays(page, url));
  if (only.includes('row-click')) collect('row-click', await checkRowClicks(page, url));
  if (only.includes('console') && watch) collect('console', watch.take());
  return found;
}
