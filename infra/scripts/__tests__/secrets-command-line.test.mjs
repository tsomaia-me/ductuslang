import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

const WORKFLOWS = fileURLToPath(new URL('../../../.github/workflows/', import.meta.url));

describe('no secret in a command line (CD-322, B1)', () => {
  const SECRET_VALUE = /secrets\.|github\.token|outputs\.token/;
  const FROM_A_DESCRIPTOR = /@<\(printf [^)]*\)/g;
  const PRESENCE_TEST = /\[\[ (?:(?:-[nz] "\$\{?\w+\}?"|&&|\|\|) )+\]\]/g;

  const leaksIn = (flow) => {
    const leaks = [];
    for (const [name, job] of Object.entries(flow.jobs)) {
      for (const entry of job.steps ?? []) {
        if (!entry.run) continue;
        const env = { ...flow.env, ...job.env, ...entry.env };
        const secrets = Object.keys(env).filter((key) => SECRET_VALUE.test(String(env[key])));
        const text = entry.run.replace(FROM_A_DESCRIPTOR, '').replace(PRESENCE_TEST, '');
        for (const secret of secrets) {
          if (new RegExp(`\\$\\{?${secret}\\b`).test(text)) leaks.push(`${name} › ${entry.name ?? entry.id}: $${secret}`);
        }
      }
    }
    return leaks;
  };

  const accessStep = (run) =>
    parse(`jobs:
  live:
    steps:
      - name: read
        env:
          CF_ACCESS_CLIENT_ID: \${{ secrets.CF_ACCESS_CLIENT_ID }}
        run: |
          ${run}
`);

  it('every deploying workflow hands a secret to a command only through its environment or a file descriptor', () => {
    for (const file of ['release.yml', 'deploy.yml', 'deploy-part.yml']) expect.soft(leaksIn(parse(readFileSync(join(WORKFLOWS, file), 'utf8'))), file).toEqual([]);
  });

  it('passes a secret tested for presence and read only by printf into a process-substitution descriptor, and fails the same secret passed to a command as a plain argument', () => {
    const throughDescriptor = accessStep(`if [[ -n "$CF_ACCESS_CLIENT_ID" ]]; then curl -H @<(printf 'CF-Access-Client-Id: %s\\n' "$CF_ACCESS_CLIENT_ID") "$url"; fi`);
    expect.soft(leaksIn(throughDescriptor), 'a descriptor').toEqual([]);
    const asArgument = accessStep(`curl -H "CF-Access-Client-Id: $CF_ACCESS_CLIENT_ID" "$url"`);
    expect.soft(leaksIn(asArgument), 'a plain argument').toEqual(['live › read: $CF_ACCESS_CLIENT_ID']);
  });
});
