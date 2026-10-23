import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

import { GATE_MAIN, pushPaths, resolveDeployable } from '../deploy-ref.mjs';
import { docsOnly } from '../harness-only.mjs';

import { childEnv } from './hermetic.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../../..');
const read = (path) => readFileSync(join(ROOT, path), 'utf8');
const workflow = parse(read('.github/workflows/release-please.yml'));
const WORKFLOWS = join(ROOT, '.github/workflows');
const LOG_BRANCH = 'releases';
const steps = (job, uses) => workflow.jobs[job].steps.filter((step) => String(step.uses).startsWith(`${uses}@`)).map((step) => step.with);

describe('release-please: the release PR, and production once it merges (CD-478)', () => {
  it('tags apart from the release PR, and deploys production through deploy.yml at the tag only when a release PR merged', () => {
    expect(workflow.on).toEqual({ push: { branches: ['main'] } });
    expect(workflow.jobs.production).toMatchObject({
      needs: ['release'],
      if: "needs.release.outputs.created == 'true'",
      uses: './.github/workflows/deploy.yml',
      secrets: 'inherit',
      with: { env: 'production', ref: '${{ needs.release.outputs.tag }}' },
    });
    expect(workflow.jobs.release.outputs).toEqual({
      created: '${{ steps.release-retry.outputs.release_created || steps.release.outputs.release_created }}',
      tag: '${{ steps.release-retry.outputs.tag_name || steps.release.outputs.tag_name }}',
    });
    expect(steps('release', 'googleapis/release-please-action')).toEqual([expect.objectContaining({ 'skip-github-pull-request': true }), expect.objectContaining({ 'skip-github-pull-request': true })]);
    expect(steps('pull-request', 'googleapis/release-please-action')).toEqual([expect.objectContaining({ 'skip-github-release': true }), expect.objectContaining({ 'skip-github-release': true })]);
    expect(workflow.jobs.release.steps.at(-1).if).toBe("steps.release.outcome == 'failure'");
    expect(workflow.jobs['pull-request'].steps.at(-1).if).toBe("steps.pr.outcome == 'failure'");
    expect(workflow.permissions).toEqual({});
    for (const job of ['release', 'pull-request']) {
      expect(workflow.jobs[job].environment, job).toBe('production');
      expect(workflow.jobs[job].permissions, job).toEqual({});
      expect(steps(job, 'actions/create-github-app-token'), job).toEqual([
        {
          'client-id': '${{ vars.RELEASE_APP_CLIENT_ID }}',
          'private-key': '${{ secrets.RELEASE_APP_PRIVATE_KEY }}',
          'permission-contents': 'write',
          'permission-pull-requests': 'write',
          'permission-issues': 'write',
        },
      ]);
    }
  });

  it("writes only the changelog and the manifest: gate-main skips the release commit, production deploys the commit under it, the release PR takes the docs lane, and a change to the config runs the checks", async () => {
    const [{ 'config-file': configFile, 'manifest-file': manifestFile }] = steps('release', 'googleapis/release-please-action');
    const config = JSON.parse(read(configFile));
    const written = [config.packages['.']['changelog-path'], manifestFile];
    expect(written).toEqual(['release/CHANGELOG.md', 'release/manifest.json']);
    const parents = { release: 'tip', tip: 'older' };
    const resolved = await resolveDeployable({
      sha: 'release',
      walk: true,
      filter: pushPaths(read(GATE_MAIN)),
      hasEvidence: async (sha) => sha === 'tip',
      firstParent: (sha) => parents[sha] ?? null,
      changed: () => written,
    });
    expect(resolved.sha).toBe('tip');
    expect(docsOnly(written)).toBe(true);
    expect(docsOnly([configFile])).toBe(false);
  });

  it("lists feat and fix and hides every other kind the title rule accepts, so a setup, test or docs merge opens no release PR", () => {
    const kinds = read('.github/workflows/pr-issue.yml').match(/\^\(([a-z|]+)\)\\\(/)[1].split('|');
    const sections = JSON.parse(read('release/release-please-config.json'))['changelog-sections'];
    expect(sections.map((section) => section.type).sort()).toEqual([...kinds].sort());
    expect(sections.filter((section) => !section.hidden).map((section) => section.type)).toEqual(['feat', 'fix']);
  });

  it('deletes the proof branches of the PRs merged since the previous v5.N.0 tag only once production deployed, with contents: write on the delete job alone', () => {
    const { proofs, 'proof-branches': remove } = workflow.jobs;
    expect(proofs).toMatchObject({ needs: ['release', 'production'], if: "${{ !cancelled() && needs.production.result == 'success' }}", permissions: { contents: 'read', 'pull-requests': 'read' } });
    expect(remove).toMatchObject({
      needs: ['production', 'proofs'],
      if: "${{ !cancelled() && needs.production.result == 'success' && needs.proofs.outputs.proofs != '' }}",
      permissions: { contents: 'write' },
      'timeout-minutes': 5,
    });
    const notes = proofs.steps.find((step) => step.id === 'notes');
    const dir = mkdtempSync(join(tmpdir(), 'release-notes-'));
    try {
      const bin = join(dir, 'bin');
      mkdirSync(bin);
      writeFileSync(join(bin, 'node'), '#!/usr/bin/env bash\nif [[ "$1" == -p ]]; then echo bootstrap; else echo "$*"; fi\n');
      chmodSync(join(bin, 'node'), 0o755);
      const git = (...args) => spawnSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@t', ...args], { cwd: dir, env: childEnv(), encoding: 'utf8' });
      git('init', '-q');
      for (const tag of ['v5.128', 'v5.9.0', 'v5.129', 'v5.10.0']) {
        git('commit', '-q', '--allow-empty', '-m', tag);
        git('tag', tag);
      }
      const from = (tag) => {
        const result = spawnSync('bash', ['-c', notes.run], { cwd: dir, env: { ...childEnv(), PATH: `${bin}:${process.env.PATH}`, TAG: tag, GITHUB_OUTPUT: join(dir, 'out') }, encoding: 'utf8' });
        return result.status === 0 ? result.stdout.trim().split(' --from ')[1] : `exit ${result.status}`;
      };
      expect(from('v5.10.0')).toBe('v5.9.0');
      expect(from('v5.9.0')).toBe('bootstrap');
      expect(from('v5.129')).toBe('exit 1');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

describe('the no-loop property (D70) and the gate split (CD-210)', () => {
  const workflows = readdirSync(WORKFLOWS)
    .filter((name) => /\.ya?ml$/.test(name))
    .map((name) => ({ name, on: parse(readFileSync(join(WORKFLOWS, name), 'utf8')).on }));
  const triggers = (on) => (typeof on === 'string' ? [on] : Array.isArray(on) ? on : Object.keys(on ?? {}));

  it('no workflow can loop: the push triggers are the main gate and release-please, on main only (CD-210, CD-478), release starts only from a finished main gate on main or a dispatch, nothing else starts on a tag, a created ref, a release event or another workflow, only the nightly on a schedule, and no workflow names the log branch', () => {
    const gateMain = workflows.find((w) => w.name === 'gate-main.yml');
    expect.soft(Object.keys(gateMain.on), 'gate-main triggers').toEqual(['push']);
    expect.soft(Object.keys(gateMain.on.push).sort(), 'gate-main push keys').toEqual(['branches', 'paths']);
    expect.soft(gateMain.on.push.branches, 'gate-main branches').toEqual(['main']);
    expect.soft(workflows.find((w) => w.name === 'release-please.yml').on, 'release-please triggers').toEqual({ push: { branches: ['main'] } });
    for (const { name, on } of workflows) {
      if (!['gate-main.yml', 'release-please.yml'].includes(name)) expect.soft(triggers(on), `${name} on push`).not.toContain('push');
    }
    const release = workflows.find((w) => w.name === 'release.yml');
    expect.soft(Object.keys(release.on).sort(), 'release triggers').toEqual(['workflow_dispatch', 'workflow_run']);
    expect.soft(release.on.workflow_run, 'release workflow_run').toEqual({ workflows: ['gate-main'], types: ['completed'], branches: ['main'] });
    const text = readFileSync(join(WORKFLOWS, 'release.yml'), 'utf8');
    expect.soft(text, 'release on success').toContain("github.event.workflow_run.conclusion == 'success'");
    expect.soft(text, 'release ref').toContain("ref: ${{ github.event.workflow_run.head_sha || 'main' }}");
    for (const { name, on } of workflows) {
      const events = triggers(on);
      for (const event of ['create', 'release', 'registry_package']) expect.soft(events, `${name} on ${event}`).not.toContain(event);
      if (name !== 'release.yml') expect.soft(events, `${name} on workflow_run`).not.toContain('workflow_run');
    }
    expect.soft(workflows.filter(({ on }) => triggers(on).includes('schedule')).map((w) => w.name), 'schedule').toEqual(['nightly.yml']);
    expect.soft(LOG_BRANCH, 'log branch').not.toBe('main');
    expect.soft(LOG_BRANCH.startsWith('deploy/'), 'log branch').toBe(false);
    for (const name of readdirSync(WORKFLOWS)) {
      const workflowText = readFileSync(join(WORKFLOWS, name), 'utf8');
      expect.soft(workflowText, name).not.toMatch(new RegExp(`branches:[^\\n]*\\b${LOG_BRANCH}\\b`));
    }
  });

  it('only the main gate posts gate/tree', () => {
    const post = (name) => readFileSync(join(WORKFLOWS, name), 'utf8').match(/gate-status\.mjs post --context (\S+)/)?.[1];
    expect(post('gate-main.yml')).toBe('gate');
    expect(readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8')).toMatch(/concurrency:\n {2}group: gate-main\n {2}cancel-in-progress: true/);
    expect(readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8')).toContain('runs-on: [self-hosted, wb]');
    expect(readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8')).not.toMatch(/verify\.sh|pnpm install|playwright/);
    expect(readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8')).toMatch(/gate-status\.mjs merged-pr --sha "\$sha" --need checks;/);
    expect(readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8')).not.toMatch(/--full/);
    const gateMain = readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8');
    const checksPost = 'gate-status.mjs post --context checks --state success --sha "$sha" --tree "$tree"';
    expect(gateMain).toContain(checksPost);
    expect(gateMain.indexOf(checksPost)).toBeGreaterThan(gateMain.indexOf('post --context gate --state success'));
    expect(gateMain.slice(0, gateMain.indexOf('exit 1'))).not.toContain(checksPost);
    const adoptEnd = gateMain.indexOf('\n          fi\n', gateMain.indexOf('exit 1'));
    expect(adoptEnd).toBeGreaterThan(gateMain.indexOf('exit 1'));
    expect(gateMain.indexOf(checksPost)).toBeGreaterThan(adoptEnd);
    expect(readFileSync(join(WORKFLOWS, 'gate-main.yml'), 'utf8')).toMatch(/gate-status\.mjs check --sha "\$sha" --need checks;/);
  });
});
