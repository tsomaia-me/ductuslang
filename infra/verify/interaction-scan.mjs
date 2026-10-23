export const OVERFLOW_TOLERANCE_PX = 1;

export function scanCellOverflow({ tolerance }) {
  const { document, getComputedStyle, NodeFilter } = globalThis;
  const CELLS =
    'td, th, [role="cell"], [role="gridcell"], [role="columnheader"], [role="rowheader"]';
  const clips = (style, axis) => style[axis] !== 'visible';

  const label = (el) => {
    const text = (el.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40);
    const role = el.getAttribute('role');
    return `${el.tagName.toLowerCase()}${role ? `[role=${role}]` : ''} "${text}"`;
  };

  const describeCell = (cell) => {
    const row = cell.closest('tr, [role="row"]');
    const table = cell.closest('table, [role="table"], [role="grid"]');
    const index = row ? [...row.children].indexOf(cell) : -1;
    const head = table?.querySelector('thead tr, [role="row"]');
    const heading = head && index >= 0 ? (head.children[index]?.textContent ?? '').trim() : '';
    const rowNumber = row && table ? [...table.querySelectorAll('tbody tr')].indexOf(row) + 1 : 0;
    const where = [heading && `column "${heading}"`, rowNumber > 0 && `row ${rowNumber}`]
      .filter(Boolean)
      .join(', ');
    return `${label(cell)}${where ? ` (${where})` : ''}`;
  };

  const visibleRect = (rect, start, cell) => {
    let { left, right, top, bottom } = rect;
    for (let a = start; a && a !== cell; a = a.parentElement) {
      const style = getComputedStyle(a);
      const box = a.getBoundingClientRect();
      if (clips(style, 'overflowX')) {
        left = Math.max(left, box.left);
        right = Math.min(right, box.right);
      }
      if (clips(style, 'overflowY')) {
        top = Math.max(top, box.top);
        bottom = Math.min(bottom, box.bottom);
      }
    }
    return { left, right, top, bottom };
  };

  const findings = [];
  for (const cell of document.querySelectorAll(CELLS)) {
    const box = cell.getBoundingClientRect();
    if (box.width === 0 || box.height === 0) continue;
    if (getComputedStyle(cell).display === 'none') continue;
    let worst = 0;
    let what = '';
    const consider = (rect, start, name) => {
      if (rect.width <= 1 || rect.height <= 1) return;
      const seen = visibleRect(rect, start, cell);
      const past = Math.max(
        seen.right - box.right,
        box.left - seen.left,
        seen.bottom - box.bottom,
        box.top - seen.top,
      );
      if (past > tolerance && past > worst) {
        worst = past;
        what = name;
      }
    };
    const style = getComputedStyle(cell);
    if (!clips(style, 'overflowX') && cell.scrollWidth > cell.clientWidth + tolerance) {
      const past = cell.scrollWidth - cell.clientWidth;
      if (past > worst) {
        worst = past;
        what = label(cell);
      }
    }
    for (const el of cell.querySelectorAll('*')) {
      if (getComputedStyle(el).visibility === 'hidden') continue;
      consider(el.getBoundingClientRect(), el.parentElement, label(el));
    }
    const walker = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (!node.textContent?.trim()) continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      for (const rect of range.getClientRects())
        consider(rect, node.parentElement, `text "${node.textContent.trim().slice(0, 40)}"`);
    }
    if (worst > tolerance)
      findings.push({
        element: describeCell(cell),
        detail: `${what} runs ${Math.round(worst)}px past the cell's box`,
      });
  }
  return findings;
}

