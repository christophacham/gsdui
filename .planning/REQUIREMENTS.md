# Requirements: GSD Pipeline UI

**Defined:** 2026-03-06
**Core Value:** Users can see what GSD is doing across all their projects in real-time and interact with running agent sessions from the browser

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Pipeline Visualization

- [ ] **PIPE-01**: User can view horizontal phase timeline showing all phases with status badges, names, and progress bars
- [ ] **PIPE-02**: User can click a phase to see the 4-stage detail rail (Discuss → Plan → Execute → Verify) with current position
- [ ] **PIPE-03**: User can see wave swim lanes within the execute stage, with plan cards arranged by dependency
- [ ] **PIPE-04**: Plan cards show agent badge, status (queued/working/done/failed), wave number, step progress, and commit stats
- [ ] **PIPE-05**: User can expand plan card output to see live agent stream or jump to Console tab
- [ ] **PIPE-06**: Plan cards show links to view PLAN.md, diff, and SUMMARY.md
- [ ] **PIPE-07**: Phase chips show decimal phase numbers (2.1, 2.2) sorted numerically
- [ ] **PIPE-08**: Phases group by milestone when multiple milestones exist, with completed milestones collapsible
- [ ] **PIPE-09**: Dependency arrows show between plans within wave visualization
- [ ] **PIPE-10**: Agent routing configuration UI with project default → stage override → plan override hierarchy
- [ ] **PIPE-11**: Pipeline updates in real-time via WebSocket without page refresh
- [ ] **PIPE-12**: Duration and timing display on phase chips and plan cards

### Console / Terminal

- [ ] **TERM-01**: User can open interactive PTY terminal sessions in the browser with full ANSI color rendering
- [ ] **TERM-02**: Terminal has scrollback buffer with auto-scroll behavior (scroll-to-bottom on new output unless user has scrolled up)
- [ ] **TERM-03**: Terminal handles resize events (responsive to window/panel size changes)
- [ ] **TERM-04**: Console tab supports multiple sub-tabs, each an independent terminal session
- [ ] **TERM-05**: Agent sessions spawned by GSD execution auto-create new Console sub-tabs
- [ ] **TERM-06**: User can create new plain shell sessions via "+" button
- [ ] **TERM-07**: Sub-tabs persist across page reloads (sessions live in daemon)
- [ ] **TERM-08**: Closing a sub-tab kills the PTY (with confirmation if agent is running)
- [ ] **TERM-09**: Flow control prevents output flooding from crashing the browser (watermark-based pause/resume)
- [ ] **TERM-10**: Binary WebSocket frames for PTY output, text frames for control messages

### System Monitoring

- [ ] **SYS-01**: System tab shows host CPU usage (current and short history)
- [ ] **SYS-02**: System tab shows host memory usage (used/total)
- [ ] **SYS-03**: System tab shows host disk usage for relevant volumes
- [ ] **SYS-04**: Service health indicators show daemon status and connection state
- [ ] **SYS-05**: Scrolling log viewer for daemon logs with auto-scroll

### State Pipeline

- [ ] **STATE-01**: Daemon watches `.planning/` directories recursively via inotify with debouncing (50-100ms)
- [ ] **STATE-02**: Parser extracts state from STATE.md frontmatter (status, progress, milestone, timestamps)
- [ ] **STATE-03**: Parser extracts phase list from ROADMAP.md (names, goals, dependencies, completion)
- [ ] **STATE-04**: Parser extracts plan metadata from PLAN.md frontmatter (wave, depends_on, type, requirements)
- [ ] **STATE-05**: Parser extracts completion data from SUMMARY.md frontmatter (duration, key-files, requirements-completed)
- [ ] **STATE-06**: Parser extracts commit counts from SUMMARY.md body (Task Commits section)
- [ ] **STATE-07**: Parser derives stage per phase from file presence (CONTEXT.md, RESEARCH.md, PLAN.md, SUMMARY.md, VERIFICATION.md)
- [ ] **STATE-08**: Parser extracts verification status from VERIFICATION.md frontmatter (passed/gaps_found/human_needed, score)
- [ ] **STATE-09**: Parser reads config.json for workflow settings and agent routing defaults
- [ ] **STATE-10**: Parser reads agent-history.json for agent execution timeline
- [ ] **STATE-11**: SQLite database caches current parsed state for fast queries and page loads
- [ ] **STATE-12**: Database stores historical execution metrics (durations, timelines) that persist across file changes
- [ ] **STATE-13**: WebSocket endpoint pushes state updates with snapshot + delta protocol
- [ ] **STATE-14**: WebSocket reconnection restores full state via snapshot (no stale UI after network blip)
- [ ] **STATE-15**: REST API serves project list, current state, and historical metrics

### Infrastructure

- [ ] **INFRA-01**: Axum daemon serves SvelteKit static assets, REST API, and WebSocket endpoints from single binary
- [ ] **INFRA-02**: Caddy reverse proxy with automatic HTTPS and forward_auth to oauth2-proxy
- [ ] **INFRA-03**: Per-user project isolation — users only see and access their own registered projects
- [ ] **INFRA-04**: User can register project folders (directories with `.planning/`)
- [ ] **INFRA-05**: PTY processes managed server-side with heartbeat-based reaping for disconnected sessions
- [ ] **INFRA-06**: Daemon handles graceful shutdown (cleanup PTY processes, close WebSocket connections)

### Theme / UI

- [ ] **UI-01**: Dark navy Aurora theme with CSS custom properties (tokens from non-planar project)
- [ ] **UI-02**: Color palette uses #040814 → #0a1224 → #0f1a31 → #162543 background hierarchy
- [ ] **UI-03**: Agent badges with distinct colors per agent type (Claude, Codex, Gemini)
- [ ] **UI-04**: Top-level tab navigation: Pipeline, Console, System
- [ ] **UI-05**: Project sidebar with selection indicator and active execution badge
- [ ] **UI-06**: Skeleton loading states while WebSocket connection establishes
- [ ] **UI-07**: Visible reconnecting indicator when WebSocket drops

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Analytics

- **ANLY-01**: Historical execution analytics with charts and trends
- **ANLY-02**: Phase velocity tracking over time
- **ANLY-03**: Agent performance comparison metrics

### Extended Features

- **EXT-01**: Log persistence with full-text search
- **EXT-02**: Webhook endpoint for external notifications
- **EXT-03**: Mobile-responsive pipeline status view
- **EXT-04**: Export/share execution reports
- **EXT-05**: PROJECT.md / REQUIREMENTS.md visualization panels
- **EXT-06**: Todo sidebar from .planning/todos/pending/
- **EXT-07**: Nyquist/UAT visualization panels

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Pipeline editing/orchestration from UI | Violates observe-only architectural constraint |
| Drag-and-drop pipeline builder | Markdown round-trip complexity, not the interaction model |
| Real-time collaborative editing | Conflicts with running agents |
| Embedded code editor | Scope explosion — terminal is sufficient |
| Plugin/extension system | Premature before core is stable |
| RBAC / role-based access | Per-user isolation is sufficient for small team |
| Mobile native app | Web-first, responsive deferred to v2 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| (populated during roadmap creation) | | |

**Coverage:**
- v1 requirements: 50 total
- Mapped to phases: 0
- Unmapped: 50

---
*Requirements defined: 2026-03-06*
*Last updated: 2026-03-06 after initial definition*
