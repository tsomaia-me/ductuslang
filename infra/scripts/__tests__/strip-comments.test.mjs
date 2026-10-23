import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, describe, expect, it } from 'vitest';

import { applyEdits, isDirective, keepReason, planFile, verifyFile } from '../strip-comments.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(HERE, '..', 'strip-comments.mjs');
const FIXTURES = join(HERE, 'fixtures', 'strip-comments');
const ts = createRequire(join(HERE, '..', '..', '..', 'package.json'))('@typescript/typescript6');

const roots = [];
afterAll(() => {
  for (const root of roots) rmSync(root, { recursive: true, force: true });
});

const tempRoot = () => {
  const root = mkdtempSync(join(tmpdir(), 'strip-comments-'));
  roots.push(root);
  return root;
};

const filesUnder = (dir) =>
  readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => relative(dir, join(entry.parentPath, entry.name)))
    .sort();

const put = (root, rel, text) => {
  mkdirSync(dirname(join(root, rel)), { recursive: true });
  writeFileSync(join(root, rel), text);
};

const read = (root, rel) => readFileSync(join(root, rel), 'utf8');

const stageFixtures = (root) => {
  const input = join(FIXTURES, 'input');
  for (const rel of filesUnder(input)) put(root, rel.replace(/\.in$/, ''), readFileSync(join(input, rel), 'utf8'));
};

const run = (root, args) => spawnSync(process.execPath, [SCRIPT, ...args], { cwd: root, encoding: 'utf8' });

const section = (stdout, title) => {
  const lines = stdout.split('\n');
  const from = lines.indexOf(title);
  const counts = {};
  for (const line of lines.slice(from + 1)) {
    const match = /^ {2}(\S+)\s+(\d+)$/.exec(line);
    if (!match) break;
    counts[match[1]] = Number(match[2]);
  }
  return counts;
};

const typeErrors = (source) => {
  const file = join(tempRoot(), 'a.tsx');
  writeFileSync(file, source);
  const options = { strict: true, noEmit: true, jsx: ts.JsxEmit.Preserve, target: ts.ScriptTarget.ES2022, types: [], skipLibCheck: true };
  return ts.getPreEmitDiagnostics(ts.createProgram([file], options)).length;
};

const strip = (source, path = 'a.ts') => {
  const plan = planFile(path, source);
  const after = applyEdits(source, plan.edits);
  return { after, plan, problems: verifyFile(path, source, after, plan.edits) };
};

