import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { REPO_ROOT, filesUnder, parse, property, resolveImport, walk } from './screens.mjs';
import { SURFACES } from './surfaces.mjs';

export const SCREEN_TAG = '@screen:';
export const PENDING_FILE = 'infra/verify/journeys-pending.json';
const JOURNEY_FILE = /\.spec\.ts$/;
const CODE = /\.[cm]?[jt]sx?$/;
const IMPORTING = new Set(['ImportDeclaration', 'ExportAllDeclaration', 'ExportNamedDeclaration']);

const testName = (callee) =>
  callee?.type === 'Identifier' ? callee.value : callee?.type === 'MemberExpression' && callee.object?.type === 'Identifier' && ['only', 'fixme', 'fail', 'slow'].includes(callee.property?.value) ? callee.object.value : null;

export function journeysIn(source, file) {
  const found = [];
  walk(parse(source, file), (node) => {
    if (node.type !== 'CallExpression' || testName(node.callee) !== 'test') return;
    const [title, details] = node.arguments ?? [];
    if (title?.expression?.type !== 'StringLiteral' && title?.expression?.type !== 'TemplateLiteral') return;
    const name = title.expression.type === 'StringLiteral' ? title.expression.value : title.expression.quasis.map((quasi) => quasi.cooked ?? quasi.raw).join('…');
    const tag = details?.expression?.type === 'ObjectExpression' ? property(details.expression, 'tag') : undefined;
    const literals = tag === undefined ? [] : tag.type === 'StringLiteral' ? [tag] : tag.type === 'ArrayExpression' ? tag.elements.map((element) => element?.expression) : [null];
    const readable = literals.every((literal) => literal?.type === 'StringLiteral');
    const tags = readable ? literals.map((literal) => literal.value) : null;
    found.push({
      file,
      title: name,
      screens: tags && tags.filter((value) => value.startsWith(SCREEN_TAG)).map((value) => value.slice(SCREEN_TAG.length)),
      tags,
    });
  });
  return found;
}

export function readJourneys(name, root = REPO_ROOT) {
  return filesUnder(root, SURFACES[name].journeys)
    .filter((file) => JOURNEY_FILE.test(file))
    .flatMap((file) => journeysIn(readFileSync(join(root, file), 'utf8'), file));
}

export function readPending(root = REPO_ROOT) {
  return JSON.parse(readFileSync(join(root, PENDING_FILE), 'utf8'));
}

export function checkJourneys({ surface, screens, journeys, pending = [] }) {
  const known = new Set(screens);
  const covered = new Set();
  const problems = [];
  for (const journey of journeys) {
    const where = `${journey.file} › ${journey.title}`;
    if (journey.screens === null) {
      problems.push(`${surface}: ${where} has a screen tag that is not a string literal`);
      continue;
    }
    if (journey.screens.length === 0) problems.push(`${surface}: ${where} names no screen (tag it ${SCREEN_TAG}<screen>)`);
    for (const screen of journey.screens) {
      if (known.has(screen)) covered.add(screen);
      else problems.push(`${surface}: ${where} names ${screen}, which is not a screen`);
    }
  }
  const waiting = new Set(pending);
  for (const screen of [...known].sort()) {
    if (!covered.has(screen) && !waiting.has(screen)) problems.push(`${surface}: ${screen} has no journey`);
    if (covered.has(screen) && waiting.has(screen)) problems.push(`${surface}: ${screen} has a journey, so it leaves the pending list`);
  }
  for (const screen of [...waiting].sort()) if (!known.has(screen)) problems.push(`${surface}: pending ${screen} is not a screen`);
  return problems;
}

export function journeysFor(journeys, screens) {
  const wanted = new Set(screens);
  return [...new Set(journeys.filter((journey) => (journey.screens ?? []).some((screen) => wanted.has(screen))).map((journey) => journey.file))].sort();
}

function importsOf(file, root) {
  const found = [];
  walk(parse(readFileSync(join(root, file), 'utf8'), file), (node) => {
    if (!IMPORTING.has(node.type) || typeof node.source?.value !== 'string') return;
    const resolved = resolveImport(file, node.source.value, root);
    if (resolved) found.push(resolved);
  });
  return found;
}

export function journeyImports(name, root = REPO_ROOT) {
  const seen = new Set();
  const queue = filesUnder(root, SURFACES[name].journeys).filter((file) => CODE.test(file));
  while (queue.length > 0) {
    const file = queue.pop();
    if (seen.has(file)) continue;
    seen.add(file);
    queue.push(...importsOf(file, root));
  }
  return seen;
}
