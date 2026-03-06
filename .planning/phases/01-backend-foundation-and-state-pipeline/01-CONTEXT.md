# Phase 1: Backend Foundation and State Pipeline - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Axum daemon that watches any registered project's `.planning/` directory, parses all GSD state files into a SQLite database, and pushes live state updates to connected WebSocket clients. Also serves a REST API for project management, current state, file content, and historical queries. No frontend UI in this phase.

</domain>

<decisions>
## Implementation Decisions

### Parsing Resilience
- Mid-write files: use last-known-good state until new write is complete and parseable (never show partial/broken data)
- Unknown frontmatter fields or unexpected structure: treat as parse error and surface to user as a warning badge on the project (strict mode)
- Missing critical files (no ROADMAP.md, STATE.md deleted): show project in sidebar with 'incomplete' state badge, parse whatever is available
- Unparseable files (corrupted, wrong encoding, invalid YAML): surface as file-level error badge on the specific phase/plan, log error in daemon

### Historical Data Depth
- Plan-level timing: store start/end timestamps and duration per plan (from SUMMARY.md frontmatter)
- Full agent timeline: record every agent switch/handoff within a plan, not just the final agent (parsed from agent-history.json)
- Keep all execution runs: every re-execution attempt stored with timestamps, old runs marked as superseded
- Full commit details: parse individual commit hashes and messages from SUMMARY.md body, not just counts

### Bootstrap Behavior
- First watch of existing project: full reconstruction of all history from .planning/ files (completed phases, durations, verification results)
- Daemon restart: reconcile database against current files on startup, update any differences from changes made while daemon was down
- New project registration: immediate watch + full initial parse, project appears in UI within seconds
- Lost project directory: mark as 'offline' with badge in sidebar, preserve all historical data in DB, periodically check if directory reappears

### State Update Granularity
- WebSocket push model: targeted deltas only (e.g., 'phase 2 stage moved to Execute', 'plan 02-01 status: working'), frontend applies patches to local state
- Rapid file changes: debounce + batch over 50-100ms window, send one batched update per burst
- Subscription model: per-project subscriptions, client sends subscribe message for project(s) of interest
- Initial snapshot: current state + recent execution history (not just latest state)

### Data Retention
- Configurable retention period with 180-day default
- Global default with per-project override capability
- Pruning action: hard delete from SQLite (original .planning/ files remain on disk as backup)

### REST API Surface
- Serve file content via API: REST endpoints return raw markdown for PLAN.md, SUMMARY.md, VERIFICATION.md (required for remote browser access)
- Historical queries: time range + filters (date range, phase, plan, agent, status) via standard REST query params
- Full CRUD for project management: register, unregister, configure projects through REST endpoints
- API versioning from day one: /api/v1/ prefix on all routes

### Daemon Health Reporting
- Comprehensive diagnostics: daemon uptime, memory usage, DB size, watcher event queue depth, WebSocket client count, per-project watcher status, parse error counts
- Dual channel: periodic health heartbeats over WebSocket (every 5-10s) for live display + REST /api/v1/health endpoint for monitoring tools
- Parse errors: accumulated with timestamps in database, queryable via REST, users can see error patterns over time
- Structured log stream via dedicated WebSocket channel for the daemon log viewer (SYS-05)

### Claude's Discretion
- Debounce timing within the 50-100ms range specified in STATE-01
- SQLite schema design and migration strategy
- Internal file watcher implementation details (inotify event handling)
- WebSocket message format (JSON structure for deltas, snapshots, health heartbeats)
- Log format and verbosity levels
- Retention pruning schedule (how often the cleanup job runs)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- None -- greenfield project, no existing code

### Established Patterns
- None yet -- this phase establishes the foundational patterns

### Integration Points
- `.planning/` directory structure is the input contract (defined by GSD tooling)
- GSD file formats: STATE.md, ROADMAP.md, PLAN.md, SUMMARY.md, VERIFICATION.md, config.json, agent-history.json
- Phase 2 (Pipeline Dashboard) will consume the WebSocket and REST API defined here
- Phase 3 (Terminal System) will add PTY WebSocket endpoints alongside the state endpoints
- Phase 4 (Multi-User) will add auth middleware to the API layer

</code_context>

<specifics>
## Specific Ideas

- Strict parsing: user explicitly chose strict error flagging over lenient/silent approaches -- they want to know when something is wrong rather than silently degrading
- Full history preservation: user consistently chose the richest data option (full agent timeline, full commit details, keep all runs) -- build the schema for comprehensive data capture
- Comprehensive health: user wants full operational visibility into daemon internals, not just basic connection state

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 01-backend-foundation-and-state-pipeline*
*Context gathered: 2026-03-06*
