---
phase: 01-backend-foundation-and-state-pipeline
plan: 03
subsystem: watcher, pipeline, retention
tags: [rust, notify, tokio, inotify, debounce, file-watcher, parse-pipeline, retention-pruning]

requires:
  - phase: 01-01
    provides: "SQLite database with query layer, AppState, Project CRUD API"
  - phase: 01-02
    provides: "9 parsers for all GSD file formats, ParseError enum, stage derivation"
provides:
  - "FileWatcher: per-project notify watchers on .planning/ directories"
  - "Custom per-file Debouncer with 75ms delay (configurable 50-100ms range)"
  - "Parse pipeline routing 8 file types to correct parsers with DB persistence"
  - "bootstrap_project: full re-parse of .planning/ directory tree"
  - "Startup reconciliation: re-parse all registered projects on daemon restart"
  - "Offline project detection: missing directories marked as offline"
  - "Retention pruning: periodic hard-delete of expired runs, sessions, errors"
  - "StateUpdate/StateChange types for downstream WebSocket broadcast"
  - "Graceful shutdown via CancellationToken for all background tasks"
affects: [01-04, 02-01]

tech-stack:
  added: [notify 8, tokio-util CancellationToken, chrono Duration]
  patterns: [per-file debounce via HashMap<PathBuf, JoinHandle>, tokio::select! for cancellation, channel-based pipeline architecture, phase number normalization]

key-files:
  created:
    - src/watcher/mod.rs
    - src/watcher/debounce.rs
    - src/watcher/pipeline.rs
    - src/watcher/retention.rs
    - tests/watcher_integration.rs
  modified:
    - src/lib.rs
    - src/state.rs
    - src/main.rs
    - src/api/projects.rs
    - tests/api_integration.rs
    - Cargo.toml

key-decisions:
  - "Per-file debounce using HashMap<PathBuf, JoinHandle> with abort on new events -- simple and correct"
  - "75ms default delay (midpoint of user's 50-100ms range)"
  - "Phase number normalization: ROADMAP '1' -> filesystem '01' for consistent DB keys"
  - "bootstrap_project is reusable for both registration and startup reconciliation"
  - "Retention pruning only deletes resolved parse errors (unresolved kept regardless of age)"
  - "MOVED_TO events treated as Create for atomic write support (rename-based file saves)"

patterns-established:
  - "Channel pipeline: raw events -> debouncer -> pipeline -> broadcast"
  - "File classification: regex-based filename matching for parser routing"
  - "Last-known-good: on parse error, record error in DB but do not overwrite existing good state"
  - "Phase stage re-derivation triggered by any file change in a phase directory"
  - "Startup reconciliation pattern: query all projects, check paths, bootstrap, add watches"

requirements-completed: [STATE-01, STATE-07]

duration: 14min
completed: 2026-03-06
---

# Phase 01 Plan 03: File Watcher and Parse Pipeline Summary

**File watcher with per-file debouncing, parse pipeline routing 8 file types to DB, startup reconciliation for offline changes, and retention pruning -- 105 tests passing**

## Performance

- **Duration:** 14 min
- **Started:** 2026-03-06T20:10:17Z
- **Completed:** 2026-03-06T20:24:26Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments
- Complete file watcher module with per-project notify watchers, custom per-file debouncer, and event filtering (Create/Modify/Remove + MOVED_TO as Create)
- Parse pipeline that routes file events to 8 parser types, persists results to SQLite, handles parse errors with last-known-good state preservation, and re-derives phase stage after changes
- bootstrap_project function for full .planning/ directory tree re-parse, used by both project registration and startup reconciliation
- Startup reconciliation on daemon restart: queries all registered projects, checks path existence (marks offline if missing), bootstraps all live projects, adds file watchers
- Retention pruning task: periodic hard-delete of expired execution_runs, agent_sessions, and resolved parse_errors per project-configurable retention_days (default 180)
- StateUpdate/StateChange types ready for WebSocket broadcast in Plan 01-04
- Graceful shutdown via CancellationToken for debouncer, pipeline, and retention tasks
- 16 unit tests for watcher/debouncer + 8 integration tests proving end-to-end behavior

