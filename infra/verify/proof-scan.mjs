import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

import { checkCellOverflow, checkClippedText, checkCoveredText, checkCutText } from './interaction-checks.mjs';
import { CONTROLS_SELECTOR, markCutText } from './interaction-scan.mjs';
import { settle } from './proof-settle.mjs';

export const SCAN_CHECKS = ['overflow', 'cut', 'covered', 'contrast', 'token', 'target', 'image', 'text'];

const COLOR = /#[0-9a-f]{8}\b|#[0-9a-f]{6}\b|#[0-9a-f]{3}\b|rgba?\([^)]*\)/gi;
const SIZE_PATH = /fontSize|font-size|^TYPE$/;
const ROOT_PX = 16;
const LOADERS = '.mantine-LoadingOverlay-root';

function rgbOf(text) {
  const hex = /^#([0-9a-f]+)$/i.exec(text);
  if (hex) {
    const digits = hex[1].length === 3 ? [...hex[1]].map((digit) => digit + digit).join('') : hex[1].slice(0, 6);
    return [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16)).join(',');
  }
  const channels = /rgba?\(([^)]*)\)/i.exec(text)?.[1].split(/[\s,/]+/).filter(Boolean);
  return channels && channels.length >= 3 ? channels.slice(0, 3).map(Number).join(',') : null;
}

function pxOf(value) {
  if (typeof value === 'number') return value;
  const rem = /([\d.]+)rem/.exec(value);
  if (rem) return Number(rem[1]) * ROOT_PX;
  const px = /([\d.]+)px/.exec(value);
  return px ? Number(px[1]) : null;
}

export function tokensOf(values) {
  const colors = new Set();
  const sizes = new Set();
  const seen = new Set();
  const visit = (value, sized) => {
    if (typeof value === 'string') {
      for (const match of value.match(COLOR) ?? []) {
        const rgb = rgbOf(match);
        if (rgb) colors.add(rgb);
      }
      if (sized && pxOf(value) !== null) sizes.add(pxOf(value));
      return;
    }
    if (typeof value === 'number') {
      if (sized) sizes.add(value);
      return;
    }
    if (value === null || typeof value !== 'object' || seen.has(value)) return;
    seen.add(value);
    for (const [key, inner] of Object.entries(value)) visit(inner, sized || SIZE_PATH.test(key));
  };
  for (const [name, value] of Object.entries(values)) visit(value, SIZE_PATH.test(name));
  return { colors: [...colors].sort(), sizes: [...sizes].sort((a, b) => a - b) };
}

const pick = (namespace, path) => path.split('.').reduce((value, key) => value?.[key], namespace);

export async function loadTokens(root, sources) {
  const vite = await import(createRequire(join(root, 'apps/admin/package.json')).resolve('vite'));
  const values = {};
  const modules = {};
  for (const [source, ...paths] of sources) {
    const id = existsSync(join(root, source)) ? join(root, source) : source;
    const { module } = await vite.runnerImport(id, { root: join(root, 'apps/admin'), configFile: false, logLevel: 'silent' });
    modules[source] = module;
    if (paths.length === 0) values[source] = module;
    for (const path of paths) values[`${source}:${path}`] = pick(module, path);
  }
  return { ...tokensOf(values), modules };
}

