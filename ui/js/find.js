// In-document find (Ctrl+F) with highlighted matches.

import { $, $$, debounce, el } from './util.js';

let hits = [];
let current = -1;
let query = '';

const bar = () => $('#lec-findbar');
const input = () => $('#lec-find-input');
const docRoot = () => $('#lec-doc');

export const isFindOpen = () => !bar().hidden;

export function openFind() {
  bar().hidden = false;
  input().focus();
  input().select();
  if (input().value) search(input().value);
}

export function closeFind() {
  clearHits();
  query = '';
  bar().hidden = true;
  updateCount();
  $('#lec-scroller').focus({ preventScroll: true });
}

export function findNext(step = 1) {
  if (!isFindOpen()) return openFind();
  if (input().value !== query) search(input().value);
  else if (hits.length) go(current + step);
}

/** Re-run the active search after the document was re-rendered. */
export function refreshFind() {
  hits = [];
  current = -1;
  if (isFindOpen() && query) search(query, { keepPosition: true });
}

function clearHits() {
  const parents = new Set();
  for (const m of $$('mark.find-hit', docRoot())) {
    parents.add(m.parentNode);
    m.replaceWith(document.createTextNode(m.textContent));
  }
  for (const p of parents) p.normalize();
  hits = [];
  current = -1;
}

function search(q, { keepPosition = false } = {}) {
  const previous = current;
  clearHits();
  query = q;
  if (!q) return updateCount();

  const needle = q.toLocaleLowerCase();
  const walker = document.createTreeWalker(docRoot(), NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement;
      if (!parent || parent.closest('svg, .katex-mathml, .code-tools, script, style')) {
        return NodeFilter.FILTER_REJECT;
      }
      return node.nodeValue.toLocaleLowerCase().includes(needle)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT;
    },
  });
  const nodes = [];
  while (walker.nextNode()) nodes.push(walker.currentNode);

  for (const node of nodes) {
    const text = node.nodeValue;
    const lower = text.toLocaleLowerCase();
    if (lower.length !== text.length) continue; // case mapping changed offsets; skip rare edge case
    const frag = document.createDocumentFragment();
    let pos = 0;
    let i;
    while ((i = lower.indexOf(needle, pos)) !== -1) {
      frag.append(text.slice(pos, i));
      const mark = el('mark', { class: 'find-hit', text: text.slice(i, i + q.length) });
      frag.append(mark);
      hits.push(mark);
      pos = i + q.length;
    }
    frag.append(text.slice(pos));
    node.replaceWith(frag);
  }

  if (!hits.length) return updateCount();
  if (keepPosition && previous >= 0) {
    go(Math.min(previous, hits.length - 1), false);
  } else {
    // Start at the first match below the current scroll position.
    const top = $('#lec-scroller').getBoundingClientRect().top;
    const first = hits.findIndex((h) => h.getBoundingClientRect().top >= top);
    go(first === -1 ? 0 : first);
  }
}

function go(i, scroll = true) {
  if (!hits.length) return;
  hits[current]?.classList.remove('current');
  current = (i + hits.length) % hits.length;
  const hit = hits[current];
  hit.classList.add('current');
  hit.closest('details:not([open])')?.setAttribute('open', '');
  if (scroll) hit.scrollIntoView({ block: 'center', inline: 'nearest' });
  updateCount();
}

function updateCount() {
  const count = $('#lec-find-count');
  if (!query) count.textContent = '';
  else count.textContent = hits.length ? `${current + 1} of ${hits.length}` : 'No matches';
  input().classList.toggle('no-match', Boolean(query) && !hits.length);
}

export function initFind() {
  const run = debounce(() => search(input().value), 120);
  input().addEventListener('input', run);
  input().addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      findNext(e.shiftKey ? -1 : 1);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      closeFind();
    }
  });
  $('#lec-find-next').addEventListener('click', () => findNext(1));
  $('#lec-find-prev').addEventListener('click', () => findNext(-1));
  $('#lec-find-close').addEventListener('click', closeFind);
}