export function scanClippedText({ tolerance, lineShare }) {
  const { document, getComputedStyle, NodeFilter } = globalThis;
  const CELLS =
    'td, th, [role="cell"], [role="gridcell"], [role="columnheader"], [role="rowheader"]';
  const SCROLLS = ['auto', 'scroll'];
  const CLIPS = ['hidden', 'clip'];
  const AXES = [
    {
      name: 'x',
      overflow: 'overflowX',
      size: 'clientWidth',
      edge: 'clientLeft',
      start: 'left',
      end: 'right',
      sides: ['left', 'right'],
    },
    {
      name: 'y',
      overflow: 'overflowY',
      size: 'clientHeight',
      edge: 'clientTop',
      start: 'top',
      end: 'bottom',
      sides: ['top', 'bottom'],
    },
  ];

  const squash = (text) => (text ?? '').replace(/\s+/g, ' ').trim();
  const label = (el) => `${el.tagName.toLowerCase()} "${squash(el.textContent).slice(0, 40)}"`;
  const where = (el) => {
    const cell = el.closest(CELLS);
    const row = cell?.closest('tr, [role="row"]');
    const table = cell?.closest('table, [role="table"], [role="grid"]');
    if (!cell || !row || !table) return '';
    const head = table.querySelector('thead tr, [role="row"]');
    const heading = squash(head?.children[[...row.children].indexOf(cell)]?.textContent);
    const rowNumber = [...table.querySelectorAll('tbody tr')].indexOf(row) + 1;
    const parts = [heading && `column "${heading}"`, rowNumber > 0 && `row ${rowNumber}`].filter(
      Boolean,
    );
    return parts.length > 0 ? ` (${parts.join(', ')})` : '';
  };
  const blockOf = (el) => {
    let node = el;
    while (node.parentElement && ['inline', 'contents'].includes(getComputedStyle(node).display))
      node = node.parentElement;
    return node;
  };

  const cutBy = (rect, start) => {
    const block = blockOf(start);
    const state = { absolute: false, clamped: false, done: { x: false, y: false } };
    for (let el = start; el && el !== document.body; el = el.parentElement) {
      const style = getComputedStyle(el);
      if (style.display === 'contents') continue;
      if (style.webkitLineClamp !== 'none' && style.webkitLineClamp !== '') state.clamped = true;
      const contained = !(state.absolute && style.position === 'static');
      if (contained && style.display !== 'inline') {
        const box = el.getBoundingClientRect();
        for (const axis of AXES) {
          if (state.done[axis.name]) continue;
          const mode = style[axis.overflow];
          if (SCROLLS.includes(mode)) state.done[axis.name] = true;
          if (!CLIPS.includes(mode)) continue;
          const size = el[axis.size];
          if (size <= 1) return null;
          const from = box[axis.start] + el[axis.edge];
          const to = from + size;
          const past = Math.max(rect[axis.end] - to, from - rect[axis.start]);
          const allowed =
            axis.name === 'y' ? Math.max(tolerance, lineShare * rect.height) : tolerance;
          if (past <= allowed) continue;
          const showsEllipsis =
            axis.name === 'x' && el === block && style.textOverflow === 'ellipsis';
          if (showsEllipsis || state.clamped) return null;
          const side =
            rect[axis.end] - to >= from - rect[axis.start] ? axis.sides[1] : axis.sides[0];
          return { el, past, side };
        }
      }
      if (style.position === 'fixed') return null;
      if (style.position === 'absolute') state.absolute = true;
      else if (style.position !== 'static') state.absolute = false;
    }
    return null;
  };

  const worst = new Map();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const parent = node.parentElement;
    if (!parent || !squash(node.textContent)) continue;
    if (getComputedStyle(parent).visibility === 'hidden' || parent.closest('[role="tooltip"]'))
      continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    for (const rect of range.getClientRects()) {
      if (rect.width <= 1 || rect.height <= 1) continue;
      const cut = cutBy(rect, parent);
      if (!cut || (worst.get(cut.el)?.past ?? 0) >= cut.past) continue;
      worst.set(cut.el, { ...cut, text: squash(node.textContent).slice(0, 40) });
    }
  }
  return [...worst.values()].map(({ el, past, side, text }) => ({
    element: `${label(el)}${where(el)}`,
    detail: `text "${text}" is cut ${Math.round(past)}px at its ${side} edge by its box, and no ellipsis or line clamp shows it is cut`,
  }));
}

export const CONTROLS_SELECTOR =
  'button, a[href], summary, [role="button"], [role="link"], [role="option"], [role="menuitem"], [role="tab"], [role="checkbox"], [role="switch"], [role="radio"]';

export const FOCUSABLE_SELECTOR =
  '[tabindex], input:not([type="hidden"]), select, textarea, a[href], button, [contenteditable]:not([contenteditable="false"])';

