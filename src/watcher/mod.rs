pub mod debounce;
pub mod pipeline;
pub mod retention;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// A file event detected by the watcher, before debouncing.
#[derive(Debug, Clone)]
pub struct FileEvent {
    /// Which registered project this event belongs to
    pub project_id: String,
    /// Full path to the changed file
    pub path: PathBuf,
    /// What kind of change was detected
    pub kind: FileEventKind,
}

/// The type of file change detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEventKind {
    Create,
    Modify,
    Remove,
}

/// Manages per-project file watchers using the notify crate.
///
/// Each registered project gets its own RecommendedWatcher instance that
/// monitors the project's `.planning/` directory recursively.
pub struct FileWatcher {
    /// Active watchers keyed by project_id
    watchers: HashMap<String, RecommendedWatcher>,
    /// Channel sender for raw file events (sent to debouncer)
    event_tx: mpsc::Sender<FileEvent>,
}

impl FileWatcher {
    /// Create a new FileWatcher that sends events on the given channel.
    pub fn new(event_tx: mpsc::Sender<FileEvent>) -> Self {
        Self {
            watchers: HashMap::new(),
            event_tx,
        }
    }

    /// Start watching a project's `.planning/` directory recursively.
    ///
    /// If the project is already being watched, the old watcher is replaced.
    pub fn watch_project(&mut self, project_id: &str, path: &Path) -> Result<(), notify::Error> {
        let planning_path = path.join(".planning");

        // Check inotify watch limits on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(contents) = std::fs::read_to_string("/proc/sys/fs/inotify/max_user_watches")
                && let Ok(max) = contents.trim().parse::<u64>()
                && max < 8192
            {
                warn!(
                    max_watches = max,
                    "Low inotify watch limit detected. Consider increasing via: \
                             sysctl fs.inotify.max_user_watches=65536"
                );
            }
        }

        let tx = self.event_tx.clone();
        let pid = project_id.to_string();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Some(kind) = map_event_kind(&event.kind) {
                            for path in event.paths {
                                let file_event = FileEvent {
                                    project_id: pid.clone(),
                                    path,
                                    kind: kind.clone(),
                                };
                                // Use blocking_send because notify callbacks are synchronous
                                if let Err(e) = tx.blocking_send(file_event) {
                                    warn!("Failed to send file event: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "File watcher error");
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(&planning_path, RecursiveMode::Recursive)?;

        info!(
            project_id = project_id,
            path = %planning_path.display(),
            "Started watching project"
        );

        self.watchers.insert(project_id.to_string(), watcher);
        Ok(())
    }

    /// Stop watching a project. Drops the watcher, which unregisters all watches.
    pub fn unwatch_project(&mut self, project_id: &str) -> Result<(), String> {
        if self.watchers.remove(project_id).is_some() {
            info!(project_id = project_id, "Stopped watching project");
            Ok(())
        } else {
            Err(format!("No watcher found for project: {}", project_id))
        }
    }

    /// Returns the number of actively watched projects.
    pub fn watched_count(&self) -> usize {
        self.watchers.len()
    }
}

/// Map a notify EventKind to our FileEventKind, filtering out irrelevant events.
///
/// - Create events -> Create
/// - Modify (data change) -> Modify
/// - Remove events -> Remove
/// - Rename TO events -> Create (for atomic write support)
/// - Metadata, Access, Other -> None (filtered out)
fn map_event_kind(kind: &EventKind) -> Option<FileEventKind> {
    match kind {
        EventKind::Create(CreateKind::File) | EventKind::Create(CreateKind::Any) => {
            Some(FileEventKind::Create)
        }
        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
            Some(FileEventKind::Modify)
        }
        EventKind::Remove(RemoveKind::File) | EventKind::Remove(RemoveKind::Any) => {
            Some(FileEventKind::Remove)
        }
        // MOVED_TO (rename to destination) -> treat as Create for atomic write support
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Some(FileEventKind::Create),
        _ => {
            debug!(kind = ?kind, "Ignoring non-relevant file event");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_map_event_kind_create() {
        assert_eq!(
            map_event_kind(&EventKind::Create(CreateKind::File)),
            Some(FileEventKind::Create)
        );
        assert_eq!(
            map_event_kind(&EventKind::Create(CreateKind::Any)),
            Some(FileEventKind::Create)
        );
    }

    #[test]
    fn test_map_event_kind_modify() {
        use notify::event::DataChange;
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Data(DataChange::Any))),
            Some(FileEventKind::Modify)
        );
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Any)),
            Some(FileEventKind::Modify)
        );
    }

    #[test]
    fn test_map_event_kind_remove() {
        assert_eq!(
            map_event_kind(&EventKind::Remove(RemoveKind::File)),
            Some(FileEventKind::Remove)
        );
    }

    #[test]
    fn test_map_event_kind_rename_to_is_create() {
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Name(RenameMode::To))),
            Some(FileEventKind::Create)
        );
    }

    #[test]
    fn test_map_event_kind_filters_metadata() {
        use notify::event::MetadataKind;
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any))),
            None
        );
    }

    #[test]
    fn test_map_event_kind_filters_access() {
        use notify::event::AccessKind;
        assert_eq!(map_event_kind(&EventKind::Access(AccessKind::Any)), None);
    }

    #[tokio::test]
    async fn test_file_watcher_watch_and_unwatch() {
        let temp_dir = TempDir::new().unwrap();
        let planning_dir = temp_dir.path().join(".planning");
        fs::create_dir_all(&planning_dir).unwrap();

        let (tx, _rx) = mpsc::channel(100);
        let mut watcher = FileWatcher::new(tx);

        // Watch a project
        watcher
            .watch_project("test-project", temp_dir.path())
            .unwrap();
        assert_eq!(watcher.watched_count(), 1);

        // Unwatch
        watcher.unwatch_project("test-project").unwrap();
        assert_eq!(watcher.watched_count(), 0);
    }

    #[tokio::test]
    async fn test_file_watcher_unwatch_missing() {
        let (tx, _rx) = mpsc::channel(100);
        let mut watcher = FileWatcher::new(tx);

        let result = watcher.unwatch_project("nonexistent");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_watcher_sends_events() {
        let temp_dir = TempDir::new().unwrap();
        let planning_dir = temp_dir.path().join(".planning");
        fs::create_dir_all(&planning_dir).unwrap();

        let (tx, mut rx) = mpsc::channel(100);
        let mut watcher = FileWatcher::new(tx);

        watcher
            .watch_project("test-project", temp_dir.path())
            .unwrap();

        // Write a file to trigger an event
        let test_file = planning_dir.join("STATE.md");
        fs::write(&test_file, "---\nstatus: executing\n---\n").unwrap();

        // Wait for event with timeout
        let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await;
        assert!(
            event.is_ok(),
            "Should receive a file event within 2 seconds"
        );
        let event = event.unwrap().unwrap();
        assert_eq!(event.project_id, "test-project");
    }

    #[tokio::test]
    async fn test_file_watcher_replace_existing() {
        let temp_dir = TempDir::new().unwrap();
        let planning_dir = temp_dir.path().join(".planning");
        fs::create_dir_all(&planning_dir).unwrap();

        let (tx, _rx) = mpsc::channel(100);
        let mut watcher = FileWatcher::new(tx);

        watcher
            .watch_project("test-project", temp_dir.path())
            .unwrap();
        // Watching same project again should replace the watcher
        watcher
            .watch_project("test-project", temp_dir.path())
            .unwrap();
        assert_eq!(watcher.watched_count(), 1);
    }
}