describe('strip-comments', () => {
  it('removes comments and keeps the code byte for byte: doc blocks, line and trailing comments, JSX comment expressions, never text inside strings, templates, regexes, URLs or JSX, never merging two words or leaving a doubled blank line, keeping a statement break, a blank line inside a comment, a .jsonc trailing comma', () => {
    const rows = [
      {
        label: 'doc block, line and trailing comments',
        source: ['/**', ' * Docs.', ' */', 'export function f(a: number) {', '  // why', '  return a + 1; // trailing', '}', ''].join('\n'),
        after: ['export function f(a: number) {', '  return a + 1;', '}', ''].join('\n'),
      },
      {
        label: 'strings, templates, regexes and URLs',
        source: [
          "const url = 'https://example.com/a'; // x",
          'const t = `// not a comment ${1 /* c */} /* nor this */`;',
          'const r = /\\/\\/[^\\n]*/g;',
          'const s = "/* keep */";',
          '',
        ].join('\n'),
        after: [
          "const url = 'https://example.com/a';",
          'const t = `// not a comment ${1} /* nor this */`;',
          'const r = /\\/\\/[^\\n]*/g;',
          'const s = "/* keep */";',
          '',
        ].join('\n'),
      },
      {
        label: 'a JSX comment expression with its braces and its line, JSX text left alone',
        path: 'a.tsx',
        source: ['export const A = () => (', '  <View>', '    {/* one gentle bob */}', '    <Text>// shown to the player</Text>', '    {x /* why */}', '  </View>', ');', ''].join('\n'),
        after: ['export const A = () => (', '  <View>', '    <Text>// shown to the player</Text>', '    {x}', '  </View>', ');', ''].join('\n'),
      },
      {
        label: 'no doubled or block-opening blank line',
        source: ['function g() {', '  // lead', '', '  const a/* c */= 1;', '', '  // mid', '', '  return a;', '}', ''].join('\n'),
        after: ['function g() {', '  const a= 1;', '', '  return a;', '}', ''].join('\n'),
      },
      { label: 'two words never merged', source: 'let/* x */y = 1;\n', after: 'let y = 1;\n' },
      { label: 'a statement break a multi-line comment carried', source: 'const a = 1 /* one\n two */ const b = 2;\n', after: 'const a = 1\nconst b = 2;\n' },
      {
        label: 'a comment with a blank line inside it, as whole lines',
        path: 'a.tsx',
        source: ['const a = 1;', '/**', ' * One.', '', ' * Two.', ' */', 'const b = (', '  <View>', '    {/* x', '', '       y */}', '  </View>', ');', ''].join('\n'),
        after: ['const a = 1;', 'const b = (', '  <View>', '  </View>', ');', ''].join('\n'),
        wholeLines: true,
      },
      { label: 'a doc block with a blank line', source: ['const a = 1;', '/**', ' * One.', '', ' */', 'const b = 2;', ''].join('\n'), after: 'const a = 1;\nconst b = 2;\n' },
      { label: 'a .jsonc trailing comma before a comment', path: 'a.jsonc', source: '{"a": 1, // x\n}\n', after: '{"a": 1,\n}\n' },
    ];
    for (const row of rows) {
      const { after, plan, problems } = strip(row.source, row.path);
      expect.soft(after, row.label).toBe(row.after);
      expect.soft(problems, row.label).toEqual([]);
      if (row.wholeLines) expect.soft(plan.edits.every((e) => e.replacement === undefined), row.label).toBe(true);
    }
  });

  it('keeps directives that tools read and every functional comment on the brief\'s list, with its reason, and a shebang', () => {
    const source = [
      '/// <reference types="expo/types" />',
      '// @ts-expect-error -- the lib types lag',
      'const a: number = "x";',
      '// eslint-disable-next-line no-console -- dev only',
      'console.log(a);',
      'import(/* @vite-ignore */ a);',
      '',
    ].join('\n');
    expect.soft(strip(source).after, 'directives').toBe(source);
    expect.soft(isDirective('// prettier-ignore')).toBe(true);
    expect.soft(isDirective('// a note about eslint')).toBe(false);
    for (const directive of [
      '// @ts-ignore',
      '// @ts-nocheck',
      '/* eslint-enable no-console */',
      '/* istanbul ignore next */',
      '/* c8 ignore start */',
      '/* v8 ignore next */',
      '/* webpackChunkName: "x" */',
      '/* #__PURE__ */',
      '/* @__PURE__ */',
      '/** @jsxImportSource @emotion/react */',
      '// @jest-environment jsdom',
      '/*! licence text */',
      '/** @license MIT */',
      '// @license MIT',
      '/**\n * @preserve\n */',
      '// prettier-ignore',
    ]) {
      expect.soft(keepReason(directive), directive).not.toBeNull();
    }
    for (const note of ['// we used istanbul once', '/** see the @license file */', '// we @preserve nothing here']) expect.soft(keepReason(note), note).toBeNull();
    const { plan } = strip('#!/usr/bin/env node\n// note\nconst a = 1;\n');
    expect.soft(applyEdits('#!/usr/bin/env node\n// note\nconst a = 1;\n', plan.edits)).toBe('#!/usr/bin/env node\nconst a = 1;\n');
  });

  it('keeps the block forms of @ts-expect-error and @ts-ignore that TypeScript honours, so the file still type-checks', () => {
    const directives = [
      '/* @ts-expect-error */',
      '/** @ts-ignore */',
      '/* the types lag\n   @ts-expect-error */',
      '/// @ts-expect-error',
      '{/* @ts-expect-error */}',
    ];
    const source = [
      'declare global { namespace JSX { interface IntrinsicElements { div: object } interface Element {} } }',
      'const Foo = (props: { label: string }): JSX.Element => props as unknown as JSX.Element;',
      '// a note that goes',
      directives[0],
      "export const n: number = 'x';",
      directives[1],
      "export const m: number = 'y';",
      directives[2],
      "export const k: number = 'z';",
      directives[3],
      "export const j: number = 'w';",
      'export const v = (',
      '  <div>',
      `    ${directives[4]}`,
      '    <Foo bad />',
      '  </div>',
      ');',
      '',
    ].join('\n');
    const { after, problems } = strip(source, 'a.tsx');
    expect(problems).toEqual([]);
    expect(after).toBe(source.replace('// a note that goes\n', ''));
    expect(typeErrors(after)).toBe(0);
    const unguarded = directives.reduce((text, directive) => text.replace(`${directive}\n`, ''), after);
    expect(typeErrors(unguarded)).toBeGreaterThanOrEqual(directives.length);
    expect(keepReason('/*\n * @ts-expect-error\n */')).toBeNull();
  });

  it('skips a generated file and records the kind of every edit', () => {
    expect(planFile('g.ts', '/**\n * GENERATED by x — do not edit.\n */\nexport const a = 1;\n').generated).toBe(true);
    expect(planFile('h.ts', 'export const a = 1;\n// the GENERATED id — do not edit it by hand\n').generated).toBe(false);
    expect(planFile('j.ts', "import a from './a';\n\n/**\n * GENERATED by `gen` — do not edit by hand.\n */\nexport const b = a;\n").generated).toBe(true);
    expect(planFile('i.ts', '/** We do not edit the draft here. */\nexport const a = 1;\n').generated).toBe(false);
    const { plan } = strip('/** d */\nconst a = 1; // l\n/* b */\nconst b = 2;\n');
    expect(plan.edits.map((e) => e.kinds.join('+'))).toEqual(['jsdoc', 'line', 'block']);
    expect(plan.comments.map((c) => [c.line, c.kind, c.text])).toEqual([
      [1, 'jsdoc', '/** d */'],
      [2, 'line', '// l'],
      [3, 'block', '/* b */'],
    ]);
  });

  it('flags an edit that removes code, a YAML result whose values differ, a shell result that no longer parses or whose commands changed, and passes the faithful ones', () => {
    const codeEdits = [{ start: 0, end: 5, startLine: 1, startCol: 1, text: 'const' }];
    const yamlEdits = [{ start: 5, end: 10, startLine: 2, startCol: 1, text: '# c\n' }];
    const shellEdits = [{ start: 7, end: 11, startLine: 1, startCol: 8, text: ' # c' }];
    const commandEdits = [{ start: 8, end: 10, startLine: 1, startCol: 9, text: '#b' }];
    const flagged = [
      ['code removed', verifyFile('a.ts', 'const a = 1;\n', applyEdits('const a = 1;\n', codeEdits), codeEdits)],
      ['YAML values differ', verifyFile('a.yml', 'a: 1\n# c\n', 'a: 2\n', yamlEdits)],
      ['shell no longer parses', verifyFile('a.sh', 'echo a # c\n', "echo 'a\n", shellEdits)],
    ];
    for (const [label, problems] of flagged) expect.soft(problems.length, label).toBeGreaterThan(0);
    expect.soft(verifyFile('a.yml', 'a: 1\n# c\n', 'a: 1\n', yamlEdits), 'YAML faithful').toEqual([]);
    expect.soft(verifyFile('a.sh', 'echo a # c\n', 'echo a\n', shellEdits), 'shell faithful').toEqual([]);
    expect.soft(verifyFile('a.sh', 'echo a\\;#b\n', 'echo a\\;\n', commandEdits), 'shell commands differ').toEqual([
      'the commands differ after the removal (declare -f of the script wrapped in a function)',
    ]);
  });

  it('still flags that removal when the caller environment defines declare through BASH_ENV and an exported function', () => {
    const root = tempRoot();
    put(root, 'hostile.sh', 'declare() { echo same; }\n');
    const saved = { BASH_ENV: process.env.BASH_ENV, declareFunction: process.env['BASH_FUNC_declare%%'] };
    process.env.BASH_ENV = join(root, 'hostile.sh');
    process.env['BASH_FUNC_declare%%'] = '() {  echo same\n}';
    try {
      const edits = [{ start: 8, end: 10, startLine: 1, startCol: 9, text: '#b' }];
      expect(verifyFile('a.sh', 'echo a\\;#b\n', 'echo a\\;\n', edits)).toEqual([
        'the commands differ after the removal (declare -f of the script wrapped in a function)',
      ]);
    } finally {
      if (saved.BASH_ENV === undefined) delete process.env.BASH_ENV;
      else process.env.BASH_ENV = saved.BASH_ENV;
      if (saved.declareFunction === undefined) delete process.env['BASH_FUNC_declare%%'];
      else process.env['BASH_FUNC_declare%%'] = saved.declareFunction;
    }
  });
});