export function scanTabStopsInControls({ controls, focusable }) {
  const { document, getComputedStyle } = globalThis;
  const squash = (text) => (text ?? '').replace(/\s+/g, ' ').trim();
  const label = (el) =>
    `${el.tagName.toLowerCase()} "${squash(el.getAttribute('aria-label') ?? el.textContent).slice(0, 40)}"`;
  const found = [];
  for (const el of document.body.querySelectorAll(focusable)) {
    const tabbable = el.hasAttribute('tabindex') ? el.tabIndex >= 0 : el.tabIndex >= 0 || el.isContentEditable;
    if (!tabbable || el.matches(':disabled')) continue;
    const control = el.parentElement?.closest(controls);
    if (!control) continue;
    const box = el.getBoundingClientRect();
    if (box.width === 0 || box.height === 0 || getComputedStyle(el).visibility === 'hidden')
      continue;
    found.push({
      element: label(el),
      detail: `takes its own tab stop inside the control ${label(control)}, so the control is not the one place a keyboard or a tap lands`,
    });
  }
  return found;
}

export function markTapTargets({ markName, limit, controls }) {
  const { document, getComputedStyle } = globalThis;
  document.querySelectorAll(`[${markName}]`).forEach((el) => el.removeAttribute(markName));
  const seen = new Set();
  const found = [];
  for (const el of document.body.querySelectorAll('*')) {
    const style = getComputedStyle(el);
    const ellipsis = style.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1;
    const clamped =
      style.webkitLineClamp !== 'none' &&
      style.webkitLineClamp !== '' &&
      el.scrollHeight > el.clientHeight + 1;
    if (!(ellipsis || clamped)) continue;
    const box = el.getBoundingClientRect();
    if (box.width === 0 || box.height === 0 || style.visibility === 'hidden') continue;
    if (el.closest(`${controls}, [role="tooltip"]`)) continue;
    const up = (node, steps) => (steps === 0 ? node : (up(node?.parentElement, steps - 1) ?? null));
    const key = [0, 1, 2]
      .map((steps) => `${up(el, steps)?.tagName}.${up(el, steps)?.getAttribute('class')}`)
      .join('|');
    if (seen.has(key)) continue;
    seen.add(key);
    el.setAttribute(markName, String(found.length));
    found.push({
      id: found.length,
      element: `${el.tagName.toLowerCase()} "${(el.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40)}"`,
    });
    if (found.length === limit) break;
  }
  return found;
}

export function stillCut({ markName, id }) {
  const { document, getComputedStyle } = globalThis;
  const el = document.querySelector(`[${markName}="${id}"]`);
  if (!el) return { gone: true, cut: false };
  const style = getComputedStyle(el);
  const ellipsis = style.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1;
  const clamped =
    style.webkitLineClamp !== 'none' &&
    style.webkitLineClamp !== '' &&
    el.scrollHeight > el.clientHeight + 1;
  return { gone: false, cut: ellipsis || clamped };
}

export function overrunAfterTap({ markName, id, tolerance }) {
  const { document, getComputedStyle, NodeFilter } = globalThis;
  const el = document.querySelector(`[${markName}="${id}"]`);
  if (!el) return null;
  const painted = (node) => {
    const style = getComputedStyle(node);
    const clear = ['rgba(0, 0, 0, 0)', 'transparent'].includes(style.backgroundColor) || /\/ 0\)$/.test(style.backgroundColor);
    const filled = !clear;
    const framed = style.borderTopStyle !== 'none' && parseFloat(style.borderTopWidth) > 0;
    return filled || framed;
  };
  let surface = el;
  while (surface && surface !== document.body && !painted(surface)) surface = surface.parentElement;
  if (!surface || surface === document.body) return null;
  const box = surface.getBoundingClientRect();
  let past = 0;
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (!(node.textContent ?? '').trim()) continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    for (const rect of range.getClientRects())
      past = Math.max(past, rect.bottom - box.bottom, rect.right - box.right, box.left - rect.left, box.top - rect.top);
  }
  if (past <= tolerance) return null;
  return {
    past,
    surface: `${surface.tagName.toLowerCase()} "${(surface.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40)}"`,
  };
}

