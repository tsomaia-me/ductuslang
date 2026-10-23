const QUIET_MS = 400;
const CAP_MS = 4_000;

export const SHOTS_NOW = Date.parse('2026-10-10T12:00:00Z');
export const STILL = { animations: 'disabled' };

export const watchPage = (context) =>
  context.addInitScript(() => {
    let pending = 0;
    const fetchFirst = globalThis.fetch;
    globalThis.fetch = (...args) => {
      pending += 1;
      return fetchFirst.apply(globalThis, args).finally(() => {
        pending -= 1;
      });
    };
    globalThis.__proofPending = () => pending;
    globalThis.addEventListener('DOMContentLoaded', () => {
      const style = globalThis.document.createElement('style');
      style.textContent = '*, *::before, *::after { transition-duration: 0s !important; transition-delay: 0s !important; animation-duration: 0s !important; animation-delay: 0s !important; }';
      globalThis.document.head.append(style);
    });
    const track = (pressed) => (event) => {
      globalThis.__proofPointer = { x: event.clientX, y: event.clientY, pressed };
    };
    globalThis.addEventListener('mousemove', track(false), true);
    globalThis.addEventListener('mousedown', track(true), true);
  });

export async function settle(page, busy) {
  const pointer = await page.evaluate(() => globalThis.__proofPointer ?? null);
  if (pointer?.pressed) await page.mouse.move(0, 0);
  else if (pointer !== null) {
    await page.mouse.move(pointer.x + 1, pointer.y);
    await page.mouse.move(pointer.x, pointer.y);
  }
  return page.evaluate(
    ({ quiet, cap, busy }) =>
      globalThis.document.fonts.ready.then(
        () =>
          new Promise((done) => {
            let timer;
            const rearm = () => {
              clearTimeout(timer);
              timer = setTimeout(check, quiet);
            };
            const loading = () => (globalThis.__proofPending?.() ?? 0) > 0 || globalThis.document.querySelector(busy) !== null;
            const check = () => (loading() ? rearm() : finish());
            const observer = new globalThis.MutationObserver(rearm);
            const finish = () => {
              observer.disconnect();
              globalThis.document.removeEventListener('scroll', rearm, true);
              clearTimeout(timer);
              clearTimeout(limit);
              done();
            };
            observer.observe(globalThis.document, { subtree: true, childList: true, attributes: true, characterData: true });
            globalThis.document.addEventListener('scroll', rearm, true);
            const limit = setTimeout(finish, cap);
            rearm();
          }),
      ),
    { quiet: QUIET_MS, cap: CAP_MS, busy },
  );
}