describe('strip-comments --write over the default paths', () => {
  it('exits 0, says how many comments it removed from how many files, writes every fixture byte for byte as expected, and is idempotent: a second --write changes nothing and --check then finds nothing', () => {
    const root = tempRoot();
    const expectedDir = join(FIXTURES, 'expected');
    stageFixtures(root);
    const first = run(root, ['--write']);
    expect.soft(first.stderr).toBe('');
    expect.soft(first.status).toBe(0);
    expect.soft(first.stdout).toMatch(/^strip-comments: removed \d+ comments from \d+ files$/m);
    for (const rel of filesUnder(expectedDir)) expect.soft(read(root, rel.replace(/\.out$/, '')), rel).toBe(readFileSync(join(expectedDir, rel), 'utf8'));
    const before = filesUnder(root).map((rel) => [rel, read(root, rel)]);
    const again = run(root, ['--write']);
    expect.soft(again.status, 'again').toBe(0);
    expect.soft(again.stdout, 'again').toMatch(/removed 0 comments from 0 files/);
    expect.soft(filesUnder(root).map((rel) => [rel, read(root, rel)]), 'again').toEqual(before);
    const check = run(root, ['--check']);
    expect.soft(check.stdout, 'check').toBe('');
    expect.soft(check.status, 'check').toBe(0);
  });
});

