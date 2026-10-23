#!/usr/bin/env node
// The import graph of a client app's src (apps/app by default, or apps/admin), the workspace
// packages it imports, its e2e files and its build inputs (app.config.ts, scripts/), built by dependency-cruiser (parsed with swc: dependency-cruiser's
// TypeScript parser does not support TypeScript 7). `importGraph()` is synchronous
// because `affected()` is: it runs this file as a child process, which prints the
// cruise as JSON, and keeps the result for the rest of the process. In a git
// checkout dependency-cruiser caches the cruise in node_modules/.cache.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
export const APP_DIR = 'apps/app';
const e2eDir = (dir) => `${dir}/e2e/`;
const configFile = (dir) => `${dir}/app.config.ts`;
const scriptsDir = (dir) => `${dir}/scripts/`;
const isBuildInput = (path, dir) =>
  path === configFile(dir) ||
  (path.startsWith(scriptsDir(dir)) && !/\.test\.[cm]?[jt]sx?$/.test(path));
const PLATFORMS = ['web'];
const EXTENSIONS = ['.tsx', '.ts', '.jsx', '.js', '.mjs', '.json'];
const PLATFORM_SUFFIX = /\.(?:web|native|ios|android)(?=\.[a-z]+$)/;

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

// The workspace packages a client app (apps/app by default) depends on, as `{ name, dir }`.
export function appPackages(root = REPO_ROOT, appDir = APP_DIR) {
  const app = readJson(join(root, appDir, 'package.json'));
  const wanted = new Set(
    Object.keys({ ...app.dependencies, ...app.devDependencies }).filter((name) =>
      name.startsWith('@wg/'),
    ),
  );
  const packagesDir = join(root, 'packages');
  if (!existsSync(packagesDir)) return [];
  return readdirSync(packagesDir)
    .filter((dir) => existsSync(join(packagesDir, dir, 'package.json')))
    .map((dir) => ({ name: readJson(join(packagesDir, dir, 'package.json')).name, dir }))
    .filter(({ name }) => wanted.has(name))
    .sort((a, b) => a.dir.localeCompare(b.dir));
}

const scopes = new Map();

// Path prefixes (repo-relative, trailing slash) the graph covers; none without an app.
export function graphScope(root = REPO_ROOT, appDir = APP_DIR) {
  const key = `${root}\0${appDir}`;
  if (!scopes.has(key))
    scopes.set(
      key,
      existsSync(join(root, appDir, 'package.json'))
        ? [`${appDir}/src/`, ...appPackages(root, appDir).map(({ dir }) => `packages/${dir}/src/`)]
        : [],
    );
  return scopes.get(key);
}

// foo.web.ts, foo.native.ts and foo.ts are one module to the bundler: an import
// of './foo' picks one of them per platform.
export function platformStem(path) {
  return path.replace(PLATFORM_SUFFIX, '');
}

// A hash of what decides how an import resolves and that dependency-cruiser's git
// cache does not watch: the app's tsconfig and the manifests behind the aliases.
export function resolutionKey(root, packages = appPackages(root), appDir = APP_DIR) {
  const hash = createHash('sha1');
  for (const file of [
    join(appDir, 'tsconfig.json'),
    join(appDir, 'package.json'),
    ...packages.map(({ dir }) => join('packages', dir, 'package.json')),
  ])
    hash.update(`${file}\0${existsSync(join(root, file)) ? readFileSync(join(root, file)) : ''}\0`);
  return hash.digest('hex').slice(0, 16);
}

