const page = globalThis.document;
const first = page.querySelector('.cut');
const markup = first.outerHTML;
let renders = Number(first.dataset.renders ?? '0');
const replaceOnHover = (target) => {
  target.addEventListener(
    'mouseenter',
    () => {
      if (renders === 0) {
        if ('gone' in target.dataset) target.remove();
        return;
      }
      renders -= 1;
      target.insertAdjacentHTML('afterend', markup);
      const fresh = target.nextElementSibling;
      target.remove();
      page.body.dataset.rendered = String(Number(page.body.dataset.rendered ?? '0') + 1);
      replaceOnHover(fresh);
    },
    { once: true },
  );
};
replaceOnHover(first);
