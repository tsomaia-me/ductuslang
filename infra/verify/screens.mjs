import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

import { graphScope, importGraph, platformStem } from './import-graph.mjs';
import { routeSegments } from './spec-routes.mjs';
import { EVERY_CLIENT, NOT_PRODUCT, SERVED, SURFACES } from './surfaces.mjs';

export const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const CODE = /\.[cm]?[jt]sx?$/;
const EXTENSIONS = ['.tsx', '.ts', '.jsx', '.js'];
const require = createRequire(import.meta.url);
let swc = null;

export function parse(source, file) {
  swc ??= require('@swc/core');
  return swc.parseSync(source, { syntax: 'typescript', tsx: /\.[jt]sx$/.test(file) });
}

export function walk(node, visit) {
  if (node === null || typeof node !== 'object') return;
  if (Array.isArray(node)) {
    for (const child of node) walk(child, visit);
    return;
  }
  visit(node);
  for (const [key, child] of Object.entries(node)) if (key !== 'span' && key !== 'ctxt') walk(child, visit);
}

const keyName = (key) => (key?.type === 'Identifier' ? key.value : key?.type === 'StringLiteral' ? key.value : null);

export const property = (object, name) =>
  object?.properties?.find((one) => one.type === 'KeyValueProperty' && keyName(one.key) === name)?.value;

export function filesUnder(root, dir) {
  const full = join(root, dir);
  if (!existsSync(full)) return [];
  return readdirSync(full, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? filesUnder(root, `${dir}${entry.name}/`) : [`${dir}${entry.name}`],
  );
}

function fileScreens(root, { dir, skip }) {
  const screens = new Map();
  for (const file of filesUnder(root, dir)) {
    if (!CODE.test(file) || skip.some((prefix) => file.startsWith(prefix))) continue;
    if (NOT_PRODUCT.some((pattern) => pattern.test(file))) continue;
    const segments = routeSegments(file);
    if (segments === null) continue;
    const id = `/${segments.join('/')}`;
    screens.set(id, [...(screens.get(id) ?? []), file]);
  }
  return screens;
}

export function resolveImport(from, specifier, root) {
  if (!specifier.startsWith('.')) return null;
  const base = normalize(join(dirname(from), specifier));
  for (const candidate of [base, ...EXTENSIONS.map((ext) => `${base}${ext}`), ...EXTENSIONS.map((ext) => `${base}/index${ext}`)])
    if (CODE.test(candidate) && existsSync(join(root, candidate))) return candidate;
  return null;
}

function declaredRoutes(file, root, found) {
  const imports = new Map();
  const ref = (name) => {
    const imported = imports.get(name);
    return imported === undefined ? `${file}#${name}` : `${imported.from}#${imported.name}`;
  };
  const component = (options) => {
    const name = property(options, 'component')?.value ?? null;
    return name === null ? undefined : (imports.get(name)?.from ?? null);
  };
  walk(parse(readFileSync(join(root, file), 'utf8'), file), (node) => {
    if (node.type === 'ImportDeclaration')
      for (const specifier of node.specifiers ?? [])
        imports.set(specifier.local.value, {
          from: resolveImport(file, node.source.value, root),
          name: specifier.type === 'ImportDefaultSpecifier' ? 'default' : (specifier.imported?.value ?? specifier.local.value),
        });
    if (node.type === 'CallExpression' && node.callee?.type === 'MemberExpression' && node.callee.property?.value === 'update' && node.callee.object?.type === 'Identifier') {
      const updated = component(node.arguments[0]?.expression);
      if (updated !== undefined) found.updates.set(ref(node.callee.object.value), updated);
      return;
    }
    if (node.type !== 'VariableDeclarator' || node.id?.type !== 'Identifier' || node.init?.type !== 'CallExpression') return;
    const call = node.init;
    if (call.callee?.type === 'CallExpression' && call.callee.callee?.value === 'createRootRouteWithContext') {
      found.root = ref(node.id.value);
      found.rootComponent = component(call.arguments[0]?.expression);
      return;
    }
    if (call.callee?.value !== 'createRoute') return;
    const options = call.arguments[0]?.expression;
    const parent = property(options, 'getParentRoute');
    found.routes.set(ref(node.id.value), {
      parent: parent?.body?.type === 'Identifier' ? ref(parent.body.value) : null,
      path: property(options, 'path')?.value ?? null,
      component: component(options),
    });
  });
  return [...new Set([...imports.values()].map((imported) => imported.from))].filter(Boolean);
}