export function scanCoveredText({ coveringPositions = ['sticky', 'fixed'], onlyStuck = false } = {}) {
  const { document, getComputedStyle, NodeFilter, innerHeight, innerWidth } = globalThis;
  const DIALOGS = '[role="dialog"], [aria-modal="true"]';
  const CELLS =
    'td, th, [role="cell"], [role="gridcell"], [role="columnheader"], [role="rowheader"]';
  const squash = (text) => (text ?? '').replace(/\s+/g, ' ').trim();
  const dialogBeside = (cover) => {
    if (cover.querySelector(DIALOGS)) return true;
    const wrapper = cover.parentElement;
    if (!wrapper || wrapper === document.body) return false;
    return [...wrapper.children].some(
      (sibling) => sibling !== cover && (sibling.matches(DIALOGS) || sibling.querySelector(DIALOGS)),
    );
  };
  const OVERLAYS = '[role="dialog"], [aria-modal="true"], [role="tooltip"], [data-portal]';
  const OPENED = 'nav, [role="navigation"], [role="listbox"], [role="menu"]';
  const opened = (node) =>
    node.matches(OPENED) &&
    [...document.querySelectorAll('[aria-expanded="true"]')].some((control) => !node.contains(control));
  const layerOf = (node) => {
    for (let at = node; at && at !== document.body; at = at.parentElement)
      if (at.matches(OVERLAYS) || at.parentElement?.matches('[data-portal]') || opened(at)) return at;
    return null;
  };
  const underLayer = (el, top) => {
    const cover = layerOf(top);
    if (cover === null || cover.contains(el)) return false;
    const own = layerOf(el);
    return (
      own === null ||
      own.contains(cover) ||
      Boolean(own.compareDocumentPosition(cover) & globalThis.Node.DOCUMENT_POSITION_FOLLOWING)
    );
  };
  const pinnedOver = (el, positions = ['sticky', 'fixed']) => {
    for (let node = el; node && node !== document.body; node = node.parentElement)
      if (positions.includes(getComputedStyle(node).position)) return node;
    return null;
  };
  const where = (el) => {
    const cell = el.closest(CELLS);
    const row = cell?.closest('tr, [role="row"]');
    const table = cell?.closest('table, [role="table"], [role="grid"]');
    if (!cell || !row || !table) return '';
    const head = table.querySelector('thead tr, [role="row"]');
    const heading = squash(head?.children[[...row.children].indexOf(cell)]?.textContent);
    const rowNumber = [...table.querySelectorAll('tbody tr')].indexOf(row) + 1;
    const parts = [heading && `column "${heading}"`, rowNumber > 0 && `row ${rowNumber}`].filter(
      Boolean,
    );
    return parts.length > 0 ? ` (${parts.join(', ')})` : '';
  };
  const shown = (rect, start) => {
    let { left, right, top, bottom } = rect;
    for (let el = start; el && el !== document.body; el = el.parentElement) {
      const style = getComputedStyle(el);
      const box = el.getBoundingClientRect();
      if (style.overflowX !== 'visible') {
        left = Math.max(left, box.left);
        right = Math.min(right, box.right);
      }
      if (style.overflowY !== 'visible') {
        top = Math.max(top, box.top);
        bottom = Math.min(bottom, box.bottom);
      }
    }
    return { left, right, top, bottom, width: right - left, height: bottom - top };
  };
  const found = new Map();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const parent = node.parentElement;
    if (!parent || !squash(node.textContent) || getComputedStyle(parent).visibility === 'hidden')
      continue;
    if (pinnedOver(parent) || parent.closest('[role="tooltip"]')) continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    for (const whole of range.getClientRects()) {
      const rect = shown(whole, parent);
      if (rect.width <= 1 || rect.height <= 1) continue;
      const y = rect.top + rect.height / 2;
      for (const x of [rect.left + 2, rect.left + rect.width / 2, rect.right - 2]) {
        if (x < 0 || y < 0 || x > innerWidth || y > innerHeight) continue;
        const top = document.elementFromPoint(x, y);
        const cover = top && !parent.contains(top) ? pinnedOver(top, coveringPositions) : null;
        if (!cover || cover.contains(parent) || cover.closest(DIALOGS) || dialogBeside(cover) || underLayer(parent, top))
          continue;
        if (onlyStuck && cover.getBoundingClientRect().bottom + rect.height <= innerHeight) continue;
        found.set(parent, {
          text: squash(node.textContent).slice(0, 40),
          by: `${cover.tagName.toLowerCase()} "${squash(cover.textContent).slice(0, 20)}"`,
        });
      }
    }
  }
  return [...found].map(([el, { text, by }]) => ({
    element: `${el.tagName.toLowerCase()} "${squash(el.textContent).slice(0, 40)}"${where(el)}`,
    detail: `text "${text}" is covered by the pinned ${by} sitting over it`,
  }));
}

