import { execFile } from 'node:child_process';
import { createServer } from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { afterAll, beforeAll, describe, expect, it } from 'vitest';

import {
  browsersPath,
  launchChromium,
  parseArgs,
  runInteractionCheck,
} from '../interaction-check.mjs';
import { checkOverlays, checkPage, settleLayout, watchConsole } from '../interaction-checks.mjs';

const run = promisify(execFile);
const here = dirname(fileURLToPath(import.meta.url));
const CLI = join(here, '..', 'interaction-check.mjs');
const TIMEOUT_MS = 60_000;
const SETTLE_CAP_MS = 300;

const LONG =
  'CD-314 second walk: test grant of 5 coins to a test player to check the whole path from the form to the wallet';

const CHANGED = '13 Oct 2026, 21:40 by owner@example.com';

const page = (body, head = '') =>
  `<!doctype html><html><head><meta charset="utf-8"><style>body{font:14px sans-serif}${head}</style></head><body>${body}</body></html>`;

const OVERFLOWING_TABLE = page(`
  <table style="table-layout:fixed;width:300px;border-collapse:collapse">
    <thead><tr><th>Reason</th><th>Granted by</th></tr></thead>
    <tbody><tr><td style="white-space:nowrap">${LONG}</td><td>owner</td></tr></tbody>
  </table>`);

const WRAPPING_TABLE = page(`
  <table style="width:300px;border-collapse:collapse">
    <thead><tr><th>Reason</th><th>Granted by</th></tr></thead>
    <tbody><tr><td>${LONG}</td><td>owner</td></tr></tbody>
  </table>`);

const CUT = (attributes = '', script = '', width = 80) =>
  page(
    `<p id="t" ${attributes} style="width:${width}px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${LONG}</p>
     <div id="tip" role="tooltip" hidden></div><script>${script}</script>`,
  );

const SHOW_TIP = (event, text) =>
  `const t=document.getElementById('t'),tip=document.getElementById('tip');
   t.addEventListener('${event}',()=>{tip.textContent=${JSON.stringify(text)};tip.hidden=false});
   t.addEventListener('${event === 'focus' ? 'blur' : 'mouseleave'}',()=>{tip.hidden=true});`;

const CLAMPED = page(
  `<p style="width:140px;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:2;overflow:hidden">${LONG}</p>`,
);

const CHANGED_TABLE = (columnWidth, cellStyle = '', cellText = CHANGED) =>
  page(`
  <table style="table-layout:fixed;width:${columnWidth + 200}px;border-collapse:collapse">
    <thead><tr><th style="width:200px">Name</th><th>Changed</th></tr></thead>
    <tbody><tr><td>Crooked Duke</td><td style="overflow:hidden;white-space:nowrap;padding:0 8px;${cellStyle}">${cellText}</td></tr></tbody>
  </table>`);

const CLIPPED_IN = (box, content = LONG) =>
  page(`<div style="${box}">${content}</div>`);

const POPOVER = (outsideButton, extra = '') =>
  page(`
    <button id="chip" aria-haspopup="dialog" aria-expanded="false">Who: Any</button>
    <div id="pop" role="dialog" hidden>Anyone</div>
    <button id="new">New group</button>
    <div id="modal" role="dialog" aria-modal="true" hidden>New group</div>
    ${extra}
    <script>
      const chip=document.getElementById('chip'),pop=document.getElementById('pop');
      const set=(open)=>{pop.hidden=!open;chip.setAttribute('aria-expanded',String(open))};
      chip.addEventListener('click',()=>set(pop.hidden));
      document.addEventListener('mousedown',(e)=>{
        if(!pop.hidden&&!pop.contains(e.target)&&!chip.contains(e.target))set(false);
      });
      ${outsideButton}
      document.getElementById('new').addEventListener('click',()=>{document.getElementById('modal').hidden=false});
    </script>`);

const COVERING_FROM_LOAD = POPOVER(
  '',
  `<p style="margin:80px 0 0">Page text under the menu bar</p><div role="menu" style="position:fixed;top:100px;left:0;right:0;height:40px;background:#222">Menu bar</div>`,
);

const ENTERING_MS = 150;
const MOUNT_MS = 60;
const FILL_IN_MS = 250;

const ENTERING_PANEL = ({ panel, open }) =>
  page(`
  <button id="chip" aria-haspopup="dialog" aria-expanded="false">Who: Any</button>
  <div id="pop" role="dialog" hidden style="position:absolute;top:60px;left:200px">Anyone</div>
  <button id="roll">Roll up</button>
  ${panel}
  <script>
    const chip=document.getElementById('chip'),pop=document.getElementById('pop'),panel=document.getElementById('panel');
    const set=(open)=>{pop.hidden=!open;chip.setAttribute('aria-expanded',String(open))};
    chip.addEventListener('click',()=>set(pop.hidden));
    document.addEventListener('mousedown',(e)=>{
      if(!pop.hidden&&!pop.contains(e.target)&&!chip.contains(e.target))set(false);
    });
    document.getElementById('roll').addEventListener('click',()=>{${open}});
  </script>`);

const FADING_IN = (role, modal = '') =>
  `<div id="panel" role="${role}" ${modal} style="display:none;opacity:0;transition:opacity 100ms">Roll up the stats</div>`;
const SHOW_THEN_FADE_IN = `panel.style.display='block';setTimeout(()=>{panel.style.opacity='1'},${ENTERING_MS});`;
const MOUNT_AFTER_THE_PRESS = `setTimeout(()=>document.body.append(document.getElementById('drawer').content.cloneNode(true)),${MOUNT_MS});`;

const ENTERING_MODAL = ENTERING_PANEL({ panel: FADING_IN('dialog', 'aria-modal="true"'), open: SHOW_THEN_FADE_IN });
const ENTERING_DIALOG = ENTERING_PANEL({ panel: FADING_IN('dialog'), open: SHOW_THEN_FADE_IN });
const ENTERING_MENU = ENTERING_PANEL({ panel: FADING_IN('menu'), open: SHOW_THEN_FADE_IN });
const ENTERING_DRAWER = ENTERING_PANEL({
  panel: `<template id="drawer"><aside id="panel" role="dialog" aria-modal="true" style="position:fixed;top:0;right:0;width:240px;height:100%;background:#222">Filters</aside></template>`,
  open: MOUNT_AFTER_THE_PRESS,
});