export function tanstackRoutes(file, root) {
  const found = { routes: new Map(), updates: new Map(), root: null, rootComponent: undefined };
  for (const imported of declaredRoutes(file, root, found)) declaredRoutes(imported, root, found);
  const { routes, updates, root: rootName } = found;
  const componentOf = (name, own) => (updates.has(name) ? updates.get(name) : own);
  const rootComponent = componentOf(rootName, found.rootComponent);
  const fullPath = (name) => {
    const route = routes.get(name);
    if (route === undefined) return '';
    const above = route.parent === rootName ? '' : fullPath(route.parent);
    if (route.path === '/' || route.path === null) return above || '/';
    return `${above}/${route.path.replace(/^\//, '')}`.replace(/\/+/g, '/');
  };
  const renderers = (name) => {
    const route = routes.get(name);
    if (route === undefined) return [];
    const component = componentOf(name, route.component);
    return [...(component === undefined ? [] : [component]), ...renderers(route.parent)];
  };
  const parents = new Set([...routes.values()].map((route) => route.parent));
  const screens = new Map();
  for (const [name, route] of routes) {
    if (parents.has(name) || componentOf(name, route.component) === undefined) continue;
    screens.set(fullPath(name), renderers(name));
  }
  return { screens, shell: rootComponent === undefined ? [] : [rootComponent] };
}

export function catalogue(name, root = REPO_ROOT) {
  const surface = SURFACES[name];
  if (surface.router.kind === 'files') return { screens: fileScreens(root, surface.router), shell: surface.shell };
  return tanstackRoutes(surface.router.file, root);
}

export function navigation(name, root = REPO_ROOT) {
  const { file, key } = SURFACES[name].navigation;
  const found = [];
  walk(parse(readFileSync(join(root, file), 'utf8'), file), (node) => {
    if (node.type !== 'ObjectExpression') return;
    const value = property(node, key);
    if (value?.type === 'StringLiteral') found.push(value.value);
  });
  return [...new Set(found)];
}

function shellFiles(shell, graph) {
  const byPath = new Map(graph.modules.map((module) => [module.source, module]));
  const direct = shell.flatMap((file) => [file, ...(byPath.get(file)?.dependencies ?? []).map((dep) => dep.resolved)]);
  return new Set(direct.filter(Boolean).map(platformStem));
}

function closure(starts, graph) {
  const byPath = new Map(graph.modules.map((module) => [module.source, module]));
  const byStem = new Map();
  for (const source of byPath.keys()) byStem.set(platformStem(source), [...(byStem.get(platformStem(source)) ?? []), source]);
  const seen = new Set();
  const queue = starts.filter(Boolean);
  while (queue.length > 0) {
    const current = queue.pop();
    for (const variant of byStem.get(platformStem(current)) ?? [current]) {
      if (seen.has(variant)) continue;
      seen.add(variant);
      for (const dep of byPath.get(variant)?.dependencies ?? []) if (byPath.has(dep.resolved)) queue.push(dep.resolved);
    }
  }
  return new Set([...seen].map(platformStem));
}

export function placeOnSurface(name, paths, { root = REPO_ROOT, graph = importGraph(root, SURFACES[name].dir) } = {}) {
  const surface = SURFACES[name];
  const { screens, shell } = catalogue(name, root);
  const scope = graphScope(root, surface.dir);
  const known = new Set(graph.modules.map((module) => platformStem(module.source)));
  const registered = surface.registries.map(({ entry, lookup }) => ({ lookup: platformStem(lookup), files: closure([entry], graph) }));
  const reached = new Map(
    [...screens].map(([id, files]) => {
      const own = closure(files, graph);
      const hosted = registered.filter(({ lookup }) => own.has(lookup)).flatMap(({ files: entries }) => [...entries]);
      return [id, new Set([...own, ...hosted])];
    }),
  );
  const shellReach = shellFiles(shell, graph);
  const result = { screens: new Set(), unplaceable: [], journeys: [], placed: [], by: {} };
  for (const path of paths) {
    if (path.startsWith(surface.journeys)) {
      result.journeys.push(path);
      continue;
    }
    if (surface.ignored.some((prefix) => path.startsWith(prefix))) continue;
    if (NOT_PRODUCT.some((pattern) => pattern.test(path))) continue;
    if (EVERY_CLIENT.some((pattern) => pattern.test(path))) {
      result.unplaceable.push(path);
      continue;
    }
    const own = path.startsWith(`${surface.dir}/`);
    if (own && path.startsWith(`${surface.dir}/e2e/`)) continue;
    const inGraph = scope.some((prefix) => path.startsWith(prefix));
    if (!own && !inGraph) continue;
    const stem = platformStem(path);
    if (graph.unresolved.length > 0 || surface.unplaceable.some((prefix) => path.startsWith(prefix)) || shellReach.has(stem)) {
      result.unplaceable.push(path);
      continue;
    }
    const touched = [...reached].filter(([, files]) => files.has(stem)).map(([id]) => id);
    if (touched.length > 0) {
      for (const id of touched) {
        result.screens.add(id);
        result.by[id] = [...(result.by[id] ?? []), path];
      }
      result.placed.push(path);
    } else if (own || (inGraph && !known.has(stem) && !existsSync(join(root, path)))) result.unplaceable.push(path);
  }
  return { ...result, screens: [...result.screens].sort() };
}

export function servedBy(paths) {
  const served = new Set();
  for (const path of paths) {
    if (NOT_PRODUCT.some((pattern) => pattern.test(path))) continue;
    const rule = SERVED.find(({ pattern }) => pattern.test(path));
    if (rule) for (const surface of rule.surfaces) served.add(surface);
  }
  return served;
}
