# Project Research Summary

**Project:** GSD Pipeline UI
**Domain:** Real-time developer dashboard with browser terminals and AI agent session visualization
**Researched:** 2026-03-06
**Confidence:** HIGH

## Executive Summary

GSD Pipeline UI is a real-time dashboard for observing and interacting with GSD (Get Shit Done) AI agent pipelines from the browser. It combines three capabilities rarely found in one tool: pipeline visualization derived from filesystem state, interactive browser-based terminal sessions (PTY), and system monitoring. The recommended approach is a Rust daemon (Axum + tokio) that watches `.planning/` directories via inotify, parses GSD state files into a SQLite cache, and pushes updates over WebSocket to a Svelte 5 SPA. The SPA is built as static assets via SvelteKit adapter-static and served directly by the daemon -- no Node.js in production, no separate frontend server, single binary deployment. This architecture is proven by analogous tools (VS Code remote terminals, Jupyter, Proxmox) and every major technology choice has HIGH confidence from verified official sources.

The core value proposition is that users can see what their GSD agents are doing and interact with them from a browser, without opening separate terminal windows. The interactive PTY terminal is the biggest differentiator: GitHub Actions, GitLab CI, and Buildkite all show read-only logs, while GSD Pipeline UI provides full bidirectional terminal sessions. The file-based state derivation model is novel -- rather than receiving state via API webhooks, the daemon derives pipeline state from the presence and contents of files in `.planning/` directories. This means the dashboard is purely observational and cannot interfere with running agents, which is a significant safety property.

The primary risks are: file watcher race conditions corrupting parsed state (mitigated by debouncing and parse validation), PTY process leaks from dropped WebSocket connections (mitigated by server-side reaping and heartbeats), output flooding from high-throughput commands crashing the browser (mitigated by watermark-based flow control), and WebSocket reconnection losing state (mitigated by snapshot-plus-delta protocol design). All four risks are well-understood, have documented prevention strategies, and must be addressed in their respective implementation phases rather than deferred.

## Key Findings

### Recommended Stack

The stack is Rust on the backend and Svelte 5 on the frontend, connected by WebSocket. Rust was chosen for memory safety without GC, async performance for PTY multiplexing and WebSocket fan-out, and single-binary deployment. Svelte 5 was chosen for compiled reactivity (no virtual DOM overhead on frequent WebSocket state updates), small bundle size, and the runes system ($state, $derived, $effect) which maps cleanly to WebSocket-driven state management. No Node.js runs in production.

**Core technologies:**
- **Rust + Axum 0.8 + Tokio**: HTTP/WebSocket server with built-in WebSocket support, tower middleware ecosystem, async-native
- **Svelte 5 + SvelteKit (adapter-static)**: Compiled SPA with file-based routing, built to static assets served by Axum
- **SQLite via SQLx 0.8**: Async, compile-time checked queries, embedded in the binary, zero external dependencies
- **pty-process 0.5**: Async PTY spawning with tokio AsyncRead/AsyncWrite, direct integration with WebSocket bridge
- **notify 8 + notify-debouncer-mini**: inotify-based file watching with event debouncing for GSD's rapid multi-file writes
- **@xterm/xterm 6.0**: Browser terminal emulation with WebGL rendering, ANSI color support, fit addon for responsive sizing
- **Caddy 2 + oauth2-proxy 7**: Reverse proxy with automatic HTTPS and forward_auth for OAuth2 authentication

**Version note:** The gray_matter crate for frontmatter extraction has LOW confidence -- may need a simple regex-based extractor instead. xterm.js addon versions need verification against @xterm/xterm 6.0 compatibility.

### Expected Features

**Must have (table stakes):**
- Pipeline list view with status badges and timing (every CI tool has this)
- Phase detail with 4-stage rail (Discuss/Plan/Execute/Verify)
- Interactive PTY terminal with ANSI color rendering (core value proposition)
- WebSocket live updates without page refresh
- File watcher + state parser for GSD files (the engine that drives everything)
- Database state cache for fast queries and page loads
- Color-coded status indicators, scrollback buffer, auto-scroll

**Should have (differentiators):**
- Multiple terminal tabs with agent session auto-creation
- Wave/plan card visualization within stage rails
- Multi-project sidebar with project registration
- Per-user project isolation with auth integration
- System tab with host metrics (CPU/Memory/Disk)
- Multi-agent routing visualization

**Defer (v2+):**
- Historical execution analytics (charts, trends, velocity)
- Webhook endpoint for external notifications
- Log persistence and full-text search
- Mobile-responsive pipeline status
- Export/share execution reports

