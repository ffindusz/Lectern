// Thin wrappers over the Tauri globals (enabled by `withGlobalTauri`).

const T = window.__TAURI__;

export const invoke = (cmd, args) => T.core.invoke(cmd, args);

/** Subscribe to a backend event; the callback receives the payload. */
export const listen = (event, cb) => T.event.listen(event, (e) => cb(e.payload));
