import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { changeSummary, cropAround } from '../../apps/app/scripts/baseline-lib.mjs';
import { changedSince } from '../scripts/check-touched.mjs';

import { PROOF_BRANCH, proofSlugOf } from './proof-branch.mjs';
import { loadTokens, scanPage, SCAN_CHECKS } from './proof-scan.mjs';
import { runQuietly } from './proof-serve.mjs';
import { REPO_ROOT, catalogue, placeOnSurface } from './screens.mjs';
import { MODULE_ROUTES, ownersOf, scopeOptions, screensFor } from './shot-scope.mjs';

export const WIDTHS = [
  { width: 1440, height: 900, label: 'desktop' },
  { width: 390, height: 844, label: 'phone' },
];
export const ADAPTERS = { app: 'apps/app/scripts/proof-adapter.mjs', admin: 'apps/admin/scripts/proof-adapter.mjs' };
const PACKAGES = { app: '@wg/app', admin: '@wg/admin' };
const DEFAULT_CONTEXTS = 6;
const SHOT_CAP_MS = 10_000;
const HASH_CHARS = 12;
const OUT_DIR = '.proof';
const SHOTS_DIR = 'shots';
const BASE_DIR = 'base';
const PAGE_FILE = 'README.md';
const PAGE_DATA = 'page.json';
const SIGNAL_EXIT = { SIGINT: 130, SIGTERM: 143 };
const VIA = 'via ';
const MAIN = 'origin/main';
const NEW = 'new';
const CATALOGUE = 'catalogue';
const FINDINGS_FILE = 'findings.json';
const NOISE_PIXELS = 16;
const CROP_MARGIN = 48;
const USAGE = 'usage: pnpm proof [--base <sha>] [--head <sha>] [--since <sha>] [--catalogue] [--out <dir>] [--contexts <n>] [--round <n>]';

