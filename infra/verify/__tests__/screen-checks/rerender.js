const page = globalThis.document;
const tip = page.getElementById('tip');
const wire = (target) => {
  if (!tip) return;
  target.addEventListener('mouseenter', () => {
    tip.textContent = target.textContent;
    tip.hidden = false;
  });
  target.addEventListener('mouseleave', () => {
    tip.hidden = true;
  });
};
const first = page.querySelector('.cut');
const markup = first.outerHTML;
wire(first);
page.addEventListener(
  'mousemove',
  () => {
    const current = page.querySelector('.cut');
    current.insertAdjacentHTML('afterend', markup);
    const fresh = current.nextElementSibling;
    if (fresh.hasAttribute('data-title-later')) fresh.title = fresh.textContent;
    current.remove();
    wire(fresh);
  },
  { once: true },
);
