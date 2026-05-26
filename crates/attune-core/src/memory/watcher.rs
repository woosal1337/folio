//! Self-healing memory: debounced reindex orchestrator. v2 finding 037
//! / GET-40.
//!
//! When the user edits a memory `.md` directly in Obsidian / their
//! editor / via `git pull`, the index can drift from the truth on disk.
//! The flat manual "Reindex" button was a debugging artifact (closes
//! R04). This module owns the debounce + dispatch logic; the OS-level
//! `notify` watcher hooks into it in a follow-up.
//!
//! Design:
//!
//!   * Events arrive as `ReindexEvent { path, kind }` on a channel.
//!   * The `Debouncer` collapses bursts: every event resets a 500ms
//!     timer; when the timer fires, the accumulated set of changed
//!     paths is handed to the `Reindexer::reindex(&paths)` callback
//!     in a single batch.
//!   * The batch always deduplicates by path. Multiple events on the
//!     same file (Created + Modified + Modified) collapse to one
//!     reindex.
//!   * Cancellation is supported by dropping the channel sender — the
//!     debouncer loop exits cleanly on the next tick.
//!
//! Everything in this module is plain Rust + crossbeam (already in
//! the workspace). No new deps. The `notify` crate is wired in the
//! follow-up at the call site.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};

pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    Created,
    Modified,
    Removed,
}

#[derive(Debug, Clone)]
pub struct ReindexEvent {
    pub path: PathBuf,
    pub kind: EventKind,
}

/// Sink the debouncer calls when a batch is ready. Production code
/// wires this to MemoryStore::reindex(paths); tests use a recording
/// stub.
pub trait Reindexer: Send + 'static {
    fn reindex(&mut self, paths: &[PathBuf]);
}

/// Bound a path that we got an event for to the `.md` file that holds
/// the memory. A `.attune/memories/foo.md.swp` event maps to `foo.md`;
/// non-memory files (anything outside `memories/`) are filtered out
/// upstream by the caller.
pub fn memory_path_for(p: &Path) -> Option<PathBuf> {
    let name = p.file_name()?.to_string_lossy().to_string();
    // Strip editor swap suffixes ("foo.md.swp", "foo.md~", "foo.md.lock").
    let stripped = if let Some(stem) = name.strip_suffix(".swp") {
        stem.to_string()
    } else if let Some(stem) = name.strip_suffix('~') {
        stem.to_string()
    } else if let Some(stem) = name.strip_suffix(".lock") {
        stem.to_string()
    } else {
        name
    };
    if !stripped.ends_with(".md") {
        return None;
    }
    let parent = p.parent()?;
    Some(parent.join(stripped))
}

/// Build a (sender, debouncer-runnable) pair. The caller spawns the
/// runnable on a thread (or polls it directly in tests), and feeds
/// events through the sender.
pub fn build<R: Reindexer>(
    debounce: Duration,
    reindexer: R,
) -> (Sender<ReindexEvent>, Debouncer<R>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let dbnc = Debouncer {
        rx,
        debounce,
        pending: HashSet::new(),
        reindexer,
    };
    (tx, dbnc)
}

pub struct Debouncer<R: Reindexer> {
    rx: Receiver<ReindexEvent>,
    debounce: Duration,
    pending: HashSet<PathBuf>,
    reindexer: R,
}

impl<R: Reindexer> Debouncer<R> {
    /// Run forever until every sender is dropped. On each loop:
    ///   - If `pending` is empty, block until an event arrives.
    ///   - Otherwise, wait up to `debounce` for a follow-up; if it
    ///     arrives, fold it in and reset the timer; if it times out,
    ///     flush the batch.
    pub fn run(mut self) {
        loop {
            if self.pending.is_empty() {
                match self.rx.recv() {
                    Ok(ev) => {
                        if let Some(path) = memory_path_for(&ev.path) {
                            self.pending.insert(path);
                        }
                    }
                    Err(_) => return,
                }
                continue;
            }
            match self.rx.recv_timeout(self.debounce) {
                Ok(ev) => {
                    if let Some(path) = memory_path_for(&ev.path) {
                        self.pending.insert(path);
                    }
                }
                Err(RecvTimeoutError::Timeout) => self.flush(),
                Err(RecvTimeoutError::Disconnected) => {
                    self.flush();
                    return;
                }
            }
        }
    }

