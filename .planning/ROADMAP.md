# Roadmap: GSD Pipeline UI

## Overview

GSD Pipeline UI delivers a browser-based dashboard for observing and interacting with GSD agent pipelines in real-time. The roadmap follows the data flow: build the backend state engine first (file watching, parsing, database, WebSocket), then the pipeline visualization frontend that consumes it, then the interactive terminal system that provides bidirectional agent interaction, and finally multi-user isolation, system monitoring, and production hardening. Four phases, each delivering a coherent vertical capability.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Backend Foundation and State Pipeline** - Axum daemon, file watcher, GSD state parsers, SQLite cache, WebSocket state endpoint, REST API
- [ ] **Phase 2: Pipeline Dashboard** - Pipeline visualization frontend with phase timeline, stage rails, plan cards, Aurora theme, and live WebSocket updates
- [ ] **Phase 3: Interactive Terminal System** - Browser-based PTY terminals via xterm.js with Console tab, sub-tabs, flow control, and agent session integration
- [ ] **Phase 4: Multi-User, System Monitoring, and Production** - Auth via reverse proxy, per-user project isolation, system metrics tab, service health, deployment packaging

## Phase Details

### Phase 1: Backend Foundation and State Pipeline
**Goal**: The daemon can watch any registered project's `.planning/` directory, parse all GSD state files into a SQLite database, and push live state updates to connected WebSocket clients
**Depends on**: Nothing (first phase)
**Requirements**: STATE-01, STATE-02, STATE-03, STATE-04, STATE-05, STATE-06, STATE-07, STATE-08, STATE-09, STATE-10, STATE-11, STATE-12, STATE-13, STATE-14, STATE-15, INFRA-01
**Success Criteria** (what must be TRUE):
  1. Daemon starts, watches a project's `.planning/` directory, and detects file changes within 100ms of write completion
  2. Parser correctly derives pipeline state from GSD files (STATE.md, ROADMAP.md, PLAN.md, SUMMARY.md, VERIFICATION.md, config.json, agent-history.json) and stores it in SQLite
  3. A WebSocket client connecting to `/api/ws/state` receives a full state snapshot immediately, then receives delta updates as files change
  4. WebSocket client that disconnects and reconnects receives a fresh snapshot with no stale data
  5. REST API returns project list and current parsed state for any registered project
**Plans:** 4 plans

Plans:
- [x] 01-01-PLAN.md -- Rust project scaffold, Cargo.toml, SQLite schema, database layer, project CRUD REST API, health endpoint
- [x] 01-02-PLAN.md -- GSD state file parsers (frontmatter extractor, 7 file-type parsers, stage derivation) with TDD
- [ ] 01-03-PLAN.md -- File watcher with custom per-file debouncer, parse pipeline connecting watcher to parsers to database
- [ ] 01-04-PLAN.md -- WebSocket state endpoint (snapshot + delta), broadcaster, REST API for state/history/files

### Phase 2: Pipeline Dashboard
**Goal**: Users can see the full GSD pipeline status for any project in real-time -- phase timeline, stage progression, plan cards with agent and status detail -- rendered in the Aurora dark navy theme
**Depends on**: Phase 1
**Requirements**: PIPE-01, PIPE-02, PIPE-03, PIPE-04, PIPE-05, PIPE-06, PIPE-07, PIPE-08, PIPE-09, PIPE-10, PIPE-11, PIPE-12, UI-01, UI-02, UI-03, UI-04, UI-05, UI-06, UI-07
**Success Criteria** (what must be TRUE):
  1. User sees a horizontal phase timeline with status badges, progress bars, decimal phase numbers sorted numerically, and milestone grouping -- all updating live without page refresh
  2. User clicks a phase and sees the 4-stage rail (Discuss/Plan/Execute/Verify) with the current stage highlighted, plus wave swim lanes with plan cards showing agent, status, wave number, progress, commits, and duration
  3. User can expand a plan card to see live output or jump to Console, and can view links to PLAN.md, diff, and SUMMARY.md
  4. User can configure agent routing (project default, stage override, plan override) from the Pipeline tab
  5. The entire UI uses the Aurora dark navy theme with correct background hierarchy, agent-colored badges, skeleton loading states, and a visible reconnecting indicator on WebSocket drop
**Plans**: TBD

Plans:
- [ ] 02-01: SvelteKit project scaffold, Aurora theme tokens, layout shell (tab navigation, project sidebar)
- [ ] 02-02: WebSocket client store, phase timeline, phase detail with stage rail
- [ ] 02-03: Wave visualization, plan cards, agent routing UI, dependency arrows

### Phase 3: Interactive Terminal System
**Goal**: Users can open interactive terminal sessions in the browser, run GSD commands, respond to agent prompts, and manage multiple terminal tabs -- all with full PTY fidelity
**Depends on**: Phase 1
**Requirements**: TERM-01, TERM-02, TERM-03, TERM-04, TERM-05, TERM-06, TERM-07, TERM-08, TERM-09, TERM-10
**Success Criteria** (what must be TRUE):
  1. User can open a terminal in the Console tab that provides a full interactive PTY session with ANSI color rendering, scrollback buffer, and auto-scroll behavior
  2. User can create multiple terminal sub-tabs, close them (with confirmation if an agent is running), and sub-tabs persist across page reloads
  3. Agent sessions spawned by GSD execution automatically create new Console sub-tabs
  4. Terminal handles resize events, and flow control prevents output flooding from crashing the browser
  5. Binary WebSocket frames carry PTY output while text frames carry control messages, with heartbeat-based reaping for disconnected sessions
**Plans**: TBD

Plans:
- [ ] 03-01: PTY manager (spawn, lifecycle, resize, reaping) and binary WebSocket `/api/ws/pty` endpoint
- [ ] 03-02: xterm.js Console tab with sub-tabs, flow control, agent session auto-creation

### Phase 4: Multi-User, System Monitoring, and Production
**Goal**: The dashboard is production-ready with authenticated multi-user access, per-user project isolation, host system monitoring, and clean deployment
**Depends on**: Phase 2, Phase 3
**Requirements**: INFRA-02, INFRA-03, INFRA-04, INFRA-05, INFRA-06, SYS-01, SYS-02, SYS-03, SYS-04, SYS-05
**Success Criteria** (what must be TRUE):
  1. Users authenticate via OAuth2 through the reverse proxy and only see their own registered projects
  2. User can register project folders and the daemon begins watching them immediately
  3. System tab shows live host CPU, memory, and disk usage, plus daemon service health indicators
  4. Daemon log viewer shows scrolling log output with auto-scroll in the System tab
  5. Daemon handles graceful shutdown (PTY cleanup, WebSocket close) and PTY sessions use heartbeat-based reaping
**Plans**: TBD

Plans:
- [ ] 04-01: Caddy + oauth2-proxy integration, per-user isolation, project registration UI
- [ ] 04-02: System tab (host metrics, service health, log viewer), graceful shutdown, PTY reaping

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4
Note: Phase 3 depends only on Phase 1, so it can begin after Phase 1 completes (parallel with Phase 2 if desired).

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Backend Foundation and State Pipeline | 4/4 | Complete | 2026-03-06 |
| 2. Pipeline Dashboard | 0/3 | Not started | - |
| 3. Interactive Terminal System | 0/2 | Not started | - |
| 4. Multi-User, System Monitoring, and Production | 0/2 | Not started | - |
