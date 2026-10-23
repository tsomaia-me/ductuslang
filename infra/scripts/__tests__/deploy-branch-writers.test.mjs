/**
 * The ruleset on
 * `refs/heads/deploy/*` ("deploy branches: release workflow only") reads `enforcement: disabled`
 * through the API (review of #213, 7 Oct), so it holds nothing back: any workflow token that can
 * write contents could move them, and the guarantee is here. No workflow promotes, rolls back
 * or names a deploy branch, and none writes contents but the two jobs that delete proof branches.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));
const files = readdirSync(WORKFLOWS).filter((name) => /\.ya?ml$/.test(name));
const raw = (name) => readFileSync(join(WORKFLOWS, name), 'utf8');
const text = (name) =>
  raw(name)
    .split('\n')
    .filter((line) => !/^\s*#/.test(line))
    .join('\n');

describe('the deploy branches have one writer (CD-215)', () => {
  it("no workflow promotes, rolls back by script or names a deploy branch: deploy.yml deploys both environments (CD-478); every workflow declares its permissions, none write-all; no workflow may write contents but the two jobs that delete proof branches, each one run step on refs/heads/proof/; release.yml's own token reads", () => {
    for (const name of files) {
      expect.soft(text(name), name).not.toMatch(/promote\.sh|release-rollback\.sh|deploy\/[\w$*{]|refs\/heads\/deploy/);
    }
    for (const name of files) {
      const workflow = parse(raw(name));
      expect.soft(workflow.permissions, `${name} has no top-level permissions`).toBeDefined();
      expect.soft(JSON.stringify(workflow), name).not.toContain('write-all');
    }
    const writes = (permissions) => permissions?.contents === 'write';
    const workflows = files.map((name) => [name, parse(raw(name))]);
    expect.soft(workflows.filter(([, workflow]) => writes(workflow.permissions)).map(([name]) => name)).toEqual([]);
    const writers = workflows.flatMap(([name, workflow]) => Object.entries(workflow.jobs ?? {}).filter(([, job]) => writes(job.permissions)).map(([id, job]) => [`${name}:${id}`, job]));
    expect.soft(writers.map(([name]) => name)).toEqual(['proof-branch.yml:delete', 'release-please.yml:proof-branches']);
    for (const [name, job] of writers) {
      expect.soft(job.steps.map((step) => step.uses ?? null), name).toEqual([null]);
      expect.soft(job.steps[0].run, name).toMatch(/\/git\/refs\/heads\/proof\/\$/);
      expect.soft(job.steps[0].run, name).not.toMatch(/deploy|git push|PATCH|POST/);
    }
    const release = parse(raw('release.yml'));
    expect.soft(Object.entries(release.jobs).map(([id, job]) => [id, job.permissions?.contents])).toEqual([['plan', 'read'], ['runner-check', undefined], ['dev', 'read'], ['smoke', 'read']]);
  });
});
