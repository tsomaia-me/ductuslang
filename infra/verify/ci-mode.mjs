#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { readJourneys } from './journeys.mjs';
import { REPO_ROOT } from './screens.mjs';

export const SMOKE_TAG = '@smoke';
const USAGE = 'usage: ci-mode.mjs journeys --plan <plan.json> (cuts the plan to the smoke journeys, prints grep=<tag>)';

export const smokeFiles = (journeys) => [...new Set(journeys.filter((journey) => (journey.tags ?? []).includes(SMOKE_TAG)).map((journey) => journey.file))].sort();

export const smokeOf = (name, root = REPO_ROOT) => smokeFiles(readJourneys(name, root));

export function journeyPlan(plan, smoke = smokeOf) {
  const surfaces = Object.fromEntries(
    Object.entries(plan.surfaces).map(([name, surface]) => {
      const reached = surface.e2e.all || surface.e2e.files.length > 0;
      return [name, { ...surface, e2e: { all: false, files: reached ? smoke(name) : [] } }];
    }),
  );
  return { ...plan, surfaces, grep: SMOKE_TAG };
}

function cli(argv) {
  const { values, positionals } = parseArgs({ args: argv, allowPositionals: true, options: { plan: { type: 'string' } } });
  if (positionals[0] === 'journeys' && values.plan) {
    const plan = journeyPlan(JSON.parse(readFileSync(values.plan, 'utf8')));
    writeFileSync(values.plan, JSON.stringify(plan));
    return [`grep=${plan.grep}`];
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
