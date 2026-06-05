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

pub trait Reindexer: Send + 'static {
    fn reindex(&mut self, paths: &[PathBuf]);
}

pub fn memory_path_for(p: &Path) -> Option<PathBuf> {
    let name = p.file_name()?.to_string_lossy().to_string();

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

pub const REINDEX_CHANNEL_CAPACITY: usize = 256;

pub fn build<R: Reindexer>(
    debounce: Duration,
    reindexer: R,
) -> (Sender<ReindexEvent>, Debouncer<R>) {
    let (tx, rx) = crossbeam_channel::bounded(REINDEX_CHANNEL_CAPACITY);
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
        assert_eq!(
            memory_path_for(&PathBuf::from("/v/memories/foo.md.swp")),
            Some(base.clone())
        );
        assert_eq!(
            memory_path_for(&PathBuf::from("/v/memories/foo.md~")),
            Some(base.clone())
        );
        assert_eq!(
            memory_path_for(&PathBuf::from("/v/memories/foo.md.lock")),
            Some(base.clone())
        );
        assert_eq!(
            memory_path_for(&PathBuf::from("/v/memories/foo.md")),
            Some(base)
        );
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
            tx.send(ReindexEvent {
                path: p.clone(),
                kind: EventKind::Modified,
            })
            .unwrap();
        }

        dbnc.tick();

        for _ in 0..5 {
            dbnc.tick();
        }

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
        tx.send(ReindexEvent {
            path: a.clone(),
            kind: EventKind::Modified,
        })
        .unwrap();
        tx.send(ReindexEvent {
            path: a.clone(),
            kind: EventKind::Modified,
        })
        .unwrap();
        tx.send(ReindexEvent {
            path: b.clone(),
            kind: EventKind::Created,
        })
        .unwrap();
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

        let handle = std::thread::spawn(move || dbnc.run());
        handle.join().unwrap();
    }
}
