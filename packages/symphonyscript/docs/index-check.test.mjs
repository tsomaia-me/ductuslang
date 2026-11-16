// Checks that every file INDEX.md names exists.
// Run: node --test packages/symphonyscript/docs/index-check.test.mjs
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const FOLDER = path.dirname(fileURLToPath(import.meta.url));

// A link's target: what follows "](" up to the first space or ")", so a title after it is no part of it.
export function linkTargets(text) {
  return [...text.matchAll(/\]\(\s*<?([^)\s>]+)/g)].map((match) => match[1]);
}

// A file is named by a link's target or by a document's name in code font.
// A target with a scheme, such as https:, names no file, and an anchor is no part of the name.
export function namedFiles(text) {
  const names = [...text.matchAll(/`([^`\s]+\.md)(?=[`\s])/gi)].map((match) => match[1]);
  const files = [...linkTargets(text), ...names]
    .filter((name) => !/^[a-z][a-z0-9+.-]*:/i.test(name))
    .map((name) => name.split('#')[0])
    .filter((name) => name !== '');
  return [...new Set(files)].sort();
}

export function missingFiles(text, folder) {
  return namedFiles(text).filter((name) => !existsSync(path.join(folder, name)));
}

test('every file the index names exists', () => {
  const text = readFileSync(path.join(FOLDER, 'INDEX.md'), 'utf8');
  assert.equal(linkTargets(text).length, text.split('](').length - 1, 'a link whose target the check cannot read');
  assert.notDeepEqual(namedFiles(text), [], 'the index names no file, so the check found nothing to check');
  assert.deepEqual(missingFiles(text, FOLDER), []);
});

test('a link target, with or without a title, and a document name in code font are found, and a web link is not', () => {
  const text = '[`A.md`](A.md), `B.MD`, [a part](C.md#part), [titled](D.md "a title"), `E.md §5`, [site](https://example.com/x.md), `code`';
  assert.deepEqual(namedFiles(text), ['A.md', 'B.MD', 'C.md', 'D.md', 'E.md']);
});

test('a named file that does not exist is reported', () => {
  const text = '[the index](INDEX.md), [gone](NO-SUCH-LINK.md "a title") and `NO-SUCH-DOCUMENT.md`';
  assert.deepEqual(missingFiles(text, FOLDER), ['NO-SUCH-DOCUMENT.md', 'NO-SUCH-LINK.md']);
});
