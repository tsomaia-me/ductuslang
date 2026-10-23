/**
 * Every rule this repository enforces by lint still fails when it is broken (CD-218,
 * owner 5 Oct: "Every current rule must keep an equivalent … so nothing silently stops
 * being enforced"). `infra/lint-fixtures/cases.mjs` holds one broken source per rule
 * (the game-module import boundary, the `DB_*` boundary, the query-cache writer, the
 * awaited refetch, `useMutation`'s `onMutate`, the copy props, the game-token read, the
 * boot script, import order, hooks, the Node built-ins in packages) and the sources
 * that must stay clean (the permitted directories, tests). Each is written into a tree
 * that mirrors the repository's paths and linted with the real `.oxlintrc.json`.
 * The comment rule is `strip-comments.mjs --check` (its own tests).
 */
import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { ALLOWED, REJECTED } from '../../lint-fixtures/cases.mjs';
import { judge, lintFixtures } from '../../lint-fixtures/run.mjs';

describe('the lint rules still bite (CD-218)', () => {
  const diagnostics = lintFixtures();

  it('names one broken fixture per enforced rule', () => {
    const rules = new Set(REJECTED.map((fixture) => fixture.rule));
    for (const rule of [
      'no-restricted-imports',
      'no-restricted-syntax',
      'jsx-no-literals',
      'order',
      'rules-of-hooks',
      'eqeqeq',
      'no-unused-vars',
      'no-duplicates',
      'no-undef',
      'require-render-return',
      'no-deprecated',
    ]) {
      expect(rules.has(rule), rule).toBe(true);
    }
    expect(ALLOWED.length).toBeGreaterThan(10);
  });

  it('fails every broken fixture and passes every permitted one', () => {
    expect(judge(diagnostics)).toEqual([]);
  });

  it('pins the React version the JS plugins judge against to the installed one', () => {
    const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
    const config = read('../../../.oxlintrc.json');
    const installed = read('../../../node_modules/react/package.json');
    expect(config.settings.react.version).toBe(installed.version);
  });
});
