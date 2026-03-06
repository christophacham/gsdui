---
phase: 01-backend-foundation-and-state-pipeline
plan: 04
subsystem: api, websocket, broadcast
tags: [rust, axum, websocket, rest-api, broadcast, futures-util, tokio-tungstenite, urlencoding]

requires:
  - phase: 01-01
    provides: "SQLite database with query layer, AppState, Project CRUD API"
  - phase: 01-02
    provides: "9 parsers for all GSD file formats, ParseError enum"
  - phase: 01-03
    provides: "File watcher, parse pipeline, StateUpdate/StateChange types, broadcast channel"
provides:
  - "WebSocket endpoint at /api/v1/ws/state with snapshot+delta protocol"
  - "Per-project Broadcaster for subscription-based real-time updates"
  - "REST API: project state, phase list, plan list, execution history with filtering/pagination"
  - "REST API: agent sessions with filtering, raw file content with path traversal prevention"
  - "Health heartbeats over WebSocket every 7 seconds"
  - "Comprehensive health diagnostics (memory, per-project watcher status, parse error counts)"
  - "Full API surface under /api/v1/ prefix ready for Phase 2 frontend"
affects: [02-01, 02-02, 02-03]

tech-stack:
  added: [futures-util 0.3, tokio-tungstenite 0.26, urlencoding 2]
  patterns: [WebSocket snapshot+delta protocol, per-project broadcast channels, dynamic SQL query building with filters, path traversal prevention via canonicalize]

key-files:
  created:
    - src/broadcast.rs
    - src/ws/mod.rs
    - src/ws/messages.rs
    - src/api/state_api.rs
    - src/api/history.rs
    - src/api/files.rs
    - tests/ws_integration.rs
  modified:
    - src/state.rs
    - src/main.rs
    - src/lib.rs
    - src/api/mod.rs
    - src/api/health.rs
    - src/db/schema.rs
    - tests/api_integration.rs
    - Cargo.toml

key-decisions:
  - "Per-project Broadcaster with RwLock<HashMap> for channel management -- simple, correct, no external dependency"
  - "WebSocket requires Subscribe as first message within 10-second timeout"
  - "Health heartbeat at 7-second interval (midpoint of user's 5-10s range)"
  - "build_project_state shared between WebSocket snapshot and REST state endpoint"
  - "Dynamic SQL query building for run/session filters to avoid N+1 queries"
  - "Path traversal prevention via both string check (contains ..) and canonicalize path comparison"
  - "Memory usage via /proc/self/status VmRSS on Linux (zero on other platforms)"

patterns-established:
  - "WebSocket lifecycle: upgrade -> wait for Subscribe -> send snapshots -> spawn delta/health/receive tasks -> select! on all"
  - "Per-project broadcast: Broadcaster.get_or_create_channel() with double-check locking"
  - "REST sub-router pattern: project_sub_router() nested at /projects/:id with state_api, history, files"
  - "Filtered query pattern: build WHERE clause dynamically, count total first, then fetch with LIMIT/OFFSET"

requirements-completed: [STATE-13, STATE-14, STATE-15, INFRA-01]

duration: 11min
completed: 2026-03-06
---

# Phase 01 Plan 04: WebSocket and REST API Summary

**WebSocket endpoint with snapshot+delta protocol, per-project broadcaster, REST API for state/history/files, and health heartbeats -- 134 tests passing**

## Performance

- **Duration:** 11 min
- **Started:** 2026-03-06T20:28:29Z
- **Completed:** 2026-03-06T20:40:12Z
- **Tasks:** 2
- **Files modified:** 17