**Anti-features (explicitly avoid):**
- Pipeline editing/orchestration from UI (violates observe-only constraint)
- Drag-and-drop pipeline builder (markdown round-trip complexity)
- Real-time collaborative editing (conflicts with running agents)
- Embedded code editor (scope explosion)
- Plugin/extension system (premature before core is stable)
- RBAC (personal productivity tool, not team platform)

### Architecture Approach

Three-layer architecture: Caddy (TLS + auth) -> Axum daemon (serves everything) -> SvelteKit SPA (static assets). The daemon is a single process with actor-style components communicating via tokio::mpsc channels: FileWatcher (notify + debounce) -> Parser (state derivation) -> SQLite (persistence) -> Broadcaster (tokio::broadcast fan-out) -> WebSocket clients. PTY sessions use separate WebSocket connections with binary frames. Two distinct WebSocket endpoints: `/api/ws/state` for JSON pipeline updates (snapshot + delta protocol) and `/api/ws/pty` for binary terminal I/O (one connection per terminal tab).

**Major components:**
1. **FileWatcher** -- watches `.planning/` recursively via inotify, debounces events, dispatches to Parser
2. **Parser** -- stateless module that reads GSD files (STATE.md, ROADMAP.md, PLAN.md, etc.) and derives pipeline state
3. **SQLite store** -- persists parsed state, historical metrics, user/project registrations (WAL mode, single writer)
4. **Broadcaster** -- tokio::broadcast channel per project for WebSocket fan-out
5. **PTYManager** -- actor-style component that spawns PTY processes, manages lifecycle, streams I/O to WebSocket
6. **SvelteKit SPA** -- class-based stores with $state runes, WebSocket client with reconnection, xterm.js terminal wrapper

### Critical Pitfalls

1. **File watcher race conditions** -- inotify fires mid-write, parser reads half-written files. Mitigate with 50-100ms debounce, parse validation (discard null/empty results), and handling CLOSE_WRITE + MOVED_TO events. Must be solid in Phase 1.

2. **PTY process leaks** -- WebSocket drops leave orphaned PTY processes consuming PIDs and memory. Mitigate with heartbeat-based reaping (kill after 3 missed pongs), session inactivity timeout, process group cleanup (kill -pgid), per-user PTY limits, and orphan cleanup on daemon restart.

3. **Output flooding crashes browser** -- Commands like `find /` produce output faster than xterm.js can render. Mitigate with watermark-based flow control (pause/resume between client and server), server-side rate limiting (flush at most every 16ms), and bounded WebSocket send buffers.

4. **WebSocket reconnection loses state** -- Network blip causes stale UI with no indication. Mitigate by designing snapshot-plus-delta protocol from day one: full snapshot on connect/reconnect, monotonic sequence numbers, replay buffer for short disconnections, visible reconnecting indicator.

5. **Database as bottleneck in hot path** -- Every file change triggers synchronous DB write, SQLite serializes all writes. Mitigate by batching writes (50-100ms), decoupling WebSocket broadcasts from DB writes (broadcast from in-memory parsed state, write to DB asynchronously), and keeping read transactions short-lived.

## Implications for Roadmap

Based on research, the architecture has clear dependency chains that dictate phase ordering. The suggested structure follows the data flow: foundation first, then the state pipeline that drives everything, then the frontend that consumes it, then the terminal system which is partially independent, and finally polish and production hardening.

### Phase 1: Project Foundation

**Rationale:** Every component depends on the database schema, Axum server skeleton, and SvelteKit project structure. Nothing can be built without this scaffolding.
**Delivers:** Compilable Rust daemon with Axum routing, SQLite schema with migrations, SvelteKit project with adapter-static, Aurora dark navy theme tokens, development tooling (cargo-watch, just tasks).
**Addresses:** Database state cache (P1), project structure
**Avoids:** Database bottleneck pitfall (design batched writes and async persistence from the start)

### Phase 2: File Watching and State Pipeline

**Rationale:** The file watcher and parser are the engine that drives the entire dashboard. Pipeline views, status updates, and agent detection all depend on correct file-to-state derivation. This must be built and validated before any meaningful frontend work.
**Delivers:** FileWatcher with inotify + debouncing, Parser for all GSD file formats (STATE.md, ROADMAP.md, PLAN.md, config.json, agent-history.json), SQLite read/write layer, Broadcaster with tokio::broadcast, WebSocket `/api/ws/state` endpoint with snapshot + delta protocol, REST API for projects and state queries.
**Addresses:** File watcher + state parser (P1), WebSocket live updates (P1), connection status/reconnection (P2)
**Avoids:** File watcher race conditions (debounce + validation), inotify watch exhaustion (check limits on startup, watch only `.planning/` paths), WebSocket reconnection state loss (snapshot + delta from day one)

