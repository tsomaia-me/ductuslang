const page = globalThis.document;
const tip = page.getElementById('tip');
const CLAMPED = 'white-space: normal; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2';
const APPS = 8;
const BOARDS = 2;
const TILES = 3;
const add = (into, tag, className = '') => {
  const el = into.appendChild(page.createElement(tag));
  if (className) el.className = className;
  return el;
};
const cut = (into, text, { tag = 'span', style = '' } = {}) => {
  const el = add(into, tag, 'cut');
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
const row = (stack, label) => {
  const line = add(stack, 'div', 'row');
  add(line, 'div', 'label').textContent = label;
  return add(line, 'div', 'value');
};
const card = (list, name, note, { noteTag = 'p', style = '' } = {}) => {
  const stack = add(add(list, 'div', 'card'), 'div', 'stack');
  withTip(cut(row(stack, 'Name'), name));
  return cut(row(stack, 'Note'), note, { tag: noteTag, style });
};
const apps = page.getElementById('apps');
for (let n = 1; n <= APPS; n += 1)
  card(apps, `App ${n} is the long name of an app on the word game platform`, `Note ${n} says the app is paused while its words list is rebuilt`, {
    noteTag: n === APPS ? 'em' : 'p',
    style: n % 2 ? '' : CLAMPED,
  });
const boards = page.getElementById('boards');
for (let n = 1; n <= BOARDS; n += 1) {
  const note = card(boards, `Board ${n} is the long name of a board kept for the weekly puzzle`, `Board note ${n} says the board waits for the editors to sign off`);
  cut(row(note.closest('.stack'), 'Owner'), `Owner ${n} is the editor who keeps the weekly puzzle board`);
}
const tiles = page.querySelector('.tiles');
for (let n = 1; n <= TILES; n += 1) cut(add(tiles, 'div', 'tile'), `Tile ${n} holds a word list waiting for the weekly puzzle`);
const covered = apps.querySelector('.card .row:last-child .value');
covered.style.position = 'relative';
add(covered, 'div').setAttribute('style', 'position: absolute; inset: 0; background: #fff');
for (const target of page.querySelectorAll('.cut'))
  target.addEventListener('mouseenter', () => {
    target.dataset.hovers = String(Number(target.dataset.hovers ?? '0') + 1);
  });
