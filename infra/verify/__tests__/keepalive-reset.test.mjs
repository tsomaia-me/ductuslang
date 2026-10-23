import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { createServer as createTcpServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as idleFor } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { stripVTControlCharacters } from 'node:util';

import { request } from 'playwright';
import { afterAll, beforeAll, describe, it } from 'vitest';

import { NO_IDLE_TIMEOUT } from '../../e2e/idle-sockets.mjs';

const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const ADMIN_ROOT = join(REPO_ROOT, 'apps', 'admin');
const HAS_ADMIN_BUILD = existsSync(join(ADMIN_ROOT, 'dist', 'index.html'));
const HOST = '127.0.0.1';
const TRIALS = 5;
const BLOCKED_MS = 400;
const LEAD_MS = 200;
const READY_MS = 20_000;
const START_MS = READY_MS * 2;
const CONNECTION_RESET = /ECONNRESET|socket hang up/;
const DEFAULT_SERVER = createServer();
const DEFAULT_IDLE_CLOSE_MS = DEFAULT_SERVER.keepAliveTimeout + DEFAULT_SERVER.keepAliveTimeoutBuffer;
const TEST_MS = TRIALS * (DEFAULT_IDLE_CLOSE_MS + BLOCKED_MS) * 2;

const listeningOn = (port) => new RegExp(`http://${HOST}:${port}`);

const handedOut = new Set();
const probedPort = () =>
  new Promise((found) => {
    const probe = createTcpServer().listen(0, HOST, () => {
      const { port } = probe.address();
      probe.close(() => found(port));
    });
  });
const freePort = async () => {
  let port = await probedPort();
  while (handedOut.has(port)) port = await probedPort();
  handedOut.add(port);
  return port;
};

function readyLines(child, wanted, readyMs) {
  const seen = { stdout: '', stderr: '' };
  const output = () => Object.entries(seen).map(([stream, text]) => `--- ${stream}:\n${text}`).join('\n');
  return new Promise((found, failed) => {
    const listeners = [];
    const listen = (emitter, event, listener) => {
      emitter.on(event, listener);
      listeners.push(() => emitter.off(event, listener));
    };
    const settle = () => {
      clearTimeout(deadline);
      for (const remove of listeners) remove();
    };
    const giveUp = (why) => {
      settle();
      child.kill();
      const awaited = Object.entries(wanted).map(([stream, pattern]) => `${pattern} on ${stream}`);
      failed(new Error(`${why}, waiting for ${awaited.join(' and ')}\n${output()}`));
    };
    const check = () => {
      const lines = Object.entries(wanted).map(([stream, pattern]) => [stream, pattern.exec(stripVTControlCharacters(seen[stream]))?.[0]]);
      if (lines.some(([, line]) => line === undefined)) return;
      settle();
      found(Object.fromEntries(lines));
    };
    const deadline = setTimeout(() => giveUp(`the server was not ready within ${String(readyMs)} ms`), readyMs);
    for (const stream of Object.keys(seen)) {
      listen(child[stream], 'data', (chunk) => {
        seen[stream] += chunk;
        check();
      });
    }
    listen(child, 'close', (code, signal) => giveUp(`the server exited (${String(code ?? signal)}) before it was ready`));
    listen(child, 'error', (error) => giveUp(`the server could not start: ${error.message}`));
  });
}

async function inspectorOf(url) {
  const socket = new WebSocket(url);
  await new Promise((open, failed) => {
    socket.addEventListener('open', open, { once: true });
    socket.addEventListener('error', failed, { once: true });
  });
  const answers = new Map();
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    answers.get(message.id)?.(message);
  });
  let next = 0;
  return {
    evaluate: (expression) =>
      new Promise((answered, failed) => {
        next += 1;
        answers.set(next, ({ result }) =>
          result.exceptionDetails ? failed(new Error(result.exceptionDetails.text)) : answered(result.result.value),
        );
        socket.send(JSON.stringify({ id: next, method: 'Runtime.evaluate', params: { expression, returnByValue: true } }));
      }),
    close: () => socket.close(),
  };
}