    fn flush(&mut self) {
        let mut batch: Vec<PathBuf> = self.pending.drain().collect();
        batch.sort();
        if !batch.is_empty() {
            self.reindexer.reindex(&batch);
        }
    }

    /// Test-only single tick: process one event (with timeout) and
    /// flush if the timeout expires. Returns true when a flush happens.
    #[cfg(test)]
    fn tick(&mut self) -> bool {
        if self.pending.is_empty() {
            if let Ok(ev) = self.rx.recv_timeout(self.debounce) {
                if let Some(path) = memory_path_for(&ev.path) {
                    self.pending.insert(path);
                }
            }
            return false;
        }
        match self.rx.recv_timeout(self.debounce) {
            Ok(ev) => {
                if let Some(path) = memory_path_for(&ev.path) {
                    self.pending.insert(path);
                }
                false
            }
            Err(RecvTimeoutError::Timeout) => {
                self.flush();
                true
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.flush();
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[derive(Default, Clone)]
    struct Recorder {
        batches: Arc<Mutex<Vec<Vec<PathBuf>>>>,
    }
    impl Reindexer for Recorder {
        fn reindex(&mut self, paths: &[PathBuf]) {
            self.batches.lock().push(paths.to_vec());
        }
    }

    #[test]
    fn memory_path_for_strips_editor_suffixes() {
        let base = PathBuf::from("/v/memories/foo.md");
        assert_eq!(memory_path_for(&PathBuf::from("/v/memories/foo.md.swp")), Some(base.clone()));
        assert_eq!(memory_path_for(&PathBuf::from("/v/memories/foo.md~")), Some(base.clone()));
        assert_eq!(memory_path_for(&PathBuf::from("/v/memories/foo.md.lock")), Some(base.clone()));
        assert_eq!(memory_path_for(&PathBuf::from("/v/memories/foo.md")), Some(base));
    }

    #[test]
    fn memory_path_for_ignores_non_md() {
        assert!(memory_path_for(&PathBuf::from("/v/memories/notes.txt")).is_none());
    }

    #[test]
    fn debouncer_collapses_burst_into_one_batch() {
        let recorder = Recorder::default();
        let batches = recorder.batches.clone();
        let (tx, mut dbnc) = build(Duration::from_millis(20), recorder);

        let p = PathBuf::from("/v/memories/a.md");
        for _ in 0..5 {
            tx.send(ReindexEvent { path: p.clone(), kind: EventKind::Modified }).unwrap();
        }
        // First tick: pulls one event into pending.
        dbnc.tick();
        // Now drain remaining events (they all go to pending; no flush).
        for _ in 0..5 {
            dbnc.tick();
        }
        // Drop sender so the timeout path fires a flush.
        drop(tx);
        let _ = dbnc.tick();

        let batches = batches.lock().clone();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0], vec![p]);
    }

    #[test]
    fn debouncer_dedupes_across_paths() {
        let recorder = Recorder::default();
        let batches = recorder.batches.clone();
        let (tx, mut dbnc) = build(Duration::from_millis(20), recorder);

        let a = PathBuf::from("/v/memories/a.md");
        let b = PathBuf::from("/v/memories/b.md");
        tx.send(ReindexEvent { path: a.clone(), kind: EventKind::Modified }).unwrap();
        tx.send(ReindexEvent { path: a.clone(), kind: EventKind::Modified }).unwrap();
        tx.send(ReindexEvent { path: b.clone(), kind: EventKind::Created }).unwrap();
        for _ in 0..3 {
            dbnc.tick();
        }
        drop(tx);
        let _ = dbnc.tick();

        let batches = batches.lock().clone();
        assert_eq!(batches.len(), 1);
        let mut got = batches[0].clone();
        got.sort();
        assert_eq!(got, vec![a, b]);
    }

    #[test]
    fn debouncer_exits_when_senders_drop() {
        let recorder = Recorder::default();
        let (tx, dbnc) = build(Duration::from_millis(5), recorder);
        drop(tx);
        // run() returns; we use a thread to enforce it doesn't hang.
        let handle = std::thread::spawn(move || dbnc.run());
        handle.join().unwrap();
    }
}