export async function cruiseApp(root = REPO_ROOT, appDir = APP_DIR) {
  const { cruise } = await import('dependency-cruiser');
  const packages = appPackages(root, appDir);
  const scope = graphScope(root, appDir);
  const e2ePrefix = e2eDir(appDir);
  const built = (path) => isBuildInput(path, appDir);
  const result = await cruise(
    [
      ...[...scope, e2ePrefix, scriptsDir(appDir)].map((prefix) => prefix.slice(0, -1)),
      configFile(appDir),
    ].filter(
      (path) => existsSync(join(root, path)),
    ),
    {
      baseDir: root,
      parser: 'swc',
      // dependency-cruiser's own cache, keyed on git: a re-run on an unchanged
      // tree reads it back instead of parsing again. It sees edited and new source
      // files, not the resolution inputs, so its folder is keyed on those.
      ...(existsSync(join(root, '.git'))
        ? {
            cache: {
              folder: join(
                root,
                'node_modules',
                '.cache',
                'dependency-cruiser',
                resolutionKey(root, packages, appDir),
              ),
              strategy: 'metadata',
            },
          }
        : {}),
      doNotFollow: { path: '(^|/)node_modules/' },
      exclude: { path: '(^|/)(node_modules|dist|playwright-report|test-results)/' },
      tsConfig: { fileName: join(root, appDir, 'tsconfig.json') },
      enhancedResolveOptions: {
        extensions: [
          ...PLATFORMS.flatMap((platform) => EXTENSIONS.map((ext) => `.${platform}${ext}`)),
          ...EXTENSIONS,
        ],
      },
    },
    {
      // The packages export their built dist/; the app's bundler and this graph
      // read their sources.
      alias: Object.fromEntries(
        packages.map(({ name, dir }) => [name, join(root, 'packages', dir, 'src')]),
      ),
      extensionAlias: { '.js': ['.ts', '.tsx', '.js'] },
    },
  );
  const inScope = (path) => scope.some((prefix) => path.startsWith(prefix));
  const modules = result.output.modules
    .filter((module) => inScope(module.source))
    .map((module) => ({
      source: module.source,
      dependencies: module.dependencies.map((dep) => ({
        module: dep.module,
        resolved: dep.resolved,
        dynamic: dep.dynamic,
        couldNotResolve: dep.couldNotResolve,
      })),
      dependents: module.dependents ?? [],
    }));
  const e2e = result.output.modules
    .filter((module) => module.source.startsWith(e2ePrefix))
    .map((module) => ({
      source: module.source,
      dependencies: module.dependencies
        .map((dep) => dep.resolved)
        .filter((resolved) => resolved.startsWith(e2ePrefix)),
      appImports: module.dependencies.map((dep) => dep.resolved).filter(inScope),
      // The workspace package specifiers it imports ('@wg/shared/apple'): which
      // subpaths of a package the e2e files use (CD-308).
      packageImports: module.dependencies
        .map((dep) => dep.module)
        .filter((specifier) => specifier.startsWith('@wg/')),
    }));
  const build = result.output.modules
    .filter((module) => built(module.source))
    .map((module) => ({
      source: module.source,
      appImports: module.dependencies.map((dep) => dep.resolved).filter(inScope),
      packageImports: module.dependencies
        .map((dep) => dep.module)
        .filter((specifier) => specifier.startsWith('@wg/')),
    }));
  const unresolved = result.output.modules
    .filter(
      (module) =>
        inScope(module.source) || module.source.startsWith(e2ePrefix) || built(module.source),
    )
    .flatMap((module) =>
      module.dependencies
        .filter((dep) => dep.couldNotResolve && /^(?:\.|@\/|@wg\/)/.test(dep.module))
        .map((dep) => `${module.source} -> ${dep.module}`),
    );
  return { scope, modules, e2e, build, unresolved };
}

const graphs = new Map();
// A cruise that takes longer than this is killed and selects everything.
const CRUISE_TIMEOUT_MS = 120_000;

// The cruise of `root`'s client app `appDir`, once per process. A cruise that fails is a
// graph with nothing resolved, which selects everything.
export function importGraph(root = REPO_ROOT, appDir = APP_DIR) {
  const key = `${root}\0${appDir}`;
  if (!graphs.has(key)) {
    try {
      const json = execFileSync(
        process.execPath,
        ['--max-semi-space-size=64', fileURLToPath(import.meta.url), root, appDir],
        {
          encoding: 'utf8',
          maxBuffer: 256 * 1024 * 1024,
          stdio: ['ignore', 'pipe', 'pipe'],
          timeout: CRUISE_TIMEOUT_MS,
        },
      );
      graphs.set(key, JSON.parse(json));
    } catch (error) {
      const why = /^\w*Error: .*$/m.exec(String(error.stderr ?? ''))?.[0] ?? String(error.message);
      graphs.set(key, {
        scope: graphScope(root, appDir),
        modules: [],
        e2e: [],
        build: [],
        unresolved: [`the import graph could not be built: ${why}`],
      });
    }
  }
  return graphs.get(key);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const root = process.argv[2] ?? REPO_ROOT;
  const appDir = process.argv[3] ?? APP_DIR;
  // Without TypeScript's API (TS 7 has none), dependency-cruiser hands the tsconfig
  // paths plugin the working directory as baseUrl. The app's tsconfig has no
  // baseUrl, so its paths are relative to the app's folder: run from there.
  process.chdir(join(root, appDir));
  // Without swc, dependency-cruiser skips every .ts and .tsx file and returns a
  // partial graph. Fail here instead, which selects everything.
  await import('@swc/core');
  const graph = await cruiseApp(root, appDir);
  process.stdout.write(JSON.stringify(graph));
}