async function startServer({ args, cwd, ready, readyMs = READY_MS }) {
  const port = await freePort();
  const inspectorPort = await freePort();
  const child = spawn(process.execPath, [`--inspect=${HOST}:${String(inspectorPort)}`, ...args(port)], {
    cwd,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const { stderr: inspectorUrl } = await readyLines(child, { stderr: /ws:\/\/\S+/, stdout: ready(port) }, readyMs);
  const inspector = await inspectorOf(inspectorUrl);
  return {
    origin: `http://${HOST}:${port}`,
    idleSettings: () =>
      inspector.evaluate(
        "process._getActiveHandles().filter((handle) => handle instanceof process.getBuiltinModule('node:http').Server).map((server) => ({ keepAliveTimeout: server.keepAliveTimeout }))",
      ),
    blockEventLoop: (ms) => inspector.evaluate(`{ const until = Date.now() + ${ms}; while (Date.now() < until); }`),
    stop: () => {
      inspector.close();
      child.kill();
    },
  };
}

const SERVERS = [
  {
    name: 'the app web export (apps/app/e2e/static-server.mjs)',
    available: true,
    start: (dist) =>
      startServer({
        args: (port) => [join(REPO_ROOT, 'apps', 'app', 'e2e', 'static-server.mjs'), dist, String(port)],
        cwd: REPO_ROOT,
        ready: () => /static server: /,
      }),
  },
  {
    name: 'the admin fixture server (apps/admin/scripts/fixture-server.mjs)',
    available: HAS_ADMIN_BUILD,
    start: () =>
      startServer({
        args: (port) => ['scripts/fixture-server.mjs', '--port', String(port), '--variant', 'long-service-dev'],
        cwd: ADMIN_ROOT,
        ready: listeningOn,
      }),
  },
];

async function resetsOfIdleReuse(server, api) {
  const resets = [];
  for (let trial = 0; trial < TRIALS; trial += 1) {
    await api.get(`${server.origin}/`);
    await idleFor(DEFAULT_IDLE_CLOSE_MS - LEAD_MS);
    const blocked = server.blockEventLoop(BLOCKED_MS);
    await idleFor(LEAD_MS / 2);
    try {
      await api.get(`${server.origin}/`);
    } catch (error) {
      const cause = String(error?.message).split('\n')[0];
      if (!CONNECTION_RESET.test(cause)) throw error;
      resets.push(cause);
    }
    await blocked;
  }
  return resets;
}

const running = new Map();
let dist;
let api;

beforeAll(async () => {
  dist = mkdtempSync(join(tmpdir(), 'keepalive-reset-'));
  writeFileSync(join(dist, 'index.html'), '<!doctype html><title>idle sockets</title>');
  api = await request.newContext();
  const available = SERVERS.filter((server) => server.available);
  const started = await Promise.allSettled(available.map((server) => server.start(dist)));
  available.forEach((server, at) => {
    if (started[at].status === 'fulfilled') running.set(server.name, started[at].value);
  });
  const failure = started.find(({ status }) => status === 'rejected');
  if (failure) throw failure.reason;
}, START_MS);

afterAll(async () => {
  for (const server of running.values()) server.stop();
  await api?.dispose();
  rmSync(dist, { recursive: true, force: true });
});

describe('starting a server for these checks', () => {
  const STUCK_MS = 500;
  const stayAlive = 'setInterval(() => {}, 1000);';

  it('finds a ready line printed in colour, as Vite prints its address under CI=true', { timeout: START_MS }, async ({ expect }) => {
    const started = startServer({
      args: (port) => ['-e', `${stayAlive} console.log('  \\x1b[36mhttp://${HOST}:\\x1b[1m${String(port)}\\x1b[22m/\\x1b[39m');`],
      cwd: REPO_ROOT,
      ready: listeningOn,
    });
    await expect(started.then((server) => server.stop())).resolves.toBeUndefined();
  });

  it('fails with the output so far when a running server never prints its ready line within its deadline, and at once when it exits first', async ({ expect }) => {
    const started = Date.now();
    await Promise.all([
      expect
        .soft(
          startServer({ args: () => ['-e', `${stayAlive} console.log('still warming up');`], cwd: REPO_ROOT, ready: () => /never printed/, readyMs: STUCK_MS }),
          'never ready',
        )
        .rejects.toThrow(new RegExp(`not ready within ${String(STUCK_MS)} ms[\\s\\S]*still warming up`)),
      expect
        .soft(startServer({ args: () => ['-e', "console.error('cannot listen'); process.exit(3);"], cwd: REPO_ROOT, ready: () => /never printed/ }), 'exits first')
        .rejects.toThrow(/exited \(3\) before it was ready[\s\S]*cannot listen/),
    ]);
    expect.soft(Date.now() - started, 'never ready').toBeLessThan(READY_MS);
  });
});

describe.concurrent('the local servers the journeys proxy through (CD-448)', () => {
  for (const { name, available } of SERVERS) {
    it(`${name} keeps every idle socket open: its running server has no keep-alive timeout`, { skip: !available }, async ({ expect }) => {
      expect(await running.get(name).idleSettings()).toEqual([{ keepAliveTimeout: NO_IDLE_TIMEOUT }]);
    });

    it(
      `${name} answers a pooled socket reused at Node's default idle close while its event loop is blocked, with no reset in ${String(TRIALS)} trials`,
      { skip: !available, timeout: TEST_MS },
      async ({ expect }) => {
        const resets = await resetsOfIdleReuse(running.get(name), api);
        console.log(`${name}: ${String(resets.length)} of ${String(TRIALS)} reuses reset: ${JSON.stringify(resets)}`);
        expect(resets).toEqual([]);
      },
    );
  }
});
