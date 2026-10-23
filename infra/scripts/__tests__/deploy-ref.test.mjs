import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { pushPaths, resolveDeployable } from '../deploy-ref.mjs';

const GATE_MAIN = readFileSync(fileURLToPath(new URL('../../../.github/workflows/gate-main.yml', import.meta.url)), 'utf8');
const FILTER = pushPaths(GATE_MAIN);

const chain = (commits) => {
  const byId = new Map(commits.map((commit, index) => [commit.sha, { ...commit, parent: commits[index + 1]?.sha ?? null }]));
  return {
    filter: FILTER,
    hasEvidence: async (sha) => byId.get(sha).evidence === true,
    firstParent: (sha) => byId.get(sha).parent,
    changed: (sha) => byId.get(sha).paths,
  };
};

const sha = (digit) => String(digit).repeat(40);

describe('deploy-ref: what a deploy of a ref deploys (CD-478)', () => {
  it("reads gate-main's push filter exactly as YAML does", () => {
    expect(FILTER).toEqual(parse(GATE_MAIN).on.push.paths);
  });

  it('walks past commits gate-main skipped to the newest one with gate evidence, refuses when a commit without it changes a path gate-main releases, deploys a sha named only when it carries the evidence itself, and stops at the walk limit', async () => {
    const skipped = chain([
      { sha: sha(1), paths: ['infra/scripts/__tests__/x.test.mjs', 'apps/app/src/a.test.tsx'] },
      { sha: sha(2), paths: ['.claude/agents/builder.md'] },
      { sha: sha(3), paths: ['apps/app/src/a.tsx'], evidence: true },
    ]);
    const walked = await resolveDeployable({ sha: sha(1), walk: true, ...skipped });
    expect.soft(walked.sha, 'walked').toBe(sha(3));
    expect.soft(walked.why, 'walked').toHaveLength(3);
    const released = chain([
      { sha: sha(1), paths: ['apps/app/src/a.test.tsx'] },
      { sha: sha(2), paths: ['apps/edge/src/api.ts', 'apps/edge/src/api.test.ts'] },
      { sha: sha(3), paths: ['apps/app/src/a.tsx'], evidence: true },
    ]);
    const refused = await resolveDeployable({ sha: sha(1), walk: true, ...released });
    expect.soft(refused.sha, 'refused').toBeNull();
    expect.soft(refused.why.at(-1), 'refused').toContain('apps/edge/src/api.ts');
    expect.soft(refused.why.at(-1), 'refused').not.toContain('api.test.ts');
    const named = chain([
      { sha: sha(1), paths: ['apps/app/src/a.test.tsx'] },
      { sha: sha(2), paths: ['apps/app/src/a.tsx'], evidence: true },
    ]);
    expect.soft((await resolveDeployable({ sha: sha(1), walk: false, ...named })).sha, 'named without evidence').toBeNull();
    expect.soft((await resolveDeployable({ sha: sha(2), walk: false, ...named })).sha, 'named with evidence').toBe(sha(2));
    const long = chain([
      { sha: sha(1), paths: ['docs/a.md'] },
      { sha: sha(2), paths: ['docs/b.md'] },
      { sha: sha(3), paths: ['apps/app/src/a.tsx'], evidence: true },
    ]);
    expect.soft((await resolveDeployable({ sha: sha(1), walk: true, limit: 2, ...long })).sha, 'the walk limit').toBeNull();
  });
});
