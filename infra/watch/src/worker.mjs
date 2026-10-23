import { handle, tick } from './watch.mjs';

export default {
  fetch(request, env) {
    return handle(request, env);
  },
  scheduled(_event, env, ctx) {
    ctx.waitUntil(tick({ env, fetchImpl: fetch, now: () => new Date() }));
  },
};