## Task Commits

Each task was committed atomically:

1. **Task 1: File watcher with custom per-file debouncer** - `95a83b5` (feat)
2. **Task 2: Parse pipeline, startup reconciliation, retention pruning, and integration tests** - `1fe8d24` (feat)

## Files Created/Modified
- `src/watcher/mod.rs` - FileWatcher struct with per-project notify watchers, FileEvent/FileEventKind types, event filtering
- `src/watcher/debounce.rs` - Custom Debouncer with per-file timers, DebouncedEvent type, spawn() helper
- `src/watcher/pipeline.rs` - Parse pipeline (run_pipeline, bootstrap_project), StateUpdate/StateChange types, file classification, per-type handlers
- `src/watcher/retention.rs` - Retention pruning task (run_retention_pruning), prune_expired_runs/sessions/errors helpers
- `src/lib.rs` - Added watcher module declaration
- `src/state.rs` - Added broadcast_tx and file_event_tx to AppState
- `src/main.rs` - Full startup sequence with channels, task spawning, reconciliation, graceful shutdown
- `src/api/projects.rs` - Project creation now triggers bootstrap_project and adds file watcher
- `Cargo.toml` - Added tokio test-util dev-dependency
- `tests/api_integration.rs` - Updated for new AppState fields
- `tests/watcher_integration.rs` - 8 integration tests for pipeline, bootstrap, reconciliation, retention, broadcast

## Decisions Made
- Per-file debounce using HashMap<PathBuf, JoinHandle> with abort on new events: simple, correct, and avoids complex batching logic
- 75ms default delay as midpoint of user's 50-100ms range per CONTEXT.md discretion
- Phase number normalization (ROADMAP "1" -> filesystem "01"): ensures consistent UNIQUE keys in phase_state table across ROADMAP and file-based data
- bootstrap_project designed as reusable function for both initial registration and startup reconciliation
- Retention pruning only deletes resolved parse errors (keeps unresolved regardless of age, since those indicate ongoing issues)
- MOVED_TO events treated as Create for atomic write support (many editors save via write-to-temp + rename)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added tokio test-util dev-dependency**
- **Found during:** Task 1 (debouncer tests)
- **Issue:** tokio "full" feature does NOT include test-util; tokio::time::advance and start_paused unavailable
- **Fix:** Added `tokio = { version = "1", features = ["full", "test-util"] }` to dev-dependencies
- **Files modified:** Cargo.toml
- **Verification:** Debouncer tests compile and pass with deterministic timing
- **Committed in:** 95a83b5 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed phase number mismatch between ROADMAP and filesystem**
- **Found during:** Task 2 (stage derivation integration test)
- **Issue:** ROADMAP parser stores phase number "1" but filesystem uses "01", causing DB key mismatch and stage updates not matching
- **Fix:** Added normalize_phase_number() helper that zero-pads single-digit phase numbers ("1" -> "01")
- **Files modified:** src/watcher/pipeline.rs
- **Verification:** test_stage_derivation_from_files integration test passes
- **Committed in:** 1fe8d24 (Task 2 commit)

**3. [Rule 3 - Blocking] Updated API integration tests for new AppState fields**
- **Found during:** Task 2 (running full test suite)
- **Issue:** Existing API integration tests construct AppState without new broadcast_tx and file_event_tx fields
- **Fix:** Added channel creation in test setup for the new AppState fields
- **Files modified:** tests/api_integration.rs
- **Verification:** All 7 API integration tests still pass
- **Committed in:** 1fe8d24 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking dependency, 1 bug, 1 blocking test fix)
**Impact on plan:** All fixes necessary for correctness and test compilation. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Parse pipeline complete and ready for WebSocket real-time broadcast (Plan 01-04)
- StateUpdate types defined for WebSocket serialization
- broadcast::Sender available in AppState for WebSocket handlers
- All background tasks have graceful shutdown via CancellationToken
- 105 tests passing as baseline (90 unit + 7 API integration + 8 watcher integration)

## Self-Check: PASSED

All created files verified present. Both task commits verified in git log.

---
*Phase: 01-backend-foundation-and-state-pipeline*
*Completed: 2026-03-06*
