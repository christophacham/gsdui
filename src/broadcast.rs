use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::{RwLock, broadcast};

use crate::watcher::pipeline::StateUpdate;

/// Per-project broadcast channel management.
///
/// Each project gets its own broadcast channel. WebSocket clients subscribe
/// to specific projects and receive only updates for those projects.
pub struct Broadcaster {
    /// Per-project broadcast channels (project_id -> sender)
    channels: RwLock<HashMap<String, broadcast::Sender<StateUpdate>>>,
    /// Number of currently connected WebSocket clients
    client_count: AtomicU32,
}

impl Broadcaster {
    /// Create a new Broadcaster with no channels.
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            client_count: AtomicU32::new(0),
        }
    }

    /// Get or create a broadcast channel for a project.
    /// Returns a clone of the Sender.
    pub async fn get_or_create_channel(
        &self,
        project_id: &str,
    ) -> broadcast::Sender<StateUpdate> {
        // Try read-only first (fast path)
        {
            let channels = self.channels.read().await;
            if let Some(tx) = channels.get(project_id) {
                return tx.clone();
            }
        }

        // Need to create a new channel
        let mut channels = self.channels.write().await;
        // Double-check after acquiring write lock
        if let Some(tx) = channels.get(project_id) {
            return tx.clone();
        }

        let (tx, _rx) = broadcast::channel(64);
        channels.insert(project_id.to_string(), tx.clone());
        tx
    }

    /// Subscribe to broadcast channels for the given projects.
    /// Returns a list of (project_id, receiver) pairs.
    /// Increments the client count.
    pub async fn subscribe(
        &self,
        project_ids: &[String],
    ) -> Vec<(String, broadcast::Receiver<StateUpdate>)> {
        self.client_count.fetch_add(1, Ordering::Relaxed);

        let mut receivers = Vec::with_capacity(project_ids.len());
        for project_id in project_ids {
            let tx = self.get_or_create_channel(project_id).await;
            let rx = tx.subscribe();
            receivers.push((project_id.clone(), rx));
        }
        receivers
    }

    /// Decrement client count when a client disconnects.
    pub fn unsubscribe(&self) {
        // Prevent underflow
        let _ = self
            .client_count
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                if v > 0 { Some(v - 1) } else { Some(0) }
            });
    }

    /// Broadcast a state update to the appropriate project channel.
    /// Returns Ok(receiver_count) or Err if no subscribers.
    pub async fn broadcast(&self, project_id: &str, update: StateUpdate) -> Result<usize, ()> {
        let channels = self.channels.read().await;
        if let Some(tx) = channels.get(project_id) {
            tx.send(update).map_err(|_| ())
        } else {
            Err(())
        }
    }

    /// Get the current number of connected WebSocket clients.
    pub fn client_count(&self) -> u32 {
        self.client_count.load(Ordering::Relaxed)
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watcher::pipeline::StateChange;

    #[tokio::test]
    async fn test_broadcaster_subscribe_creates_channel() {
        let broadcaster = Broadcaster::new();
        let projects = vec!["proj-1".to_string()];
        let receivers = broadcaster.subscribe(&projects).await;
        assert_eq!(receivers.len(), 1);
        assert_eq!(receivers[0].0, "proj-1");
        assert_eq!(broadcaster.client_count(), 1);
    }

    #[tokio::test]
    async fn test_broadcaster_unsubscribe_decrements() {
        let broadcaster = Broadcaster::new();
        let projects = vec!["proj-1".to_string()];
        let _receivers = broadcaster.subscribe(&projects).await;
        assert_eq!(broadcaster.client_count(), 1);

        broadcaster.unsubscribe();
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[tokio::test]
    async fn test_broadcaster_unsubscribe_no_underflow() {
        let broadcaster = Broadcaster::new();
        broadcaster.unsubscribe();
        assert_eq!(broadcaster.client_count(), 0);
    }

    #[tokio::test]
    async fn test_broadcaster_broadcast_delivers() {
        let broadcaster = Broadcaster::new();
        let projects = vec!["proj-1".to_string()];
        let mut receivers = broadcaster.subscribe(&projects).await;

        let update = StateUpdate {
            project_id: "proj-1".to_string(),
            change: StateChange::ConfigUpdated,
        };

        let result = broadcaster.broadcast("proj-1", update).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let (_id, rx) = &mut receivers[0];
        let received = rx.recv().await.unwrap();
        assert_eq!(received.project_id, "proj-1");
    }

    #[tokio::test]
    async fn test_broadcaster_broadcast_unknown_project() {
        let broadcaster = Broadcaster::new();
        let update = StateUpdate {
            project_id: "unknown".to_string(),
            change: StateChange::ConfigUpdated,
        };

        let result = broadcaster.broadcast("unknown", update).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_broadcaster_multiple_projects() {
        let broadcaster = Broadcaster::new();
        let projects = vec!["proj-1".to_string(), "proj-2".to_string()];
        let receivers = broadcaster.subscribe(&projects).await;
        assert_eq!(receivers.len(), 2);
        assert_eq!(broadcaster.client_count(), 1);
    }
}
