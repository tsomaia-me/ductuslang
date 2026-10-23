import { spawn } from 'node:child_process';
import { createServer } from 'node:net';

const READY_MS = 180_000;
const POLL_MS = 500;
const STOP_MS = 10_000;

export function freePort() {
  return new Promise((done, fail) => {
    const probe = createServer().listen(0, '127.0.0.1', () => {
      const { port } = probe.address();
      probe.close(() => done(port));
    });
    probe.on('error', fail);
  });
}

async function answers(origin, deadline) {
  while (Date.now() < deadline) {
    try {
      if ((await fetch(origin)).ok) return;
    } catch {}
    await new Promise((wait) => setTimeout(wait, POLL_MS));
  }
  throw new Error(`nothing answered at ${origin} within ${READY_MS} ms`);
}

export function runQuietly(command, args, cwd) {
  return new Promise((done, fail) =>
    spawn(command, args, { cwd, stdio: 'ignore' })
      .on('error', fail)
      .on('exit', (code) => (code === 0 ? done() : fail(new Error(`${[command, ...args].join(' ')} exited ${code}`)))),
  );
}

export async function serveCommand({ name, cwd, command, args, env = {}, port }) {
  const origin = `http://localhost:${port}`;
  const server = spawn(command, args, { cwd, detached: true, stdio: 'ignore', env: { ...process.env, ...env } });
  const exited = new Promise((_, fail) => server.on('exit', (code) => fail(new Error(`${name} exited ${code}`))));
  exited.catch(() => {});
  const signal = (kind) => {
    try {
      process.kill(-server.pid, kind);
      return true;
    } catch {
      return false;
    }
  };
  const stop = async () => {
    if (!signal('SIGTERM')) return;
    for (let waited = 0; signal(0) && waited < STOP_MS; waited += POLL_MS) await new Promise((wait) => setTimeout(wait, POLL_MS));
    signal('SIGKILL');
  };
  try {
    await Promise.race([answers(origin, Date.now() + READY_MS), exited]);
  } catch (error) {
    await stop();
    throw new Error(`${name}: ${error.message}`);
  }
  return { origin, stop };
}