const MOUNT_ON_OPEN = `document.body.append(document.getElementById('late').content.cloneNode(true));const mounted=document.getElementById('panel');setTimeout(()=>{mounted.style.opacity='1'},${ENTERING_MS});`;
const MOUNTED_FADING_IN = (role) =>
  ENTERING_PANEL({
    panel: `<template id="late"><div id="panel" role="${role}" style="opacity:0;transition:opacity 100ms">Roll up the stats</div></template>`,
    open: MOUNT_ON_OPEN,
  });
const MOUNTED_DIALOG = MOUNTED_FADING_IN('dialog');
const MOUNTED_MENU = MOUNTED_FADING_IN('menu');

const ARMING_POPOVER = page(`
  <button id="chip" aria-haspopup="dialog" aria-expanded="false">Who: Any</button>
  <div id="pop" role="dialog" hidden style="opacity:0;transition:opacity 150ms">Anyone</div>
  <script>
    const chip=document.getElementById('chip'),pop=document.getElementById('pop');
    let armed=false;
    chip.addEventListener('click',()=>{
      chip.setAttribute('aria-expanded','true');
      setTimeout(()=>{pop.hidden=false;requestAnimationFrame(()=>{pop.style.opacity='1'});armed=true},150);
    });
    document.addEventListener('mousedown',(e)=>{
      if(armed&&!pop.contains(e.target)&&!chip.contains(e.target)){
        armed=false;pop.hidden=true;pop.style.opacity='0';chip.setAttribute('aria-expanded','false');
      }
    });
  </script>`);

const STUCK_POPOVER = page(`
  <button id="chip" aria-haspopup="menu" aria-expanded="false">Row actions</button>
  <div id="pop" role="menu" hidden>Open</div>
  <script>
    const chip=document.getElementById('chip'),pop=document.getElementById('pop');
    chip.addEventListener('click',()=>{pop.hidden=false;chip.setAttribute('aria-expanded','true')});
  </script>`);

const UNMARKED_STUCK_MENU = (afterOpen = '') =>
  page(`
  <button id="chip" aria-haspopup="menu">Row actions</button>
  <template id="late"><div id="panel" role="menu">Open</div></template>
  <script>
    document.getElementById('chip').addEventListener('click',()=>{
      document.body.append(document.getElementById('late').content.cloneNode(true));
      ${afterOpen}
    });
  </script>`);

const FILLS_IN_AFTER_SHOWN = `setTimeout(()=>{document.getElementById('panel').textContent='Rename, Duplicate, Delete'},${FILL_IN_MS});`;

const ACTION_TIMEOUT_MS = 3000;
const LOADING_STEP_MS = 300;

const CLOSING_CHIPS = (names, { hideWhenClosed = false } = {}) =>
  names
    .map(
      (name, at) => `<button class="chip" aria-haspopup="dialog" aria-expanded="false" data-pop="pop${at}">${name}</button>
        <div id="pop${at}" role="dialog" hidden>Anyone</div>`,
    )
    .join('') +
  `<script>
    for (const chip of document.querySelectorAll('.chip')) {
      const pop=document.getElementById(chip.dataset.pop);
      const set=(open)=>{pop.hidden=!open;chip.setAttribute('aria-expanded',String(open));if(!open&&${hideWhenClosed})chip.hidden=true};
      chip.addEventListener('click',()=>set(pop.hidden));
      document.addEventListener('mousedown',(e)=>{
        if(!pop.hidden&&!pop.contains(e.target)&&!chip.contains(e.target))set(false);
      });
    }
  </script>`;

const STUCK_MENU = `<button id="actions" aria-haspopup="menu" aria-expanded="false">Row actions</button>
  <div id="menu" role="menu" hidden>Open</div>
  <script>
    document.getElementById('actions').addEventListener('click',()=>{
      document.getElementById('menu').hidden=false;
      document.getElementById('actions').setAttribute('aria-expanded','true');
    });
  </script>`;

const NEW_GROUP_OPENS_MODAL = `<button id="new">New group</button>
  <div id="modal" role="dialog" aria-modal="true" hidden>New group</div>
  <script>document.getElementById('new').addEventListener('click',()=>{document.getElementById('modal').hidden=false});</script>`;

const FAR_BELOW_THE_BUTTON = page(
  `${NEW_GROUP_OPENS_MODAL}<div style="height:2400px"></div>${CLOSING_CHIPS(['Who: Any', 'When: Any'])}${STUCK_MENU}`,
);

const TRIGGER_HIDES_WHEN_CLOSED = page(
  `${NEW_GROUP_OPENS_MODAL}${CLOSING_CHIPS(['Who: Any'], { hideWhenClosed: true })}`,
);

const RENDERED_AFTER_LOADING = page(`
  <div id="screen"></div>
  <template id="loaded">${NEW_GROUP_OPENS_MODAL}${CLOSING_CHIPS(['Who: Any'])}${STUCK_MENU}</template>
  <script>
    fetch('/slow').then(()=>fetch('/slow')).then(()=>{
      const screen=document.getElementById('screen');
      screen.append(document.getElementById('loaded').content.cloneNode(true));
      for (const script of screen.querySelectorAll('script')) {
        const live=document.createElement('script');
        live.textContent=script.textContent;
        script.replaceWith(live);
      }
    });
  </script>`);

const LIVE_POLL_MS = 400;

const POLLS_ONCE_PRESSED = page(`
  ${NEW_GROUP_OPENS_MODAL}${STUCK_MENU}
  <script>
    if (sessionStorage.getItem('live')) setInterval(()=>fetch('/echo'),${LIVE_POLL_MS});
    document.getElementById('actions').addEventListener('click',()=>sessionStorage.setItem('live','1'));
  </script>`);

const STUCK_ROW_ACTIONS = [
  ['button "Row actions"', 'still open after a press outside it'],
  ['button "Row actions"', 'still open after a press on the button "New group"'],
];

const ROW = (target) =>
  page(`
    <table><thead><tr><th>Name</th></tr></thead>
      <tbody><tr id="row" style="cursor:pointer"><td>Ana Lomidze</td></tr></tbody></table>
    <script>document.getElementById('row').addEventListener('click',()=>{${target}});</script>`);

