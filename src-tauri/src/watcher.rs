//! Watch one file for changes, debounced.
//!
//! We watch the file's *directory* (non-recursively) rather than the file itself, because
//! many editors save by writing a temp file and renaming it over the original, which would
//! silently end an inode-based watch on the old file.

use std::ffi::OsString;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

const DEBOUNCE: Duration = Duration::from_millis(150);

/// Dropping this stops watching (and ends the debounce thread).
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

/// Call `on_change` (from a background thread) whenever `file` is modified, created,
/// replaced or removed. Bursts of events within [`DEBOUNCE`] are coalesced into one call.
pub fn watch(file: &Path, on_change: impl Fn() + Send + 'static) -> notify::Result<FileWatcher> {
    let dir = file
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let name: OsString = file
        .file_name()
        .ok_or_else(|| notify::Error::generic("path has no file name"))?
        .to_owned();

    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        let relevant = matches!(
            event.kind,
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) | EventKind::Any
        ) && event
            .paths
            .iter()
            .any(|p| p.file_name() == Some(name.as_os_str()));
        if relevant {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(&dir, RecursiveMode::NonRecursive)?;

    thread::Builder::new()
        .name("lectern-watch".into())
        .spawn(move || {
            // Ends when the watcher (and thus `tx`) is dropped.
            while rx.recv().is_ok() {
                loop {
                    match rx.recv_timeout(DEBOUNCE) {
                        Ok(()) => continue,
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }
                on_change();
            }
        })
        .map_err(notify::Error::io)?;

    Ok(FileWatcher { _watcher: watcher })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn detects_write_and_ignores_siblings() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("doc.md");
        std::fs::write(&file, "a").unwrap();
        let (tx, rx) = channel();
        let _w = watch(&file, move || {
            let _ = tx.send(());
        })
        .unwrap();
        thread::sleep(Duration::from_millis(200));

        std::fs::write(dir.path().join("other.md"), "x").unwrap();
        assert!(
            rx.recv_timeout(Duration::from_millis(600)).is_err(),
            "sibling must not trigger"
        );

        std::fs::write(&file, "b").unwrap();
        assert!(
            rx.recv_timeout(Duration::from_secs(3)).is_ok(),
            "write must trigger"
        );
    }
}
