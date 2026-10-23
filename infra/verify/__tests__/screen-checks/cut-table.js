const page = globalThis.document;
const tip = page.getElementById('tip');
const table = page.querySelector('table');
const rows = table.tBodies[0];
const ROWS = 12;
const CLAMPED = 'white-space: normal; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2';
const cut = (into, text, { tag = 'span', style = '' } = {}) => {
  const el = into.appendChild(page.createElement(tag));
  el.className = 'cut';
  el.textContent = text;
  if (style) el.setAttribute('style', style);
  return el;
};
const withTip = (el) => {
  el.addEventListener('mouseenter', () => {
    tip.textContent = el.textContent;
    tip.hidden = false;
  });
  el.addEventListener('mouseleave', () => {
    tip.hidden = true;
  });
};
const header = table.createTHead().insertRow();
header.appendChild(page.createElement('th'));
withTip(cut(header.appendChild(page.createElement('th')), 'Pause count is the long header of the column the editors read'));
for (let row = 1; row <= ROWS; row += 1) {
  const tr = rows.insertRow();
  const name = `Board ${row} is the long name of a word list kept for the weekly puzzle`;
  if (row === ROWS) cut(tr.insertCell(), name, { tag: 'strong' });
  else withTip(cut(tr.insertCell(), name));
  cut(tr.insertCell(), `Paused ${row} times while the editors rebuild the words list for the English board`, { style: row % 2 ? '' : CLAMPED });
}
const spanning = rows.insertRow();
spanning.insertCell().colSpan = 2;
cut(spanning.insertCell(), 'Rebuilt list waits for the editors to sign it off before Friday');
const covered = rows.rows[0].cells[1];
covered.style.position = 'relative';
covered.appendChild(page.createElement('div')).setAttribute('style', 'position: absolute; inset: 0; background: #fff');
for (const target of page.querySelectorAll('.cut'))
  target.addEventListener('mouseenter', () => {
    target.dataset.hovers = String(Number(target.dataset.hovers ?? '0') + 1);
  });
