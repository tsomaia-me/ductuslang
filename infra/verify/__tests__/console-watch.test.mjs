import { describe, expect, it } from 'vitest';

import { watchConsole } from '../interaction-checks.mjs';

const PAGE_URL = 'https://dev.crookedduke.com/home';
const refused = (url, code = 302) => `WebSocket connection to '${url}' failed: Error during WebSocket handshake: Unexpected response code: ${code}`;
const DEV_HUB = 'wss://dev.crookedduke.com/ws/hub?ticket=t1';

function fakePage() {
  const handlers = {};
  return {
    on: (event, handler) => {
      handlers[event] = handler;
    },
    log: (text, type = 'error') => handlers.console({ type: () => type, text: () => text, location: () => ({ url: PAGE_URL }) }),
    throw: (message) => handlers.pageerror(new Error(message)),
  };
}

const seenAfter = (act) => {
  const page = fakePage();
  const watch = watchConsole(page);
  act(page);
  return watch.take();
};

describe('watchConsole counts a refused /ws/hub handshake: the Access bridge carries the token to /ws (CD-326)', () => {
  it('counts a refused or closed /ws/hub handshake on any origin, any other console error, and a page error', () => {
    for (const [name, text] of [
      ['a 302 to the Access sign-in', refused(DEV_HUB, 302)],
      ['a 403', refused(DEV_HUB, 403)],
      ['no ticket', refused('wss://dev.crookedduke.com/ws/hub', 302)],
      ['another origin’s /ws/hub', refused('wss://api.crookedduke.com/ws/hub?ticket=t1')],
      ['a local /ws/hub', refused('ws://127.0.0.1:8787/ws/hub?ticket=t1', 403)],
      ['the dev /ws/hub closing before its handshake', `WebSocket connection to '${DEV_HUB}' failed: Connection closed before receiving a handshake response`],
      ['any other console error', 'Uncaught TypeError: Cannot read properties of undefined'],
    ])
      expect.soft(seenAfter((page) => page.log(text)), name).toEqual([{ element: PAGE_URL, detail: text }]);
    expect.soft(seenAfter((page) => page.throw(refused(DEV_HUB))), 'a page error').toEqual([{ element: 'page', detail: refused(DEV_HUB) }]);
  });

  it('does not count a resource that answered 4xx, or a console message that is not an error', () => {
    expect(
      seenAfter((page) => {
        page.log('Failed to load resource: the server responded with a status of 404 (Not Found)');
        page.log(refused(DEV_HUB), 'warning');
      }),
    ).toEqual([]);
  });
});