### Phase 3: Pipeline Dashboard Frontend

**Rationale:** With the state pipeline delivering data over WebSocket, the frontend can now render meaningful pipeline visualizations. This is the primary interface and must precede the terminal system because it validates the core "see what GSD is doing" value proposition.
**Delivers:** WebSocket client store with reconnection, pipeline list view with status badges, phase detail with 4-stage rail (Discuss/Plan/Execute/Verify), plan card visualization, sidebar with project list, duration/timing display, color-coded status indicators, skeleton loading states.
**Addresses:** Pipeline list view (P1), phase detail/stage rail (P1), color-coded status (P1), duration/timing (P1), wave/plan cards (P2)
**Avoids:** Dark theme accessibility pitfall (test all colors against WCAG AA from the start), re-rendering performance trap (fine-grained Svelte reactivity per component)

### Phase 4: Interactive Terminal System

**Rationale:** The terminal system is the second core pillar and is architecturally independent of the pipeline frontend. PTYManager, WebSocket /pty endpoint, and xterm.js integration form their own vertical slice. This phase delivers the "interact with GSD from the browser" value proposition.
**Delivers:** PTYManager with pty-process (spawn, lifecycle, resize, cleanup), WebSocket `/api/ws/pty` endpoint with binary frames, xterm.js terminal component with FitAddon and WebGL renderer, Console tab with terminal sub-tabs, single interactive terminal session, flow control (pause/resume).
**Addresses:** Interactive PTY terminal (P1), ANSI color rendering (P1), scrollback buffer (P1), auto-scroll (P1), multiple terminal tabs (P2)
**Avoids:** PTY process leaks (heartbeat reaping, process group cleanup, per-user limits from day one), output flooding (watermark-based flow control built into the PTY-WebSocket bridge), FitAddon resize chaos (debounced resize, dimension guards, visibility checks)

### Phase 5: Multi-User, System Monitoring, and Production Hardening

**Rationale:** Auth integration, per-user isolation, system metrics, and deployment packaging are important for production use but not for validating core product value. Group these together as the final phase before launch.
**Delivers:** Caddy + oauth2-proxy integration, per-user project isolation (data-layer enforcement), System tab with host metrics (CPU/Memory/Disk), service health indicators, agent session auto-tab creation, agent routing visualization, multi-project support with project registration, deployment packaging (systemd unit, Caddyfile).
**Addresses:** Per-user isolation (P2), system metrics (P2), service health (P2), agent session auto-creation (P2), agent routing visualization (P2), search/filter projects (P2)
**Avoids:** Terminal escape sequence injection (user isolation at data layer), path traversal in project registration (validate paths), WebSocket auth bypass (validate token on upgrade)

### Phase Ordering Rationale

- **Phase 1 before all:** Database schema and server skeleton are universal dependencies.
- **Phase 2 before Phase 3:** The frontend cannot render anything meaningful without WebSocket state data. Building the state pipeline first means the frontend can be developed against real data immediately.
- **Phase 3 before Phase 4:** Pipeline visualization is the primary interface and has fewer unknowns than terminal integration. Validating the pipeline view first confirms the state pipeline works correctly end-to-end.
- **Phase 4 can partially overlap Phase 3:** The PTYManager backend is independent of the pipeline frontend components. If resources allow, PTY backend work can begin during Phase 3.
- **Phase 5 last:** Multi-user features, auth, and system monitoring are production concerns. The product can be validated in single-user mode first.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2 (State Pipeline):** The GSD file format parsing (STATE.md frontmatter, ROADMAP.md markdown structure, config.json schema) needs precise specification. Research the exact file formats GSD produces, including edge cases like partial writes and format evolution across GSD versions.
- **Phase 4 (Terminal System):** PTY-to-WebSocket flow control implementation has nuances (OS-level TTY flow control, xterm.js write callback API). Research exact xterm.js 6.0 flow control API and pty-process async pause/resume patterns.
- **Phase 5 (Multi-User):** Caddy + oauth2-proxy + WebSocket integration needs testing. Research whether oauth2-proxy handles WebSocket upgrade transparently or needs explicit configuration.