describe('strip-comments --check', () => {
  it('lists each comment as file:line and its first 60 characters, writes nothing and exits 1, also as the default mode without a flag', () => {
    const root = tempRoot();
    const long = `// ${'x'.repeat(80)}`;
    const source = `export const a = 1; // first\n${long}\n/**\n * Multi\n *   line\n */\nexport const b = <p>{/* jsx */}</p>;\n`;
    put(root, 'apps/app/a.tsx', source);
    put(root, 'infra/scripts/s.sh', '#!/bin/sh\n# shellcheck disable=SC2086\necho hi # greet\n');
    put(root, '.github/workflows/w.yml', 'name: w # the name\n');
    const listed = [
      '.github/workflows/w.yml:1 # the name',
      `apps/app/a.tsx:1 // first`,
      `apps/app/a.tsx:2 ${long.slice(0, 60)}`,
      'apps/app/a.tsx:3 /** * Multi * line */',
      'apps/app/a.tsx:7 /* jsx */',
      'infra/scripts/s.sh:3 # greet',
    ];
    for (const args of [['--check'], []]) {
      const label = args.join(' ') || 'no flag';
      const result = run(root, args);
      expect.soft(result.stdout.split('\n').filter(Boolean), label).toEqual(listed);
      expect.soft(result.status, label).toBe(1);
      expect.soft(read(root, 'apps/app/a.tsx'), label).toBe(source);
    }
  });

  it('exits 0 and prints nothing on a tree without comments', () => {
    const root = tempRoot();
    put(root, 'apps/app/a.ts', 'export const a = 1;\n');
    put(root, 'infra/scripts/s.sh', '#!/bin/sh\n# shellcheck disable=SC2086\necho hi\n');
    const result = run(root, ['--check']);
    expect(result).toMatchObject({ status: 0, stdout: '' });
  });

  it('checks the paths it is given instead of the default paths', () => {
    const root = tempRoot();
    put(root, 'apps/app/a.ts', 'export const a = 1; // in apps\n');
    put(root, 'docs/b.ts', 'export const b = 1; // in docs\n');
    expect(run(root, ['--check', 'docs']).stdout).toBe('docs/b.ts:1 // in docs\n');
    expect(run(root, ['--check', 'docs/b.ts', 'apps']).stdout).toBe('apps/app/a.ts:1 // in apps\ndocs/b.ts:1 // in docs\n');
  });

  it('looks only at the file types --type names', () => {
    const root = tempRoot();
    put(root, 'apps/a.ts', 'export const a = 1; // ts\n');
    put(root, 'apps/b.mjs', 'export const b = 1; // mjs\n');
    put(root, 'apps/c.sh', 'echo c # sh\n');
    expect(run(root, ['--check', '--type', 'ts']).stdout).toBe('apps/a.ts:1 // ts\n');
    expect(run(root, ['--check', '--type', 'ts,sh']).stdout).toBe('apps/a.ts:1 // ts\napps/c.sh:1 # sh\n');
  });
});