export function scanStatic({ colors, sizes, target, phone, controls, scaled = null }) {
  const { document, innerWidth, innerHeight, NodeFilter } = globalThis;
  const styles = new Map();
  const getComputedStyle = (el) => {
    if (!styles.has(el)) styles.set(el, globalThis.getComputedStyle(el));
    return styles.get(el);
  };
  const FLOOR = 4.5;
  const LARGE_FLOOR = 3;
  const LARGE_PX = 24;
  const LARGE_BOLD_PX = 18.66;
  const BOLD = 700;
  const RAW = 'pre, code, kbd, samp';
  const TEXT = [
    [/\bundefined\b/, 'shows "undefined"'],
    [/\bNaN\b/, 'shows "NaN"'],
    [/\[object Object\]/, 'shows "[object Object]"'],
    [/lorem ipsum|\bTODO\b|\bFIXME\b|\bTBD\b|\{\{|\}\}/i, 'shows placeholder text'],
    [/\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b|\b[0-9a-f]{24,}\b/i, 'shows a raw id'],
    [/\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/, 'shows a raw ISO date'],
  ];
  const palette = new Set(colors);
  const scale = new Set(sizes.map((size) => Math.round(size * 100) / 100));
  const squash = (text) => (text ?? '').replace(/\s+/g, ' ').trim();
  const label = (el) => `${el.tagName.toLowerCase()} "${squash(el.getAttribute('aria-label') || el.textContent).slice(0, 40)}"`;
  const grouped = new Map();
  const add = (check, el, detail) => {
    const key = `${check}|${detail}`;
    const entry = grouped.get(key) ?? { check, element: typeof el === 'string' ? el : label(el), detail, more: 0 };
    if (grouped.has(key)) entry.more += 1;
    grouped.set(key, entry);
  };
  const page = { width: document.documentElement.scrollWidth, height: document.documentElement.scrollHeight };
  const shown = (el) => {
    const style = getComputedStyle(el);
    const box = el.getBoundingClientRect();
    const left = box.left + globalThis.scrollX;
    const top = box.top + globalThis.scrollY;
    const onPage = left + box.width > 0 && left < page.width && top + box.height > 0 && top < page.height;
    return style.display !== 'none' && style.visibility !== 'hidden' && box.width > 1 && box.height > 1 && onPage;
  };
  const parse = (value) => {
    const rgb = /^rgba?\(([^)]*)\)$/.exec(value);
    if (rgb) {
      const [r, g, b, a = 1] = rgb[1].split(/[\s,/]+/).filter(Boolean).map(Number);
      return { r, g, b, a };
    }
    const srgb = /^color\(srgb ([^)]*)\)$/.exec(value);
    if (!srgb) return null;
    const [r, g, b, a = 1] = srgb[1].split(/[\s/]+/).filter(Boolean).map(Number);
    return { r: r * 255, g: g * 255, b: b * 255, a };
  };
  const key = ({ r, g, b }) => [r, g, b].map(Math.round).join(',');
  const over = (top, bottom) => {
    const a = top.a + bottom.a * (1 - top.a);
    if (a === 0) return { r: 0, g: 0, b: 0, a: 0 };
    const mix = (channel) => (top[channel] * top.a + bottom[channel] * bottom.a * (1 - top.a)) / a;
    return { r: mix('r'), g: mix('g'), b: mix('b'), a };
  };
  const PAPER = { r: 255, g: 255, b: 255, a: 1 };
  const remembered = (compute) => {
    const known = new Map();
    const of = (el) => {
      if (!el) return undefined;
      if (!known.has(el)) known.set(el, compute(el, of));
      return known.get(el);
    };
    return of;
  };
  const opacity = remembered((el, of) => Number(getComputedStyle(el).opacity) * (of(el.parentElement) ?? 1));
  const ground = remembered((el, of) => {
    const style = getComputedStyle(el);
    if (style.backgroundImage !== 'none') return null;
    const color = parse(style.backgroundColor);
    if (color && color.a === 1) return color;
    const below = el.parentElement ? of(el.parentElement) : PAPER;
    if (below === null) return null;
    return color && color.a > 0 ? over(color, below) : below;
  });
  const luminance = ({ r, g, b }) => {
    const linear = (channel) => {
      const value = channel / 255;
      return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
  };
  const ratio = (a, b) => {
    const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
    return (light + 0.05) / (dark + 0.05);
  };
  const disabled = (el) => el.closest(':disabled, [aria-disabled="true"], [data-disabled]') !== null;
  const BLEND_SPREAD = 8;
  const BLEND_SLACK = 3;
  const ALPHA_SLACK = 0.03;
  const swatches = colors.map((rgb) => rgb.split(',').map(Number));
  const blendOf = (seen, top, bottom) => {
    const alphas = [];
    for (let channel = 0; channel < 3; channel += 1) {
      const spread = top[channel] - bottom[channel];
      if (Math.abs(spread) < BLEND_SPREAD) {
        if (Math.abs(seen[channel] - bottom[channel]) > BLEND_SLACK) return false;
      } else alphas.push((seen[channel] - bottom[channel]) / spread);
    }
    return alphas.length > 0 && alphas.every((alpha) => alpha > 0 && alpha < 1 && Math.abs(alpha - alphas[0]) <= ALPHA_SLACK);
  };
  const blended = remembered((rgb) => {
    const seen = rgb.split(',').map(Number);
    return swatches.some((top) => swatches.some((bottom) => top !== bottom && blendOf(seen, top, bottom)));
  });
  const offToken = (value) => {
    const color = parse(value);
    if (!color || color.a === 0 || palette.has(key(color))) return null;
    return blended(key(color)) ? null : key(color);
  };

  const texts = new Set();
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const parent = node.parentElement;
    if (!parent || !squash(node.textContent) || parent.closest('svg') || !shown(parent)) continue;
    texts.add(parent);
    if (parent.closest(RAW)) continue;
    for (const [pattern, detail] of TEXT) if (pattern.test(node.textContent)) add('text', parent, `${detail}: "${squash(node.textContent).slice(0, 60)}"`);
  }
  const scaledBy = scaled === null ? null : new RegExp(scaled);
  const CONTAINER_UNIT = /\d(cqw|cqh|cqi|cqb|cqmin|cqmax)\b/;
  const containerSized = (el) => {
    for (let at = el; at && at !== document.body; at = at.parentElement) if (at.style.fontSize) return CONTAINER_UNIT.test(at.style.fontSize);
    return false;
  };
  const scalesWithItsBox = (el) => containerSized(el) || (scaledBy !== null && scaledBy.test(el.closest('[data-testid]')?.getAttribute('data-testid') ?? ''));
  const paintOf = (node) => {
    const style = getComputedStyle(node);
    if (style.backgroundImage !== 'none') return null;
    return parse(style.backgroundColor);
  };
  const groundUnder = (el, x, y) => {
    const stack = document.elementsFromPoint(x, y);
    const start = stack.indexOf(el);
    if (start < 0) return undefined;
    const layers = [];
    for (const node of stack.slice(start)) {
      const color = paintOf(node);
      if (color === null) return null;
      if (!color || color.a === 0) continue;
      layers.push(color);
      if (color.a === 1) break;
    }
    return layers.reduceRight((below, layer) => over(layer, below), PAPER);
  };
  const readable = [];
  for (const el of texts) {
    const style = getComputedStyle(el);
    const size = parseFloat(style.fontSize);
    if (!scale.has(Math.round(size * 100) / 100) && !scalesWithItsBox(el)) add('token', el, `font size ${size}px is not a theme token`);
    const ink = offToken(style.color);
    if (ink) add('token', el, `text colour rgb(${ink}) is not a theme token`);
    if (disabled(el) || el.closest('[aria-hidden="true"]')) continue;
    const box = el.getBoundingClientRect();
    readable.push({ el, style, size, x: box.left + box.width / 2, y: box.top + globalThis.scrollY + box.height / 2 });
  }
  readable.sort((a, b) => a.y - b.y);
  const resting = { x: globalThis.scrollX, y: globalThis.scrollY };
  const unhittable = [...document.body.querySelectorAll('*')].filter((node) => getComputedStyle(node).pointerEvents === 'none').map((node) => [node, node.style.getPropertyValue('pointer-events'), node.style.getPropertyPriority('pointer-events')]);
  for (const [node] of unhittable) node.style.setProperty('pointer-events', 'auto', 'important');
  try {
    for (const { el, style, size, x, y } of readable) {
      if (y < globalThis.scrollY || y >= globalThis.scrollY + innerHeight) globalThis.scrollTo(globalThis.scrollX, Math.max(0, y - innerHeight / 2));
      const inView = x >= 0 && x < innerWidth;
      const under = inView ? groundUnder(el, x, y - globalThis.scrollY) : undefined;
      const back = under === undefined ? ground(el) : under;
      const color = parse(style.color);
      if (!color || !back) continue;
      const seen = over({ ...color, a: color.a * opacity(el) }, back);
      const floor = size >= LARGE_PX || (size >= LARGE_BOLD_PX && Number(style.fontWeight) >= BOLD) ? LARGE_FLOOR : FLOOR;
      const measured = ratio(seen, back);
      if (measured < floor) add('contrast', el, `text contrast ${measured.toFixed(2)}:1 is under ${floor}:1 (rgb(${key(seen)}) on rgb(${key(back)}))`);
    }
  } finally {
    for (const [node, value, priority] of unhittable) {
      if (value) node.style.setProperty('pointer-events', value, priority);
      else node.style.removeProperty('pointer-events');
    }
  }
  globalThis.scrollTo(resting.x, resting.y);
  for (const el of document.body.querySelectorAll('*')) {
    if (el.closest('svg') || !shown(el)) continue;
    const style = getComputedStyle(el);
    const fill = offToken(style.backgroundColor);
    if (fill) add('token', el, `background rgb(${fill}) is not a theme token`);
    if (style.borderTopStyle !== 'none' && parseFloat(style.borderTopWidth) > 0) {
      const edge = offToken(style.borderTopColor);
      if (edge) add('token', el, `border rgb(${edge}) is not a theme token`);
    }
  }
  for (const img of document.body.querySelectorAll('img')) {
    if (img.getAttribute('src') && img.complete && img.naturalWidth === 0) add('image', img, `the image ${img.getAttribute('src').slice(0, 80)} does not load`);
  }
  const FIELDS = 'input:not([type="hidden"]), select, textarea';
  const anyLayer = [...document.querySelectorAll('[role="dialog"], [aria-modal="true"]')].some(shown);
  const all = [...document.body.querySelectorAll(`${controls}, ${FIELDS}`)].filter(
    (el) => shown(el) && !disabled(el) && Number(getComputedStyle(el).opacity) !== 0 && getComputedStyle(el).pointerEvents !== 'none',
  );
  const painted = (el) => {
    const style = getComputedStyle(el);
    return (style.borderTopStyle !== 'none' && parseFloat(style.borderTopWidth) > 0) || (parse(style.backgroundColor)?.a ?? 0) > 0;
  };
  const wrapperOf = (el) => {
    if (!el.matches(FIELDS) || painted(el)) return null;
    for (let node = el.parentElement; node && node !== document.body; node = node.parentElement) {
      if (all.some((other) => other !== el && node.contains(other))) return null;
      if (painted(node)) return node;
    }
    return null;
  };
  const hits = new Map(
    all.map((el) => {
      const rects = [el, ...(el.labels ?? []), wrapperOf(el)].filter(Boolean).map((node) => node.getBoundingClientRect());
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      return [el, { left, top, right, bottom, width: right - left, height: bottom - top, x: (left + right) / 2, y: (top + bottom) / 2 }];
    }),
  );
  const small = (el) => {
    const hit = hits.get(el);
    const inline = el.matches('a') && getComputedStyle(el).display === 'inline' && squash(el.parentElement?.textContent).length > squash(el.textContent).length;
    return !inline && (hit.width < target || hit.height < target);
  };
  const SPACING = 24;
  const reach = SPACING / 2;
  const near = (hit, other) => Math.hypot(Math.max(other.left - hit.x, 0, hit.x - other.right), Math.max(other.top - hit.y, 0, hit.y - other.bottom)) < reach;
  const crowded = (el) => {
    const hit = hits.get(el);
    return all.some((other) => {
      if (other === el || other.contains(el) || el.contains(other)) return false;
      const theirs = hits.get(other);
      return near(hit, theirs) || (small(other) && Math.hypot(hit.x - theirs.x, hit.y - theirs.y) < SPACING);
    });
  };
  const scrolledAway = (el, x, y) => {
    for (let node = el.parentElement; node && node !== document.body && node !== document.documentElement; node = node.parentElement) {
      const style = getComputedStyle(node);
      if (style.overflowX === 'visible' && style.overflowY === 'visible') continue;
      const box = node.getBoundingClientRect();
      if (x < box.left || x > box.right || y < box.top || y > box.bottom) return true;
    }
    return false;
  };
  const pinnedOf = (node) => {
    for (let at = node; at && at !== document.body; at = at.parentElement) if (['fixed', 'sticky'].includes(getComputedStyle(at).position)) return at;
    return null;
  };
  const scrollerOf = (el) => {
    for (let node = el.parentElement; node && node !== document.body; node = node.parentElement) {
      const style = getComputedStyle(node);
      if (['auto', 'scroll'].includes(style.overflowY) && node.scrollHeight > node.clientHeight) return node;
    }
    return document.scrollingElement;
  };
  const scrolledUnder = (el, top) => {
    const bar = pinnedOf(top);
    if (bar === null || bar.contains(el)) return false;
    const scroller = scrollerOf(el);
    if (getComputedStyle(bar).position === 'sticky' && scrollerOf(bar) === scroller) return false;
    const pinned = bar.getBoundingClientRect();
    const view = scroller === document.scrollingElement ? { top: 0, bottom: innerHeight } : scroller.getBoundingClientRect();
    if (pinned.top + pinned.bottom < view.top + view.bottom) return scroller.scrollTop > 0;
    return scroller.scrollTop + scroller.clientHeight < scroller.scrollHeight - 1;
  };
  const OVERLAYS = '[role="dialog"], [aria-modal="true"], [role="tooltip"], [data-portal]';
  const OPENED = 'nav, [role="navigation"], [role="listbox"], [role="menu"]';
  const opened = (node) => node.matches(OPENED) && [...document.querySelectorAll('[aria-expanded="true"]')].some((control) => !node.contains(control));
  const layerOf = (node) => {
    for (let at = node; at && at !== document.body; at = at.parentElement) if (at.matches(OVERLAYS) || at.parentElement?.matches('[data-portal]') || opened(at)) return at;
    return null;
  };
  const underLayer = (el, top) => {
    const cover = layerOf(top);
    if (cover === null || cover.contains(el)) return false;
    const own = layerOf(el);
    return own === null || own.contains(cover) || Boolean(own.compareDocumentPosition(cover) & globalThis.Node.DOCUMENT_POSITION_FOLLOWING);
  };
  for (const el of all) {
    const box = el.getBoundingClientRect();
    if (phone && small(el) && crowded(el)) {
      const hit = hits.get(el);
      add('target', el, `the control is ${Math.round(hit.width)}x${Math.round(hit.height)}px, under the ${target}px touch size, and another target sits within ${reach}px of its centre`);
    }
    if (anyLayer && !el.closest('[role="dialog"], [aria-modal="true"]')) continue;
    const x = box.left + box.width / 2;
    const y = box.top + box.height / 2;
    if (x < 0 || y < 0 || x > innerWidth || y > innerHeight || scrolledAway(el, x, y)) continue;
    const top = document.elementFromPoint(x, y);
    if (!top || el.contains(top) || top.contains(el) || [...(el.labels ?? [])].some((labelled) => labelled.contains(top))) continue;
    if (underLayer(el, top) || scrolledUnder(el, top)) continue;
    add('covered', el, `its centre point belongs to ${label(top)}`);
  }
  if (phone && document.documentElement.scrollWidth > innerWidth + 1)
    add('overflow', 'page', `the page scrolls sideways: ${document.documentElement.scrollWidth}px of content in a ${innerWidth}px window`);
  return [...grouped.values()].map(({ more, ...finding }) => (more > 0 ? { ...finding, detail: `${finding.detail} (and ${more} more)` } : finding));
}

