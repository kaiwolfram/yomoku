//! Cooperative import cancellation, serialized with the per-file commit.

use std::sync::{Arc, Mutex, MutexGuard};

/// Create a fresh handle for each file attempt and share a clone with its caller.
/// Cancellation does not interrupt a running decoder, analyzer or SQLite call;
/// the worker observes it at checkpoints and rolls back before returning.
#[derive(Clone, Debug, Default)]
pub struct ImportCancellation {
    cancelled: Arc<Mutex<bool>>,
}

impl ImportCancellation {
    /// Request rollback. If COMMIT already holds the gate, wait for it to finish;
    /// a successfully committed file remains completed and must be retained.
    /// This handle carries no database connection and cannot cancel other files.
    /// Call off the UI thread: a COMMIT already in progress may wait on SQLite.
    pub fn cancel(&self) {
        *self.lock() = true;
    }

    pub fn is_cancelled(&self) -> bool {
        *self.lock()
    }

    /// The final cancellation check and COMMIT share a critical section. A check
    /// followed by an unlocked commit could publish a file after cancel returned.
    pub(crate) fn commit<T, E>(
        &self,
        commit: impl FnOnce() -> Result<T, E>,
    ) -> Result<Option<T>, E> {
        let cancelled = self.lock();
        if *cancelled {
            return Ok(None);
        }
        let result = commit();
        drop(cancelled);
        result.map(Some)
    }

    fn lock(&self) -> MutexGuard<'_, bool> {
        self.cancelled
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, sync::mpsc, thread, time::Duration};

    use super::*;

    #[test]
    fn cancellation_wins_before_commit_and_is_shared_and_idempotent() {
        let cancellation = ImportCancellation::default();
        let caller = cancellation.clone();
        caller.cancel();
        caller.cancel();
        assert!(cancellation.is_cancelled());
        assert_eq!(
            cancellation.commit(|| -> Result<(), Infallible> {
                panic!("cancelled import must not commit")
            }),
            Ok(None)
        );
    }

    #[test]
    fn commit_winner_finishes_before_cancellation_returns_including_on_failure() {
        for fail in [false, true] {
            let cancellation = ImportCancellation::default();
            let worker = cancellation.clone();
            let (entered, ready) = mpsc::channel();
            let (release, finish) = mpsc::channel();
            let commit = thread::spawn(move || {
                worker.commit(|| {
                    entered.send(()).unwrap();
                    finish.recv_timeout(Duration::from_secs(10)).unwrap();
                    if fail {
                        Err("commit failed")
                    } else {
                        Ok("completed")
                    }
                })
            });
            ready.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(cancellation.cancelled.try_lock().is_err());
            let caller = cancellation.clone();
            let (requested, request) = mpsc::channel();
            let (returned, result) = mpsc::channel();
            let cancel = thread::spawn(move || {
                requested.send(()).unwrap();
                caller.cancel();
                returned.send(()).unwrap();
            });
            request.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(matches!(result.try_recv(), Err(mpsc::TryRecvError::Empty)));
            release.send(()).unwrap();
            assert_eq!(
                commit.join().unwrap(),
                if fail {
                    Err("commit failed")
                } else {
                    Ok(Some("completed"))
                }
            );
            result.recv_timeout(Duration::from_secs(10)).unwrap();
            cancel.join().unwrap();
            assert!(cancellation.is_cancelled());
        }
    }
}