describe('strip-comments --report', () => {
  it('counts comments per file type and per top-level folder after the list, and alone only counts: nothing listed, nothing written, exit 0', () => {
    const root = tempRoot();
    put(root, 'apps/app/a.ts', 'export const a = 1; // one\n// two\n');
    put(root, 'apps/app/b.mjs', 'export const b = 1; // three\n');
    put(root, 'infra/scripts/s.sh', 'echo a # four\necho b # five\n# six\n');
    put(root, '.github/workflows/w.yml', 'name: w # seven\n');
    const result = run(root, ['--check', '--report']);
    expect.soft(result.status).toBe(1);
    expect.soft(section(result.stdout, 'by file type')).toEqual({ ts: 2, mjs: 1, sh: 3, yml: 1 });
    expect.soft(section(result.stdout, 'by folder')).toEqual({ apps: 3, infra: 3, '.github': 1 });
    expect.soft(section(result.stdout, 'by folder and file type')).toEqual({
      'apps/ts': 2,
      'apps/mjs': 1,
      'infra/sh': 3,
      '.github/yml': 1,
    });
    expect.soft(result.stdout).toMatch(/^files with comments: 4 of 4 scanned$/m);
    expect.soft(result.stdout).toContain('apps/app/a.ts:1 // one');
    const counted = run(root, ['--report']);
    expect.soft(counted.status, 'alone').toBe(0);
    expect.soft(counted.stdout, 'alone').not.toContain('apps/app/a.ts:1');
    expect.soft(section(counted.stdout, 'by file type'), 'alone').toEqual({ ts: 2, mjs: 1, sh: 3, yml: 1 });
    expect.soft(read(root, 'apps/app/a.ts'), 'alone').toBe('export const a = 1; // one\n// two\n');
  });
});

describe('strip-comments file selection', () => {
  it('never reads node_modules, dist, .git, .claude/archive, markdown, lock files or pnpm-lock.yaml, and reads JSON with comments only for .jsonc, tsconfig*.json and .vscode/*.json', () => {
    const root = tempRoot();
    const untouched = {
      'apps/app/node_modules/pkg/index.js': '// vendored\nmodule.exports = 1;\n',
      'apps/app/dist/main.js': '// built\nmodule.exports = 1;\n',
      '.github/.git/hooks/pre.sh': '# hook\necho 1\n',
      '.claude/archive/old.ts': '// history\nexport const a = 1;\n',
      'packages/notes.md': '// prose\n',
      'packages/yarn.lock': '# lock\n',
      'packages/pnpm-lock.yaml': '# lock\na: 1\n',
      '.vscode/notes.txt': '// kept\n',
      'packages/other.json': '{} // not a config\n',
    };
    for (const [rel, text] of Object.entries(untouched)) put(root, rel, text);
    put(root, '.vscode/settings.json', '{\n  // editor\n  "a": 1\n}\n');
    put(root, 'packages/tsconfig.json', '{} // base\n');
    const result = run(root, ['--write', 'apps', 'packages', '.github', '.claude', '.vscode']);
    expect.soft(result.status, result.stderr).toBe(0);
    for (const [rel, text] of Object.entries(untouched)) expect.soft(read(root, rel), rel).toBe(text);
    expect.soft(read(root, '.vscode/settings.json')).toBe('{\n  "a": 1\n}\n');
    expect.soft(read(root, 'packages/tsconfig.json')).toBe('{}\n');
  });

  it('skips what .gitignore ignores inside a repository, and still reads untracked files', () => {
    const root = tempRoot();
    expect(spawnSync('git', ['init', '-q'], { cwd: root }).status).toBe(0);
    put(root, '.gitignore', 'build-output/\n');
    put(root, 'apps/app/build-output/bundle.mjs', '// generated bundle\nexport const a = 1;\n');
    put(root, 'apps/app/new.mjs', '// not yet added\nexport const b = 1;\n');
    const result = run(root, ['--write']);
    expect(result.status).toBe(0);
    expect(read(root, 'apps/app/build-output/bundle.mjs')).toBe('// generated bundle\nexport const a = 1;\n');
    expect(read(root, 'apps/app/new.mjs')).toBe('export const b = 1;\n');
  });
});

