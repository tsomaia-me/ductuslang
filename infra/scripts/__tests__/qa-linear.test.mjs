import { describe, expect, it } from 'vitest';

import { keyOf, run } from '../qa-linear.mjs';

const KEY = 'lin_api_must_not_appear';

function fakeLinear({ issues = [], projects = { 'Admin dashboard': 'p-admin' } } = {}) {
  const store = issues.map((issue) => ({ ...issue }));
  const calls = [];
  const fetchImpl = async (url, init) => {
    expect(url).toBe('https://api.linear.app/graphql');
    expect(init.headers.authorization).toBe(KEY);
    const { query, variables } = JSON.parse(init.body);
    calls.push(query.match(/(?:query|mutation) (\w+)/)[1]);
    const json = (data) => ({ ok: true, status: 200, json: async () => ({ data }) });
    if (query.includes('query QaBatch')) {
      const open = store.filter((issue) => issue.title === variables.title && !['completed', 'canceled'].includes(issue.state));
      return json({ issues: { nodes: open.map(({ id, identifier, description }) => ({ id, identifier, description })) } });
    }
    if (query.includes('query QaTeam')) return json({ teams: { nodes: variables.key === 'CD' ? [{ id: 'team-cd' }] : [] } });
    if (query.includes('query QaProject')) return json({ projects: { nodes: projects[variables.name] ? [{ id: projects[variables.name] }] : [] } });
    if (query.includes('mutation QaBatchAdd')) {
      store.find((issue) => issue.id === variables.id).description = variables.input.description;
      return json({ issueUpdate: { success: true } });
    }
    if (query.includes('mutation QaBatchNew')) {
      const identifier = `CD-${900 + store.length}`;
      store.push({ id: `id-${identifier}`, identifier, state: 'unstarted', ...variables.input });
      return json({ issueCreate: { success: true, issue: { id: `id-${identifier}`, identifier } } });
    }
    return { ok: false, status: 400, json: async () => ({ errors: [{ message: 'unknown query' }] }) };
  };
  return { store, calls, fetchImpl };
}

const blocking = { severity: 'blocking', screen: '/store', what: 'The buy button does nothing', source: 'verifier', link: 'https://claude.ai/artifact/x' };
const polish = { severity: 'polish', screen: '/users', what: 'A long name runs out of its cell', source: 'verifier' };
const overlap = { severity: 'blocking', screen: '/store', what: 'The price overlaps the buy button', source: 'verifier' };

const file = async (world, surface, findings, extra = []) => {
  const out = [];
  const err = [];
  const code = await run(['--surface', surface, '--version', 'v5.12', '--findings', 'f.json', ...extra], { LINEAR_API_KEY: KEY }, {
    fetchImpl: world.fetchImpl,
    read: () => JSON.stringify(findings),
    out: (line) => out.push(line),
    err: (line) => err.push(line),
  });
  return { code, out: out.join('\n'), err: err.join('\n') };
};

