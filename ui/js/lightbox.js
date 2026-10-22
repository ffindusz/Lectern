// Full-size image viewer. Click toggles fit-to-window / actual size; Esc or backdrop closes.

import { $ } from './util.js';

const box = () => $('#lec-lightbox');

export const isLightboxOpen = () => !box().hidden;

export function openLightbox(src, caption) {
  const lb = box();
  const img = lb.querySelector('img');
  img.src = src;
  img.alt = caption || '';
  lb.querySelector('.caption').textContent = caption || '';
  lb.querySelector('.caption').hidden = !caption;
  lb.classList.add('fit');
  lb.hidden = false;
}

export function closeLightbox() {
  const lb = box();
  lb.hidden = true;
  lb.querySelector('img').removeAttribute('src');
}

export function initLightbox() {
  const lb = box();
  lb.addEventListener('click', (e) => {
    if (e.target.tagName === 'IMG') {
      lb.classList.toggle('fit');
    } else {
      closeLightbox();
    }
  });
}
