import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join, relative } from 'node:path';

import { importGraph } from './import-graph.mjs';
import { REPO_ROOT, parse, resolveImport, walk } from './screens.mjs';
import { EVERY_CLIENT, NOT_PRODUCT } from './surfaces.mjs';

export const MODULE_ROUTES = {
  apps: ['/apps'],
  audit: ['/audit'],
  currencies: ['/currencies'],
  economy: ['/economy'],
  home: ['/'],
  insights: ['/groups'],
  items: ['/items'],
  media: ['/media'],
  moderation: ['/moderation'],
  releases: ['/releases'],
  retirements: [],
  settings: ['/settings'],
  spend: ['/spend'],
  users: ['/users'],
  words: ['/words'],
};

const ADMIN = 'apps/admin/';
const MODULES = 'apps/admin/src/modules/';
const UNRENDERED = ['apps/admin/e2e/', 'apps/admin/src/test/', 'apps/admin/scripts/'];
const HARNESS_ENTRIES = ['apps/admin/scripts/shots.mjs', 'apps/admin/scripts/proof-adapter.mjs', 'infra/verify/proof.mjs', 'infra/verify/shot-scope.mjs'];
const ROUTER = 'apps/admin/src/app/router.tsx';

const moduleOf = (file) => (file.startsWith(MODULES) ? file.slice(MODULES.length).split('/')[0] : null);
const owns = (prefix, pathname) => pathname === prefix || pathname.startsWith(`${prefix}/`);
export const ownersOf = (path, map) => {
  const pathname = path.split(/[?#]/)[0];
  return Object.keys(map).filter((name) => map[name].some((prefix) => owns(prefix, pathname)));
};

export function unmappedRoutes(routes, map) {
  return routes.flatMap((route) => {
    const owners = ownersOf(route, map);
    if (owners.length === 1) return [];
    return [`${route}: ${owners.length === 0 ? 'no module' : owners.sort().join(', ')}`];
  });
}

const isRendered = (file) => !NOT_PRODUCT.some((pattern) => pattern.test(file)) && !UNRENDERED.some((prefix) => file.startsWith(prefix));

function reach(file, { importers, packages, harness, unresolved }) {
  if (harness.includes(file)) return 'everything';
  if (file.startsWith('packages/'))
    return packages.some((dir) => file.startsWith(dir)) && !NOT_PRODUCT.some((pattern) => pattern.test(file)) ? 'everything' : [];
  if (EVERY_CLIENT.some((pattern) => pattern.test(file))) return 'everything';
  if (!file.startsWith(ADMIN) || !isRendered(file)) return [];
  if (moduleOf(file) === null || unresolved.length > 0) return 'everything';
  const users = (importers(file) ?? [file]).filter((user) => user.startsWith(ADMIN) && isRendered(user));
  if (users.some((user) => moduleOf(user) === null)) return 'everything';
  return users.map(moduleOf);
}

export function screensFor(changedFiles, screens, { map = MODULE_ROUTES, importers, packages, harness, unresolved = [] }) {
  const owned = screens.map((screen) => ({ ...screen, owners: ownersOf(screen.path, map) }));
  const loose = owned.filter(({ owners }) => owners.length !== 1).map(({ name, path }) => `${name}: ${path}`);
  if (loose.length > 0) throw new Error(`these screens belong to no module, or to more than one, in MODULE_ROUTES: ${loose.join('; ')}`);
  const reached = changedFiles.map((file) => reach(file, { importers, packages, harness, unresolved }));
  if (reached.includes('everything')) return { everything: true, modules: Object.keys(map).sort(), screens: screens.map(({ name }) => name) };
  const modules = [...new Set(reached.flat())].sort();
  return { everything: false, modules, screens: owned.filter(({ owners }) => modules.includes(owners[0])).map(({ name }) => name) };
}

export function importersFrom(graph, routers = []) {
  const direct = new Map();
  for (const module of graph.modules.filter(({ source }) => !routers.includes(source)))
    for (const dependency of module.dependencies)
      direct.set(dependency.resolved, [...(direct.get(dependency.resolved) ?? []), module.source]);
  const known = new Set([...graph.modules.map((module) => module.source), ...direct.keys()]);
  return (file) => {
    if (!known.has(file)) return null;
    const seen = new Set([file]);
    const queue = [file];
    while (queue.length > 0)
      for (const user of direct.get(queue.pop()) ?? [])
        if (!seen.has(user)) {
          seen.add(user);
          queue.push(user);
        }
    return [...seen];
  };
}

export function harnessFiles(root = REPO_ROOT) {
  const seen = new Set();
  const queue = [...HARNESS_ENTRIES];
  while (queue.length > 0) {
    const file = queue.pop();
    if (seen.has(file)) continue;
    seen.add(file);
    walk(parse(readFileSync(join(root, file), 'utf8'), file), (node) => {
      if (node.type !== 'ImportDeclaration' && node.type !== 'ExportAllDeclaration' && node.type !== 'ExportNamedDeclaration') return;
      const imported = node.source?.value && resolveImport(file, node.source.value, root);
      if (imported) queue.push(imported);
    });
  }
  return [...seen].sort();
}

function adminPackages(root) {
  const listed = JSON.parse(execFileSync('pnpm', ['--filter', '@wg/admin^...', 'list', '--depth', '-1', '--json'], { cwd: root, encoding: 'utf8' }));
  return listed.map((pkg) => `${relative(root, pkg.path)}/`).sort();
}

export function scopeOptions(root = REPO_ROOT) {
  const graph = importGraph(root, 'apps/admin');
  return { importers: importersFrom(graph, [ROUTER]), packages: adminPackages(root), harness: harnessFiles(root), unresolved: graph.unresolved };
}
