/**
 * `wb-release-user.sh` (CD-269; owner, 8 Oct): the `wb-release` runner gets an OS user of its own,
 * `ghrelease`, with its own HOME, runner tool cache and pnpm store, and nothing copied from the
 * shared account's HOME. The real script runs against a throwaway root (WB_RELEASE_HOME_ROOT),
 * with the commands only root may run (useradd, install -o, sudo, systemctl, chown) as stubs that
 * record their arguments and keep the state a real run would leave.
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { assertStubsRun, childEnv } from './hermetic.mjs';

const SCRIPT = fileURLToPath(new URL('../wb-release-user.sh', import.meta.url));
const SHA = 'ab'.repeat(32);
const UNIT = 'actions.runner.parlornights-CrookedDuke.wb-release-1.service';

let root;
let homes;
let bin;
let log;
let state;
let units;

const stub = (name, body) => writeFileSync(join(bin, name), `#!/usr/bin/env bash\necho "${name} $*" >>"${log}"\n${body}\n`, { mode: 0o755 });

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), 'wb-release-user-'));
  homes = join(root, 'home');
  bin = join(root, 'bin');
  state = join(root, 'state');
  log = join(root, 'calls.log');
  units = join(root, 'units');
  for (const dir of [homes, bin, state, units, join(homes, 'owner', 'actions-runner-release')]) mkdirSync(dir, { recursive: true });
  // The old runner's service, installed by the shared account under the name the new runner gets too.
  writeFileSync(join(units, UNIT), '[Service]\nUser=owner\n');
  writeFileSync(join(homes, 'owner', 'actions-runner-release', 'svc.sh'), `#!/usr/bin/env bash\necho "old-svc.sh $*" >>"${log}"\n`, { mode: 0o755 });
  writeFileSync(join(homes, 'owner', '.gitconfig'), '[core]\n\thooksPath = /tmp/evil\n');
  assertStubsRun(bin);
  // Root, unless the test says otherwise; ghrelease exists once useradd ran.
  stub('id', `if [[ "$*" == "-u" ]]; then cat "${state}/uid" 2>/dev/null || echo 0; exit 0; fi\n[[ -f "${state}/user" ]]`);
  stub('useradd', `touch "${state}/user"`);
  stub('install', `dir="\${@: -1}"; mkdir -p "$dir"; echo "$dir" >>"${state}/owned"`);
  stub('stat', `grep -qx "\${@: -1}" "${state}/owned" 2>/dev/null && echo "ghrelease 700" || echo "owner 755"`);
  stub('chown', '');
  stub('sha256sum', `echo "${SHA}  $1"`);
  stub(
    'tar',
    `dir="\${@: -1}"
printf '#!/usr/bin/env bash\\ntouch .runner\\n' >"$dir/config.sh"
printf '#!/usr/bin/env bash\\necho "svc.sh $*" >>"${log}"\\nif [[ "$1" == install ]]; then [[ -f "${units}/${UNIT}" ]] && { echo "error: exists ${units}/${UNIT}"; exit 1; }; printf "[Service]\\\\nUser=$2\\\\n" >"${units}/${UNIT}"; touch "${state}/service"; fi\\nexit 0\\n' >"$dir/svc.sh"
chmod +x "$dir/config.sh" "$dir/svc.sh"`,
  );
  stub(
    'sudo',
    `args=("$@")
for arg in "$@"; do [[ "$arg" == HOME=* ]] && home="\${arg#HOME=}"; done
case "$*" in
  *"node --version"*) echo v22.11.0 ;;
  *config.sh*) ./config.sh ;;
  *"playwright@"*) mkdir -p "$home/.cache/ms-playwright/chromium-1" ;;
esac`,
  );
  stub('systemctl', `case "$1" in show) [[ -f "${units}/${UNIT}" ]] && sed -n 's/^User=//p' "${units}/${UNIT}" ;; is-active) [[ -f "${state}/service" ]] ;; esac; exit 0`);
  // Chromium's libraries: present unless a test lists some in state/missing; apt-get installs them.
  stub('dpkg', `[[ "$1" == -s ]] && ! grep -qx "$2" "${state}/missing" 2>/dev/null`);
  stub('apt-get', `echo "DEBIAN_FRONTEND=$DEBIAN_FRONTEND" >>"${log}"; : >"${state}/missing"`);
  stub('getent', `[[ "$2" == owner ]] && echo "owner:x:1000:1000::${homes}/owner:/bin/bash"`);
});
afterEach(() => rmSync(root, { recursive: true, force: true }));

const setup = (args, { uid } = {}) => {
  if (uid !== undefined) writeFileSync(join(state, 'uid'), `${uid}\n`);
  const run = spawnSync('bash', [SCRIPT, ...args], {
    encoding: 'utf8',
    env: childEnv({ PATH: `${bin}:${process.env.PATH}`, WB_RELEASE_HOME_ROOT: homes, WB_RELEASE_UNIT_DIR: units, SUDO_USER: 'owner' }),
  });
  return { ...run, calls: existsSync(log) ? readFileSync(log, 'utf8') : '' };
};
const FULL = ['--from', 'owner', '--token', 'AAA-registration', '--tarball', '/tmp/actions-runner-linux-x64.tar.gz', '--sha256', SHA];

describe('wb-release-user.sh: wb-release runs under its own OS user (CD-269; owner, 8 Oct)', () => {
  const LIBS = ['libatk1.0-0t64', 'libatk-bridge2.0-0t64', 'libatspi2.0-0t64', 'libxcomposite1', 'libxdamage1', 'libxfixes3', 'libxrandr2', 'libgbm1'];
  const missing = (list) => writeFileSync(join(state, 'missing'), list.join('\n') + '\n');

  it('a dry run says what it would do, the missing Chromium libraries too, and changes nothing, without root', () => {
    missing(LIBS);
    const run = setup(['--dry-run'], { uid: 1000 });
    expect.soft(run.status, run.stderr).toBe(0);
    expect.soft(run.stdout).toContain('would create the user ghrelease (system user, own group, no skeleton copied)');
    expect.soft(run.stdout).toContain(`would create ${homes}/ghrelease/.pnpm-store (ghrelease, 0700, empty)`);
    expect.soft(run.stdout).toContain('would register wb-release-1, label wb-release, replacing the old registration');
    expect.soft(run.stdout).toContain(`would retire the old runner's service ${UNIT} (runs as owner): disable --now, remove its unit file, daemon-reload`);
    expect.soft(run.stdout).toContain(`would remove the old runner folder ${homes}/owner/actions-runner-release`);
    expect.soft(run.stdout).toContain('wb-release-user: dry run — nothing changed');
    expect.soft(run.calls).not.toMatch(/^(useradd|install|chown|sudo|tar|old-svc\.sh|svc\.sh) |^systemctl (disable|daemon-reload)/m);
    expect.soft(existsSync(join(units, UNIT))).toBe(true);
    expect.soft(existsSync(join(homes, 'ghrelease'))).toBe(false);
    expect.soft(existsSync(join(homes, 'owner', 'actions-runner-release'))).toBe(true);
    expect.soft(run.stdout).toContain(`would install Chromium's system libraries (${LIBS.join(' ')}) with apt`);
    expect.soft(run.calls).not.toContain('apt-get');
  });

  it('refuses to change anything without root, and a --from that is not the shared account', () => {
    expect(setup(FULL, { uid: 1000 }).status).toBe(2);
    expect(setup([...FULL, '--from', 'ghrelease']).status).toBe(2);
    expect(existsSync(log) ? readFileSync(log, 'utf8') : '').not.toMatch(/^useradd /m);
  });

  it('in one run: the user, its own HOME, pnpm store and TMPDIR, a fresh runner, the old service retired by systemd before the new one installs, then the old folder, with Chromium\'s libraries already there; a second run changes nothing and says ok for every item', () => {
    const run = setup(FULL);
    expect(run.status, run.stdout + run.stderr).toBe(0);
    expect.soft(run.calls).toContain(`useradd --system --user-group --no-create-home --home-dir ${homes}/ghrelease --shell /bin/bash ghrelease`);
    for (const dir of ['', '/.pnpm-store', '/wg-verify', '/wg-verify/tmp']) expect.soft(run.calls, dir).toContain(`install -d -o ghrelease -g ghrelease -m 0700 ${homes}/ghrelease${dir}\n`);
    // A fresh runner from the checked tarball, registered as ghrelease with a clean environment.
    expect.soft(run.calls).toContain(`tar -xzf /tmp/actions-runner-linux-x64.tar.gz -C ${homes}/ghrelease/actions-runner-release`);
    expect.soft(run.calls).toMatch(
      new RegExp(`sudo -u ghrelease -H env -i HOME=${homes}/ghrelease USER=ghrelease LOGNAME=ghrelease PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin LANG=C\\.UTF-8 \\./config\\.sh --url https://github\\.com/parlornights/CrookedDuke --token AAA-registration --name wb-release-1 --labels wb-release --unattended --replace`),
    );
    // The old unit is the new one's name: svc.sh install refuses while it exists, so it goes first, through systemd.
    const calls = run.calls.split('\n');
    const retired = calls.indexOf(`systemctl disable --now ${UNIT}`);
    expect.soft(retired).toBeGreaterThan(-1);
    expect.soft(calls.indexOf('systemctl daemon-reload')).toBeGreaterThan(retired);
    expect.soft(calls.indexOf('svc.sh install ghrelease')).toBeGreaterThan(retired);
    expect.soft(readFileSync(join(units, UNIT), 'utf8')).toContain('User=ghrelease');
    expect.soft(readFileSync(join(homes, 'ghrelease', 'actions-runner-release', '.env'), 'utf8')).toBe(`npm_config_store_dir=${homes}/ghrelease/.pnpm-store\n`);
    expect.soft(run.calls).toMatch(/sudo .*npx --yes playwright@1\.56\.0 install chromium/);
    // Root runs nothing from the shared account's folder; the folder goes last; the rest of that HOME is untouched.
    expect.soft(run.calls).not.toContain('old-svc.sh');
    expect.soft(existsSync(join(homes, 'owner', 'actions-runner-release'))).toBe(false);
    expect.soft(readFileSync(join(homes, 'owner', '.gitconfig'), 'utf8')).toContain('hooksPath');
    expect.soft(existsSync(join(homes, 'ghrelease', '.gitconfig'))).toBe(false);
    expect.soft(run.stdout).toContain('wb-release-user: ok');
    expect.soft(run.stdout).toContain("ok    Chromium's system libraries");
    expect.soft(run.calls).not.toContain('apt-get');
    writeFileSync(log, '');
    const again = setup(['--from', 'owner']);
    expect.soft(again.status, again.stdout).toBe(0);
    expect.soft(again.stdout.split('\n').filter(Boolean).every((line) => /^ok /.test(line) || line === 'wb-release-user: ok'), again.stdout).toBe(true);
    expect.soft(again.calls).not.toMatch(/^(useradd|install|chown|tar|svc\.sh) /m);
  });

  it("needs --from when the shared account's runner folder is not where SUDO_USER's HOME says, and a --from that is no account", () => {
    rmSync(join(homes, 'owner', 'actions-runner-release'), { recursive: true });
    const guessed = setup(['--dry-run']);
    expect(guessed.stdout).toContain(`need  no runner folder at ${homes}/owner/actions-runner-release: name the shared account that ran it with --from`);
    expect(guessed.stdout).not.toMatch(/^ok +no runner left/m);
    expect(setup(['--dry-run', '--from', 'owner']).stdout).toContain(`ok    no runner left in owner's HOME (${homes}/owner)`);
    expect(setup(['--dry-run', '--from', 'nobody']).stdout).toContain('need  no account nobody on this machine');
  });

  describe("Chromium's system libraries", () => {
    it('installs the missing ones with apt as root before Chromium, says did, and a second run says ok', () => {
      missing(['libgbm1', 'libxdamage1']);
      const run = setup(FULL);
      expect(run.status, run.stdout + run.stderr).toBe(0);
      expect(run.stdout).toContain("did   install Chromium's system libraries (libxdamage1 libgbm1) with apt");
      expect(run.calls).toContain('apt-get install -y libxdamage1 libgbm1');
      expect(run.calls).toContain('DEBIAN_FRONTEND=noninteractive');
      for (const lib of LIBS) expect(run.calls).toContain(`dpkg -s ${lib}\n`);
      expect(run.calls.indexOf('apt-get install')).toBeLessThan(run.calls.indexOf('playwright@'));
      writeFileSync(log, '');
      const again = setup(['--from', 'owner']);
      expect(again.stdout).toContain("ok    Chromium's system libraries");
      expect(again.calls).not.toContain('apt-get');
    });

    it('says need and exits 1 when apt-get fails', () => {
      missing(['libgbm1']);
      stub('apt-get', 'exit 1');
      const run = setup(FULL);
      expect(run.status).toBe(1);
      expect(run.stdout).toContain("need  install Chromium's system libraries (libgbm1) with apt (failed)");
    });

  });

  it('refuses a tarball whose SHA-256 does not match, and keeps the old runner', () => {
    const run = setup(['--from', 'owner', '--token', 'T', '--tarball', '/tmp/x.tar.gz', '--sha256', 'cd'.repeat(32)]);
    expect(run.status).toBe(1);
    expect(run.stdout).toContain('need  unpack a fresh runner');
    expect(run.calls).not.toMatch(/^tar |^systemctl disable/m);
    expect(existsSync(join(homes, 'owner', 'actions-runner-release'))).toBe(true);
    expect(existsSync(join(units, UNIT))).toBe(true);
  });

  it('copies nothing from the shared HOME, and adds no hidepid, runner group or ephemeral runner (the owner\'s middle ground)', () => {
    const script = readFileSync(SCRIPT, 'utf8').split('\n').filter((line) => !line.startsWith('#')).join('\n');
    expect(script).not.toMatch(/\b(cp|rsync|ln)\s|hidepid|--ephemeral|runner.?group/i);
  });
});