const git = (root, ...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
const lines = (text) => text.split('\n').filter(Boolean);
const familyOfPath = (id) => id.split('/')[1] || 'home';

export function changedFiles(root, base, head, filter = []) {
  if (head) return lines(git(root, 'diff', '--name-only', ...filter, base, head)).sort();
  const tracked = lines(git(root, 'diff', '--name-only', ...filter, base));
  const untracked = lines(git(root, 'ls-files', '--others', '--exclude-standard'));
  return [...new Set([...tracked, ...untracked])].sort();
}

export const addedFiles = (root, base, head) => changedFiles(root, base, head, ['--diff-filter=A']);

export function appSet(changed, { root = REPO_ROOT, graph, added = [] } = {}) {
  if (changed.length === 0) return [];
  const placed = placeOnSurface('app', changed, { root, graph });
  const { screens } = catalogue('app', root);
  const everything = placed.unplaceable.length > 0;
  const ids = everything ? [...screens.keys()].sort() : placed.screens;
  return ids.map((id) => {
    const files = screens.get(id) ?? [];
    const own = (list) => list.some((path) => files.includes(path));
    const reachers = [...new Set([...(placed.by[id] ?? []), ...placed.unplaceable])];
    const reason = own(added) ? NEW : own(changed) ? 'changed' : `${VIA}${reachers[0]}`;
    return { surface: 'app', id, files, family: familyOfPath(id), reason, reachers };
  });
}

function routePattern(route) {
  const escaped = route.split('/').map((part) => (part.startsWith('$') ? '[^/]+' : part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  return new RegExp(`^${escaped.join('/')}$`);
}

export function ownComponent(path, routes) {
  const pathname = path.split(/[?#]/)[0];
  const matches = [...routes].filter(([route]) => routePattern(route).test(pathname));
  const params = (route) => route.split('/').filter((part) => part.startsWith('$')).length;
  const [, files] = matches.sort(([a], [b]) => params(a) - params(b))[0] ?? [null, []];
  return files[0] ?? null;
}

export function adminSet(changed, screens, walks, { root = REPO_ROOT, scope, map = MODULE_ROUTES, added = [] } = {}) {
  if (changed.length === 0) return [];
  const { screens: routes } = catalogue('admin', root);
  const options = { ...(scope ?? scopeOptions(root)), map };
  const reacher = new Map();
  for (const file of changed) for (const name of screensFor([file], screens, options).screens) reacher.set(name, [...(reacher.get(name) ?? []), file]);
  const reasonOf = (screen) => {
    const own = ownComponent(screen.path, routes);
    if (own !== null && added.includes(own)) return NEW;
    return own !== null && changed.includes(own) ? 'changed' : `${VIA}${reacher.get(screen.name)[0]}`;
  };
  const record = (screen, reason) => ({ surface: 'admin', id: screen.name, path: screen.path, family: ownersOf(screen.path, map)[0], reason, reachers: reacher.get(screen.name) });
  const reached = screens.filter(({ name }) => reacher.has(name)).map((screen) => record(screen, reasonOf(screen)));
  const walked = walks.flatMap((walk) => {
    const trigger = reached.find(({ id }) => id.startsWith(walk.prefix));
    return trigger ? [{ surface: 'admin', id: walk.name, path: null, family: trigger.family, reason: trigger.reason, reachers: trigger.reachers }] : [];
  });
  return [...reached, ...walked];
}

export function jobsFor(set, adapters) {
  const jobs = [];
  for (const screen of set) {
    const adapter = adapters[screen.surface];
    for (const size of WIDTHS) for (const theme of adapter.themes) for (const state of adapter.states(screen, size)) jobs.push({ screen, ...size, theme, state });
  }
  for (const [surface, adapter] of Object.entries(adapters)) {
    const first = set.find((screen) => screen.surface === surface);
    if (adapter.sample && first) jobs.push({ screen: first, ...WIDTHS[0], theme: adapter.sample, state: 'default' });
  }
  const rank = (job) => (job.state === 'default' ? 0 : 1);
  return jobs.sort((a, b) => rank(a) - rank(b));
}

const slug = (text) => text.replace(/[^A-Za-z0-9]+/g, '-').replace(/^-|-$/g, '').toLowerCase() || 'home';
const shotFile = (job, state) => `${job.screen.surface}-${slug(job.screen.id)}-${job.width}-${job.theme}-${slug(state)}.png`;

function capped(cap) {
  let timer;
  let fail;
  const expired = new Promise((_, reject) => {
    fail = reject;
  });
  const arm = () => {
    clearTimeout(timer);
    timer = setTimeout(() => fail(new Error(`no shot within ${cap} ms`)), cap);
  };
  return { arm, race: (work) => (arm(), Promise.race([work, expired]).finally(() => clearTimeout(timer))) };
}

export function contextMaker(browser, origin, cached) {
  const answers = new Map();
  const answer = (route) => {
    const url = route.request().url();
    if (!answers.has(url))
      answers.set(
        url,
        route.fetch().then(async (response) => ({ status: response.status(), headers: response.headers(), body: await response.body() })),
      );
    return answers.get(url).then(
      (cached) => route.fulfill(cached),
      () => {
        answers.delete(url);
        return route.continue();
      },
    );
  };
  return async (options) => {
    const context = await browser.newContext(options);
    if (cached) await context.route((url) => url.origin === origin, (route) => (route.request().method() === 'GET' ? answer(route) : route.continue()));
    return context;
  };
}

export async function runJob(newContext, runner, job, shotsDir, scan, cap = SHOT_CAP_MS) {
  const shots = [];
  const tally = { console: 0, requests: 0 };
  const taken = { console: 0, requests: 0 };
  let mark = Date.now();
  let context = null;
  const record = (state, fields) => {
    shots.push({ job, state, console: tally.console - taken.console, requests: tally.requests - taken.requests, ms: Date.now() - mark, ...fields });
    Object.assign(taken, tally);
    mark = Date.now();
  };
  const clock = capped(cap);
  try {
    context = await newContext(runner.contextOptions(job));
    context.setDefaultTimeout(cap);
    await runner.prepare(context, job);
    const page = await context.newPage();
    page.on('console', (message) => message.type() === 'error' && (tally.console += 1));
    page.on('pageerror', () => (tally.console += 1));
    page.on('requestfailed', () => (tally.requests += 1));
    page.on('response', (response) => response.status() >= 400 && (tally.requests += 1));
    const shoot = async (state, how) => {
      const found = scan ? await scan(page, job) : [];
      const buffer = await runner.capture(page, job, how);
      const file = shotFile(job, state);
      writeFileSync(join(shotsDir, file), buffer);
      record(state, { file, bytes: buffer.length, hash: createHash('sha256').update(buffer).digest('hex').slice(0, HASH_CHARS), scan: found });
      clock.arm();
    };
    await clock.race(runner.show(page, job, shoot));
  } catch (error) {
    record(job.state, { file: null, bytes: 0, hash: null, error: String(error.message ?? error).split('\n')[0] });
  } finally {
    await context?.close().catch(() => {});
  }
  return shots;
}

export async function pool(items, size, work) {
  const results = new Array(items.length);
  let next = 0;
  const lane = async () => {
    while (next < items.length) {
      const index = next;
      next += 1;
      results[index] = await work(items[index]);
    }
  };
  await Promise.all(Array.from({ length: Math.min(size, items.length) }, lane));
  return results.flat();
}

export function dropAbsentStates(shots) {
  const defaults = new Map(shots.filter((shot) => shot.state === 'default' && shot.hash).map((shot) => [key(shot), shot.hash]));
  const absent = (shot) => shot.job.state === shot.state && shot.state !== 'default' && shot.hash !== null && defaults.get(key(shot)) === shot.hash;
  return { kept: shots.filter((shot) => !absent(shot)), dropped: shots.filter(absent) };
}

export const baseJobsOf = (jobs) => jobs.filter((job) => job.screen.reason.startsWith(VIA));

export const isNoise = (difference) => difference.sizes === null && difference.changed <= NOISE_PIXELS;

export function compareToBase(shots, baseShots, measure = null) {
  const byFile = new Map(baseShots.filter((shot) => !broken(shot)).map((shot) => [shotFile(shot.job, shot.state), shot]));
  const failedJob = new Map(baseShots.filter(broken).map((shot) => [shot.job, shot]));
  return shots.map((shot) => {
    if (!shot.job.screen.reason.startsWith(VIA) || broken(shot)) return shot;
    const before = byFile.get(shotFile(shot.job, shot.state)) ?? failedJob.get(shot.job) ?? null;
    if (before?.hash === shot.hash) {
      const { scan: _onMain, ...unscanned } = shot;
      return { ...unscanned, mark: 'same', before };
    }
    const difference = measure && before && !broken(before) ? measure(shot, before) : null;
    return { ...shot, mark: difference && isNoise(difference) ? 'noise' : 'differs', before, difference };
  });
}

export const JUDGED_VIEW = { app: { widths: [390, 1440], theme: 'light' }, admin: { widths: [1440], theme: 'dark' } };

export function judgedSet(shots) {
  const viewOf = ({ job }) => JUDGED_VIEW[job.screen.surface];
  const atView = (shot) => viewOf(shot).widths.includes(shot.job.width) && shot.job.theme === viewOf(shot).theme;
  const ownScreens = new Map();
  for (const shot of shots.filter((one) => (one.job.screen.reason === NEW || one.job.screen.reason === 'changed') && one.file !== null && one.job.theme === viewOf(one).theme))
    ownScreens.set(shot.job.screen, [...(ownScreens.get(shot.job.screen) ?? []), shot]);
  const judged = new Set();
  for (const own of ownScreens.values()) {
    const state = own.some((shot) => shot.state === 'default') ? 'default' : own[0].state;
    const stated = own.filter((shot) => shot.state === state);
    const atWidth = stated.filter((shot) => viewOf(shot).widths.includes(shot.job.width));
    for (const shot of atWidth.length > 0 ? atWidth : stated) judged.add(shot);
  }
  const changedPixels = (shot) => shot.difference?.changed ?? 0;
  const largest = new Map();
  for (const shot of shots.filter((one) => one.mark === 'differs' && one.file !== null && atView(one)))
    for (const file of shot.job.screen.reachers) if (!largest.has(file) || changedPixels(shot) > changedPixels(largest.get(file))) largest.set(file, shot);
  for (const shot of largest.values()) judged.add(shot);
  return shots.map((shot) => ({ ...shot, judged: judged.has(shot) }));
}

export const cropPairs = (shots, dirs) =>
  shots.map((shot) => {
    const box = shot.difference?.box;
    if (!shot.judged || shot.mark !== 'differs' || !box || shot.difference.sizes !== null || !shot.before?.file) return shot;
    let crop;
    for (const [dir, file] of [[dirs.head, shot.file], [dirs.base, shot.before.file]]) {
      const cut = cropAround(readFileSync(join(dir, file)), box, CROP_MARGIN);
      writeFileSync(join(dir, file), cut.buffer);
      crop = cut.box;
    }
    return { ...shot, crop };
  });

const key = (shot) => `${shot.job.screen.surface}:${shot.job.screen.id}:${shot.job.width}:${shot.job.theme}`;
const screenName = (screen) => `${screen.surface}:${screen.id}`;
const broken = (shot) => shot.error !== undefined;
export const isFail = (shot) => broken(shot) || (shot.scan?.length ?? 0) > 0;
const describeDifference = ({ changed, box, sizes }) => `diff=${changed}px${box ? `@${box.x},${box.y},${box.w}x${box.h}` : ''}${sizes ? `,size:${sizes.replace(' → ', '->')}` : ''}`;
export const describeFinding = ({ check, element, detail }) => `${check} ${element}: ${detail}`;

export const failures = (shots) =>
  shots.filter(isFail).map((shot) => ({ surface: shot.job.screen.surface, screen: shot.job.screen.id, width: shot.job.width, theme: shot.job.theme, state: shot.state, error: shot.error ?? null, scan: shot.scan ?? [] }));

export function summaryLine(shot, same = null) {
  const fields = [
    isFail(shot) ? 'FAIL' : 'OK',
    screenName(shot.job.screen),
    shot.job.width,
    shot.job.theme,
    JSON.stringify(shot.state),
    `console=${shot.console}`,
    `requests=${shot.requests}`,
    `bytes=${shot.bytes}`,
    `hash=${shot.hash ?? '-'}`,
    `ms=${shot.ms}`,
  ];
  if (shot.mark === 'same') fields.push('same=base');
  else if (same) fields.push(`same=${same}`);
  if (shot.mark === 'differs' || shot.mark === 'noise') fields.push(shot.mark);
  if (shot.difference) fields.push(describeDifference(shot.difference));
  if (shot.judged) fields.push('judged');
  if (broken(shot)) fields.push(`error=${JSON.stringify(shot.error)}`);
  if (shot.scan?.length > 0) fields.push(`scan=${JSON.stringify(shot.scan.map(describeFinding).join(' | '))}`);
  return fields.join(' ');
}

const marked = (shots, mark) => shots.filter((shot) => shot.mark === mark).length;

export function summary(shots, { dropped = 0, base = [], seconds, page }) {
  const seen = new Map();
  const body = shots.map((shot) => {
    const same = shot.hash ? (seen.get(shot.hash) ?? null) : null;
    if (shot.hash && !seen.has(shot.hash)) seen.set(shot.hash, shot.file);
    return summaryLine(shot, same);
  });
  const fails = shots.filter(isFail).length;
  const judged = shots.filter((shot) => shot.judged).length;
  const counts = `${shots.length} shots, ${shots.length - fails} OK, ${fails} FAIL, ${dropped} states the screens do not have, ${marked(shots, 'same')} same=base, ${marked(shots, 'differs')} differs, ${marked(shots, 'noise')} noise, ${judged} judged`;
  const findings = shots.flatMap((shot) => shot.scan ?? []);
  const byCheck = SCAN_CHECKS.map((check) => [check, findings.filter((finding) => finding.check === check).length]).filter(([, count]) => count > 0);
  const scanned = byCheck.length === 0 ? 'none' : byCheck.map(([check, count]) => `${count} ${check}`).join(', ');
  return [...body, `proof: ${counts}; scan: ${scanned}; base: ${base.length} shots, ${base.filter(broken).length} FAIL; ${seconds.toFixed(1)} s; page ${page}`].join('\n');
}

const escapeHtml = (text) => String(text).replace(/[&<>"|]/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', '|': '&#124;' })[char]);
const escapeMarkdown = (text) => escapeHtml(text).replace(/[\\`*_[\]]/g, '\\$&');

export const caption = (shot, round) => [screenName(shot.job.screen), shot.job.width, shot.job.theme, shot.state, `round ${round}`].join(' · ');

export function pageModel({ head, base, dirty, round, fixtures, set, shots }) {
  const label = (shot, prefix) => {
    const text = `${shot.judged ? 'judged · ' : ''}${prefix}${caption(shot, round)}`;
    const why = [...(broken(shot) ? [shot.error] : []), ...(shot.scan ?? []).map(describeFinding)].join(' | ');
    return isFail(shot) ? `FAIL: ${why} · ${text}` : text;
  };
  const image = (folder, shot, text) => (shot?.file ? `<img src="${folder}/${shot.file}" alt="${escapeHtml(text)}">` : '');
  const atBase = `base ${base.slice(0, 7)} · `;
  const card = (shot) => {
    if (shot.mark !== 'differs') {
      const text = label(shot, '');
      return `**${escapeMarkdown(text)}**\n\n${image(SHOTS_DIR, shot, text)}\n`;
    }
    const cropped = shot.crop ? ` · crop ${shot.crop.x},${shot.crop.y},${shot.crop.w}x${shot.crop.h}` : '';
    const after = `${label(shot, 'differs · ')}${cropped}`;
    const before = `${shot.before ? label(shot.before, atBase) : `${atBase}no shot`}${cropped}`;
    return `| ${escapeMarkdown(after)} | ${escapeMarkdown(before)} |\n| --- | --- |\n| ${image(SHOTS_DIR, shot, after)} | ${image(BASE_DIR, shot.before, before)} |\n`;
  };
  const filesOf = (shot) => [`${SHOTS_DIR}/${shot.file}`, ...(shot.mark === 'differs' && shot.before?.file ? [`${BASE_DIR}/${shot.before.file}`] : [])];
  const asText = (shot) => [screenName(shot.job.screen), shot.job.width, shot.job.theme, shot.state, shot.difference ? describeDifference(shot.difference) : 'not measured'].join(' · ');
  const screens = set.map((screen) => {
    const own = shots.filter((shot) => shot.job.screen === screen);
    const shown = own.filter((shot) => shot.judged);
    const compared = own.filter((shot) => shot.mark);
    const counts = compared.length > 0 ? `; ${marked(compared, 'same')} same=base, ${marked(compared, 'differs')} differs, ${marked(compared, 'noise')} noise` : '';
    const unjudged = own.filter((shot) => shot.mark === 'differs' && !shot.judged).map(asText);
    const via = screen.reason.startsWith(VIA) ? screen.reason.slice(VIA.length) : null;
    const same = own.length > 0 && own.every((shot) => shot.mark === 'same');
    return { name: screenName(screen), family: `${screen.surface} · ${screen.family ?? 'other'}`, reason: `${screen.reason}${counts}`, via, same, round, cards: shown.map(card), unjudged, files: shown.flatMap(filesOf) };
  });
  const judged = shots.filter((shot) => shot.judged).length;
  const total = `${shots.length}, ${shots.filter(isFail).length} failed, ${marked(shots, 'same')} same as the base and ${marked(shots, 'noise')} noise, ${marked(shots, 'differs')} differ from the base, ${judged} judged and shown`;
  return { head, base, dirty, round, fixtures, shots: total, screens, sameAsBase: sameAsBaseOf(screens) };
}

const sameAsBaseOf = (screens) => {
  const counts = {};
  for (const { via, same } of screens) if (same) counts[via] = (counts[via] ?? 0) + 1;
  return counts;
};

export const pageFiles = (model) => model.screens.flatMap((screen) => screen.files);

export const carryOver = (previous, current) => {
  const shot = new Set(current.screens.map(({ name }) => name));
  const screens = [...current.screens, ...previous.screens.filter(({ name }) => !shot.has(name))];
  return { ...current, screens, sameAsBase: sameAsBaseOf(screens) };
};

export function renderPage({ head, base, dirty, round, fixtures, shots, screens, sameAsBase }) {
  const families = new Map();
  for (const screen of screens) if (screen.cards.length > 0) families.set(screen.family, [...(families.get(screen.family) ?? []), ...screen.cards]);
  const unjudged = screens.flatMap((screen) => screen.unjudged);
  const facts = [
    ['Head', `${head}${dirty ? ' with uncommitted changes' : ''}`],
    ['Base', base],
    ['Round', round],
    ...Object.entries(fixtures).map(([surface, text]) => [`Fixture, ${surface}`, text]),
    ['Shots', shots],
  ];
  return [
    `# Proof round ${round} at ${head.slice(0, 7)}`,
    facts.map(([name, value]) => `- **${escapeMarkdown(name)}**: ${escapeMarkdown(value)}`).join('\n'),
    '## The screens, from the diff',
    [
      ...screens.filter((screen) => !screen.same).map((screen) => `- ${escapeMarkdown(`${screen.name}: ${screen.reason} · round ${screen.round}`)}`),
      ...Object.entries(sameAsBase).map(([file, count]) => `- ${escapeMarkdown(`via ${file}: ${count} ${count === 1 ? 'screen' : 'screens'} same as base`)}`),
    ].join('\n'),
    ...[...families].map(([family, cards]) => `## ${escapeMarkdown(family)}\n\n${cards.join('\n')}`),
    ...(unjudged.length > 0 ? ['## Differs, not judged', unjudged.map((line) => `- ${escapeMarkdown(line)}`).join('\n')] : []),
  ].join('\n\n');
}

export function publishPage({ root, out, branch, round, head, carry }) {
  const remote = git(root, 'remote', 'get-url', 'origin');
  const ref = `refs/heads/${branch}`;
  const repository = /github\.com[:/](.+?)(?:\.git)?$/.exec(remote)?.[1];
  const url = repository ? `https://github.com/${repository}/tree/${branch}` : `${remote} ${ref}`;
  const current = JSON.parse(readFileSync(join(out, PAGE_DATA), 'utf8'));
  const dir = mkdtempSync(join(tmpdir(), 'proof-push-'));
  const at = (...args) => git(dir, ...args);
  try {
    at('init', '-q', '-b', branch);
    const known = at('ls-remote', remote, ref) !== '';
    let page = current;
    if (known) {
      at('fetch', '-q', '--depth=1', remote, ref);
      at('reset', '-q', '--soft', 'FETCH_HEAD');
      if (carry) page = carryOver(JSON.parse(at('show', `FETCH_HEAD:${PAGE_DATA}`)), current);
      const carried = pageFiles(page).filter((file) => !pageFiles(current).includes(file));
      if (carried.length > 0) at('checkout', 'FETCH_HEAD', '--', ...carried);
    }
    writeFileSync(join(dir, PAGE_FILE), renderPage(page));
    writeFileSync(join(dir, PAGE_DATA), `${JSON.stringify(page, null, 2)}\n`);
    for (const file of pageFiles(current)) {
      mkdirSync(dirname(join(dir, file)), { recursive: true });
      copyFileSync(join(out, file), join(dir, file));
    }
    at('add', '-A');
    if (known && at('status', '--porcelain') === '') return url;
    const identity = ['user.name', 'user.email'].flatMap((key) => ['-c', `${key}=${git(root, 'config', key)}`]);
    at(...identity, 'commit', '-qm', `${branch.slice(PROOF_BRANCH.length)} round ${round} at ${head.slice(0, 7)}`);
    at('push', '-q', remote, `HEAD:${ref}`);
    return url;
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const SURFACES = {
  app: {
    set: (changed, _adapter, root, added) => appSet(changed, { root, added }),
    all: (_adapter, root) => [...catalogue('app', root).screens].map(([id, files]) => ({ surface: 'app', id, files, family: familyOfPath(id), reason: CATALOGUE })),
  },
  admin: {
    set: (changed, adapter, root, added) => adminSet(changed, adapter.SCREENS, adapter.WALKS, { root, added }),
    all: (adapter) => {
      const familyOf = (path) => ownersOf(path, MODULE_ROUTES)[0];
      const entries = adapter.CATALOGUE.map((screen) => ({ surface: 'admin', id: screen.name, path: screen.path, family: familyOf(screen.path), reason: CATALOGUE }));
      const walks = adapter.WALKS.map((walk) => ({ surface: 'admin', id: walk.name, path: null, family: entries.find(({ id }) => id.startsWith(walk.prefix))?.family, reason: CATALOGUE }));
      return [...entries, ...walks];
    },
  },
};

const loadAdapters = (root) => Promise.all(Object.entries(ADAPTERS).map(async ([name, path]) => [name, await import(pathToFileURL(join(root, path)).href)])).then(Object.fromEntries);
export async function scannerFor(root, adapters) {
  const options = {};
  for (const [surface, adapter] of Object.entries(adapters)) {
    const { modules, ...tokens } = await loadTokens(root, adapter.TOKENS);
    options[surface] = { tokens, target: adapter.target(modules), scaled: adapter.SCALED_TEXT ?? null };
  }
  return (page, job) => scanPage(page, { ...options[job.screen.surface], phone: job.label === 'phone' });
}

const launchChromium = (root) => createRequire(join(root, 'apps/admin/package.json'))('playwright').chromium.launch({ args: ['--no-proxy-server', '--disable-partial-raster'] });

export async function installAt(dir, surfaces) {
  await runQuietly('pnpm', ['install', '--frozen-lockfile', '--prefer-offline', '--reporter=silent'], dir);
  await runQuietly('pnpm', [...surfaces.flatMap((surface) => ['--filter', `${PACKAGES[surface]}^...`]), 'build'], dir);
}

async function removeCheckout(root, dir) {
  await runQuietly('git', ['worktree', 'remove', '--force', dir], root).catch(() => {});
  rmSync(dir, { recursive: true, force: true });
}

export async function main(argv, { root = REPO_ROOT, load = loadAdapters, launch = launchChromium, install = installAt, scanner = scannerFor, publish = publishPage } = {}) {
  const started = Date.now();
  let values;
  try {
    ({ values } = parseArgs({
      args: argv.filter((arg) => arg !== '--'),
      options: { base: { type: 'string' }, head: { type: 'string' }, catalogue: { type: 'boolean' }, out: { type: 'string' }, contexts: { type: 'string' }, round: { type: 'string' }, since: { type: 'string' } },
    }));
  } catch (error) {
    console.error(`${error.message}\n${USAGE}`);
    return 2;
  }
  const contexts = Number(values.contexts ?? DEFAULT_CONTEXTS);
  const round = Number(values.round ?? 0);
  const since = values.since !== undefined;
  const sinceClash = since && (values.base !== undefined || values.head !== undefined || values.catalogue);
  if (!Number.isInteger(contexts) || contexts < 1 || !Number.isInteger(round) || round < 0 || sinceClash) {
    console.error(USAGE);
    return 2;
  }
  const slug = values.catalogue ? null : proofSlugOf(git(root, 'rev-parse', '--abbrev-ref', 'HEAD'));
  if (!values.catalogue && slug === null) {
    console.error(`proof: run on a branch claude/<name>, whose page goes to ${PROOF_BRANCH}<name>\n${USAGE}`);
    return 2;
  }
  const mainBase = () => git(root, 'merge-base', MAIN, 'HEAD');
  const given = values.since ?? values.base;
  const base = values.catalogue ? git(root, 'rev-parse', values.head ?? 'HEAD') : given === undefined ? mainBase() : git(root, 'rev-parse', given);
  const head = git(root, 'rev-parse', values.head ?? 'HEAD');
  const dirty = values.head === undefined && git(root, 'status', '--porcelain') !== '';
  const changed = since ? changedSince(root, base, mainBase()) : changedFiles(root, base, values.head);
  const added = addedFiles(root, base, values.head).filter((file) => changed.includes(file));
  const adapters = await load(root);
  let set;
  try {
    set = Object.entries(adapters).flatMap(([surface, adapter]) => (values.catalogue ? SURFACES[surface].all(adapter, root) : SURFACES[surface].set(changed, adapter, root, added)));
  } catch (error) {
    console.error(`proof: ${error.message}`);
    return 2;
  }
  if (set.length === 0) {
    console.log('proof: no screen');
    return 0;
  }
  const out = values.out === undefined ? join(root, OUT_DIR) : isAbsolute(values.out) ? values.out : join(root, values.out);
  const shotsDir = join(out, SHOTS_DIR);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(shotsDir, { recursive: true });
  const surfaces = [...new Set(set.map(({ surface }) => surface))];
  const dirs = { head: shotsDir, base: join(out, BASE_DIR) };
  mkdirSync(dirs.base);
  const browser = await launch(root);
  const servers = [];
  let checkout = null;
  let atBase = Promise.resolve();
  let compared;
  const release = async () => {
    await browser.close().catch(() => {});
    await Promise.all(servers.map((server) => server.stop()));
    if (checkout !== null) await removeCheckout(root, checkout);
  };
  const interrupted = (signal) => release().finally(() => process.exit(SIGNAL_EXIT[signal]));
  for (const signal of Object.keys(SIGNAL_EXIT)) process.once(signal, interrupted);
  try {
    const start = async (dir, served) =>
      Object.fromEntries(
        await Promise.all(
          served.map(async (surface) => {
            const server = await adapters[surface].serve(dir);
            servers.push(server);
            const newContext = contextMaker(browser, server.origin, adapters[surface].cached);
            return [surface, { newContext, runner: await adapters[surface].open(newContext, server.origin) }];
          }),
        ),
      );
    const used = Object.fromEntries(surfaces.map((surface) => [surface, adapters[surface]]));
    const scan = await scanner(root, used);
    const jobs = jobsFor(set, used);
    const baseJobs = baseJobsOf(jobs);
    const baseSurfaces = [...new Set(baseJobs.map((job) => job.screen.surface))];
    if (baseJobs.length > 0)
      atBase = (async () => {
        checkout = mkdtempSync(join(tmpdir(), 'proof-base-'));
        await runQuietly('git', ['worktree', 'add', '--detach', '--quiet', checkout, base], root);
        await install(checkout, baseSurfaces);
        return start(checkout, baseSurfaces);
      })().catch((error) => ({ error: String(error.message ?? error).split('\n')[0] }));
    const runnersOf = { head: await start(root, surfaces), base: atBase };
    const tagged = [...jobs.map((job) => ({ side: 'head', job })), ...baseJobs.map((job) => ({ side: 'base', job }))];
    const shots = await pool(tagged, contexts, async ({ side, job }) => {
      const runners = await runnersOf[side];
      if (runners.error) return [{ job, state: job.state, console: 0, requests: 0, ms: 0, file: null, bytes: 0, hash: null, error: runners.error, side }];
      const { newContext, runner } = runners[job.screen.surface];
      return (await runJob(newContext, runner, job, dirs[side], side === 'head' ? scan : null)).map((shot) => ({ ...shot, side }));
    });
    const baseShots = shots.filter(({ side }) => side === 'base');
    const { kept, dropped } = dropAbsentStates(shots.filter(({ side }) => side === 'head'));
    for (const shot of dropped) unlinkSync(join(shotsDir, shot.file));
    const measure = (shot, before) => changeSummary(readFileSync(join(dirs.base, before.file)), readFileSync(join(shotsDir, shot.file)));
    compared = cropPairs(judgedSet(compareToBase(kept, baseShots, measure)), dirs);
    const pagePath = join(out, PAGE_FILE);
    const fixtures = Object.fromEntries(surfaces.map((surface) => [surface, adapters[surface].FIXTURE]));
    const model = pageModel({ head, base, dirty, round, fixtures, set, shots: compared });
    writeFileSync(pagePath, renderPage(model));
    writeFileSync(join(out, PAGE_DATA), `${JSON.stringify(model, null, 2)}\n`);
    const text = summary(compared, { dropped: dropped.length, base: baseShots, seconds: (Date.now() - started) / 1000, page: relative(root, pagePath) || pagePath });
    writeFileSync(join(out, 'summary.txt'), `${text}\n`);
    writeFileSync(join(out, FINDINGS_FILE), `${JSON.stringify(failures(compared), null, 2)}\n`);
    console.log(text);
  } finally {
    for (const signal of Object.keys(SIGNAL_EXIT)) process.off(signal, interrupted);
    await atBase;
    await release();
  }
  if (compared.some(isFail)) return 1;
  if (slug !== null) console.log(`proof: page ${await publish({ root, out, branch: `${PROOF_BRANCH}${slug}`, round, head, carry: since })}`);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) process.exitCode = await main(process.argv.slice(2));
