// lectern frontend entry point: wires the toolbar, documents, shortcuts and backend events.

import { invoke, listen } from './tauri.js';
import { $, $$, copyText, el, nextFrame, toast } from './util.js';
import {
  apply as applySettings, changeFontSize, cycleTheme, initSettingsUI, settings, toggleToc,
} from './settings.js';
import { initToc, refreshSpy, renderToc, scrollToId } from './toc.js';
import { enhance, handleCopyClick } from './content.js';
import { rerenderMermaid } from './diagrams.js';
import { closeFind, findNext, initFind, isFindOpen, openFind, refreshFind } from './find.js';
import { closeLightbox, initLightbox, isLightboxOpen, openLightbox } from './lightbox.js';

const MD_LINK = /\.(md|markdown|mdown|mkdn?|mdwn|mdte?xt|txt|text)$/i;

/** The document on screen: {path, fileName, html, toc, title} */
let current = null;
const history = { back: [], forward: [] }; // entries: {path, scroll}
let renderSeq = 0;

const scroller = () => $('#lec-scroller');

// ------------------------------------------------------------------ documents

async function showDoc(doc, { keepScroll = false, scrollTop = null, fragment = null } = {}) {
  const seq = ++renderSeq;
  const s = scroller();
  const previousTop = s.scrollTop;
  current = doc;

  $('#lec-welcome').hidden = true;
  const article = $('#lec-doc');
  article.classList.remove('empty');
  article.innerHTML = doc.html;
  showPath(doc);
  renderToc(doc.toc);
  for (const id of ['#lec-reload-btn', '#lec-find-btn']) $(id).disabled = false;

  const restore = () => {
    if (keepScroll) s.scrollTop = previousTop;
    else if (fragment) scrollToId(fragment, false) || (s.scrollTop = 0);
    else s.scrollTop = scrollTop ?? 0;
  };
  restore();
  const settled = s.scrollTop;

  await enhance(article);
  if (seq !== renderSeq) return; // a newer document replaced this one meanwhile
  // Diagrams/math can change heights; restore again unless the user scrolled meanwhile.
  if (s.scrollTop === settled) restore();
  refreshFind();
  refreshSpy();
}

function showPath(doc) {
  const btn = $('#lec-path');
  const name = doc.fileName;
  const dir = doc.path.slice(0, doc.path.length - name.length);
  btn.querySelector('.path-dir').textContent = dir;
  btn.querySelector('.path-name').textContent = name;
  btn.title = `${doc.path}\nClick to copy the full path`;
  btn.disabled = false;
}

function showWelcome(error) {
  $('#lec-welcome').hidden = false;
  const err = $('#lec-welcome-error');
  err.hidden = !error;
  err.textContent = error ?? '';
  fillRecent($('#lec-welcome-recent'), true);
}

/** Show a newly opened document, recording navigation history. */
function navigateTo(doc, opts = {}) {
  if (current && current.path !== doc.path) {
    history.back.push({ path: current.path, scroll: scroller().scrollTop });
    history.forward = [];
  }
  updateHistoryButtons();
  return showDoc(doc, opts);
}

async function go(direction) {
  const from = direction < 0 ? history.back : history.forward;
  const to = direction < 0 ? history.forward : history.back;
  const entry = from.pop();
  if (!entry) return;
  try {
    const doc = await invoke('open_path', { path: entry.path });
    if (current) to.push({ path: current.path, scroll: scroller().scrollTop });
    await showDoc(doc, { scrollTop: entry.scroll });
  } catch (e) {
    toast(String(e), { error: true });
  }
  updateHistoryButtons();
}

function updateHistoryButtons() {
  $('#lec-back-btn').disabled = !history.back.length;
  $('#lec-fwd-btn').disabled = !history.forward.length;
}

async function run(promise) {
  try {
    return await promise;
  } catch (e) {
    toast(String(e), { error: true });
    return null;
  }
}

async function openDialog() {
  closePopovers();
  const doc = await run(invoke('open_dialog'));
  if (doc) navigateTo(doc);
}

async function openPath(path) {
  closePopovers();
  const doc = await run(invoke('open_path', { path }));
  if (doc) navigateTo(doc);
}