export function markCutText() {
  const { document, getComputedStyle, NodeFilter } = globalThis;
  const visible = (el) => {
    const style = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    return (
      style.display !== 'none' && style.visibility !== 'hidden' && box.width > 0 && box.height > 0
    );
  };
  const longestRun = (el) => {
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    let longest = '';
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const run = (node.textContent ?? '').replace(/\s+/g, ' ').trim();
      if (run.length > longest.length) longest = run;
    }
    return longest;
  };
  const label = (el) => {
    const all = (el.textContent ?? '').replace(/\s+/g, ' ').trim();
    return { text: longestRun(el), element: `${el.tagName.toLowerCase()} "${all.slice(0, 40)}"` };
  };
  const tables = [...document.querySelectorAll('table')];
  const lists = [];
  const shapeOf = (from, to) => {
    const shape = [];
    for (let node = from; node !== to.parentElement; node = node.parentElement)
      shape.push(`${node.tagName}.${node.getAttribute('class') ?? ''}`);
    return shape;
  };
  const repeated = (node) =>
    [...node.parentElement.children].some(
      (sibling) => sibling !== node && sibling.tagName === node.tagName && sibling.getAttribute('class') === node.getAttribute('class'),
    );
  const repeatedAbove = (node) => {
    for (let item = node.parentElement; item?.parentElement; item = item.parentElement) if (repeated(item)) return item;
    return null;
  };
  const listOf = (list) => {
    const item = repeatedAbove(list);
    if (item) return [listOf(item.parentElement), ...shapeOf(list, item)].join('>');
    if (!lists.includes(list)) lists.push(list);
    return `list ${lists.indexOf(list)}`;
  };
  const caseOf = (el, kind, id) => {
    const cell = el.closest('td, th');
    if (cell) {
      const column = [...cell.parentElement.cells].slice(0, cell.cellIndex).reduce((sum, before) => sum + before.colSpan, 0);
      return [kind, tables.indexOf(cell.closest('table')), column, ...shapeOf(el, cell)].join('|');
    }
    const item = repeatedAbove(el);
    if (!item) return `${kind}|${id}`;
    const list = item.parentElement;
    const row = repeatedAbove(list) ? [...list.children].indexOf(item) : '';
    return [kind, listOf(list), row, ...shapeOf(el, item)].join('|');
  };
  document.querySelectorAll('[data-interaction-cut]').forEach((el) => {
    el.removeAttribute('data-interaction-cut');
  });
  const cut = [];
  for (const el of document.body.querySelectorAll('*')) {
    const style = getComputedStyle(el);
    const ellipsis = style.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1;
    const clamped =
      style.webkitLineClamp !== 'none' &&
      style.webkitLineClamp !== '' &&
      el.scrollHeight > el.clientHeight + 1;
    if (!(ellipsis || clamped) || !visible(el)) continue;
    const { text, element } = label(el);
    if (!text) continue;
    let titled = false;
    for (let node = el; node && !titled; node = node.parentElement)
      titled = (node.getAttribute('title') ?? '').replace(/\s+/g, ' ').includes(text);
    el.setAttribute('data-interaction-cut', String(cut.length));
    cut.push({ id: cut.length, text, element, titled, case: caseOf(el, ellipsis ? 'ellipsis' : 'clamp', cut.length) });
  }
  return cut;
}

export function tooltipTexts() {
  const { document, getComputedStyle } = globalThis;
  return [...document.querySelectorAll('[role="tooltip"]')]
    .filter((el) => {
      const style = getComputedStyle(el);
      return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
    })
    .map((el) => (el.textContent ?? '').replace(/\s+/g, ' ').trim());
}

