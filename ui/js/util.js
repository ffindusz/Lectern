// Small DOM helpers shared by the modules.

export const $ = (sel, root = document) => root.querySelector(sel);
export const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

/** Create an element: el('div', {class: 'x', text: 'hi'}, child, …) */
export function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v == null || v === false) continue;
    if (k === 'class') node.className = v;
    else if (k === 'text') node.textContent = v;
    else if (k.startsWith('on')) node.addEventListener(k.slice(2), v);
    else node.setAttribute(k, v === true ? '' : v);
  }
  for (const c of children) if (c != null) node.append(c);
  return node;
}

let toastTimer;
/** Show a transient message at the bottom of the window. */
export function toast(message, { error = false, ms = 2600 } = {}) {
  const t = $('#lec-toast');
  t.textContent = message;
  t.classList.toggle('error', error);
  t.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove('show'), error ? Math.max(ms, 5000) : ms);
}

export function debounce(fn, ms) {
  let t;
  return (...args) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...args), ms);
  };
}

export const nextFrame = () => new Promise((r) => requestAnimationFrame(() => r()));

const scripts = new Map();
/** Load a classic script once (vendored libraries, lazily). */
export function loadScript(src) {
  if (!scripts.has(src)) {
    scripts.set(src, new Promise((resolve, reject) => {
      const s = el('script', { src });
      s.onload = resolve;
      s.onerror = () => {
        scripts.delete(src);
        reject(new Error(`Failed to load ${src}`));
      };
      document.head.append(s);
    }));
  }
  return scripts.get(src);
}

const styles = new Map();
export function loadCss(href) {
  if (!styles.has(href)) {
    styles.set(href, new Promise((resolve, reject) => {
      const l = el('link', { rel: 'stylesheet', href });
      l.onload = resolve;
      l.onerror = () => reject(new Error(`Failed to load ${href}`));
      document.head.append(l);
    }));
  }
  return styles.get(href);
}

/** Copy text to the clipboard (async API with a fallback for older WebKitGTK). */
export async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const ta = el('textarea', { readonly: true });
    ta.value = text;
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.append(ta);
    ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    return ok;
  }
}