describe('QA findings in one Linear batch per surface (CD-322)', () => {
  it("opens a batch in the team with no project for the app, one checklist item per finding, and the admin's in the Admin dashboard project", async () => {
    const world = fakeLinear();
    const { code, out } = await file(world, 'app', [blocking, overlap]);
    expect.soft(code).toBe(0);
    expect.soft(out).toContain('opened CD-900 with 2 item(s)');
    const [batch] = world.store;
    expect.soft(batch).toMatchObject({ teamId: 'team-cd', title: 'QA batch: app' });
    expect.soft(batch.projectId).toBeUndefined();
    const items = batch.description.split('\n').filter((line) => line.startsWith('- [ ]'));
    expect.soft(items).toEqual([
      `- [ ] blocking · /store · The buy button does nothing · v5.12 · verifier · https://claude.ai/artifact/x · ${keyOf('app', blocking)}`,
      `- [ ] blocking · /store · The price overlaps the buy button · v5.12 · verifier · ${keyOf('app', overlap)}`,
    ]);
    const admin = fakeLinear();
    await file(admin, 'admin', [polish]);
    expect.soft(admin.store[0], 'admin').toMatchObject({ title: 'QA batch: admin', projectId: 'p-admin' });
  });

  it('adds to the open batch and skips what an unticked item already holds, the same finding worded alike included; adds a finding again when its item was ticked (fixed, and back); opens a new batch when the last one is done', async () => {
    const world = fakeLinear();
    await file(world, 'app', [blocking]);
    const { out } = await file(world, 'app', [{ ...blocking, what: 'the buy button  does nothing!' }, overlap]);
    expect.soft(out, 'open batch').toContain('added 1 item(s) to CD-900');
    expect.soft(world.store, 'open batch').toHaveLength(1);
    expect.soft(world.store[0].description.match(/^- \[ \]/gm), 'open batch').toHaveLength(2);
    const again = await file(world, 'app', [blocking, overlap]);
    expect.soft(again.out, 'open batch').toContain('every finding is already open in CD-900');
    const ticked = fakeLinear({
      issues: [{ id: 'id-1', identifier: 'CD-1', title: 'QA batch: app', state: 'started', description: `header\n- [x] blocking · /store · old · v5.10 · verifier · ${keyOf('app', blocking)}` }],
    });
    expect.soft((await file(ticked, 'app', [blocking])).out, 'ticked').toContain('added 1 item(s) to CD-1');
    const done = fakeLinear({ issues: [{ id: 'id-1', identifier: 'CD-1', title: 'QA batch: app', state: 'completed', description: '' }] });
    expect.soft((await file(done, 'app', [blocking])).out, 'done').toContain('opened CD-901');
  });

  it('keys a finding by its surface, screen and wording alone, whatever its source says (owner, 15 Oct)', () => {
    expect(keyOf('app', { ...blocking, source: 'e2e apps/app/e2e/journeys/store.spec.ts › store: buy' })).toBe(keyOf('app', blocking));
    expect(keyOf('app', { ...blocking, source: 'e2e apps/app/e2e/journeys/home.spec.ts › home: cards' })).toBe(keyOf('app', blocking));
    expect(keyOf('admin', blocking)).not.toBe(keyOf('app', blocking));
    expect(keyOf('app', { ...blocking, screen: '/play' })).not.toBe(keyOf('app', blocking));
  });

  it('writes nothing for no findings or a dry run, and never prints the key', async () => {
    const world = fakeLinear();
    expect((await file(world, 'app', [])).out).toContain('no findings');
    expect((await file(world, 'app', [blocking], ['--dry-run'])).out).toContain('would add 1 item(s) to a new "QA batch: app"');
    expect(world.store).toEqual([]);
    expect(world.calls).toEqual(['QaBatch']);
  });

  it('refuses a malformed finding, and fails when Linear refuses or no key is set', async () => {
    const world = fakeLinear();
    expect((await file(world, 'app', [{ severity: 'minor', screen: '/', what: 'x', source: 'verifier' }])).code).toBe(2);
    const down = { fetchImpl: async () => ({ ok: false, status: 503, json: async () => ({}) }) };
    const refused = await file(down, 'app', [blocking]);
    expect(refused).toMatchObject({ code: 1 });
    expect(refused.err).toContain('Linear answered 503');
    expect(refused.err).not.toContain(KEY);
    const out = [];
    expect(await run(['--surface', 'app', '--version', 'v5.12', '--findings', 'f'], {}, { read: () => '[]', err: (line) => out.push(line) })).toBe(1);
    expect(out.join()).toContain('LINEAR_API_KEY is not set');
  });
});

describe('the nightly failures in the one open MVP debt issue', () => {
  const nightly = async (world, surface, findings, version = '3221bf33') => {
    const err = [];
    const code = await run(['--surface', surface, '--version', version, '--findings', 'f.json', '--batch', 'mvp-debt'], { LINEAR_API_KEY: KEY }, {
      fetchImpl: world.fetchImpl,
      read: () => JSON.stringify(findings),
      out: () => {},
      err: (line) => err.push(line),
    });
    return { code, err: err.join('\n') };
  };

  it('opens "MVP debt" in the team with no project, labelled with the commit of main, for either surface, adds both surfaces to the same open issue and skips what is still open', async () => {
    const world = fakeLinear();
    expect.soft((await nightly(world, 'admin', [polish])).code).toBe(0);
    const [debt] = world.store;
    expect.soft(debt).toMatchObject({ teamId: 'team-cd', title: 'MVP debt' });
    expect.soft(debt.projectId).toBeUndefined();
    expect.soft(debt.description).toContain('nightly');
    expect.soft(debt.description).toContain(`- [ ] polish · /users · A long name runs out of its cell · 3221bf33 · verifier · ${keyOf('admin', polish)}`);
    await nightly(world, 'app', [blocking]);
    await nightly(world, 'app', [blocking]);
    expect.soft(world.store, 'both surfaces').toHaveLength(1);
    expect.soft(world.store[0].description.split('\n').filter((line) => line.startsWith('- [ ]')), 'both surfaces').toHaveLength(2);
  });

  it('refuses an unknown batch and a version that is neither a release nor a commit', async () => {
    const world = fakeLinear();
    expect((await run(['--surface', 'app', '--version', '3221bf33', '--findings', 'f', '--batch', 'other'], { LINEAR_API_KEY: KEY }, { read: () => '[]', err: () => {} }))).toBe(2);
    expect((await nightly(world, 'app', [blocking], 'main')).code).toBe(2);
    expect(world.store).toEqual([]);
  });
});
