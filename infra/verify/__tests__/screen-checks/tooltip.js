const page = globalThis.document;
const { open, close, value, closeMs } = page.currentScript.dataset;
const tip = page.getElementById('tip');
let closing;
for (const target of page.querySelectorAll('.cut')) {
  target.addEventListener(open, () => {
    clearTimeout(closing);
    tip.textContent = value ?? target.textContent;
    tip.style.opacity = '1';
    tip.hidden = false;
  });
  target.addEventListener(close, () => {
    if (!closeMs) {
      tip.hidden = true;
      return;
    }
    tip.style.opacity = '0.5';
    closing = setTimeout(() => {
      tip.hidden = true;
    }, Number(closeMs));
  });
}
