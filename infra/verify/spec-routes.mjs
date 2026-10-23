// Which routes each e2e spec opens, read from the syntax tree (swc's parser) of
// the spec and of the e2e files it imports (the imports come from the
// dependency-cruiser graph):
// - `goto(...)`: a literal path, a template up to its first interpolation, or a
//   name followed to its declarations, the `for ... of` lists it walks, the
//   arguments at its function's call sites (`fn(...)`, `obj.fn(...)`, aliases), or
//   the e2e file it is imported from;
// - tab-bar clicks (`getByTestId('nav-<key>')`), mapped through the app's NAV_ITEMS.
// Any node it does not understand counts as every route (`{ prefix: '/' }`).
// The app's own links (`href`, `router.push/replace`) are read the same way.
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, normalize } from 'node:path';

const ROUTER = 'apps/app/src/app/';
const NAV_ITEMS = 'apps/app/src/navigation/nav-items.ts';
const SPEC = /^apps\/app\/e2e\/(?:first-glance\/)?([a-z0-9-]+)\.spec\.ts$/;
const PLATFORM = /\.(?:web|native|ios|android)$/;
const EVERY_ROUTE = { prefix: '/' };
const MAX_DEPTH = 6;
const UNKNOWN = null;

// The URL segments of a route file ('[id]' for a dynamic one), or null for a
// file that is not a page (a layout, +html, +not-found).
export function routeSegments(route) {
  const name = route
    .slice(ROUTER.length)
    .replace(/\.[cm]?[jt]sx?$/, '')
    .replace(PLATFORM, '');
  const segments = name.split('/');
  if (/^[_+]/.test(segments.at(-1))) return null;
  if (segments.at(-1) === 'index') segments.pop();
  return segments.filter((segment) => !/^\(.*\)$/.test(segment));
}

const dynamic = (segment) => /^\[.*\]$/.test(segment);

