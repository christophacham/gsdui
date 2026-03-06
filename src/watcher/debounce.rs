use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, trace};

use super::FileEventKind;

/// A debounced file event ready for processing by the parse pipeline.
#[derive(Debug, Clone)]
pub struct DebouncedEvent {
    /// Full path to the changed file
    pub path: PathBuf,
    /// Which registered project this event belongs to
    pub project_id: String,
    /// What kind of change was detected
    pub kind: FileEventKind,
}

/// Per-file debouncer that batches rapid changes into single events.
///
/// When multiple changes happen to the same file within the debounce delay,
/// only the last change is emitted. Different files are debounced independently.
pub struct Debouncer {
    /// Per-file pending timers (path -> abort handle for the timer task)
    pending: HashMap<PathBuf, JoinHandle<()>>,
    /// Output channel for debounced events
    output: mpsc::Sender<Vec<DebouncedEvent>>,
    /// Debounce delay (default 75ms, within the 50-100ms range)
    delay: Duration,
}

impl Debouncer {
    /// Create a new debouncer with the given output channel and delay.
    pub fn new(output: mpsc::Sender<Vec<DebouncedEvent>>, delay: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            output,
            delay,
        }
    }

    /// Create a new debouncer with the default 75ms delay.
    pub fn with_default_delay(output: mpsc::Sender<Vec<DebouncedEvent>>) -> Self {
        Self::new(output, Duration::from_millis(75))
    }

    /// Handle a raw file event. Resets the debounce timer for this file path.
    ///
    /// If a timer was already running for this path, it is cancelled and a new
    /// one is started. The event is emitted only after the delay elapses without
    /// any new events for the same path.
    pub fn handle_event(&mut self, project_id: String, path: PathBuf, kind: FileEventKind) {
        // Cancel existing timer for this path
        if let Some(handle) = self.pending.remove(&path) {
            handle.abort();
            trace!(path = %path.display(), "Cancelled pending debounce timer");
        }

        let tx = self.output.clone();
        let delay = self.delay;
        let event_path = path.clone();

        // Start a new timer
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            let event = DebouncedEvent {
                path: event_path.clone(),
                project_id,
                kind,
            };

            debug!(path = %event_path.display(), "Debounce timer fired, emitting event");

            // Send as a single-event batch
            if let Err(e) = tx.send(vec![event]).await {
                tracing::warn!("Failed to send debounced event: {}", e);
            }
        });

        self.pending.insert(path, handle);
    }

    /// Spawn a debouncer task that reads raw events and processes them.
    ///
    /// Returns the join handle for the spawned task.
    pub fn spawn(
        mut raw_rx: mpsc::Receiver<super::FileEvent>,
        debounced_tx: mpsc::Sender<Vec<DebouncedEvent>>,
        delay: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut debouncer = Debouncer::new(debounced_tx, delay);

            while let Some(event) = raw_rx.recv().await {
                debouncer.handle_event(event.project_id, event.path, event.kind);
            }

            debug!("Debouncer task shutting down (input channel closed)");
        })
    }
}