Phases with standard patterns (skip deep research):
- **Phase 1 (Foundation):** Axum server setup, SQLite migrations, SvelteKit project scaffold -- all extremely well-documented with official examples.
- **Phase 3 (Pipeline Frontend):** Svelte 5 components, WebSocket client stores, CSS custom properties -- standard frontend patterns with abundant documentation.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All core technologies verified against official sources (crates.io, npm, docs.rs, official blogs). Version numbers confirmed. Only low-confidence items: gray_matter crate (may need replacement), xterm addon version compatibility |
| Features | HIGH | Comprehensive competitor analysis across 9+ products. Feature priorities grounded in real product comparisons. Clear table-stakes vs. differentiator separation |
| Architecture | HIGH | Architecture pattern proven by VS Code remote, Jupyter, Proxmox. Component boundaries and data flows are well-defined. tokio channel-based actor pattern is standard Rust async |
| Pitfalls | HIGH | 8 critical pitfalls identified with specific prevention strategies, all backed by real-world failure reports (VS Code zombie processes, Deno file watcher race, xterm.js DCS vulnerability). Recovery costs assessed |

**Overall confidence:** HIGH

### Gaps to Address

- **GSD file format specification:** The parser must handle STATE.md, ROADMAP.md, PLAN.md, SUMMARY.md, VERIFICATION.md, config.json, and agent-history.json. Exact frontmatter schemas and markdown structures need to be documented from actual GSD output. Research the current GSD codebase for authoritative format definitions during Phase 2 planning.
- **gray_matter crate viability:** LOW confidence on the frontmatter extraction crate. May need to write a simple regex-based TOML/YAML frontmatter extractor. Validate during Phase 2 implementation.
- **xterm.js addon version compatibility:** The addon versions in STACK.md are estimates. Verify exact compatible versions for @xterm/xterm 6.0 before Phase 4 implementation.
- **pty-process vs. portable-pty:** STACK.md recommends pty-process for its tokio integration, but ARCHITECTURE.md references portable-pty in several places. Settle on one during Phase 4 planning. pty-process is the stronger recommendation given the async-native design.
- **notify-debouncer variant:** STACK.md recommends notify-debouncer-mini, ARCHITECTURE.md references notify-debouncer-full (for event type information). Settle on the right variant during Phase 2 planning -- full is likely the correct choice since the parser needs to distinguish create/modify/delete events.

## Sources

### Primary (HIGH confidence)
- [Axum 0.8 docs and announcement](https://docs.rs/axum/latest/axum/) -- WebSocket support, routing, static file serving
- [Rust 1.85 / Edition 2024](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/) -- Language version
- [Svelte 5 / SvelteKit 2 official docs](https://svelte.dev/docs/kit/adapter-static) -- SPA mode, runes, adapter-static
- [@xterm/xterm 6.0](https://www.npmjs.com/package/@xterm/xterm) -- Terminal emulation, flow control, security
- [SQLx 0.8](https://github.com/launchbadge/sqlx) -- Async SQLite driver
- [notify crate](https://docs.rs/notify/latest/notify/) -- File watching
- [pty-process](https://crates.io/crates/pty-process) -- Async PTY management
- [Caddy forward_auth](https://caddyserver.com/docs/caddyfile/directives/forward_auth) -- Auth proxy integration
- [SQLite WAL mode](https://sqlite.org/wal.html) -- Database concurrency
- [tokio::broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html) -- WebSocket fan-out

### Secondary (MEDIUM confidence)
- [Svelte 5 global state patterns (Mainmatter)](https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/) -- Class-based stores with runes
- [Real-time WebSocket with SvelteKit (Inngest)](https://www.inngest.com/blog/building-a-realtime-websocket-app-using-sveltekit) -- WebSocket client patterns
- [Rust state machine patterns (Hoverbear)](https://hoverbear.org/blog/rust-state-machine-pattern/) -- State derivation
- [WebSocket scaling patterns](https://ably.com/topic/websocket-architecture-best-practices) -- Backpressure, fan-out
- [Competitor analysis sources](https://buildkite.com/docs/pipelines/dashboard-walkthrough) -- Buildkite, GitLab, Railway, Coolify, Portainer, agentsview, agent-deck

### Tertiary (LOW confidence)
- gray_matter crate -- Frontmatter extraction, needs validation
- xterm.js addon version estimates -- Need verification against @xterm/xterm 6.0

---
*Research completed: 2026-03-06*
*Ready for roadmap: yes*
