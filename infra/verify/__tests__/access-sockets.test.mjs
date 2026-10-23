import { createServer } from 'node:http';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { WebSocketServer } from 'ws';

import { SOCKET_ABNORMAL_CLOSE, routeAccessHeaders } from '../../e2e/access.mjs';
import { browsersPath, launchChromium, useInstalledBrowsers } from '../interaction-check.mjs';

const TIMEOUT_MS = 60_000;
const ACCESS = { 'CF-Access-Client-Id': 'dummy-socket-id.access', 'CF-Access-Client-Secret': 'dummy-socket-secret-q7' };
const NO_ACCESS = [undefined, undefined];
const REFUSED = 403;
const TICKET = 'ticket-not-for-logs';
const HUB = `/ws/hub?ticket=${TICKET}`;
const REFUSING = `/ws/refusing?ticket=${TICKET}`;
const PAGE_CLOSE = { code: 4000, reason: 'page done' };
const SERVER_CLOSE = { code: 4001, reason: 'server done' };
const LOCAL_NETWORK_CHECKS = 'LocalNetworkAccessChecks,BlockInsecurePrivateNetworkRequests';

const hasChromium = browsersPath() !== undefined;
let server;
let sockets;
let site;
let elsewhere;
let browser;
const handshakes = [];
const closes = [];

beforeAll(async () => {
  sockets = new WebSocketServer({ noServer: true });
  server = createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end('<!doctype html><title>site</title>');
  });
  server.on('upgrade', (request, socket, head) => {
    const access = [request.headers['cf-access-client-id'], request.headers['cf-access-client-secret']];
    const path = new URL(request.url, 'http://fixture.test').pathname;
    handshakes.push({ host: request.headers.host, path, access });
    if (path === '/ws/refusing' || access.join() !== Object.values(ACCESS).join()) {
      socket.end(`HTTP/1.1 ${REFUSED} Forbidden\r\nconnection: close\r\n\r\n`);
      return;
    }
    sockets.handleUpgrade(request, socket, head, (client) => {
      client.on('message', (data, binary) => {
        if (!binary && String(data) === 'close me') return client.close(SERVER_CLOSE.code, SERVER_CLOSE.reason);
        return client.send(binary ? Buffer.concat([Buffer.from([0]), data]) : `echo ${String(data)}`, { binary });
      });
      client.on('close', (code, reason) => closes.push({ code, reason: String(reason) }));
    });
  });
  await new Promise((ready) => server.listen(0, '127.0.0.1', ready));
  const { port } = server.address();
  site = `http://127.0.0.1:${port}`;
  elsewhere = `http://localhost:${port}`;
  useInstalledBrowsers();
  if (hasChromium) browser = await launchChromium({ args: [`--disable-features=${LOCAL_NETWORK_CHECKS}`] });
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
  sockets.close();
  await new Promise((done) => server.close(done));
});

const socketTo = (origin, path) => `${origin.replace(/^http/, 'ws')}${path}`;

async function inContext(headers, walk) {
  handshakes.length = 0;
  closes.length = 0;
  const context = await browser.newContext();
  try {
    const bridge = await routeAccessHeaders(context, [site], headers);
    const page = await context.newPage();
    await page.goto(`${site}/`);
    const result = await walk(page, context);
    return { result, failures: bridge.socketFailures() };
  } finally {
    await context.close();
  }
}

const talk = (page, address, script) =>
  page.evaluate(
    ({ address, script }) =>
      new Promise((done) => {
        const heard = [];
        const socket = new WebSocket(address);
        socket.binaryType = 'arraybuffer';
        socket.onopen = () => {
          for (const message of script.send) socket.send(message === 'binary' ? new Uint8Array([1, 2, 3]) : message);
          if (script.closeWith) socket.close(script.closeWith.code, script.closeWith.reason);
        };
        socket.onmessage = (event) => {
          heard.push(typeof event.data === 'string' ? event.data : [...new Uint8Array(event.data)]);
          if (heard.length === script.expect && script.closeAfter) socket.close(script.closeAfter.code, script.closeAfter.reason);
        };
        socket.onclose = (event) => done({ heard, close: { code: event.code, reason: event.reason } });
      }),
    { address, script },
  );

describe.skipIf(!hasChromium)('the Access bridge for WebSockets (CD-326)', { timeout: TIMEOUT_MS }, () => {
  it('opens the protected origin\'s socket from Node with the Access headers, and frames pass both ways', async () => {
    const { result, failures } = await inContext(ACCESS, (page) =>
      talk(page, socketTo(site, HUB), { send: ['hi', 'binary'], expect: 2, closeAfter: PAGE_CLOSE }),
    );
    expect(handshakes).toEqual([{ host: new URL(site).host, path: '/ws/hub', access: Object.values(ACCESS) }]);
    expect(result.heard).toEqual(['echo hi', [0, 1, 2, 3]]);
    expect(failures).toEqual([]);
  });

  it('passes a close from the page to the server and one from the server to the page, with code and reason', async () => {
    const fromPage = await inContext(ACCESS, (page) => talk(page, socketTo(site, HUB), { send: [], closeWith: PAGE_CLOSE }));
    expect(fromPage.result.close.code).toBe(PAGE_CLOSE.code);
    await expect.poll(() => closes).toEqual([PAGE_CLOSE]);
    const fromServer = await inContext(ACCESS, (page) => talk(page, socketTo(site, HUB), { send: ['close me'] }));
    expect(fromServer.result.close).toEqual(SERVER_CLOSE);
  });

  it('bridges no other origin and nothing without Access headers: the page opens its own socket, which never sees the headers', async () => {
    for (const [name, headers, origin] of [
      ['another origin', ACCESS, elsewhere],
      ['no Access headers', {}, site],
    ]) {
      const { result, failures } = await inContext(headers, (page) => talk(page, socketTo(origin, HUB), { send: [] }));
      expect.soft(handshakes, name).toEqual([{ host: new URL(origin).host, path: '/ws/hub', access: NO_ACCESS }]);
      expect.soft(result.close.code, name).toBe(SOCKET_ABNORMAL_CLOSE);
      expect.soft(failures, name).toEqual([]);
    }
  });

  it('closes the page\'s socket when the handshake is refused, and names the address without its ticket or the headers', async () => {
    const { result, failures } = await inContext(ACCESS, (page) => talk(page, socketTo(site, REFUSING), { send: [] }));
    expect(result.close.code).toBe(SOCKET_ABNORMAL_CLOSE);
    expect(failures).toEqual([`${socketTo(site, '/ws/refusing')}: the handshake through the Access bridge failed`]);
    for (const secret of [TICKET, ...Object.values(ACCESS)]) expect(failures.join('\n')).not.toContain(secret);
  });

  it('drops bridged sockets when the context goes offline, and opens none while it is', async () => {
    const { result } = await inContext(ACCESS, async (page, context) => {
      await page.evaluate((address) => {
        const socket = new WebSocket(address);
        globalThis.bridgedClose = new Promise((done) => {
          socket.onclose = (event) => done(event.code);
        });
        return new Promise((opened) => {
          socket.onopen = opened;
        });
      }, socketTo(site, HUB));
      await context.setOffline(true);
      const dropped = await page.evaluate(() => globalThis.bridgedClose);
      const offline = await talk(page, socketTo(site, HUB), { send: [] });
      return { dropped, offline: offline.close.code, handshakes: handshakes.length };
    });
    expect(result).toEqual({ dropped: SOCKET_ABNORMAL_CLOSE, offline: SOCKET_ABNORMAL_CLOSE, handshakes: 1 });
  });
});