const TAP_CHIP = (chipStyle) =>
  page(
    `<div style="${chipStyle};width:150px;padding:0 8px;background:#345;border-radius:10px;overflow:visible">
       <p id="t" tabindex="0" title="${LONG}" style="margin:0;line-height:20px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${LONG}</p>
     </div>
     <div style="margin-top:2px;width:150px;height:20px;background:#543">Another chip</div>
     <script>
       const t=document.getElementById('t');let touched=false;
       t.addEventListener('pointerdown',(e)=>{touched=e.pointerType==='touch'});
       t.addEventListener('click',(e)=>{
         if(!touched)return;
         e.stopPropagation();
         const open=t.style.whiteSpace==='normal';
         t.style.whiteSpace=open?'nowrap':'normal';
         t.style.overflow=open?'hidden':'visible';
         t.style.textOverflow=open?'ellipsis':'clip';
       });
     </script>`,
  );

const PINNED_TABLE = (width) =>
  page(`
  <div style="width:${width}px;overflow:auto">
    <table style="border-collapse:collapse;width:620px;table-layout:fixed">
      <thead><tr><th style="width:200px">Change</th><th style="width:300px">Status</th>
        <th style="width:120px;position:sticky;right:0;background:#222">Menu</th></tr></thead>
      <tbody><tr><td>+ 50 coins</td><td style="white-space:nowrap">Applied and settled by the ledger worker</td>
        <td style="position:sticky;right:0;background:#222"><button>Row actions</button></td></tr></tbody>
    </table>
  </div>`);

const PINNED_TRUNCATED_TABLE = page(`
  <div style="width:500px;overflow:auto">
    <table style="border-collapse:collapse;width:620px;table-layout:fixed">
      <thead><tr><th style="width:200px">Change</th><th style="width:300px">Changed</th>
        <th style="width:120px;position:sticky;right:0;background:#222">Menu</th></tr></thead>
      <tbody><tr><td>+ 50 coins</td>
        <td><p title="${CHANGED}" style="margin:0;width:150px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${CHANGED}</p></td>
        <td style="position:sticky;right:0;background:#222"><button>Row actions</button></td></tr></tbody>
    </table>
  </div>`, 'th{text-align:left}');