export function overlayState({ markName, shownName, openedName, triggerMark }) {
  const { document, getComputedStyle } = globalThis;
  const OVERLAYS =
    '[role="listbox"], [role="menu"], [role="dialog"]:not([aria-modal="true"]), [role="tooltip"]';
  const PANELS = '[role="listbox"], [role="menu"], [role="dialog"], [aria-modal="true"]';
  const rendered = (el) => {
    const style = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    return (
      style.display !== 'none' && style.visibility !== 'hidden' && box.width > 0 && box.height > 0
    );
  };
  const visible = (el) => rendered(el) && getComputedStyle(el).opacity !== '0';
  const label = (el) => {
    const text = (el.getAttribute('aria-label') ?? el.textContent ?? '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 40);
    const role = el.getAttribute('role');
    return `${el.tagName.toLowerCase()}${role ? `[role=${role}]` : ''} "${text}"`;
  };
  const open = [...document.querySelectorAll(OVERLAYS)]
    .filter((el) => el.getAttribute('role') !== 'tooltip' && visible(el))
    .map((el) => ({
      element: label(el),
      fresh: !el.hasAttribute(markName),
      openedByTrigger: el.hasAttribute(openedName),
      opaque: Number(getComputedStyle(el).opacity) >= 1,
    }));
  const appeared = [...document.querySelectorAll(PANELS)]
    .filter((el) => rendered(el) && !el.hasAttribute(shownName))
    .map((el) => ({ element: label(el) }));
  const trigger = document.querySelector(`[${triggerMark}]`);
  return {
    open,
    appeared,
    triggerExpanded: trigger?.getAttribute('aria-expanded') === 'true',
  };
}

export function markOverlays({ markName, shownName, openedName }) {
  const { document, getComputedStyle } = globalThis;
  const OVERLAYS =
    '[role="listbox"], [role="menu"], [role="dialog"]:not([aria-modal="true"]), [role="tooltip"]';
  const PANELS = '[role="listbox"], [role="menu"], [role="dialog"], [aria-modal="true"]';
  const rendered = (el) => {
    const style = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    return (
      style.display !== 'none' && style.visibility !== 'hidden' && box.width > 0 && box.height > 0
    );
  };
  for (const name of [markName, shownName, openedName])
    document.querySelectorAll(`[${name}]`).forEach((el) => el.removeAttribute(name));
  document.querySelectorAll(OVERLAYS).forEach((el) => el.setAttribute(markName, ''));
  document.querySelectorAll(PANELS).forEach((el) => {
    if (rendered(el)) el.setAttribute(shownName, '');
  });
}

export function markOpenedByTrigger({ markName, openedName }) {
  const { document, getComputedStyle } = globalThis;
  const OVERLAYS = '[role="listbox"], [role="menu"], [role="dialog"]:not([aria-modal="true"])';
  const rendered = (el) => {
    const style = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    return (
      style.display !== 'none' &&
      style.visibility !== 'hidden' &&
      style.opacity !== '0' &&
      box.width > 0 &&
      box.height > 0
    );
  };
  document.querySelectorAll(OVERLAYS).forEach((el) => {
    if (!el.hasAttribute(markName) && rendered(el)) el.setAttribute(openedName, '');
  });
}

export function listTriggers({ markName, limit }) {
  const { document, getComputedStyle } = globalThis;
  const TRIGGERS = '[aria-haspopup]:not([aria-haspopup="false"])';
  document.querySelectorAll(`[${markName}]`).forEach((el) => el.removeAttribute(markName));
  const seen = new Set();
  const found = [];
  for (const el of document.querySelectorAll(TRIGGERS)) {
    const box = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    if (box.width === 0 || box.height === 0 || style.visibility === 'hidden') continue;
    if (el.disabled || el.getAttribute('aria-disabled') === 'true') continue;
    if (el.getAttribute('aria-expanded') === 'true') continue;
    if (el.closest('nav, [role="dialog"], [role="listbox"], [role="menu"]')) continue;
    const name = (el.getAttribute('aria-label') ?? el.textContent ?? '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 40);
    const key = [el.tagName, el.getAttribute('aria-haspopup'), name.replace(/[:].*$/, '')].join(
      '|',
    );
    if (seen.has(key)) continue;
    seen.add(key);
    el.setAttribute(markName, String(found.length));
    found.push({
      id: found.length,
      element: `${el.tagName.toLowerCase()} "${name}"`,
    });
    if (found.length === limit) break;
  }
  return found;
}

export function listOutsideButtons({ markName, triggerMark, limit }) {
  const { document, getComputedStyle, innerHeight, innerWidth } = globalThis;
  const INSIDE =
    '[role="dialog"], [role="listbox"], [role="menu"], [role="tooltip"], [aria-modal="true"]';
  document.querySelectorAll(`[${markName}]`).forEach((el) => el.removeAttribute(markName));
  const found = [];
  for (const el of document.querySelectorAll('button, [role="button"]')) {
    const box = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    if (box.width === 0 || box.height === 0 || style.visibility === 'hidden') continue;
    if (el.disabled || el.getAttribute('aria-disabled') === 'true') continue;
    if (el.hasAttribute(triggerMark) || el.hasAttribute('aria-haspopup')) continue;
    if (el.hasAttribute('aria-expanded')) continue;
    if (el.closest(INSIDE) || el.closest('nav')) continue;
    if (el.getAttribute('type') === 'submit') continue;
    const inViewport =
      box.bottom > 0 && box.top < innerHeight && box.right > 0 && box.left < innerWidth;
    if (!inViewport) continue;
    const name = (el.getAttribute('aria-label') ?? el.textContent ?? '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 40);
    el.setAttribute(markName, String(found.length));
    found.push({ id: found.length, name });
    if (found.length === limit) break;
  }
  return found;
}

export function neutralPoint() {
  const { document, getComputedStyle, innerHeight, innerWidth } = globalThis;
  const INTERACTIVE =
    'a, button, input, select, textarea, label, summary, [role], [tabindex], [contenteditable], [data-interaction-keep]';
  const step = 24;
  for (let y = innerHeight - step; y > 0; y -= step)
    for (let x = innerWidth - step; x > 0; x -= step) {
      const el = document.elementFromPoint(x, y);
      if (!el || el === document.documentElement) continue;
      if (el.closest(INTERACTIVE)) continue;
      if (getComputedStyle(el).cursor === 'pointer') continue;
      return { x, y };
    }
  return null;
}

export function markClickableRows({ markName, limit }) {
  const { document, getComputedStyle } = globalThis;
  document.querySelectorAll(`[${markName}]`).forEach((el) => el.removeAttribute(markName));
  const found = [];
  for (const table of document.querySelectorAll('table, [role="table"], [role="grid"]')) {
    if (found.length === limit) break;
    const box = table.getBoundingClientRect();
    if (box.width === 0 || box.height === 0) continue;
    const rows = table.querySelectorAll('tbody tr, [role="row"]:not(:first-child)');
    for (const row of rows) {
      if (getComputedStyle(row).cursor !== 'pointer') continue;
      const cell = [...row.children].find(
        (c) => !c.querySelector('a, button, input, select, textarea, [role="button"]'),
      );
      if (!cell) continue;
      const rect = cell.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      cell.setAttribute(markName, String(found.length));
      found.push({
        id: found.length,
        element: `${row.tagName.toLowerCase()} "${(row.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 40)}"`,
      });
      break;
    }
  }
  return found;
}

export function isReached(el) {
  const { document } = globalThis;
  const box = el.getBoundingClientRect();
  const top = document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2);
  return top !== null && el.contains(top);
}

export function layoutHasSettled({ markName, quietPolls }) {
  const { document } = globalThis;
  const html = document.documentElement;
  const signature = `${document.fonts.status}|${html.scrollWidth}|${html.scrollHeight}|${document.getElementsByTagName('*').length}`;
  const before = globalThis[markName] ?? { signature: '', quiet: 0 };
  const quiet = before.signature === signature ? before.quiet + 1 : 0;
  globalThis[markName] = { signature, quiet };
  return quiet >= quietPolls && document.fonts.status === 'loaded';
}
