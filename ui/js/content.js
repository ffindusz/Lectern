// Post-processing of the sanitized HTML injected from Rust.

import { $$, copyText, el } from './util.js';
import { renderDiagrams } from './diagrams.js';
import { renderMath } from './math.js';

/** Synchronous enhancements, then async rendering (math, diagrams). Resolves when done. */
export function enhance(root) {
  wrapCodeBlocks(root);
  decorateLinks(root);
  watchImages(root);
  return Promise.allSettled([renderMath(root), renderDiagrams(root)]);
}

function wrapCodeBlocks(root) {
  for (const pre of $$('pre', root)) {
    if (
      pre.classList.contains('diagram-source') ||
      pre.closest('.front-matter, .code-block') ||
      pre.querySelector('code[data-math-style]')
    ) continue;
    const code = pre.querySelector('code');
    const lang = code?.className.match(/(?:^|\s)language-(\S+)/)?.[1];
    const wrap = el('div', { class: 'code-block' });
    pre.replaceWith(wrap);
    wrap.append(
      pre,
      el('div', { class: 'code-tools' },
        lang ? el('span', { class: 'code-lang', text: lang }) : null,
        el('button', { class: 'copy-btn', type: 'button', title: 'Copy code', text: 'Copy' }),
      ),
    );
  }
}

function decorateLinks(root) {
  for (const a of $$('a[href]', root)) {
    const href = a.getAttribute('href');
    if (/^https?:/i.test(href)) {
      a.classList.add('external');
      a.title ||= href;
    }
  }
}

function watchImages(root) {
  for (const img of $$('img', root)) {
    img.decoding = 'async';
    img.addEventListener('error', () => {
      const src = img.getAttribute('src') ?? '';
      const shown = decodeURIComponent(src.replace(/^(mdfile:\/\/localhost\/|http:\/\/mdfile\.localhost\/)/, ''));
      img.classList.add('broken');
      img.alt = `⚠ Image failed to load: ${img.getAttribute('alt') || shown}`;
      img.title = shown;
    }, { once: true });
  }
}

/** Delegated handler for copy buttons. Returns true if it handled the click. */
export async function handleCopyClick(e) {
  const btn = e.target.closest('.copy-btn');
  if (!btn) return false;
  const pre = btn.closest('.code-block')?.querySelector('pre');
  if (!pre) return true;
  const ok = await copyText(pre.textContent);
  btn.textContent = ok ? 'Copied' : 'Failed';
  btn.classList.toggle('done', ok);
  setTimeout(() => {
    btn.textContent = 'Copy';
    btn.classList.remove('done');
  }, 1400);
  return true;
}