const tagged = (check) => (items) => items.map(({ element, detail }) => ({ check, element, detail }));

const MARK_UNSEEN = 'data-proof-unseen';

export function hideUnseen(mark) {
  const { document, getComputedStyle } = globalThis;
  const width = document.documentElement.scrollWidth;
  const height = document.documentElement.scrollHeight;
  let count = 0;
  for (const el of document.body.querySelectorAll('*')) {
    const box = el.getBoundingClientRect();
    if (box.width === 0 && box.height === 0) continue;
    const style = getComputedStyle(el);
    const left = box.left + globalThis.scrollX;
    const top = box.top + globalThis.scrollY;
    const offPage = box.width > 0 && box.height > 0 && (left + box.width <= 0 || left >= width || top + box.height <= 0 || top >= height);
    const clipped = style.clip === 'rect(0px, 0px, 0px, 0px)' || style.clipPath === 'inset(50%)' || ((box.width <= 1 || box.height <= 1) && style.overflow === 'hidden');
    if (!offPage && !clipped) continue;
    el.setAttribute(mark, `${el.style.getPropertyValue('visibility')}|${el.style.getPropertyPriority('visibility')}`);
    el.style.setProperty('visibility', 'hidden', 'important');
    count += 1;
  }
  return count;
}

export function showUnseen(mark) {
  for (const el of globalThis.document.querySelectorAll(`[${mark}]`)) {
    const [value, priority] = el.getAttribute(mark).split('|');
    if (value) el.style.setProperty('visibility', value, priority);
    else el.style.removeProperty('visibility');
    el.removeAttribute(mark);
  }
}

export async function scanPage(page, { tokens, target, phone, scaled = null }) {
  await settle(page, LOADERS);
  const at = await page.evaluate(() => ({ x: globalThis.scrollX, y: globalThis.scrollY }));
  await page.evaluate(hideUnseen, MARK_UNSEEN);
  try {
    const untitled = (await page.evaluate(markCutText)).some((item) => !item.titled);
    return [
      ...(await page.evaluate(scanStatic, { ...tokens, target, phone, scaled, controls: CONTROLS_SELECTOR })),
      ...tagged('overflow')(await checkCellOverflow(page)),
      ...tagged('cut')(await checkClippedText(page)),
      ...(untitled ? tagged('cut')(await checkCutText(page)) : []),
      ...tagged('covered')(await checkCoveredText(page)),
    ];
  } finally {
    await page.evaluate(showUnseen, MARK_UNSEEN);
    await page.evaluate(({ x, y }) => globalThis.scrollTo(x, y), at);
  }
}