const PAGES = {
  '/overflow-bug': OVERFLOWING_TABLE,
  '/overflow-clean': WRAPPING_TABLE,
  '/cut-bug': CUT(),
  '/cut-clamp-bug': CLAMPED,
  '/cut-title': CUT(`title="${LONG}"`),
  '/cut-title-other': CUT()
    .replace('<p id="t"', '<div title="Open details"><p id="t"')
    .replace('</p>', '</p></div>'),
  '/cut-title-ancestor': CUT()
    .replace('<p id="t"', `<div title="${LONG}"><p id="t"`)
    .replace('</p>', '</p></div>'),
  '/cut-tooltip': CUT('', SHOW_TIP('mouseenter', LONG)),
  '/cut-tooltip-wrong': CUT('', SHOW_TIP('mouseenter', 'Something else')),
  '/cut-focus': CUT('tabindex="0"', SHOW_TIP('focus', LONG)),
  '/clip-bug': CHANGED_TABLE(90),
  '/clip-wide': CHANGED_TABLE(400),
  '/clip-ellipsis-elsewhere': CHANGED_TABLE(
    90,
    'text-overflow:ellipsis',
    `<p title="${CHANGED}" style="white-space:nowrap;margin:0">${CHANGED}</p>`,
  ),
  '/clip-titled': CHANGED_TABLE(
    90,
    '',
    `<span title="${CHANGED}">${CHANGED}</span>`,
  ),
  '/clip-tall': CLIPPED_IN('width:200px;height:20px;overflow:hidden', `${LONG} ${LONG}`),
  '/clip-tall-fits': CLIPPED_IN('width:200px;height:200px;overflow:hidden', `${LONG} ${LONG}`),
  '/clip-scroll': CLIPPED_IN('width:120px;overflow:auto;white-space:nowrap'),
  '/clip-hidden-label': CLIPPED_IN(
    'position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap',
    'Close the dialog and go back to the list',
  ),
  '/clip-tight': page(
    `<span id="s" style="display:inline-block;overflow:hidden">Keep yours</span>
     <script>
       const s=document.getElementById('s');
       const r=document.createRange();r.selectNodeContents(s);
       s.style.height=(r.getBoundingClientRect().height-2)+'px';
     </script>`,
  ),
  '/clip-page-locked': page(
    `<p style="white-space:nowrap;margin-left:600px">${LONG}</p>`,
    'body{overflow:hidden;width:300px}',
  ),
  '/clip-tooltip': page(
    `<div style="position:relative;width:200px;height:40px;overflow:hidden"><div role="tooltip" style="position:absolute;white-space:nowrap">${LONG}</div></div>`,
  ),
  '/clip-collapsed': CLIPPED_IN('height:0;overflow:hidden'),
  '/clip-escaped': page(
    `<div style="width:60px;height:20px;overflow:hidden"><div style="position:absolute;white-space:nowrap">${LONG}</div></div>`,
  ),
  '/clip-clamped-wide': page(
    `<div style="width:140px;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:2;overflow:hidden" title="${LONG}">${LONG}</div>`,
  ),
  '/cut-fits': CUT('', '', 2000),
  '/grows': page(
    `<div id="rows"></div>
     <script>
       let rows = 0;
       const add = () => {
         const row = document.createElement('p');
         row.style.cssText = 'height:60px;margin:0';
         row.textContent = 'row';
         document.getElementById('rows').append(row);
         rows += 1;
         if (rows < 8) setTimeout(add, 60);
       };
       setTimeout(add, 60);
     </script>`,
  ),
  '/still': page('<div id="rows"><p>row</p></div>'),
  '/grows-forever': page(
    `<div id="rows"></div>
     <script>
       setInterval(() => {
         const row = document.createElement('p');
         row.textContent = 'row';
         document.getElementById('rows').append(row);
       }, 30);
     </script>`,
  ),
  '/covered-bug': PINNED_TABLE(500),
  '/covered-fits': PINNED_TABLE(700),
  '/covered-truncated': PINNED_TRUNCATED_TABLE,
  '/covered-dialog': page(
    `<div role="dialog" aria-modal="true" style="position:sticky;top:0;height:40px;margin-bottom:-40px;background:#222">New app</div><p style="margin:0;padding-top:8px">Page text under the dialog</p>`,
  ),
  '/covered-fixed-bug': page(
    `<p style="margin:0;padding-top:8px">Page text under the bar</p><div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Bar</div>`,
  ),
  '/covered-fixed-scrolled': page(
    `<div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Header</div><div style="height:1500px;padding-top:60px"><p style="margin:0">Page text first</p><p style="margin:300px 0 0">Page text second</p></div><script>addEventListener('load',()=>scrollTo(0,50))</script>`,
  ),
  '/covered-fixed-scroll-wide': page(
    `<div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Header</div><div id="wide" style="display:none;position:absolute;top:0;left:0;width:3000px;height:1px"></div><div style="height:1500px;padding-top:60px"><p style="margin:0">Page text first</p></div><script>let hide;addEventListener('scroll',()=>{document.getElementById('wide').style.display='block';clearTimeout(hide);hide=setTimeout(()=>{document.getElementById('wide').style.display='none'},120)});addEventListener('load',()=>scrollTo(0,50))</script>`,
  ),
  '/covered-fixed-own': page(
    `<div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Text in the bar itself</div>`,
  ),
  '/covered-fixed-in-dialog': page(
    `<p style="margin:0;padding-top:8px">Page text under the dialog</p><div role="dialog" aria-modal="true"><div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Inside the dialog</div></div>`,
  ),
  '/covered-fixed-tooltip': page(
    `<div style="position:fixed;top:0;left:0;width:300px;height:100px;background:#222">Navigation</div><div role="tooltip" style="position:absolute;top:20px;left:20px;pointer-events:none;background:#fff;color:#000">A tooltip over the bar</div>`,
  ),
  '/covered-fixed-scrim': page(
    `<p style="margin:0;padding-top:8px">Page text under the scrim</p><div><div style="position:fixed;inset:0;background:#0008"></div><div role="dialog" aria-modal="true" style="position:fixed;top:60px;left:60px;width:200px;height:80px;background:#222">The dialog</div></div>`,
  ),
  '/covered-fixed-footer-bug': page(
    `<div style="height:2000px"><p style="margin:0">Top of a long page</p></div><p style="margin:0">Last line of the page</p><div style="position:fixed;bottom:0;left:0;right:0;height:40px;background:#222">Footer</div>`,
  ),
  '/covered-fixed-footer-padded': page(
    `<div style="height:2000px"><p style="margin:0">Top of a long page</p></div><p style="margin:0;padding-bottom:60px">Last line of the page</p><div style="position:fixed;bottom:0;left:0;right:0;height:40px;background:#222">Footer</div>`,
  ),
  '/covered-fixed-header-long': page(
    `<div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Header</div><div style="padding-top:60px">${Array.from({ length: 30 }, (_, row) => `<p style="margin:0;height:100px">Row ${row} of a long page</p>`).join('')}</div>`,
  ),
  '/covered-fixed-dialog-wrapper': page(
    `<p style="margin:0;padding-top:8px">Page text behind the drawer</p><div><div style="position:fixed;inset:0;background:#0008"></div><div style="position:fixed;inset:0;background:#222"><section role="dialog" aria-modal="true">Drawer text</section></div></div>`,
  ),
  '/covered-fixed-bar-with-dialog': page(
    `<p style="margin:0;padding-top:8px">Page text under the bar</p><div style="position:fixed;top:0;left:0;right:0;height:40px;background:#222">Bar</div><div role="dialog" aria-modal="true" style="position:fixed;top:300px;left:300px;width:200px;height:80px;background:#222">An open dialog</div>`,
  ),
  '/covered-middle': page(
    `<p style="margin:0;white-space:nowrap">${LONG}</p><div style="position:sticky;top:0;height:17px;margin:-17px 0 0 300px;width:150px;background:#222">Pinned</div>`,
  ),
  '/tap-chip-bug': TAP_CHIP('height:20px'),
  '/tap-chip-grows': TAP_CHIP('min-height:20px'),
  '/tap-does-nothing': CUT(`tabindex="0" title="${LONG}"`),
  '/tap-in-button': page(
    `<button id="b" style="width:80px;overflow:hidden"><span id="t" tabindex="-1" style="display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${LONG}</span></button>
     <script>document.getElementById('b').addEventListener('click',()=>fetch('/api/save',{method:'POST',body:'{}'}))</script>`,
  ),
  '/stop-in-button': page(
    `<button style="width:80px;overflow:hidden"><span tabindex="0" style="display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${LONG}</span></button>`,
  ),
  '/stop-in-link': page(`<a href="/echo"><span tabindex="0">Open the wallet</span></a>`),
  '/stop-in-field': page(`<button>Choose <input aria-label="Amount"></button>`),
  '/stop-in-select': page(`<div role="button">Choose <select aria-label="Kind"><option>One</option></select></div>`),
  '/stop-in-textarea': page(`<div role="tab">Choose <textarea aria-label="Note"></textarea></div>`),
  '/stop-in-editable': page(`<div role="menuitem">Choose <span contenteditable="true">Edit me</span></div>`),
  '/stop-in-nested-button': page(
    `<button id="outer">Choose</button><script>const inner = document.createElement('button'); inner.textContent = 'Inner'; document.getElementById('outer').append(inner);</script>`,
  ),
  '/stop-in-nested-link': page(
    `<a id="outer" href="/echo">Open</a><script>const inner = document.createElement('a'); inner.href = '/echo'; inner.textContent = 'Inner'; document.getElementById('outer').append(inner);</script>`,
  ),
  '/stop-in-quiet-children': page(
    `<button>Choose <input disabled aria-label="Off"><span tabindex="-1">Skipped</span><button id="inner" tabindex="-1" type="button">Not a stop</button></button><a href="/echo">Link <span contenteditable="false">Plain</span></a>`,
  ),
  '/stop-clean': page(
    `<button><span>Choose</span></button><span tabindex="0">Free text</span><button tabindex="0">Plain</button><div role="option" tabindex="0">Option</div>`,
  ),
  '/cut-composite': page(
    `<div id="t" style="width:140px;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:2;overflow:hidden"><b>Admin</b><br><span>${LONG}</span></div>
     <div id="tip" role="tooltip" hidden></div><script>${SHOW_TIP('mouseenter', LONG)}</script>`,
  ),
  '/cut-covered': CUT().replace(
    '</body>',
    '<div role="dialog" aria-modal="true" style="position:fixed;inset:0;background:#0008">A dialog</div></body>',
  ),
  '/overlay-bug': POPOVER(
    "document.getElementById('new').addEventListener('mousedown',(e)=>e.stopPropagation());",
  ),
  '/overlay-clean': POPOVER(''),
  '/overlay-stuck': STUCK_POPOVER,
  '/overlay-far-below-the-button': FAR_BELOW_THE_BUTTON,
  '/overlay-trigger-hides': TRIGGER_HIDES_WHEN_CLOSED,
  '/overlay-rendered-after-loading': RENDERED_AFTER_LOADING,
  '/overlay-polls-once-pressed': POLLS_ONCE_PRESSED,
  '/overlay-arming': ARMING_POPOVER,
  '/overlay-entering-modal': ENTERING_MODAL,
  '/overlay-entering-dialog': ENTERING_DIALOG,
  '/overlay-entering-menu': ENTERING_MENU,
  '/overlay-entering-drawer': ENTERING_DRAWER,
  '/overlay-mounted-dialog': MOUNTED_DIALOG,
  '/overlay-mounted-menu': MOUNTED_MENU,
  '/overlay-unmarked-stuck': UNMARKED_STUCK_MENU(),
  '/overlay-unmarked-stuck-filling': UNMARKED_STUCK_MENU(FILLS_IN_AFTER_SHOWN),
  '/overlay-covering-from-load': COVERING_FROM_LOAD,
  '/row-bug': ROW("location.href='/elsewhere'"),
  '/row-search': ROW("history.pushState({}, '', location.pathname + '?entry=1')"),
  '/rows': ROW("location.href='/rows/1'"),
  '/rows/1': page('<p>Row one</p>'),
  '/releases/versions': ROW("location.href='/releases/drafts/7'"),
  '/releases/drafts/7': page('<p>Draft seven</p>'),
  '/releases/stray': ROW("location.href='/users/1'"),
  '/users/1': page('<p>Ana</p>'),
  '/elsewhere': page('<p>Elsewhere</p>'),
  '/console-bug': page("<p>x</p><script>console.error('the list failed to render')</script>"),
  '/console-uncaught': page(
    "<p>x</p><script>setTimeout(()=>{throw new Error('late failure')},50)</script>",
  ),
  '/console-server-error': page("<p>x</p><script>fetch('/boom')</script>"),
  '/console-client-error': page("<p>x</p><script>fetch('/missing')</script>"),
  '/write': page("<p>x</p><script>fetch('/api/save',{method:'POST',body:'{}'})</script>"),
  '/echo': page('<p>echo</p>'),
  '/forbidden': page('<p>Forbidden</p>'),
  '/login': page('<p>Sign in</p>'),
};
const STATUS = { '/forbidden': 403 };
const RUNS_AFTER_LOAD = /fetch\(|setTimeout|setInterval|requestAnimationFrame|animation/;
const readyOn = (path) => (path in PAGES && !RUNS_AFTER_LOAD.test(PAGES[path]) ? 'load' : 'networkidle');
const open = (tab, path) => tab.goto(new URL(path, base).href, { waitUntil: readyOn(path) });

const hasChromium = browsersPath() !== undefined;
let server;
let elsewhere;
let base;
let signIn;
let browser;
const requests = [];

const listen = async (handler) => {
  const created = createServer(handler);
  await new Promise((ready) => created.listen(0, '127.0.0.1', ready));
  return { created, origin: `http://127.0.0.1:${created.address().port}` };
};

beforeAll(async () => {
  const other = await listen((request, response) => {
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(PAGES['/login']);
  });
  elsewhere = other.created;
  signIn = other.origin;
  const own = await listen((request, response) => {
    const url = new URL(request.url, 'http://fixture.test');
    requests.push({ method: request.method, path: url.pathname, headers: request.headers });
    if (url.pathname === '/boom') return response.writeHead(500).end('boom');
    if (url.pathname === '/slow')
      return setTimeout(() => response.writeHead(200).end('ok'), LOADING_STEP_MS);
    if (url.pathname === '/to-sign-in')
      return response.writeHead(302, { location: `${signIn}/login` }).end();
    if (url.pathname in PAGES) {
      response.writeHead(STATUS[url.pathname] ?? 200, { 'content-type': 'text/html; charset=utf-8' });
      return response.end(PAGES[url.pathname]);
    }
    return response.writeHead(404).end('missing');
  });
  server = own.created;
  base = own.origin;
  if (hasChromium) browser = await launchChromium();
}, TIMEOUT_MS);

afterAll(async () => {
  await browser?.close();
  await Promise.all([server, elsewhere].map((open) => new Promise((done) => open.close(done))));
});

async function inspect(path, only) {
  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  try {
    const tab = await context.newPage();
    const watch = watchConsole(tab);
    await open(tab, path);
    await settleLayout(tab, SETTLE_CAP_MS);
    return await checkPage(tab, { screen: path, viewport: 'test', watch, only });
  } finally {
    await context.close();
  }
}

async function panelShownAfterOverlayCheck(path) {
  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  try {
    const tab = await context.newPage();
    await open(tab, path);
    await settleLayout(tab, SETTLE_CAP_MS);
    await checkOverlays(tab, tab.url());
    return await tab
      .waitForFunction(
        () => {
          const { document, getComputedStyle } = globalThis;
          const panel = document.getElementById('panel');
          return panel !== null && getComputedStyle(panel).display !== 'none' && getComputedStyle(panel).opacity === '1';
        },
        undefined,
        { polling: 25, timeout: ENTERING_MS * 4 },
      )
      .then(() => true)
      .catch(() => false);
  } finally {
    await context.close();
  }
}

const checksOf = (findings) => [...new Set(findings.map((finding) => finding.check))];
const PAGES_AT_ONCE = 4;

async function inParallel(items, work) {
  const results = [];
  let next = 0;
  const lane = async () => {
    while (next < items.length) {
      const index = next++;
      results[index] = await work(items[index]);
    }
  };
  await Promise.all(Array.from({ length: Math.min(PAGES_AT_ONCE, items.length) }, lane));
  return results;
}

const one = (fields) => [expect.objectContaining(fields)];
const containing = (text) => expect.stringContaining(text);
const all = (findings) => findings;
const unwrapped = (findings) => findings.filter((finding) => finding.detail.includes('no ellipsis'));
const tabStops = (findings) => findings.filter((finding) => finding.detail.includes('tab stop'));
const pressed = (findings) => findings.map((finding) => [finding.element, finding.detail]);

async function expectFindings(only, cases) {
  const found = await inParallel(cases, ([path]) => inspect(path, only));
  cases.forEach(([path, expected, pick = all], index) => expect.soft(pick(found[index]), path).toEqual(expected));
}

async function onPage(path, look) {
  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  try {
    const tab = await context.newPage();
    await open(tab, path);
    return await look(tab);
  } finally {
    await context.close();
  }
}

describe('parsing the command line', () => {
  it('takes the base, paths, headers and the Access token from the environment names, never from the line', () => {
    const saved = { ...process.env };
    process.env['CF_ACCESS_CLIENT_ID'] = 'id';
    process.env['CF_ACCESS_CLIENT_SECRET'] = 'secret';
    try {
      const options = parseArgs([
        '--base',
        'http://x.test',
        '--path',
        '/a',
        '--paths',
        '/b,/c',
        '--header',
        'X-Check: yes: really',
        '--access-headers',
        '--viewports',
        'phone',
        '--checks',
        'overflow,console',
      ]);
      expect.soft(options.paths).toEqual(['/a', '/b', '/c']);
      expect.soft(options.headers).toEqual({
        'CF-Access-Client-Id': 'id',
        'CF-Access-Client-Secret': 'secret',
        'X-Check': 'yes: really',
      });
      expect.soft(options.viewports).toEqual(['phone']);
      expect.soft(options.only).toEqual(['overflow', 'console']);
    } finally {
      process.env = saved;
    }
  });

  it('refuses a missing base, no address, an unknown option, viewport or check, and a header with no colon without printing what was typed', () => {
    const typed = 'the-secret-value';
    const noColon = ['--base', 'http://x.test', '--path', '/a', '--header', typed];
    expect.soft(() => parseArgs(noColon)).toThrow('--header');
    expect.soft(() => parseArgs(noColon)).not.toThrow(typed);
    for (const [line, named] of [
      [['--path', '/a'], '--base'],
      [['--base', 'http://x.test'], '--path'],
      [['--base', 'http://x.test', '--path', '/a', '--nope'], '--nope'],
      [['--base', 'http://x.test', '--path', '/a', '--viewports', 'tv'], 'tv'],
      [['--base', 'http://x.test', '--path', '/a', '--checks', 'luck'], 'luck'],
    ])
      expect.soft(() => parseArgs(line), line.join(' ')).toThrow(named);
  });
});

describe.skipIf(!hasChromium)('the wait after a page loads', { timeout: TIMEOUT_MS }, () => {
  const CAP_MS = 4000;

  async function settled(path, cap = CAP_MS) {
    const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
    try {
      const tab = await context.newPage();
      await tab.goto(new URL(path, base).href, { waitUntil: 'domcontentloaded' });
      const started = Date.now();
      await settleLayout(tab, cap);
      return {
        waited: Date.now() - started,
        rows: await tab.locator('#rows').evaluate((el) => el.children.length),
      };
    } finally {
      await context.close();
    }
  }

  it('lasts until the layout has stopped changing, is short for a page already still, and stops at its cap for a page that never stops', async () => {
    const grows = await settled('/grows');
    expect.soft(grows.rows, '/grows').toBe(8);
    expect.soft(grows.waited, '/grows').toBeLessThan(CAP_MS);
    const { waited: still } = await settled('/still');
    expect.soft(still, '/still').toBeLessThan(CAP_MS / 2);
    const { waited: forever } = await settled('/grows-forever', 1000);
    expect.soft(forever, '/grows-forever').toBeGreaterThanOrEqual(900);
    expect.soft(forever, '/grows-forever').toBeLessThan(CAP_MS);
  });
});

describe.skipIf(!hasChromium)('the interaction checks', { timeout: TIMEOUT_MS }, () => {
  it('finds a table cell whose content runs past its box, and not one that wraps its text', async () => {
    await expectFindings(
      ['overflow'],
      [
        [
          '/overflow-bug',
          one({
            screen: '/overflow-bug',
            check: 'overflow',
            element: containing('(column "Reason", row 1)'),
            detail: containing("past the cell's box"),
          }),
        ],
        ['/overflow-clean', []],
      ],
    );
  });

  it('finds text cut with an ellipsis when nothing gives the full value, and stays quiet when a title or tooltip holds it, the text fits or a dialog covers it', async () => {
    await expectFindings(
      ['truncation'],
      [
        ['/cut-bug', one({ check: 'truncation', detail: containing('no tooltip or title') })],
        ['/cut-clamp-bug', one({ check: 'truncation', detail: containing('no tooltip or title') })],
        ['/cut-tooltip-wrong', one({ detail: containing('does not hold the full value') })],
        ['/cut-title-other', one({ detail: containing('no tooltip or title') })],
        ['/cut-composite', []],
        ['/cut-title', []],
        ['/cut-title-ancestor', []],
        ['/cut-tooltip', []],
        ['/cut-focus', []],
        ['/cut-fits', []],
        ['/cut-covered', []],
      ],
    );
  });

  it('finds text clipped by its box with no ellipsis, whatever another box says, and stays quiet where the text is whole, scrolls, hides or escapes', async () => {
    await expectFindings(
      ['truncation'],
      [
        [
          '/clip-bug',
          one({
            check: 'truncation',
            element: expect.stringMatching(/13 Oct 2026.*column "Changed"/s),
            detail: containing('no ellipsis'),
          }),
        ],
        ['/clip-ellipsis-elsewhere', one({ detail: containing('no ellipsis') }), unwrapped],
        ['/clip-titled', one({ detail: containing('no ellipsis') }), unwrapped],
        ['/clip-tall', one({ detail: containing('no ellipsis') })],
        ['/clip-wide', []],
        ['/clip-tall-fits', []],
        ['/clip-scroll', []],
        ['/clip-hidden-label', []],
        ['/clip-collapsed', []],
        ['/clip-tight', []],
        ['/clip-page-locked', []],
        ['/clip-tooltip', []],
        ['/clip-escaped', []],
        ['/clip-clamped-wide', []],
      ],
    );
  });

  it('finds text a pinned column or a fixed bar covers, and stays quiet for text its own box hides, a dialog covers, or a fixed element owns', async () => {
    await expectFindings(
      ['covered'],
      [
        ['/covered-bug', one({ check: 'covered', element: containing('column "Status"'), detail: containing('covered by') })],
        ['/covered-fixed-bug', one({ check: 'covered', detail: containing('covered by') })],
        [
          '/covered-fixed-footer-bug',
          one({ check: 'covered', element: containing('Last line of the page'), detail: containing('Footer') }),
        ],
        ['/covered-fixed-bar-with-dialog', one({ check: 'covered', detail: containing('Bar') })],
        ['/covered-middle', one({ check: 'covered', detail: containing('Pinned') })],
        ['/covered-truncated', []],
        ['/covered-fits', []],
        ['/covered-dialog', []],
        ['/covered-fixed-own', []],
        ['/covered-fixed-in-dialog', []],
        ['/covered-fixed-scrim', []],
        ['/covered-fixed-dialog-wrapper', []],
        ['/covered-fixed-tooltip', []],
        ['/covered-fixed-footer-padded', []],
        ['/covered-fixed-header-long', []],
      ],
    );
  });

  it('hands the page back scrolled where it was and with its layout settled after it looks for covered text', async () => {
    const [scrolled, wide, padded] = await Promise.all([
      onPage('/covered-fixed-scrolled', async (tab) => {
        const before = await tab.evaluate(() => globalThis.scrollY);
        const findings = await checkPage(tab, { screen: 'x', only: ['covered'] });
        return { before, findings, after: await tab.evaluate(() => globalThis.scrollY) };
      }),
      onPage('/covered-fixed-scroll-wide', async (tab) => {
        await tab.waitForFunction(() => globalThis.document.documentElement.scrollWidth === globalThis.document.documentElement.clientWidth);
        await checkPage(tab, { screen: 'x', only: ['covered'] });
        return tab.evaluate(() => globalThis.document.documentElement.scrollWidth - globalThis.document.documentElement.clientWidth);
      }),
      onPage('/covered-fixed-footer-padded', async (tab) => {
        await checkPage(tab, { screen: 'x', only: ['covered'] });
        return tab.evaluate(() => globalThis.scrollY);
      }),
    ]);
    expect.soft(scrolled, '/covered-fixed-scrolled').toEqual({ before: 50, findings: [], after: 50 });
    expect.soft(wide, '/covered-fixed-scroll-wide').toBe(0);
    expect.soft(padded, '/covered-fixed-footer-padded').toBe(0);
  });

  it('finds text a tap opens up running out of its chip, puts the text back after the tap, and never taps inside a button', async () => {
    requests.length = 0;
    await expectFindings(
      ['truncation'],
      [
        ['/tap-chip-bug', one({ check: 'truncation', detail: containing('after a tap') })],
        ['/tap-chip-grows', []],
        ['/tap-does-nothing', []],
      ],
    );
    const whiteSpace = await onPage('/tap-chip-bug', async (tab) => {
      await checkPage(tab, { screen: 'x', only: ['truncation'] });
      return tab.locator('#t').evaluate((el) => el.style.whiteSpace);
    });
    expect.soft(whiteSpace, '/tap-chip-bug after the tap').toBe('nowrap');
    await inspect('/tap-in-button', ['truncation']);
    expect.soft(requests.filter((request) => request.method === 'POST'), '/tap-in-button').toEqual([]);
  });

  it('finds a tab stop inside a control, and stays quiet for a disabled or untabbable child, free text and a focusable control', async () => {
    await expectFindings(
      ['truncation'],
      [
        ['/stop-in-button', one({ detail: containing('inside') }), tabStops],
        ['/stop-in-link', one({ detail: containing('inside') }), tabStops],
        ['/stop-in-field', one({ detail: containing('tab stop') }), tabStops],
        ['/stop-in-select', one({ detail: containing('tab stop') }), tabStops],
        ['/stop-in-textarea', one({ detail: containing('tab stop') }), tabStops],
        ['/stop-in-editable', one({ detail: containing('tab stop') }), tabStops],
        ['/stop-in-nested-button', one({ detail: containing('inside the control') }), tabStops],
        ['/stop-in-nested-link', one({ detail: containing('inside the control') }), tabStops],
        ['/stop-in-quiet-children', [], tabStops],
        ['/stop-clean', []],
      ],
    );
  });

  it('finds an overlay left open after a press, loading the page afresh between presses, and stays quiet for one every press outside closes', async () => {
    requests.length = 0;
    await expectFindings(
      ['overlay'],
      [
        [
          '/overlay-bug',
          one({ check: 'overlay', element: containing('Who: Any'), detail: containing('after a press on the button "New group"') }),
        ],
        ['/overlay-stuck', one({ detail: containing('after a press outside it') }), (findings) => findings.slice(0, 1)],
        [
          '/overlay-unmarked-stuck',
          one({ check: 'overlay', element: containing('Row actions'), detail: containing('still open after a press outside it') }),
        ],
        [
          '/overlay-unmarked-stuck-filling',
          one({ check: 'overlay', element: containing('Row actions'), detail: containing('still open after a press outside it') }),
        ],
        ['/overlay-rendered-after-loading', STUCK_ROW_ACTIONS, pressed],
        ['/overlay-polls-once-pressed', STUCK_ROW_ACTIONS, pressed],
        ['/overlay-arming', []],
        ['/overlay-mounted-dialog', []],
        ['/overlay-mounted-menu', []],
        ['/overlay-clean', []],
      ],
    );
    expect
      .soft(
        requests.some((request) => request.path === '/echo' && request.headers.referer?.endsWith('/overlay-polls-once-pressed')),
        'the press after the reload on /overlay-polls-once-pressed',
      )
      .toBe(true);
    const fromLoad = await inspect('/overlay-covering-from-load', ['covered', 'overlay']);
    expect.soft(checksOf(fromLoad), '/overlay-covering-from-load').toEqual(['covered']);
    expect.soft(fromLoad[0]?.detail, '/overlay-covering-from-load').toContain('Page text under the menu bar');
  });

  it('leaves nothing open when a press opens a dialog, menu or drawer that is still entering, or mounts one', async () => {
    const paths = [
      '/overlay-entering-modal',
      '/overlay-entering-dialog',
      '/overlay-entering-menu',
      '/overlay-entering-drawer',
      '/overlay-mounted-dialog',
      '/overlay-mounted-menu',
    ];
    const shown = await inParallel(paths, panelShownAfterOverlayCheck);
    paths.forEach((path, index) => expect.soft(shown[index], path).toBe(false));
  });

  it('moves on from a trigger the scroll took off the screen or the page no longer shows without waiting for it', async () => {
    const offScreen = Date.now();
    const findings = await inspect('/overlay-far-below-the-button', ['overlay']);
    expect.soft(pressed(findings), '/overlay-far-below-the-button').toEqual(STUCK_ROW_ACTIONS);
    expect.soft(Date.now() - offScreen, '/overlay-far-below-the-button').toBeLessThan(3 * ACTION_TIMEOUT_MS);
    const hidden = Date.now();
    expect.soft(await inspect('/overlay-trigger-hides', ['overlay']), '/overlay-trigger-hides').toEqual([]);
    expect.soft(Date.now() - hidden, '/overlay-trigger-hides').toBeLessThan(ACTION_TIMEOUT_MS);
  });

  it('finds a row click that leaves the page or the module, and stays quiet for one that changes the search or stays inside', async () => {
    await expectFindings(
      ['row-click'],
      [
        ['/row-bug', one({ check: 'row-click', detail: 'a click on the row changed the page from /row-bug to /elsewhere' })],
        ['/releases/stray', one({ detail: 'a click on the row changed the page from /releases/stray to /users/1' })],
        ['/row-search', []],
        ['/rows', []],
        ['/releases/versions', []],
      ],
    );
  });

  it('finds console errors, uncaught page errors and a server error, and stays quiet for a page that logs nothing or a refused read (4xx)', async () => {
    await expectFindings(
      ['console'],
      [
        ['/console-bug', one({ check: 'console', detail: containing('the list failed to render') })],
        ['/console-uncaught', one({ check: 'console', detail: containing('late failure') })],
        ['/console-server-error', one({ check: 'console', detail: containing('status of 500') })],
        ['/overflow-clean', []],
        ['/console-client-error', []],
      ],
    );
  });

  it('makes a finding of a screen the run never saw, another origin or a 403 or 404 document, whatever checks were asked for', async () => {
    const seen = (path) => runInteractionCheck({ base, paths: [path], viewports: ['desktop'], only: ['console'], browser });
    const never = (screen, detail) => [{ screen, viewport: 'desktop', check: 'screen', element: 'the document', detail }];
    const [signedOut, forbidden, nowhere, otherChecks] = await Promise.all([
      seen('/to-sign-in'),
      seen('/forbidden'),
      seen('/nowhere'),
      runInteractionCheck({ base, paths: ['/forbidden', '/echo'], viewports: ['desktop'], only: ['overflow'], browser }),
    ]);
    expect.soft(signedOut.screens, '/to-sign-in').toEqual([{ screen: '/to-sign-in', viewport: 'desktop' }]);
    expect.soft(signedOut.findings, '/to-sign-in').toEqual(never('/to-sign-in', `the page ended on ${signIn}, not on ${base}`));
    expect.soft(forbidden.findings, '/forbidden').toEqual(never('/forbidden', 'the document answered 403'));
    expect.soft(nowhere.findings, '/nowhere').toEqual(never('/nowhere', 'the document answered 404'));
    expect
      .soft(
        otherChecks.findings.map(({ screen, check }) => [screen, check]),
        'overflow asked',
      )
      .toEqual([['/forbidden', 'screen']]);
  });

  it('exits 0 with no finding, 1 with the findings as JSON, 2 on a bad line without printing it, and 3 with no report when the base cannot be reached', async () => {
    const args = (path) => [CLI, '--base', base, '--path', path, '--viewports', 'desktop', '--checks', 'overflow'];
    const closed = await listen((request, response) => response.end());
    await new Promise((done) => closed.created.close(done));
    const typed = 'the-secret-value';
    const [clean, dirty, unreachable, bad, noColon] = await Promise.all([
      run('node', args('/overflow-clean')),
      run('node', args('/overflow-bug')).catch((error) => error),
      run('node', [CLI, '--base', closed.origin, '--path', '/x', '--viewports', 'desktop']).catch((error) => error),
      run('node', [CLI, '--path', '/x']).catch((error) => error),
      run('node', [CLI, '--base', base, '--path', '/x', '--header', typed]).catch((error) => error),
    ]);
    expect.soft(JSON.parse(clean.stdout).findings, 'clean').toEqual([]);
    expect.soft(dirty.code, 'findings').toBe(1);
    const report = JSON.parse(dirty.stdout);
    expect.soft(report.findings.map((finding) => finding.check), 'findings').toEqual(['overflow']);
    expect.soft(Object.keys(report.findings[0]).sort(), 'findings').toEqual(['check', 'detail', 'element', 'screen', 'viewport'].sort());
    expect.soft(bad.code, 'no base').toBe(2);
    expect.soft(bad.stderr, 'no base').toContain('usage');
    expect.soft(noColon.code, 'a header with no colon').toBe(2);
    expect.soft(noColon.stderr, 'a header with no colon').not.toContain(typed);
    expect.soft(unreachable.code, 'unreachable').toBe(3);
    expect.soft(unreachable.stdout, 'unreachable').toBe('');
    expect.soft(unreachable.stderr, 'unreachable').toContain('the run failed');
  });

  it('reports clean pages with no finding, and lists every screen it opened at each width', async () => {
    const result = await runInteractionCheck({
      base,
      paths: ['/overflow-clean', '/overlay-clean', '/cut-title'],
      browser,
    });
    expect.soft(result.findings).toEqual([]);
    expect.soft(result.screens).toEqual(
      ['desktop', 'phone'].flatMap((viewport) =>
        ['/overflow-clean', '/overlay-clean', '/cut-title'].map((screen) => ({
          screen,
          viewport,
        })),
      ),
    );
  });

  it('sends the headers it is given, and no write', async () => {
    requests.length = 0;
    await runInteractionCheck({
      base,
      paths: ['/echo', '/write'],
      headers: { 'CF-Access-Client-Id': 'abc' },
      viewports: ['desktop'],
      only: ['console'],
      browser,
    });
    const echoed = requests.find((request) => request.path === '/echo');
    expect.soft(echoed.headers['cf-access-client-id']).toBe('abc');
    expect.soft(requests.filter((request) => request.method === 'POST')).toEqual([]);
  });

  it('reports each kind of bug under its own check name when every check runs', async () => {
    const found = await inParallel(['/overflow-bug', '/cut-bug', '/covered-bug', '/overlay-bug', '/row-bug', '/console-bug'], (path) => inspect(path));
    expect(checksOf(found.flat()).sort()).toEqual(['console', 'covered', 'overflow', 'overlay', 'row-click', 'truncation'].sort());
  });
});
