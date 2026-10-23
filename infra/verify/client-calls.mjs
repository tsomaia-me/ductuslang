const CALLS_OUTSIDE_REQUEST = ['POST /auth/refresh', 'POST /auth/logout', 'GET /auth/:p/start'];

const BASE_URL = '${this.baseUrl}';
const SEND_PATH_AND_QUERY = ':p:p';
const SNIPPET_CHARS = 80;

const HAND_LISTED_SITES = {
  '/friends/requests/:p/:p': { sites: 1, calls: ['POST /friends/requests/:p/accept', 'POST /friends/requests/:p/decline'] },
  '/games/:p/rematch:p': { sites: 1, calls: ['POST /games/:p/rematch', 'POST /games/:p/rematch/accept', 'POST /games/:p/rematch/decline'] },
  path: { sites: 3, calls: ['POST /auth/guest', 'POST /auth/register', 'POST /auth/login'] },
  [SEND_PATH_AND_QUERY]: { sites: 1, calls: [] },
};

const CLIENT_TOKEN = /\bthis\.(?:request|send)\b/g;
const CLIENT_SITE = /this\.(?:request|send)(?:<[^(]*?>)?\(\s*(`[^`]*`|'[^']*'|[^\s,)]+)/g;
const FETCH_TOKEN = /\bfetch\b/g;
const RAW_FETCH_SITE = /(?<![.\w])fetch\(\s*(`[^`]*`|'[^']*'|[^\s,)]+)/g;
const AMBIGUOUS_PATH = /:p:p|[a-z]:p$|\/:p\/:p$/;

function sitesOf(source, pattern) {
  return [...source.matchAll(pattern)].map((match) => {
    const argument = match[1].replace(BASE_URL, '');
    if (!argument.startsWith('`') && !argument.startsWith("'")) return { index: match.index, key: argument, call: null };
    const tail = source.slice(match.index, match.index + 400);
    const end = tail.indexOf(');');
    const method = /method:\s*'([A-Z]+)'/.exec(end > 0 ? tail.slice(0, end) : tail)?.[1] ?? 'GET';
    const path = argument.slice(1, -1).replace(/\$\{[^}]*\}/g, ':p').replace(/\?.*$/, '');
    return { index: match.index, key: path, call: AMBIGUOUS_PATH.test(path) ? null : `${method} ${path}` };
  });
}

function unmatchedTokens(source, token, sites, what) {
  const matched = new Set(sites.map((site) => site.index));
  return [...source.matchAll(token)]
    .filter((hit) => !matched.has(hit.index))
    .map((hit) => `\`${source.slice(hit.index, hit.index + SNIPPET_CHARS).split('\n')[0]}\`: ${what} the reader cannot match`);
}

export function clientCalls(source) {
  const calls = new Set(CALLS_OUTSIDE_REQUEST);
  for (const { call } of sitesOf(source, CLIENT_SITE)) if (call !== null) calls.add(call);
  for (const { calls: listed } of Object.values(HAND_LISTED_SITES)) for (const call of listed) calls.add(call);
  return [...calls].sort();
}

export function unreadSites(source) {
  const clientSites = sitesOf(source, CLIENT_SITE);
  const fetches = sitesOf(source, RAW_FETCH_SITE);
  const seen = {};
  for (const { key, call } of [...clientSites, ...fetches]) if (call === null) seen[key] = (seen[key] ?? 0) + 1;
  const miscounted = [...new Set([...Object.keys(seen), ...Object.keys(HAND_LISTED_SITES)])]
    .filter((key) => (seen[key] ?? 0) !== (HAND_LISTED_SITES[key]?.sites ?? 0))
    .map((key) => `\`${key}\`: ${seen[key] ?? 0} call site(s) in the client, ${HAND_LISTED_SITES[key]?.sites ?? 0} hand-listed in client-calls.mjs`);
  const unlisted = fetches
    .filter(({ call }) => call !== null && !CALLS_OUTSIDE_REQUEST.includes(call))
    .map(({ call }) => `\`${call}\`: a raw fetch call that CALLS_OUTSIDE_REQUEST in client-calls.mjs does not list`);
  return [
    ...miscounted,
    ...unlisted,
    ...unmatchedTokens(source, CLIENT_TOKEN, clientSites, 'this.request or this.send site'),
    ...unmatchedTokens(source, FETCH_TOKEN, fetches, 'fetch call'),
  ];
}

export function missingRoutes(calls, routes) {
  const patterns = routes.map((route) => ({
    method: route.method,
    pattern: new RegExp(`^${route.path.replace(/:[A-Za-z_]+(\{[^}]*\})?/g, '[^/]+')}$`),
  }));
  return calls.filter((call) => {
    const [method, path] = call.split(' ');
    const sample = path.replace(/:p/g, 'x1');
    return !patterns.some((route) => route.method === method && route.pattern.test(sample));
  });
}