impl Drop for Debouncer {
    fn drop(&mut self) {
        // Cancel all pending timers on drop
        for (_, handle) in self.pending.drain() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: receive an event with implicit time advancement.
    /// In paused-time mode, `rx.recv()` will not resolve until the spawned sleep
    /// completes, so the runtime auto-advances time when all tasks are blocked.
    async fn recv_event(rx: &mut mpsc::Receiver<Vec<DebouncedEvent>>) -> Vec<DebouncedEvent> {
        tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("Timed out waiting for debounced event")
            .expect("Channel closed unexpectedly")
    }

    #[tokio::test(start_paused = true)]
    async fn test_debouncer_single_event() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut debouncer = Debouncer::new(tx, Duration::from_millis(75));

        debouncer.handle_event(
            "project-1".to_string(),
            PathBuf::from("/test/.planning/STATE.md"),
            FileEventKind::Modify,
        );

        // In paused mode, recv() auto-advances time until the sleep completes
        let batch = recv_event(&mut rx).await;
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].project_id, "project-1");
        assert_eq!(
            batch[0].path,
            PathBuf::from("/test/.planning/STATE.md")
        );
        assert_eq!(batch[0].kind, FileEventKind::Modify);
    }

    #[tokio::test(start_paused = true)]
    async fn test_debouncer_batches_rapid_events_same_file() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut debouncer = Debouncer::new(tx, Duration::from_millis(75));

        let path = PathBuf::from("/test/.planning/STATE.md");

        // Send 3 rapid events for the same file at 0ms, 20ms, 40ms
        debouncer.handle_event(
            "project-1".to_string(),
            path.clone(),
            FileEventKind::Modify,
        );

        // Advance 20ms (timer resets)
        tokio::time::sleep(Duration::from_millis(20)).await;

        debouncer.handle_event(
            "project-1".to_string(),
            path.clone(),
            FileEventKind::Modify,
        );

        // Advance 20ms (timer resets again)
        tokio::time::sleep(Duration::from_millis(20)).await;

        debouncer.handle_event(
            "project-1".to_string(),
            path.clone(),
            FileEventKind::Create,
        );

        // The debounce timer restarts from the last event.
        // After 75ms from the last event, we get a single batch.
        let batch = recv_event(&mut rx).await;
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].kind, FileEventKind::Create); // Last event wins
    }

    #[tokio::test(start_paused = true)]
    async fn test_debouncer_separate_events_different_files() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut debouncer = Debouncer::new(tx, Duration::from_millis(75));

        let path_a = PathBuf::from("/test/.planning/STATE.md");
        let path_b = PathBuf::from("/test/.planning/ROADMAP.md");

        // Send events for two different files simultaneously
        debouncer.handle_event(
            "project-1".to_string(),
            path_a.clone(),
            FileEventKind::Modify,
        );
        debouncer.handle_event(
            "project-1".to_string(),
            path_b.clone(),
            FileEventKind::Create,
        );

        // Receive both events (they have independent timers)
        let batch1 = recv_event(&mut rx).await;
        let batch2 = recv_event(&mut rx).await;

        let mut received_paths = Vec::new();
        for event in batch1.iter().chain(batch2.iter()) {
            received_paths.push(event.path.clone());
        }

        assert_eq!(received_paths.len(), 2);
        assert!(received_paths.contains(&path_a));
        assert!(received_paths.contains(&path_b));
    }

    #[tokio::test(start_paused = true)]
    async fn test_debouncer_output_within_100ms() {
        let (tx, mut rx) = mpsc::channel(100);
        let mut debouncer = Debouncer::new(tx, Duration::from_millis(75));

        let start = tokio::time::Instant::now();

        debouncer.handle_event(
            "project-1".to_string(),
            PathBuf::from("/test/.planning/STATE.md"),
            FileEventKind::Modify,
        );

        // Receive the event (auto-advances time)
        let batch = recv_event(&mut rx).await;
        assert_eq!(batch.len(), 1);

        // Verify the event arrived within 100ms of being submitted
        let elapsed = start.elapsed();
        assert!(
            elapsed <= Duration::from_millis(100),
            "Event should arrive within 100ms, took {:?}",
            elapsed
        );
        assert!(
            elapsed >= Duration::from_millis(75),
            "Event should not arrive before 75ms delay, took {:?}",
            elapsed
        );
    }

    #[tokio::test(start_paused = true)]
    async fn test_debouncer_spawn_task() {
        use crate::watcher::FileEvent;

        let (raw_tx, raw_rx) = mpsc::channel::<FileEvent>(100);
        let (debounced_tx, mut debounced_rx) = mpsc::channel(100);

        let _handle = Debouncer::spawn(raw_rx, debounced_tx, Duration::from_millis(50));

        // Send a raw event
        raw_tx
            .send(FileEvent {
                project_id: "p1".to_string(),
                path: PathBuf::from("/test/STATE.md"),
                kind: FileEventKind::Modify,
            })
            .await
            .unwrap();

        // Receive the debounced event (auto-advances time)
        let batch = tokio::time::timeout(Duration::from_secs(5), debounced_rx.recv())
            .await
            .expect("Timed out")
            .expect("Channel closed");
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].project_id, "p1");
    }

    #[tokio::test(start_paused = true)]
    async fn test_debouncer_cancel_on_drop() {
        let (tx, mut rx) = mpsc::channel(100);

        {
            let mut debouncer = Debouncer::new(tx, Duration::from_millis(75));
            debouncer.handle_event(
                "project-1".to_string(),
                PathBuf::from("/test/STATE.md"),
                FileEventKind::Modify,
            );
            // Debouncer is dropped here, should cancel pending timer
        }

        // Even after advancing past the delay, nothing should arrive
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should not receive any events since debouncer was dropped
        assert!(rx.try_recv().is_err());
    }
}
