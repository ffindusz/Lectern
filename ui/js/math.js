// KaTeX math. Rust (comrak) marks math with `data-math-style="inline|display"`.

import { $$, el, loadCss, loadScript } from './util.js';

export async function renderMath(root) {
  const nodes = $$('[data-math-style]', root);
  if (!nodes.length) return;
  let katex;
  try {
    await Promise.all([loadScript('vendor/katex/katex.min.js'), loadCss('vendor/katex/katex.min.css')]);
    katex = window.katex;
  } catch (err) {
    nodes.forEach((n) => n.replaceWith(errorNode(n.textContent, n.dataset.mathStyle === 'display', err)));
    return;
  }
  for (const node of nodes) {
    const display = node.dataset.mathStyle === 'display';
    const tex = node.textContent;
    // ```math blocks arrive as <pre><code data-math-style="display">; replace the whole <pre>.
    const target = node.parentElement?.tagName === 'PRE' ? node.parentElement : node;
    const out = el('span', { class: display ? 'math-display' : 'math-inline' });
    try {
      katex.render(tex, out, {
        displayMode: display,
        throwOnError: true,
        strict: 'ignore',
        trust: false,
        output: 'htmlAndMathml',
      });
      target.replaceWith(out);
    } catch (err) {
      target.replaceWith(errorNode(tex, display, err));
    }
  }
}

function errorNode(tex, display, err) {
  const message = (err?.message ?? String(err)).replace(/^KaTeX parse error:\s*/, '');
  if (!display) {
    return el('span', { class: 'math-error', title: message, text: `$${tex}$` });
  }
  return el('div', { class: 'diagram-error' },
    el('div', { class: 'diagram-error-title', text: '⚠ Math could not be rendered' }),
    el('div', { class: 'diagram-error-message', text: message }),
    el('pre', {}, el('code', { text: tex.trim() })),
  );
}
