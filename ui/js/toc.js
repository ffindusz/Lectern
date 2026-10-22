// Table of contents sidebar with scroll-spy.

import { $, el } from './util.js';

const SPY_OFFSET = 80; // a heading becomes "current" once it is this close to the top

let headings = []; // heading elements, parallel to `links`
let links = [];
let active = -1;
let lockUntilIdle = null; // while a TOC click scrolls smoothly, keep the clicked entry active
let lockTimer = 0;

const scroller = () => $('#lec-scroller');

/** Find a heading/anchor by id inside the document only (never the app chrome). */
export function findTarget(id) {
  const doc = $('#lec-doc');
  try {
    return doc.querySelector(`[id="${CSS.escape(id)}"]`) ?? doc.querySelector(`a[name="${CSS.escape(id)}"]`);
  } catch {
    return null;
  }
}

export function scrollToId(id, smooth = true) {
  const target = findTarget(id);
  if (!target) return false;
  target.closest('details:not([open])')?.setAttribute('open', '');
  target.scrollIntoView({ behavior: smooth ? 'smooth' : 'auto', block: 'start' });
  return true;
}

/** Render the TOC for `entries` ([{level, text, id}]). */
export function renderToc(entries) {
  const list = $('#lec-toc-list');
  list.replaceChildren();
  headings = [];
  links = [];
  active = -1;

  const usable = entries.filter((e) => findTarget(e.id));
  const minLevel = Math.min(...usable.map((e) => e.level));
  for (const [i, e] of usable.entries()) {
    const depth = Math.min(3, e.level - minLevel);
    const a = el('a', { class: `toc-link depth-${depth}`, href: `#${e.id}`, title: e.text, text: e.text });
    a.addEventListener('click', (ev) => {
      ev.preventDefault();
      setActive(i, true);
      lockUntilIdle = i;
      scrollToId(e.id, true);
    });
    list.append(el('li', {}, a));
    links.push(a);
    headings.push(findTarget(e.id));
  }

  const available = usable.length >= 2;
  document.body.classList.toggle('toc-unavailable', !available);
  $('#lec-toc-btn').disabled = !available;
  updateActive();
}

function setActive(i, fromClick = false) {
  if (i === active) return;
  links[active]?.classList.remove('active');
  active = i;
  const link = links[active];
  if (!link) return;
  link.classList.add('active');
  // Keep the active entry visible in a long TOC.
  const toc = $('#lec-toc');
  if (!fromClick && toc.offsetParent !== null) {
    const r = link.getBoundingClientRect();
    const t = toc.getBoundingClientRect();
    if (r.top < t.top + 30 || r.bottom > t.bottom - 30) link.scrollIntoView({ block: 'center' });
  }
}

function updateActive() {
  if (!headings.length) return;
  if (lockUntilIdle !== null) return;
  const s = scroller();
  const top = s.getBoundingClientRect().top + SPY_OFFSET;
  let idx = 0;
  for (let i = 0; i < headings.length; i++) {
    if (headings[i].getBoundingClientRect().top <= top) idx = i;
    else break;
  }
  // At the very bottom the last sections may never reach the top; select the last one.
  if (s.scrollTop > 0 && s.scrollTop + s.clientHeight >= s.scrollHeight - 2) {
    const lastVisible = headings.findLastIndex((h) => h.getBoundingClientRect().top < s.getBoundingClientRect().bottom);
    idx = Math.max(idx, lastVisible);
  }
  setActive(idx);
}

export function initToc() {
  let ticking = false;
  scroller().addEventListener('scroll', () => {
    if (lockUntilIdle !== null) {
      clearTimeout(lockTimer);
      lockTimer = setTimeout(() => {
        lockUntilIdle = null;
      }, 160);
    }
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      ticking = false;
      updateActive();
    });
  }, { passive: true });
  // If the click didn't scroll at all (already in place), release the lock.
  document.addEventListener('pointerup', () => {
    clearTimeout(lockTimer);
    lockTimer = setTimeout(() => {
      lockUntilIdle = null;
    }, 900);
  });
}

export const refreshSpy = updateActive;
