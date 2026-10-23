#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { journeyImports, journeysFor, readJourneys } from './journeys.mjs';
import { REPO_ROOT, navigation, placeOnSurface, servedBy } from './screens.mjs';
import { NOT_PRODUCT, SURFACES, SURFACE_NAMES } from './surfaces.mjs';

const PACKAGE_SOURCE = /^packages\/[^/]+\/src\//;
const USAGE = "usage: qa-plan.mjs --base <sha|''> --head <sha> | qa-plan.mjs view --plan <plan.json> | qa-plan.mjs files --plan <plan.json> --surface app|admin (prints none, some or all, then the files)";

export function planSurface(name, { root, placed, served, journeys, everything }) {
  const changedJourneys = placed.journeys.filter((file) => existsSync(join(root, file)));
  const journeyHelper = placed.journeys.some((file) => !/\.spec\.ts$/.test(file));
  const unplaceable = everything ? ['(no QA-passed release to diff from)'] : placed.unplaceable;
  const all = everything || unplaceable.length > 0 || journeyHelper;
  const files = all
    ? []
    : [...new Set([...journeysFor(journeys, placed.screens), ...changedJourneys.filter((file) => file.endsWith('.spec.ts'))])].sort();
  const verify =
    unplaceable.length > 0
      ? { scope: 'navigation', screens: navigation(name, root) }
      : placed.screens.length > 0
        ? { scope: 'screens', screens: placed.screens }
        : null;
  return { screens: placed.screens, unplaceable, served, e2e: { all, files }, verify };
}

function withJourneyHelpers(placed, paths, helpers) {
  const extra = paths.filter((path) => helpers.has(path) && !placed.journeys.includes(path));
  return { ...placed, journeys: [...placed.journeys, ...extra] };
}

export function planRelease(paths, { root = REPO_ROOT, everything = false, place = placeOnSurface, journeysOf = readJourneys } = {}) {
  const placed = Object.fromEntries(SURFACE_NAMES.map((name) => [name, withJourneyHelpers(place(name, paths, { root }), paths, journeyImports(name, root))]));
  const served = servedBy(paths);
  for (const path of paths) {
    if (!PACKAGE_SOURCE.test(path) || NOT_PRODUCT.some((pattern) => pattern.test(path))) continue;
    const somewhere = SURFACE_NAMES.some((name) => placed[name].unplaceable.includes(path) || placed[name].placed.includes(path));
    if (!somewhere) for (const name of SURFACE_NAMES) served.add(name);
  }
  const surfaces = Object.fromEntries(
    SURFACE_NAMES.map((name) => [
      name,
      planSurface(name, { root, placed: placed[name], served: served.has(name), journeys: journeysOf(name, root), everything }),
    ]),
  );
  const screenChange = SURFACE_NAMES.some((name) => surfaces[name].unplaceable.length > 0 || surfaces[name].screens.length > 0);
  return { screenChange, surfaces };
}

export function changedBetween(base, head, root = REPO_ROOT) {
  return execFileSync('git', ['diff', '--name-only', '--no-renames', base, head, '--'], { cwd: root, encoding: 'utf8' })
    .split('\n')
    .filter(Boolean)
    .sort();
}

const e2eRun = (e2e) => (e2e.all ? 'all' : e2e.files.length > 0 ? 'some' : 'none');

export function planView(plan) {
  return SURFACE_NAMES.map((name) => {
    const { e2e, verify } = plan.surfaces[name];
    return `${name}=${JSON.stringify({ e2e: e2eRun(e2e), verify: Boolean(verify) })}`;
  });
}

export function journeySelection(plan, name) {
  const { e2e } = plan.surfaces[name];
  return { run: e2eRun(e2e), files: e2e.all ? [] : e2e.files.map((file) => file.slice(`${SURFACES[name].dir}/`.length)) };
}

function cli(argv) {
  const { values, positionals } = parseArgs({ args: argv, allowPositionals: true, options: { base: { type: 'string' }, head: { type: 'string' }, plan: { type: 'string' }, surface: { type: 'string' } } });
  const [verb = 'plan'] = positionals;
  if (verb === 'plan' && values.head) return [JSON.stringify(values.base ? planRelease(changedBetween(values.base, values.head)) : planRelease([], { everything: true }))];
  const plan = values.plan ? JSON.parse(readFileSync(values.plan, 'utf8')) : null;
  if (verb === 'view' && plan) return planView(plan);
  if (verb === 'files' && plan && SURFACES[values.surface]) {
    const { run, files } = journeySelection(plan, values.surface);
    return [run, ...files];
  }
  throw new Error(USAGE);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  let lines;
  try {
    lines = cli(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exit(2);
  }
  for (const line of lines) process.stdout.write(`${line}\n`);
}
