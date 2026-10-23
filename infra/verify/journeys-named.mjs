#!/usr/bin/env node
import { existsSync } from 'node:fs';
import { join, posix } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { REPO_ROOT } from './screens.mjs';
import { SURFACES, SURFACE_NAMES } from './surfaces.mjs';

const JOURNEY_SPEC = /\.spec\.ts$/;
const LIST_SEPARATOR = /[\s,]+/;
const USAGE = 'usage: journeys-named.mjs --files "<journey spec paths, space or comma separated>"';

const surfacePlan = (files) => ({ screens: [], unplaceable: [], served: false, e2e: { all: false, files }, verify: null });

function surfaceOf(file, root) {
  const name = SURFACE_NAMES.find((surface) => file.startsWith(SURFACES[surface].journeys));
  if (!name || !JOURNEY_SPEC.test(file) || !existsSync(join(root, file))) {
    throw new Error(`${file} is not a journey spec of a surface (apps/<app|admin>/e2e/journeys/<name>.spec.ts, present at the ref)`);
  }
  return name;
}

export function namedPlan(list, root = REPO_ROOT) {
  const entries = list.split(LIST_SEPARATOR).filter(Boolean);
  if (entries.length === 0) throw new Error(USAGE);
  const files = Object.fromEntries(SURFACE_NAMES.map((name) => [name, new Set()]));
  for (const entry of entries) {
    const file = posix.normalize(entry);
    files[surfaceOf(file, root)].add(file);
  }
  return { screenChange: false, surfaces: Object.fromEntries(SURFACE_NAMES.map((name) => [name, surfacePlan([...files[name]].sort())])) };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  let plan;
  try {
    const { values } = parseArgs({ args: process.argv.slice(2), options: { files: { type: 'string' }, root: { type: 'string' } } });
    plan = JSON.stringify(namedPlan(values.files ?? '', values.root));
  } catch (error) {
    process.stderr.write(`journeys-named: ${error.message}\n`);
    process.exit(2);
  }
  process.stdout.write(`${plan}\n`);
}