## Accomplishments
- Complete WebSocket endpoint at /api/v1/ws/state with subscribe/snapshot/delta/health protocol
- Per-project Broadcaster enabling clients to subscribe to specific projects and receive only relevant updates
- REST API: /state (full ProjectState), /phases, /phases/:phase/plans, /errors endpoints
- REST API: /history/runs (date/status/pagination filtering), /history/runs/:id/commits, /history/agents
- REST API: /files/*path returning raw file content with path traversal prevention and correct Content-Type
- Comprehensive health diagnostics: memory usage, per-project watcher status, parse error counts
- Broadcast forwarder task bridging pipeline broadcast channel to per-project Broadcaster

## Task Commits

Each task was committed atomically:

1. **Task 1: Broadcaster, WebSocket handler with snapshot+delta, and message types** - `8bc47bd` (feat)
2. **Task 2: REST API for state queries, historical metrics, and file content** - `290afd3` (feat)

## Files Created/Modified
- `src/broadcast.rs` - Per-project broadcast channel management with subscribe/unsubscribe/client count
- `src/ws/mod.rs` - WebSocket handler: upgrade, subscribe, snapshot, delta forwarding, health heartbeats
- `src/ws/messages.rs` - WsMessage (Snapshot/Delta/Health/Error), ClientMessage (Subscribe/Unsubscribe), ProjectState
- `src/api/state_api.rs` - REST endpoints for current project state, phases, plans, errors
- `src/api/history.rs` - REST endpoints for execution runs (filtered/paginated), commits, agent sessions
- `src/api/files.rs` - REST endpoint for raw .planning/ file content with path traversal prevention
- `src/api/health.rs` - Updated with memory usage, per-project watcher status, parse error counts
- `src/api/mod.rs` - Router updated with nested project sub-routes
- `src/db/schema.rs` - Added get_runs_filtered, get_sessions_filtered, get_run_by_id, get_parse_error_counts
- `src/state.rs` - AppState now includes Broadcaster field
- `src/main.rs` - WebSocket route, Broadcaster wiring, broadcast forwarder task
- `src/lib.rs` - Added broadcast and ws module declarations
- `tests/ws_integration.rs` - 6 WebSocket integration tests
- `tests/api_integration.rs` - Extended to 24 tests (17 new for state/history/files/health)
- `Cargo.toml` - Added futures-util, tokio-tungstenite, urlencoding

## Decisions Made
- Per-project Broadcaster with RwLock<HashMap> for channel management: simple, efficient, no external dependency needed
- WebSocket protocol requires Subscribe as first message within 10-second timeout (prevents idle connections)
- Health heartbeat interval at 7 seconds (midpoint of user's 5-10 second range per CONTEXT.md)
- build_project_state() is a public function shared between WebSocket snapshot and REST /state endpoint (DRY)
- Dynamic SQL query building for filtered runs/sessions using positional bind parameters
- Dual path traversal prevention: string check for `..` AND canonicalize path comparison against .planning/ root
- Memory usage from /proc/self/status VmRSS on Linux, returns 0 on other platforms (graceful degradation)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Rust 2024 edition pattern matching in broadcast test**
- **Found during:** Task 1 (unit tests)
- **Issue:** Rust 2024 edition disallows explicit `ref mut` in implicitly-borrowing patterns
- **Fix:** Removed `ref mut` from pattern, used `&mut receivers[0]` instead
- **Files modified:** src/broadcast.rs
- **Verification:** Unit tests compile and pass
- **Committed in:** 8bc47bd (Task 1 commit)

**2. [Rule 1 - Bug] Fixed ownership issue with URL-decoded file path**
- **Found during:** Task 2 (files.rs compilation)
- **Issue:** `urlencoding::decode` consumes `file_path` via `.into()`, then later closures try to borrow it
- **Fix:** Used `.into_owned()` and `.clone()` to avoid move-after-borrow
- **Files modified:** src/api/files.rs
- **Verification:** Compiles without errors
- **Committed in:** 290afd3 (Task 2 commit)

**3. [Rule 1 - Bug] Fixed path traversal test for URL normalization behavior**
- **Found during:** Task 2 (integration tests)
- **Issue:** HTTP clients resolve `..` in URLs before sending, so raw `../../etc/passwd` never reaches handler
- **Fix:** URL-encoded the `..` components (`%2F..%2F..`) to bypass client-side normalization and test server-side validation
- **Files modified:** tests/api_integration.rs
- **Verification:** test_files_endpoint_path_traversal_blocked passes with 403
- **Committed in:** 290afd3 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (3 bugs)
**Impact on plan:** All fixes necessary for correct compilation and testing. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Complete Phase 1 API surface ready for Phase 2 (Pipeline Dashboard):
  - WebSocket at /api/v1/ws/state for live updates
  - REST API for state, history, files at /api/v1/projects/:id/...
  - Health diagnostics at /api/v1/health
- Single binary serves static files, REST API, and WebSocket from one port (INFRA-01 complete)
- All 134 tests passing (96 unit + 24 API integration + 8 watcher integration + 6 WebSocket integration)
- Phase 1 complete: database, parsers, file watcher, parse pipeline, WebSocket, REST API all operational

## Self-Check: PASSED

All created files verified present. Both task commits verified in git log.

---
*Phase: 01-backend-foundation-and-state-pipeline*
*Completed: 2026-03-06*