// A target is `{ path }` (an exact path) or `{ prefix }` (the static start of a
// dynamic one). Query and hash end the path.
function target(text, complete) {
  const cut = text.search(/[?#]/);
  if (cut >= 0) return { path: text.slice(0, cut) || '/' };
  return complete ? { path: text } : { prefix: text };
}

// Whether a route can be the page a target opens.
export function routeMatches(segments, { path, prefix }) {
  if (path !== undefined) {
    const parts = path.split('/').filter(Boolean);
    return (
      parts.length === segments.length &&
      parts.every((part, i) => dynamic(segments[i]) || part === segments[i])
    );
  }
  const parts = prefix.slice(1).split('/');
  const partial = parts.pop();
  if (segments.length < parts.length + (partial === '' ? 0 : 1)) return false;
  if (!parts.every((part, i) => dynamic(segments[i]) || part === segments[i])) return false;
  if (partial === '') return true;
  const next = segments[parts.length];
  return dynamic(next) || next.startsWith(partial);
}

// --- the syntax tree -------------------------------------------------------------

let swc = null;
const require = createRequire(import.meta.url);

// One parsed file: its tree, each node's parent, and the nodes this reader uses.
function index(source, file) {
  swc ??= require('@swc/core');
  const tree = swc.parseSync(source, {
    syntax: 'typescript',
    tsx: /\.[jt]sx$/.test(file),
  });
  const parents = new Map();
  const found = { calls: [], declarators: [], loops: [], functions: [], imports: [] };
  const visit = (node, parent) => {
    if (node === null || typeof node !== 'object') return;
    if (Array.isArray(node)) {
      for (const child of node) visit(child, parent);
      return;
    }
    parents.set(node, parent);
    switch (node.type) {
      case 'CallExpression':
        found.calls.push(node);
        break;
      case 'VariableDeclarator':
        found.declarators.push(node);
        break;
      case 'ForOfStatement':
        found.loops.push(node);
        break;
      case 'ImportDeclaration':
        found.imports.push(node);
        break;
      case 'FunctionDeclaration':
      case 'FunctionExpression':
      case 'ArrowFunctionExpression':
      case 'MethodProperty':
      case 'ClassMethod':
        found.functions.push(node);
        break;
      default:
    }
    for (const [key, child] of Object.entries(node))
      if (key !== 'span' && key !== 'ctxt') visit(child, node);
  };
  visit(tree, null);
  return { file, tree, parents, ...found };
}

const WRAPPERS = new Set([
  'TsAsExpression',
  'TsConstAssertion',
  'TsSatisfiesExpression',
  'TsNonNullExpression',
  'TsTypeAssertion',
  'ParenthesisExpression',
  'AwaitExpression',
]);

const unwrap = (node) => {
  let current = node;
  while (current !== null && current !== undefined && WRAPPERS.has(current.type))
    current = current.expression ?? current.argument;
  return current;
};

const nameOf = (node) =>
  node?.type === 'Identifier' ? node.value : node?.type === 'StringLiteral' ? node.value : null;

const paramsOf = (fn) =>
  fn.type === 'ArrowFunctionExpression'
    ? fn.params
    : (fn.type === 'ClassMethod' ? fn.function.params : fn.params).map(
        (param) => param.pat ?? param,
      );

// Whether a binding pattern declares `name`.
function binds(pattern, name) {
  if (pattern === null || pattern === undefined) return false;
  switch (pattern.type) {
    case 'Identifier':
      return pattern.value === name;
    case 'AssignmentPattern':
      return binds(pattern.left, name);
    case 'ArrayPattern':
      return pattern.elements.some((element) => binds(element, name));
    case 'ObjectPattern':
      return pattern.properties.some((property) =>
        property.type === 'AssignmentPatternProperty'
          ? property.key.value === name
          : binds(property.value ?? property.argument, name),
      );
    case 'RestElement':
      return binds(pattern.argument, name);
    default:
      return false;
  }
}

// --- following a value ---------------------------------------------------------

// A value is `{ node, at }` (`at` the parsed file it is written in) or UNKNOWN.
const value = (node, at) => (node === null || node === undefined ? UNKNOWN : { node, at });

// What `name` holds when `pattern` is matched against `given` (a value, UNKNOWN,
// or undefined for an argument left out).
function extract(pattern, name, given, at, reader, depth) {
  switch (pattern.type) {
    case 'Identifier':
      return pattern.value === name ? [given ?? UNKNOWN] : [];
    case 'AssignmentPattern':
      return extract(
        pattern.left,
        name,
        given === undefined ? value(pattern.right, at) : given,
        at,
        reader,
        depth,
      );
    case 'ArrayPattern': {
      const position = pattern.elements.findIndex((element) => binds(element, name));
      if (position < 0) return [];
      return reader.values(given, depth).flatMap((held) => {
        const element =
          held !== UNKNOWN && held.node.type === 'ArrayExpression'
            ? held.node.elements[position]
            : undefined;
        if (held === UNKNOWN || element === undefined || element?.spread) return [UNKNOWN];
        return extract(
          pattern.elements[position],
          name,
          value(element?.expression, held.at),
          at,
          reader,
          depth,
        );
      });
    }
    case 'ObjectPattern': {
      const property = pattern.properties.find((one) =>
        one.type === 'AssignmentPatternProperty'
          ? one.key.value === name
          : binds(one.value ?? one.argument, name),
      );
      if (property === undefined) return [];
      if (property.type === 'RestElement') return [UNKNOWN];
      const key = nameOf(property.key);
      return reader.values(given, depth).flatMap((held) => {
        const entry =
          held !== UNKNOWN && held.node.type === 'ObjectExpression'
            ? held.node.properties.find(
                (one) => one.type === 'KeyValueProperty' && nameOf(one.key) === key,
              )
            : undefined;
        const fallback =
          property.type === 'AssignmentPatternProperty' ? value(property.value, at) : undefined;
        if (entry === undefined) return [held === UNKNOWN ? UNKNOWN : (fallback ?? UNKNOWN)];
        const inner = property.type === 'AssignmentPatternProperty' ? property.key : property.value;
        return extract(inner, name, value(entry.value, held.at), at, reader, depth);
      });
    }
    default:
      return [UNKNOWN];
  }
}

// The names a function is called by: its own, the variable or property it is
// assigned to, and their aliases (`const go = open`, `import { open as go }`).
function callNames(fn, at, files) {
  const parent = at.parents.get(fn);
  const names = new Set();
  if (fn.identifier?.value) names.add(fn.identifier.value);
  if (parent?.type === 'VariableDeclarator' && parent.id.type === 'Identifier')
    names.add(parent.id.value);
  if ((fn.type === 'MethodProperty' || fn.type === 'ClassMethod') && nameOf(fn.key))
    names.add(nameOf(fn.key));
  if (parent?.type === 'KeyValueProperty' && nameOf(parent.key)) names.add(nameOf(parent.key));
  for (let grew = true; grew;) {
    grew = false;
    for (const other of files) {
      for (const declarator of other.declarators)
        if (
          declarator.id.type === 'Identifier' &&
          names.has(nameOf(unwrap(declarator.init))) &&
          !names.has(declarator.id.value)
        ) {
          names.add(declarator.id.value);
          grew = true;
        }
      for (const declaration of other.imports)
        for (const specifier of declaration.specifiers)
          if (
            specifier.type === 'ImportSpecifier' &&
            names.has(specifier.imported?.value ?? specifier.local.value) &&
            !names.has(specifier.local.value)
          ) {
            names.add(specifier.local.value);
            grew = true;
          }
    }
  }
  return names;
}

const calleeName = (call) => {
  const callee = unwrap(call.callee);
  if (callee?.type === 'Identifier') return callee.value;
  if (callee?.type === 'MemberExpression' && callee.property.type === 'Identifier')
    return callee.property.value;
  return null;
};

const ITERATORS = new Set(['forEach', 'map', 'flatMap', 'filter', 'some', 'every', 'find']);

// The reader of one spec's files: what a name can hold, and what a node opens.
function reader(files) {
  const byPath = new Map(files.map((one) => [one.file, one]));
  let fulfilled = null;
  const self = {
    // The URLs the spec's files answer themselves: `page.route(url, (route) =>
    // route.fulfill(...))`.
    fulfilled() {
      if (fulfilled !== null) return fulfilled;
      fulfilled = new Set();
      for (const at of files)
        for (const call of at.calls) {
          if (calleeName(call) !== 'route' || call.arguments.length < 2) continue;
          if (!JSON.stringify(call.arguments[1]).includes('"value":"fulfill"')) continue;
          for (const held of self.values(value(call.arguments[0].expression, at), 0))
            if (held !== UNKNOWN && held.node?.type === 'StringLiteral')
              fulfilled.add(held.node.value);
        }
      return fulfilled;
    },

    // The values `name`, written at `node` in `at`, can hold.
    holds(name, node, at, depth) {
      if (depth > MAX_DEPTH) return [UNKNOWN];
      for (let up = at.parents.get(node); up; up = at.parents.get(up)) {
        if (!at.functions.includes(up)) continue;
        const params = paramsOf(up);
        const position = params.findIndex((param) => binds(param, name));
        if (position < 0) continue;
        return self.arguments(up, position, params[position], name, at, depth);
      }
      const found = [];
      for (const declarator of at.declarators) {
        if (!binds(declarator.id, name)) continue;
        const loop = at.parents.get(at.parents.get(declarator));
        if (loop?.type === 'ForOfStatement') {
          for (const list of self.values(value(loop.right, at), depth + 1)) {
            const items =
              list !== UNKNOWN && list.node.type === 'ArrayExpression' ? list.node.elements : null;
            if (items === null) found.push(UNKNOWN);
            else
              for (const item of items)
                found.push(
                  ...(item?.spread
                    ? [UNKNOWN]
                    : extract(
                        declarator.id,
                        name,
                        value(item?.expression, list.at),
                        at,
                        self,
                        depth + 1,
                      )),
                );
          }
        } else
          found.push(
            ...extract(declarator.id, name, value(declarator.init, at), at, self, depth + 1),
          );
      }
      if (found.length > 0) return found;
      for (const declaration of at.imports)
        for (const specifier of declaration.specifiers)
          if (specifier.local.value === name) {
            const from = self.resolve(at.file, declaration.source.value);
            if (from === undefined) return [UNKNOWN];
            const exported = specifier.imported?.value ?? name;
            return self.holds(exported, from.tree, from, depth + 1);
          }
      return [UNKNOWN];
    },

    // The values parameter `position` of `fn` takes: the arguments at every call
    // site in the spec's files, or the elements when `fn` is an array iterator's
    // callback. A function passed on as a value is called where this reader
    // cannot see: UNKNOWN.
    arguments(fn, position, pattern, name, at, depth) {
      const parent = at.parents.get(fn);
      const call = parent && !parent.type ? at.parents.get(parent) : null;
      if (call?.type === 'CallExpression' && call.arguments.some((arg) => arg.expression === fn)) {
        const callee = unwrap(call.callee);
        if (
          callee?.type === 'MemberExpression' &&
          ITERATORS.has(callee.property.value) &&
          position === 0
        )
          return self
            .values(value(callee.object, at), depth + 1)
            .flatMap((list) =>
              list !== UNKNOWN && list.node.type === 'ArrayExpression'
                ? list.node.elements.flatMap((item) =>
                    item?.spread
                      ? [UNKNOWN]
                      : extract(
                          pattern,
                          name,
                          value(item?.expression, list.at),
                          at,
                          self,
                          depth + 1,
                        ),
                  )
                : [UNKNOWN],
            );
        return [UNKNOWN];
      }
      const names = callNames(fn, at, files);
      if (names.size === 0) return [UNKNOWN];
      const found = [];
      for (const other of files)
        for (const site of other.calls) {
          for (const arg of site.arguments) {
            const passed = unwrap(arg.expression);
            const passedName =
              passed?.type === 'Identifier'
                ? passed.value
                : passed?.type === 'MemberExpression'
                  ? passed.property.value
                  : null;
            if (names.has(passedName)) found.push(UNKNOWN);
          }
          if (!names.has(calleeName(site))) continue;
          const arg = site.arguments[position];
          if (arg?.spread) found.push(UNKNOWN);
          else
            found.push(
              ...extract(
                pattern,
                name,
                arg === undefined ? undefined : value(arg.expression, other),
                at,
                self,
                depth + 1,
              ),
            );
        }
      return found;
    },

    // A value with names followed and wrappers removed.
    values(given, depth) {
      if (given === UNKNOWN || given === undefined || depth > MAX_DEPTH) return [UNKNOWN];
      const node = unwrap(given.node);
      if (node?.type === 'Identifier')
        return self
          .holds(node.value, node, given.at, depth + 1)
          .flatMap((held) => self.values(held, depth + 1));
      const listed = objectList(node);
      if (listed !== null)
        return self
          .values(value(node.arguments[0]?.expression, given.at), depth + 1)
          .map((object) => (object === UNKNOWN ? UNKNOWN : listOf(object.node, listed, object.at)));
      return [{ node, at: given.at }];
    },

    // The targets a value opens.
    opens(given, depth = 0) {
      return self.values(given, depth).flatMap((held) => {
        if (held === UNKNOWN) return [EVERY_ROUTE];
        const { node, at } = held;
        switch (node?.type) {
          case 'StringLiteral':
            return urlTargets(node.value, true, self.fulfilled());
          case 'TemplateLiteral':
            return urlTargets(
              node.quasis[0]?.cooked ?? node.quasis[0]?.raw ?? '',
              node.expressions.length === 0,
              self.fulfilled(),
            );
          case 'ConditionalExpression':
            return [
              ...self.opens(value(node.consequent, at), depth + 1),
              ...self.opens(value(node.alternate, at), depth + 1),
            ];
          case 'ArrayExpression':
            return node.elements.flatMap((item) =>
              item?.spread ? [EVERY_ROUTE] : self.opens(value(item?.expression, at), depth + 1),
            );
          case 'MemberExpression': {
            if (node.property.type !== 'Identifier') return [EVERY_ROUTE];
            return self.values(value(node.object, at), depth + 1).flatMap((object) => {
              const entry =
                object !== UNKNOWN && object.node.type === 'ObjectExpression'
                  ? object.node.properties.find(
                      (one) =>
                        one.type === 'KeyValueProperty' && nameOf(one.key) === node.property.value,
                    )
                  : undefined;
              return entry === undefined
                ? [EVERY_ROUTE]
                : self.opens(value(entry.value, object.at), depth + 1);
            });
          }
          default:
            return [EVERY_ROUTE];
        }
      });
    },

    // The parsed e2e file an import names, if the spec reads it.
    resolve(file, specifier) {
      if (!specifier.startsWith('.')) return undefined;
      const base = normalize(join(dirname(file), specifier)).replace(/\.[cm]?[jt]sx?$/, '');
      return ['.ts', '.tsx', '.mjs', '.js', '/index.ts']
        .map((ext) => byPath.get(`${base}${ext}`))
        .find(Boolean);
    },
  };
  return self;
}

// The hosts the app is served from in e2e: playwright.config.ts's default base URL
// is 127.0.0.1; localhost is the same machine.
const APP_HOSTS = /^(?:127\.0\.0\.1|localhost)(?::\d+)?$/;

// The targets a URL's static text opens: an app path; for an absolute http(s) URL
// on the app's host, or one the spec fulfils itself, its path; nothing for a
// file:// page, which is never the app; every route for anything else (another
// host, a host built at run time, a relative path).
function urlTargets(text, complete, fulfilled = new Set()) {
  if (text.startsWith('/')) return [target(text, complete)];
  if (text.startsWith('file:')) return [];
  const absolute = /^https?:\/\/([^/?#]+)(.*)$/.exec(text);
  if (absolute !== null && !APP_HOSTS.test(absolute[1]) && !fulfilled.has(text))
    return [EVERY_ROUTE];
  if (absolute !== null) {
    const rest = absolute[2];
    if (rest === '' && !complete) return [EVERY_ROUTE];
    return [target(rest === '' || rest.startsWith('?') ? `/${rest}` : rest, complete)];
  }
  return [EVERY_ROUTE];
}

// `Object.entries(x)` / `Object.values(x)`: which one, or null.
function objectList(node) {
  const callee = node?.type === 'CallExpression' ? unwrap(node.callee) : null;
  if (
    callee?.type !== 'MemberExpression' ||
    nameOf(callee.object) !== 'Object' ||
    !['entries', 'values'].includes(callee.property.value)
  )
    return null;
  return callee.property.value;
}

// The array `Object.entries`/`Object.values` makes of an object literal, as a
// syntax node; UNKNOWN when the object is not a literal of plain entries.
function listOf(object, listed, at) {
  if (
    object?.type !== 'ObjectExpression' ||
    !object.properties.every((one) => one.type === 'KeyValueProperty' && nameOf(one.key) !== null)
  )
    return UNKNOWN;
  const item = (expression) => ({ spread: null, expression });
  return {
    node: {
      type: 'ArrayExpression',
      elements: object.properties.map((one) =>
        listed === 'values'
          ? item(one.value)
          : item({
              type: 'ArrayExpression',
              elements: [item({ type: 'StringLiteral', value: nameOf(one.key) }), item(one.value)],
            }),
      ),
    },
    at,
  };
}

// --- what a spec opens -----------------------------------------------------------

const isGoto = (call) => {
  const callee = unwrap(call.callee);
  return callee?.type === 'MemberExpression' && callee.property.value === 'goto';
};

// The goto targets in the spec's parsed files.
function gotoTargetsIn(files) {
  const read = reader(files);
  return files.flatMap((at) =>
    at.calls
      .filter(isGoto)
      .flatMap((call) =>
        call.arguments[0] === undefined || call.arguments[0].spread
          ? [EVERY_ROUTE]
          : read.opens(value(call.arguments[0].expression, at)),
      ),
  );
}

// The goto targets of one source on its own (its names followed within it).
export function gotoTargets(source) {
  return gotoTargetsIn([index(source, 'spec.ts')]);
}

// key -> href, from the app's NAV_ITEMS.
export function navItems(source) {
  const nav = new Map();
  const visit = (node) => {
    if (node === null || typeof node !== 'object') return;
    if (Array.isArray(node)) return node.forEach(visit);
    if (node.type === 'ObjectExpression') {
      const entry = (key) =>
        node.properties.find((one) => one.type === 'KeyValueProperty' && nameOf(one.key) === key)
          ?.value;
      const key = entry('key');
      const href = entry('href');
      if (key?.type === 'StringLiteral' && href?.type === 'StringLiteral')
        nav.set(key.value, href.value);
    }
    for (const [name, child] of Object.entries(node)) if (name !== 'span') visit(child);
  };
  visit(index(source, 'nav-items.ts').tree);
  return nav;
}

// The routes a tab-bar destination can open: its href, and below it (the play tab
// opens a game's own screen when there is only one).
const navTargets = (href) =>
  href === '/' ? [{ path: '/' }] : [{ path: href }, { prefix: `${href}/` }];

// The tab-bar clicks in parsed files: `getByTestId('nav-<key>')`. A key built at
// run time stands for every destination it can be, an unknown key for every
// route. A test id that is not `nav-...` is not navigation.
function navTargetsIn(files, nav) {
  const read = reader(files);
  const found = [];
  for (const at of files)
    for (const call of at.calls) {
      if (calleeName(call) !== 'getByTestId' || call.arguments[0] === undefined) continue;
      for (const held of read.values(value(call.arguments[0].expression, at), 0)) {
        if (held === UNKNOWN) continue;
        const { node } = held;
        const fixed =
          node.type === 'StringLiteral'
            ? node.value
            : node.type === 'TemplateLiteral'
              ? (node.quasis[0]?.cooked ?? '')
              : null;
        if (fixed === null || !fixed.startsWith('nav-')) continue;
        const key = fixed.slice('nav-'.length);
        const built = node.type === 'TemplateLiteral' && node.expressions.length > 0;
        const keys = [...nav.keys()].filter((one) =>
          built ? one.startsWith(key) : key === one || key.startsWith(`${one}-`),
        );
        if (keys.length === 0) found.push(EVERY_ROUTE);
        for (const one of keys) found.push(...navTargets(nav.get(one)));
      }
    }
  return found;
}

export function navClickTargets(source, nav) {
  return navTargetsIn([index(source, 'spec.ts')], nav);
}

// The e2e files a spec reads: itself and the e2e files it imports, transitively.
function specFiles(spec, e2e) {
  const imports = new Map(e2e.map((module) => [module.source, module.dependencies]));
  const seen = new Set([spec]);
  const queue = [spec];
  while (queue.length > 0)
    for (const next of imports.get(queue.pop()) ?? [])
      if (!seen.has(next)) {
        seen.add(next);
        queue.push(next);
      }
  return [...seen];
}

const parsed = new Map();

function parsedFile(root, file) {
  const key = join(root, file);
  if (!parsed.has(key)) parsed.set(key, index(readFileSync(key, 'utf8'), file));
  return parsed.get(key);
}

const navCache = new Map();

export function navOf(root) {
  if (!navCache.has(root))
    navCache.set(
      root,
      existsSync(join(root, NAV_ITEMS))
        ? navItems(readFileSync(join(root, NAV_ITEMS), 'utf8'))
        : new Map(),
    );
  return navCache.get(root);
}

// The targets a spec opens: its gotos and tab clicks, and those of its e2e imports.
// A file that does not parse counts as opening every route.
export function specTargets(spec, e2e, root) {
  let files;
  try {
    files = specFiles(spec, e2e).map((file) => parsedFile(root, file));
  } catch {
    return [EVERY_ROUTE];
  }
  return [...gotoTargetsIn(files), ...navTargetsIn(files, navOf(root))];
}

// route file -> the projects whose specs open it. `routes` may name a route that is
// no longer on disk (deleted or renamed in the change): its old URL still counts.
export function routeVisitors(graph, specs, routes, root) {
  const pages = routes
    .map((route) => [route, routeSegments(route)])
    .filter(([, segments]) => segments !== null);
  const visitors = new Map();
  for (const { source } of graph.e2e ?? []) {
    const name = SPEC.exec(source)?.[1];
    if (name === undefined || !specs.has(name)) continue;
    const project = specs.get(name);
    for (const found of specTargets(source, graph.e2e, root))
      for (const [route, segments] of pages)
        if (routeMatches(segments, found))
          visitors.set(route, new Set([...(visitors.get(route) ?? []), project]));
  }
  return visitors;
}

// The literal link targets in an app file: `href="/x"` and `href: '/x'`, and
// `.push('/x')`, `.replace('/x')`, `.navigate('/x')`. A target built at run time is
// not a link this reader can follow; it is skipped. A file that does not parse
// links to every route.
const LINKING = /href|\.push\(|\.replace\(|\.navigate\(/;
const linkCache = new Map();

export function linkTargets(root, file) {
  const key = join(root, file);
  if (linkCache.has(key)) return linkCache.get(key);
  const source = readFileSync(key, 'utf8');
  const found = [];
  if (LINKING.test(source)) {
    let at;
    try {
      at = index(source, file);
    } catch {
      at = null;
      found.push(EVERY_ROUTE);
    }
    const literal = (node) => {
      const inner = unwrap(node?.type === 'JSXExpressionContainer' ? node.expression : node);
      if (inner?.type === 'StringLiteral' && inner.value.startsWith('/'))
        found.push(target(inner.value, true));
      if (inner?.type === 'TemplateLiteral') {
        const head = inner.quasis[0]?.cooked ?? '';
        if (head.startsWith('/')) found.push(target(head, inner.expressions.length === 0));
      }
    };
    const visit = (node) => {
      if (node === null || typeof node !== 'object') return;
      if (Array.isArray(node)) return node.forEach(visit);
      if (node.type === 'JSXAttribute' && nameOf(node.name) === 'href') literal(node.value);
      if (node.type === 'KeyValueProperty' && nameOf(node.key) === 'href') literal(node.value);
      if (
        node.type === 'CallExpression' &&
        ['push', 'replace', 'navigate'].includes(calleeName(node)) &&
        unwrap(node.callee)?.type === 'MemberExpression'
      )
        literal(node.arguments[0]?.expression);
      for (const [name, child] of Object.entries(node)) if (name !== 'span') visit(child);
    };
    if (at !== null) visit(at.tree);
  }
  linkCache.set(key, found);
  return found;
}
