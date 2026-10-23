#!/usr/bin/env node
import { existsSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { routeAccessHeaders } from '../e2e/access.mjs';
import { READ_METHODS, refuseWrite } from '../e2e/write-guard.mjs';
import { accessHeaders } from '../scripts/access-headers.mjs';

import { CHECKS, checkPage, settleLayout, watchConsole } from './interaction-checks.mjs';

export { READ_METHODS, refuseWrite };

export const SCREEN_CHECK = 'screen';
export const EXIT = { clean: 0, findings: 1, usage: 2, failed: 3 };

export const VIEWPORTS = {
  desktop: { width: 1440, height: 900 },
  phone: { width: 390, height: 844 },
};

export const USAGE = `usage: interaction-check.mjs --base <url> --path </a> [--path </b> ...]
         [--paths </a,/b>] [--header "Name: value" ...] [--access-headers]
         [--viewports desktop,phone] [--checks ${CHECKS.join(',')}] [--allow-writes]

Opens each address in headless Chromium and reports, as JSON on stdout, what the interaction checks
find: a table cell whose content runs past its box, text cut with an ellipsis that has no tooltip
or title with its full value, an overlay left open after pressing outside it (or a button that
opens a dialog), a row click that moves to another page, and console or page errors. An address
whose page ends on another origin than --base (a sign-in), or whose document is not a 2xx, is a
finding of the check "${SCREEN_CHECK}" and is never checked further; that check always runs.
The exit status is ${EXIT.clean} when clean, ${EXIT.findings} when there are findings, ${EXIT.usage} on a usage error and
${EXIT.failed} when the run itself failed (no browser, the base unreachable), which prints no report.
Writes (any request but GET, HEAD, OPTIONS) are refused in the browser unless --allow-writes is
given. --access-headers sends the Cloudflare Access service token from CF_ACCESS_CLIENT_ID and
CF_ACCESS_CLIENT_SECRET. Headers, --header's and the token alike, go only to the origin of --base,
never to another origin a page loads or a redirect leads to.`;

const BROWSER_DIRS = [
  process.env['PLAYWRIGHT_BROWSERS_PATH'],
  join(homedir(), '.cache', 'ms-playwright'),
  '/opt/pw-browsers',
];
const SETTLE_MS = 300;
const DOCUMENT = 'the document';

export function browsersPath() {
  return BROWSER_DIRS.find(
    (dir) =>
      dir && existsSync(dir) && readdirSync(dir).some((name) => name.startsWith('chromium-')),
  );
}

export function useInstalledBrowsers() {
  const found = browsersPath();
  if (found) process.env['PLAYWRIGHT_BROWSERS_PATH'] = found;
}

export async function launchChromium(options = {}) {
  useInstalledBrowsers();
  const { chromium } = await import('playwright');
  return chromium.launch(options);
}

export async function refuseWrites(context) {
  await context.route('**/*', (route) => (READ_METHODS.has(route.request().method()) ? route.fallback() : refuseWrite(route)));
}

function unseenBecause(response, page, base) {
  const origin = new URL(page.url()).origin;
  const expected = new URL(base).origin;
  if (origin !== expected) return `the page ended on ${origin}, not on ${expected}`;
  if (!response) return 'the document gave no response';
  if (!response.ok()) return `the document answered ${response.status()}`;
  return null;
}

export async function runInteractionCheck({
  base,
  paths,
  headers = {},
  viewports = Object.keys(VIEWPORTS),
  only = CHECKS,
  allowWrites = false,
  browser = null,
}) {
  const own = browser ?? (await launchChromium());
  const screens = [];
  const findings = [];
  try {
    for (const label of viewports) {
      const context = await own.newContext({ viewport: VIEWPORTS[label], colorScheme: 'dark' });
      await routeAccessHeaders(context, [new URL(base).origin], headers);
      if (!allowWrites) await refuseWrites(context);
      const page = await context.newPage();
      const watch = watchConsole(page);
      for (const path of paths) {
        watch.take();
        const response = await page.goto(new URL(path, base).href, { waitUntil: 'networkidle' });
        await settleLayout(page, SETTLE_MS);
        screens.push({ screen: path, viewport: label });
        const unseen = unseenBecause(response, page, base);
        if (unseen) {
          findings.push({
            screen: path,
            viewport: label,
            check: SCREEN_CHECK,
            element: DOCUMENT,
            detail: unseen,
          });
          continue;
        }
        findings.push(...(await checkPage(page, { screen: path, viewport: label, watch, only })));
      }
      await context.close();
    }
  } finally {
    if (!browser) await own.close();
  }
  return { screens, findings };
}

export function parseArgs(argv) {
  const paths = [];
  const headers = {};
  let base;
  let viewports;
  let only;
  let allowWrites = false;
  let useAccess = false;
  const list = (value) => value.split(',').filter(Boolean);
  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    const value = () => {
      i += 1;
      if (argv[i] === undefined) throw new Error(`${flag} needs a value`);
      return argv[i];
    };
    if (flag === '--base') base = value();
    else if (flag === '--path') paths.push(value());
    else if (flag === '--paths') paths.push(...list(value()));
    else if (flag === '--header') {
      const [name, ...rest] = value().split(':');
      if (rest.length === 0) throw new Error('--header needs "Name: value"');
      headers[name.trim()] = rest.join(':').trim();
    } else if (flag === '--access-headers') useAccess = true;
    else if (flag === '--viewports') viewports = list(value());
    else if (flag === '--checks') only = list(value());
    else if (flag === '--allow-writes') allowWrites = true;
    else throw new Error(`unknown option ${flag}`);
  }
  if (!base) throw new Error('--base is required');
  if (paths.length === 0) throw new Error('give at least one --path');
  for (const label of viewports ?? [])
    if (!VIEWPORTS[label]) throw new Error(`no viewport ${label}`);
  for (const check of only ?? []) if (!CHECKS.includes(check)) throw new Error(`no check ${check}`);
  return {
    base,
    paths,
    headers: { ...(useAccess ? accessHeaders() : {}), ...headers },
    ...(viewports ? { viewports } : {}),
    ...(only ? { only } : {}),
    allowWrites,
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(`${error.message}\n${USAGE}`);
    process.exit(EXIT.usage);
  }
  let result;
  try {
    result = await runInteractionCheck(options);
  } catch (error) {
    console.error(`the run failed: ${String(error.message).split('\n')[0]}`);
    process.exit(EXIT.failed);
  }
  console.log(JSON.stringify(result, null, 2));
  process.exit(result.findings.length > 0 ? EXIT.findings : EXIT.clean);
}
