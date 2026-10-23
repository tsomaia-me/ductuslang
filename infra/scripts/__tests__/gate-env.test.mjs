import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, realpathSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { assertStubsRun, childEnv } from './hermetic.mjs';

const SCRIPT = fileURLToPath(new URL('../gate-env.sh', import.meta.url));
const SHARED_STICKY_TMP = '/tmp';
const OWNER_ONLY = 0o700;
const GROUP_WRITABLE = 0o775;
const WRITABLE_BY_OTHERS = 0o022;
const STICKY = 0o1000;
const STICKY_WORLD_WRITABLE = '1777';
const LOOSE_UMASK = '002';
const STAT_STUB = `#!/usr/bin/env bash
if [ "$*" = "-c %a %u $STAT_STUB_DIR" ]; then
  echo "$STAT_STUB_REPORT"
  exit 0
fi
exec "$STAT_STUB_REAL" "$@"
`;

let root;
let sharedTmp;
const owned = [];

const ownedDir = (parent, prefix) => {
  const dir = mkdtempSync(join(parent, prefix));
  chmodSync(dir, OWNER_ONLY);
  owned.push(dir);
  return dir;
};

beforeEach(() => {
  root = ownedDir(tmpdir(), 'gate-env-');
  sharedTmp = join(root, 'shared-tmp');
  mkdirSync(sharedTmp, { mode: OWNER_ONLY });
});

afterEach(() => {
  for (const dir of owned.splice(0)) rmSync(dir, { recursive: true, force: true });
});

const statStub = (env, reported) => {
  const bin = join(root, 'bin');
  mkdirSync(bin, { recursive: true });
  writeFileSync(join(bin, 'stat'), STAT_STUB, { mode: 0o755 });
  assertStubsRun(bin);
  const real = spawnSync('bash', ['-c', 'command -v stat'], { encoding: 'utf8' }).stdout.trim();
  Object.assign(env, {
    PATH: `${bin}:${env.PATH}`,
    STAT_STUB_REAL: real,
    STAT_STUB_DIR: realpathSync(env.TMPDIR),
    STAT_STUB_REPORT: reported,
  });
};

const gateEnv = ({ tmp = join(root, 'tmp'), cache, statReports } = {}) => {
  mkdirSync(tmp, { recursive: true });
  const env = childEnv({ TMPDIR: tmp, GATE_SHARED_TMP: sharedTmp, GATE_LOG_DIR: join(root, 'logs') });
  delete env.SWC_NATIVE_BINDING_CACHE;
  if (cache) env.SWC_NATIVE_BINDING_CACHE = cache;
  if (statReports) statStub(env, statReports);
  const run = spawnSync('bash', ['-c', `umask ${LOOSE_UMASK}; . "$1" && printf '%s' "$cache"`, 'gate-env', SCRIPT], { cwd: root, env, encoding: 'utf8' });
  return { status: run.status, cache: run.stdout };
};

const mode = (path) => statSync(path).mode & 0o7777;

const untrustedAncestors = (path) => {
  const found = [];
  for (let dir = dirname(path); ; dir = dirname(dir)) {
    const bits = mode(dir);
    if (bits & WRITABLE_BY_OTHERS && !(bits & STICKY)) found.push(dir);
    if (dir === dirname(dir)) return found;
  }
};

describe('gate-env.sh: the swc native binding cache', () => {
  it('makes the swc cache owner-only under an owner-only folder it creates in a trusted TMPDIR (also when that folder was left group-writable) or in a sticky world-writable TMPDIR this user owns, and keeps it out of a group-writable TMPDIR or a sticky world-writable one another user owns, so no folder above it is writable by others', () => {
    expect(mode(SHARED_STICKY_TMP) & STICKY).toBe(STICKY);
    const fallback = join(sharedTmp, `swc-binding-${process.getuid()}`, 'cache');
    const trusted = ownedDir(SHARED_STICKY_TMP, 'gate-env-trusted-');
    const leftover = join(trusted, `swc-binding-${process.getuid()}`);
    mkdirSync(leftover, { mode: GROUP_WRITABLE });
    chmodSync(leftover, GROUP_WRITABLE);
    const groupWritable = join(root, 'shared');
    mkdirSync(groupWritable);
    chmodSync(groupWritable, GROUP_WRITABLE);
    const sticky = join(root, 'sticky');
    mkdirSync(sticky, { mode: OWNER_ONLY });
    const rows = [
      { label: 'a trusted TMPDIR', tmp: trusted, cache: join(leftover, 'cache'), ownerOnly: true },
      { label: 'a group-writable TMPDIR', tmp: groupWritable, cache: fallback, ownerOnly: true },
      { label: "another user's sticky TMPDIR", tmp: sticky, statReports: `${STICKY_WORLD_WRITABLE} ${process.getuid() + 1}`, cache: fallback },
      { label: "this user's sticky TMPDIR", tmp: sticky, statReports: `${STICKY_WORLD_WRITABLE} ${process.getuid()}`, cache: join(realpathSync(sticky), `swc-binding-${process.getuid()}`, 'cache') },
    ];
    for (const row of rows) {
      rmSync(join(sharedTmp, `swc-binding-${process.getuid()}`), { recursive: true, force: true });
      const { status, cache } = gateEnv({ tmp: row.tmp, statReports: row.statReports });
      expect.soft(status, row.label).toBe(0);
      expect.soft(cache, row.label).toBe(row.cache);
      if (row.ownerOnly) {
        expect.soft(mode(cache), row.label).toBe(OWNER_ONLY);
        expect.soft(mode(dirname(cache)), row.label).toBe(OWNER_ONLY);
        expect.soft(untrustedAncestors(cache), row.label).toEqual([]);
      }
    }
  });

  it('uses a cache the caller names and makes it owner-only', () => {
    const cache = join(root, 'mine');
    mkdirSync(cache, { mode: GROUP_WRITABLE });
    chmodSync(cache, GROUP_WRITABLE);
    expect(gateEnv({ cache }).cache).toBe(cache);
    expect(mode(cache)).toBe(OWNER_ONLY);
  });
});
