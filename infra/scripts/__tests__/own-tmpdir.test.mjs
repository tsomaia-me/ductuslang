/**
 * `own-tmpdir.mjs`: a command gets a temp directory of its own, and nothing it
 * put there outlives it (register, 24 Sep: ~3,900 vitest folders in `/tmp`).
 *
 * Every case runs the real script as a child process with `OWN_TMPDIR_PARENT` at
 * a folder this test owns, so "nothing is left behind" is a count of what is
 * in that folder afterwards — the same instrument the owner's disk was read
 * with — and never a reach into `/tmp` itself.
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { OWN_TMPDIR_PREFIX, diskReason, ownTmpdirEnv, ownTmpdirParent } from '../own-tmpdir-lib.mjs';

const SCRIPT = fileURLToPath(new URL('../own-tmpdir.mjs', import.meta.url));

let parent = '';
beforeEach(() => {
  parent = mkdtempSync(join(tmpdir(), 'wg-own-tmpdir-spec-'));
});
afterEach(() => {
  rmSync(parent, { recursive: true, force: true });
});

/** Runs `node -e <source>` under the wrapper, with the test's folder as the temp root. */
function wrapped(source) {
  return spawnSync(process.execPath, [SCRIPT, process.execPath, '-e', source], {
    encoding: 'utf8',
    env: { ...process.env, OWN_TMPDIR_PARENT: parent, TMPDIR: parent, TMP: parent, TEMP: parent },
  });
}

/** What vitest does: a `<nanoid>/ssr/<sha1>` folder under `os.tmpdir()`, never removed. */
const LEAKY = `
  const { mkdirSync, writeFileSync } = require('node:fs');
  const { tmpdir } = require('node:os');
  const { join } = require('node:path');
  const dir = join(tmpdir(), 'nanoidLikeName_0123', 'ssr');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'aedcd7d3b44224e25f237445617c937b2ef0a81b'), 'module');
  process.stdout.write(tmpdir());
`;

describe('own-tmpdir', () => {
  it('points every temp variable at the one folder', () => {
    expect(ownTmpdirEnv({ PATH: '/bin' }, '/x/wg-run-1')).toEqual({
      PATH: '/bin',
      TMPDIR: '/x/wg-run-1',
      TMP: '/x/wg-run-1',
      TEMP: '/x/wg-run-1',
    });
  });

  it('makes its folders in memory whatever TMPDIR is, unless OWN_TMPDIR_PARENT names a place or memory cannot be written', () => {
    const missing = join(parent, 'no-such-dir');
    const cases = [{}, { TMPDIR: '/runner-tmp' }, { OWN_TMPDIR_PARENT: '/chosen' }];
    expect([...cases.map((env) => ownTmpdirParent(env, parent)), ownTmpdirParent({}, missing)]).toEqual([
      parent,
      parent,
      '/chosen',
      tmpdir(),
    ]);
    expect([...cases.map((env) => diskReason(env, parent)), diskReason({}, missing)]).toEqual([
      null,
      null,
      'OWN_TMPDIR_PARENT is set to /chosen',
      `${missing} cannot be written`,
    ]);
  });

  it('gives the command a folder of its own inside the real temp directory, and leaves nothing behind when the command leaves a folder behind', () => {
    expect.soft(readdirSync(parent), 'before').toEqual([]);
    const run = wrapped(LEAKY);
    expect.soft(run.status).toBe(0);
    expect.soft(run.stdout.trim().startsWith(join(parent, OWN_TMPDIR_PREFIX)), 'its own folder').toBe(true);
    expect.soft(readdirSync(parent), 'after').toEqual([]);
  });

  it('passes how the command ended through (a failure, SIGTERM, SIGKILL, a command that cannot run at all, saying so) and still leaves nothing', () => {
    const rows = [
      { label: 'exit 3', run: () => wrapped(`${LEAKY}; process.exit(3);`), status: 3 },
      { label: 'SIGTERM', run: () => wrapped(`${LEAKY}; process.kill(process.pid, 'SIGTERM');`), signal: 'SIGTERM' },
      { label: 'SIGKILL', run: () => wrapped(`${LEAKY}; process.kill(process.pid, 'SIGKILL');`), signal: 'SIGKILL' },
      {
        label: 'cannot run',
        run: () => spawnSync(process.execPath, [SCRIPT, 'wg-no-such-command-anywhere'], { encoding: 'utf8', env: { ...process.env, OWN_TMPDIR_PARENT: parent } }),
        status: 127,
        says: 'could not run wg-no-such-command-anywhere',
      },
    ];
    for (const row of rows) {
      rmSync(parent, { recursive: true, force: true });
      parent = mkdtempSync(join(tmpdir(), 'wg-own-tmpdir-spec-'));
      const run = row.run();
      if (row.status !== undefined) expect.soft(run.status, row.label).toBe(row.status);
      if (row.signal !== undefined) expect.soft(run.signal, row.label).toBe(row.signal);
      if (row.says) expect.soft(run.stderr, row.label).toContain(row.says);
      expect.soft(readdirSync(parent), row.label).toEqual([]);
    }
  });

  it('refuses to run nothing', () => {
    const run = spawnSync(process.execPath, [SCRIPT], { encoding: 'utf8' });
    expect(run.status).toBe(2);
    expect(run.stderr).toContain('usage');
  });

});

/**
 * THROUGH A SYMLINK (cold review of 02ce616). The CLI used to decide whether it
 * was the main module by comparing `import.meta.url` — which Node resolves
 * through symlinks — with `process.argv[1]`, which it does not: run through a
 * symlinked path, the wrapper exited 0 and never ran the command, so a failing
 * suite behind it "passed". The CLI now has no such check; this runs it the
 * way that broke, with a command that FAILS, and asks both halves: the exit
 * code came through, and the command actually ran.
 */
describe('own-tmpdir through a symlinked path', () => {
  let links = '';
  beforeEach(() => {
    links = mkdtempSync(join(tmpdir(), 'wg-own-tmpdir-link-'));
  });
  afterEach(() => {
    rmSync(links, { recursive: true, force: true });
  });

  it('runs the command and passes its failure and its success through', () => {
    const link = join(links, 'own-tmpdir.mjs');
    symlinkSync(SCRIPT, link);
    const marker = join(links, 'ran');
    const env = { ...process.env, OWN_TMPDIR_PARENT: parent, TMPDIR: parent, TMP: parent, TEMP: parent };
    const failed = spawnSync(
      process.execPath,
      [
        link,
        process.execPath,
        '-e',
        `require('node:fs').writeFileSync(${JSON.stringify(marker)}, 'ran'); process.exit(3);`,
      ],
      { encoding: 'utf8', env },
    );
    expect.soft(failed.status, 'failure').toBe(3);
    expect.soft(existsSync(marker), 'the command never ran').toBe(true);
    expect.soft(existsSync(marker) && readFileSync(marker, 'utf8'), 'failure').toBe('ran');
    expect.soft(readdirSync(parent), 'failure').toEqual([]);
    const passed = spawnSync(process.execPath, [link, process.execPath, '-e', 'process.stdout.write("ok")'], { encoding: 'utf8', env });
    expect.soft(passed.status, 'success').toBe(0);
    expect.soft(passed.stdout, 'success').toBe('ok');
  });
});