describe('strip-comments line endings', () => {
  it('keeps CRLF line endings in every language', () => {
    const root = tempRoot();
    put(root, 'apps/a.ts', '// one\r\nexport const a = 1; // two\r\n');
    put(root, 'apps/b.yml', '# one\r\na: 1 # two\r\n');
    put(root, 'apps/c.sh', '#!/bin/sh\r\n# one\r\necho a\r\n');
    expect(run(root, ['--write']).status).toBe(0);
    expect(read(root, 'apps/a.ts')).toBe('export const a = 1;\r\n');
    expect(read(root, 'apps/b.yml')).toBe('a: 1\r\n');
    expect(read(root, 'apps/c.sh')).toBe('#!/bin/sh\r\necho a\r\n');
  });
});

describe('strip-comments refuses what it cannot remove safely', () => {
  const refuseWrite = (files, reasons) => {
    const root = tempRoot();
    put(root, 'apps/good.ts', 'export const a = 1; // good\n');
    for (const [rel, text] of Object.entries(files)) put(root, rel, text);
    const result = run(root, ['--write']);
    expect.soft(result.status).toBe(2);
    for (const reason of reasons) expect.soft(result.stderr, reason).toContain(reason);
    expect.soft(read(root, 'apps/good.ts')).toBe('export const a = 1; // good\n');
    for (const [rel, text] of Object.entries(files)) expect.soft(read(root, rel), rel).toBe(text);
    return root;
  };

  it('a shell script it cannot scan: an unterminated quote, a comment right after a line continuation, a heredoc that never ends; exit 2 in --write and --check, nothing written anywhere', () => {
    const root = refuseWrite(
      { 'infra/bad.sh': "echo 'oops # here\n", 'infra/join.sh': 'echo a \\\n  # note\n  b\n', 'infra/open.sh': 'cat <<EOF\n# body\n' },
      ['infra/bad.sh', 'infra/join.sh', 'infra/open.sh'],
    );
    expect.soft(run(root, ['--check']).status, '--check').toBe(2);
  });

  it('a removal it cannot check or that changes the code: bash -n rejects the script, bash cannot print it as a function, bash rejects it after the removal, a CRLF carriage return joins the last word, two tokens join into one; exit 2, nothing written anywhere', () => {
    refuseWrite(
      {
        'infra/open-if.sh': 'if true; then # never closed\n  echo a\n',
        'infra/tail.sh': '# note\necho a \\',
        'infra/regex.sh': '[[ $x =~ ^a(#)b ]]\n',
        'infra/crlf.sh': '#!/bin/sh\r\necho a # two\r\n',
        'apps/joined.ts': 'let z = x -/**/-g;\n',
      },
      [
        'infra/open-if.sh: bash -n rejects the script, so the removal cannot be checked',
        'infra/tail.sh: bash cannot print the script as a function',
        'infra/regex.sh: bash -n rejects the script after the removal',
        'infra/crlf.sh: the commands differ after the removal',
        'apps/joined.ts: token count',
      ],
    );
  });
});

describe('strip-comments command line', () => {
  it('exits 2 with a usage message on an unknown flag, both modes, a missing path or a --type without a list, and --help prints the usage and exits 0', () => {
    for (const [args, label] of [
      [['--bogus'], 'unknown flag'],
      [['--check', '--write'], 'both modes'],
      [['--check', 'no-such-folder'], 'missing path'],
      [['--check', '--type'], 'a --type without a list'],
    ]) {
      const result = run(tempRoot(), args);
      expect.soft(result.status, label).toBe(2);
      expect.soft(result.stderr, label).toMatch(/strip-comments/);
    }
    const help = run(tempRoot(), ['--help']);
    expect.soft(help.status, '--help').toBe(0);
    expect.soft(help.stdout, '--help').toMatch(/--check/);
  });
});