async function reload({ quiet = false } = {}) {
  if (!current) return;
  try {
    const doc = await invoke('reload');
    await showDoc(doc, { keepScroll: true });
    if (!quiet) toast('Reloaded', { ms: 900 });
  } catch (e) {
    toast(String(e), { error: true });
  }
}

async function print() {
  closePopovers();
  try {
    await invoke('print');
  } catch {
    window.print();
  }
}

// ------------------------------------------------------------------ links & images

function onDocClick(e) {
  if (e.button !== 0) return;
  if (e.target.closest('.copy-btn')) {
    handleCopyClick(e);
    return;
  }
  const a = e.target.closest('a');
  if (a) {
    const href = a.getAttribute('href') ?? a.getAttribute('xlink:href');
    if (href == null) return;
    e.preventDefault();
    followLink(href.trim());
    return;
  }
  if (e.target.tagName === 'IMG' && !e.target.classList.contains('broken')) {
    openLightbox(e.target.currentSrc || e.target.src, e.target.getAttribute('alt'));
  }
}

function followLink(href) {
  if (href.startsWith('#')) {
    const id = decodeURIComponent(href.slice(1));
    if (!scrollToId(id)) toast(`No section “${id}” in this document`);
    return;
  }
  if (/^(https?:|mailto:)/i.test(href)) {
    run(invoke('open_external', { url: href }));
    return;
  }
  const isWindowsPath = /^[a-z]:[\\/]/i.test(href);
  if (/^[a-z][a-z0-9+.-]*:/i.test(href) && !/^file:/i.test(href) && !isWindowsPath) {
    toast(`Unsupported link: ${href}`);
    return;
  }
  const [pathPart, fragment] = href.split('#', 2);
  const bare = pathPart.split('?')[0];
  if (!MD_LINK.test(decodeURIComponent(bare))) {
    toast(`Not a Markdown file: ${decodeURIComponent(bare)}`);
    return;
  }
  run(invoke('open_link', { href })).then((doc) => {
    if (doc) navigateTo(doc, { fragment: fragment ? decodeURIComponent(fragment) : null });
  });
}

// ------------------------------------------------------------------ menus

function closePopovers(except) {
  for (const [btn, pop] of [['#lec-file-btn', '#lec-file-dropdown'], ['#lec-settings-btn', '#lec-settings-popover']]) {
    if (pop === except) continue;
    $(pop).hidden = true;
    $(btn).setAttribute('aria-expanded', 'false');
  }
}

function togglePopover(btnSel, popSel) {
  const pop = $(popSel);
  const open = pop.hidden;
  closePopovers(popSel);
  pop.hidden = !open;
  $(btnSel).setAttribute('aria-expanded', String(open));
  return open;
}

async function fillRecent(container, withHeading = false) {
  const files = (await run(invoke('recent_files'))) ?? [];
  container.replaceChildren();
  if (withHeading && files.length) container.append(el('div', { class: 'menu-heading', text: 'Recent files' }));
  if (!files.length && !withHeading) container.append(el('div', { class: 'menu-empty', text: 'No recent files' }));
  for (const f of files) {
    const dir = f.path.slice(0, f.path.length - f.name.length);
    container.append(el('button', { class: 'menu-item', role: 'menuitem', title: f.path, onclick: () => openPath(f.path) },
      el('span', { class: 'label', text: f.name }),
      el('span', { class: 'hint', text: dir })));
  }
}

const actions = {
  open: openDialog,
  reload: () => {
    closePopovers();
    reload();
  },
  print,
  'clear-recent': async () => {
    await run(invoke('clear_recent'));
    fillRecent($('#lec-recent-list'));
    if (!current) fillRecent($('#lec-welcome-recent'), true);
  },
  close: () => invoke('close_window'),
};

// ------------------------------------------------------------------ keyboard

