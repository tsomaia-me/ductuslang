export const STATE_KEY = 'state';
export const FAIL_THRESHOLD = 2;
export const LINEAR_URL = 'https://api.linear.app/graphql';
export const PRODUCTION_TITLE = 'URGENT: production is not answering';

const FRESH_STATE = { failures: 0, productionPaged: null };

const reply = (status, body, type = 'text/plain') => new Response(body, { status, headers: { 'Content-Type': type } });

const withTimeout = (ms) => AbortSignal.timeout(ms);

export async function probe({ env, fetchImpl, timeoutMs = 10_000 }) {
  const problems = [];
  try {
    const response = await fetchImpl(env.API_HEALTH_URL, { headers: { 'cache-control': 'no-cache' }, signal: withTimeout(timeoutMs) });
    const body = await response.json().catch(() => null);
    if (!response.ok || body?.status !== 'ok') problems.push(`API health answered ${response.status}${body?.status ? ` (${body.status})` : ''}`);
  } catch (error) {
    problems.push(`API health did not answer (${error.name})`);
  }
  try {
    const response = await fetchImpl(env.WEB_BUILD_URL, { headers: { 'cache-control': 'no-cache' }, signal: withTimeout(timeoutMs) });
    const body = await response.json().catch(() => null);
    if (!response.ok || typeof body?.sha !== 'string') problems.push(`web build.json answered ${response.status}${body ? '' : ' without JSON'}`);
  } catch (error) {
    problems.push(`web build.json did not answer (${error.name})`);
  }
  return problems;
}

async function linearAsk({ env, fetchImpl }, query, variables) {
  const response = await fetchImpl(LINEAR_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: env.LINEAR_API_KEY },
    body: JSON.stringify({ query, variables }),
    signal: withTimeout(10_000),
  });
  const body = await response.json().catch(() => null);
  if (!response.ok || body?.errors?.length || !body?.data) throw new Error(`Linear answered ${response.status}`);
  return body.data;
}

async function pageLinear(context, title, text) {
  const { env } = context;
  const open = await linearAsk(
    context,
    `query WatchOpen($team: String!, $title: String!) { issues(first: 5, filter: { team: { key: { eq: $team } }, title: { eq: $title }, state: { type: { nin: ["completed", "canceled"] } } }) { nodes { id identifier } } }`,
    { team: env.LINEAR_TEAM, title },
  );
  const twin = open.issues.nodes[0];
  if (twin) {
    await linearAsk(context, `mutation WatchComment($input: CommentCreateInput!) { commentCreate(input: $input) { success } }`, { input: { issueId: twin.id, body: text } });
    return `linear: commented on ${twin.identifier}`;
  }
  const team = await linearAsk(context, `query WatchTeam($key: String!) { teams(filter: { key: { eq: $key } }) { nodes { id } } }`, { key: env.LINEAR_TEAM });
  const teamId = team.teams.nodes[0]?.id;
  if (!teamId) throw new Error(`Linear has no team ${env.LINEAR_TEAM}`);
  const made = await linearAsk(context, `mutation WatchIssue($input: IssueCreateInput!) { issueCreate(input: $input) { issue { identifier } } }`, {
    input: { teamId, title, description: text, priority: 1 },
  });
  return `linear: filed ${made.issueCreate.issue.identifier}`;
}

async function pageWebhook({ env, fetchImpl }, title, text) {
  const response = await fetchImpl(env.PAGE_WEBHOOK_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text: `${title}\n${text}` }),
    signal: withTimeout(10_000),
  });
  if (!response.ok) throw new Error(`the page webhook answered ${response.status}`);
  return 'webhook: sent';
}

export async function page(context, title, text) {
  const { env, log = console } = context;
  const sent = [];
  const channels = [
    ...(env.LINEAR_API_KEY ? [['linear', pageLinear]] : []),
    ...(env.PAGE_WEBHOOK_URL ? [['webhook', pageWebhook]] : []),
  ];
  for (const [name, send] of channels) {
    try {
      sent.push(await send(context, title, text));
    } catch (error) {
      log.error(`cd-watch: the ${name} page failed: ${error.message}`);
    }
  }
  return sent;
}

export async function readState(env) {
  try {
    return { ...FRESH_STATE, ...JSON.parse((await env.WATCH.get(STATE_KEY)) ?? '{}') };
  } catch {
    return { ...FRESH_STATE };
  }
}

export async function tick({ env, fetchImpl, now, log = console }) {
  const at = now();
  const context = { env, fetchImpl, log };
  const state = await readState(env);
  const lines = [];

  const problems = await probe({ env, fetchImpl });
  if (problems.length === 0) {
    if (state.productionPaged) {
      await page(context, PRODUCTION_TITLE, `Resolved at ${at.toISOString()}: production answers again (down since about ${state.productionPaged}).`);
      lines.push('production: recovered');
    }
    state.failures = 0;
    state.productionPaged = null;
  } else {
    state.failures += 1;
    lines.push(`production: ${problems.join('; ')} (${state.failures} in a row)`);
    if (state.failures >= FAIL_THRESHOLD && !state.productionPaged) {
      const sent = await page(context, PRODUCTION_TITLE, `cd-watch, ${at.toISOString()}: ${problems.join('; ')} — ${state.failures} probes in a row, 5 minutes apart.`);
      if (sent.length > 0) state.productionPaged = at.toISOString();
      lines.push(`production: paged (${sent.join(', ') || 'no channel answered; retrying next tick'})`);
    }
  }

  await env.WATCH.put(STATE_KEY, JSON.stringify(state));
  for (const line of lines) log.log(`cd-watch: ${line}`);
  return { problems, state, lines };
}

export async function handle(request, env) {
  const url = new URL(request.url);
  if (url.pathname === '/status' && request.method === 'GET') {
    const state = await readState(env);
    return reply(200, JSON.stringify(state), 'application/json');
  }
  return reply(404, 'not found');
}