function onKeyDown(e) {
  const mod = e.ctrlKey || e.metaKey;
  const key = e.key;
  const handled = () => {
    e.preventDefault();
    e.stopPropagation();
  };

  if (key === 'Escape') {
    if (isLightboxOpen()) return handled(), closeLightbox();
    if (isFindOpen()) return handled(), closeFind();
    if (!$('#lec-file-dropdown').hidden || !$('#lec-settings-popover').hidden) return handled(), closePopovers();
    return;
  }
  if (key === 'F5') return handled(), reload();
  if (key === 'F3') return handled(), current && findNext(e.shiftKey ? -1 : 1);
  if (e.altKey && !mod && key === 'ArrowLeft') return handled(), go(-1);
  if (e.altKey && !mod && key === 'ArrowRight') return handled(), go(1);
  if (!mod) return;

  // Ctrl+\ (US layouts report "\"; others are matched by physical key).
  if (key === '\\' || e.code === 'Backslash' || e.code === 'IntlBackslash') {
    return handled(), toggleToc();
  }
  if (e.shiftKey && e.code === 'KeyT') return handled(), cycleTheme();
  if (key === '+' || key === '=' || e.code === 'NumpadAdd') return handled(), changeFontSize(1);
  if (key === '-' || key === '_' || e.code === 'NumpadSubtract') return handled(), changeFontSize(-1);
  if (key === '0' || e.code === 'Numpad0') return handled(), changeFontSize(0);
  if (e.shiftKey || e.altKey) return;

  switch (key.toLowerCase()) {
    case 'o': return handled(), openDialog();
    case 'r': return handled(), reload();
    case 'p': return handled(), print();
    case 'f': return handled(), current && openFind();
    case 'g': return handled(), current && findNext(1);
    case 'w':
    case 'q': return handled(), invoke('close_window');
    case ',': return handled(), togglePopover('#lec-settings-btn', '#lec-settings-popover');
    default:
  }
}

// ------------------------------------------------------------------ startup

function wireUi() {
  $('#lec-file-btn').addEventListener('click', () => {
    if (togglePopover('#lec-file-btn', '#lec-file-dropdown')) fillRecent($('#lec-recent-list'));
  });
  $('#lec-settings-btn').addEventListener('click', () => togglePopover('#lec-settings-btn', '#lec-settings-popover'));
  $('#lec-toc-btn').addEventListener('click', () => toggleToc());
  $('#lec-back-btn').addEventListener('click', () => go(-1));
  $('#lec-fwd-btn').addEventListener('click', () => go(1));
  $('#lec-reload-btn').addEventListener('click', () => reload());
  $('#lec-find-btn').addEventListener('click', () => (isFindOpen() ? closeFind() : openFind()));
  $('#lec-path').addEventListener('click', async () => {
    if (!current) return;
    toast((await copyText(current.path)) ? 'Path copied to clipboard' : 'Could not copy path', { ms: 1400 });
  });
  for (const b of $$('[data-action]')) {
    b.addEventListener('click', () => actions[b.dataset.action]?.());
  }

  // Close popovers when clicking elsewhere.
  document.addEventListener('pointerdown', (e) => {
    if (!e.target.closest('.popover, #lec-file-btn, #lec-settings-btn')) closePopovers();
  });

  const article = $('#lec-doc');
  article.addEventListener('click', onDocClick);
  // Never let the webview navigate away (middle-click, etc.).
  article.addEventListener('auxclick', (e) => {
    if (e.target.closest('a')) e.preventDefault();
  });
  document.addEventListener('dragstart', (e) => {
    if (e.target.closest?.('a, img')) e.preventDefault();
  });
  window.addEventListener('keydown', onKeyDown, true);

  document.addEventListener('appearance-change', () => rerenderMermaid($('#lec-doc')));
}

async function listenBackend() {
  await listen('file-changed', (path) => {
    if (current && path === current.path) reload({ quiet: true });
  });
  await listen('open-file', (path) => openPath(path));
  await listen('drag-state', (on) => {
    $('#lec-drop-overlay').hidden = !on;
  });
}

async function main() {
  initSettingsUI();
  initToc();
  initFind();
  initLightbox();
  wireUi();

  let start = null;
  try {
    start = await invoke('startup');
  } catch (e) {
    start = { error: String(e) };
  }
  if (start.settings) Object.assign(settings, start.settings);
  document.documentElement.dataset.platform = start.platform ?? '';
  applySettings();
  listenBackend().catch((e) => console.error('listen', e));

  if (start.doc) {
    showDoc(start.doc);
  } else {
    showWelcome(start.error);
  }
  updateHistoryButtons();
  await nextFrame();
  invoke('ready');
  scroller().focus({ preventScroll: true });
}

main();
