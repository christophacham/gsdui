# GSD Pipeline UI

## What This Is

A web-based dashboard for visualizing and interacting with GSD workflow execution. It provides a Pipeline view (phase timeline, stage rails, wave lanes with plan cards), a Console with interactive browser terminals (xterm.js) for running GSD commands and responding to agent prompts, and a System tab for host metrics, service health, and logs. Multiple users access it remotely via browser, each with their own registered project folders.

## Core Value

Users can see what GSD is doing across all their projects in real-time and interact with running agent sessions from the browser — no SSH required.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

(None yet — ship to validate)

### Active

<!-- Current scope. Building toward these. -->

- [ ] Pipeline tab with horizontal phase timeline showing all phases, stages, progress
- [ ] Phase detail view with 4-stage rail (Discuss → Plan → Execute → Verify)
- [ ] Wave visualization with plan cards showing agent, status, progress, commits
- [ ] Console tab with xterm.js browser terminals and sub-tabs
- [ ] Agent sessions auto-create Console sub-tabs during GSD execution
- [ ] Interactive PTY sessions — users can respond to GSD prompts and inject commands
- [ ] System tab with host metrics, service health, and scrolling log output
- [ ] Multi-project sidebar — each user registers their own project folders
- [ ] Daemon observes `.planning/` file changes (inotify) and parses state
- [ ] Database as parsed cache + historical record (execution timelines, durations, metrics)
- [ ] WebSocket streaming for live state updates to connected clients
- [ ] Agent routing configuration (project default → stage override → plan override)
- [ ] Multi-agent support (Claude, Codex, Gemini) with per-stage and per-plan routing
- [ ] Authentication via reverse proxy (Caddy + oauth2-proxy or similar)
- [ ] Per-user project isolation — users only see their own registered projects
- [ ] Dark navy Aurora theme (tokens from non-planar project)

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- Mobile app — web-first, responsive later
- GSD orchestration — daemon observes only, never imports or calls GSD
- PROJECT.md / REQUIREMENTS.md visualization panels — deferred to future UI iteration
- Todo sidebar — deferred to future UI iteration
- Nyquist/UAT visualization — deferred to future UI iteration

## Context

The daemon follows an **observe, don't orchestrate** architecture. GSD runs as a plugin inside agent sessions (Claude Code, Codex). Users kick off GSD commands in Console tabs. The daemon watches `.planning/` file changes and visualizes state in real-time. It parses STATE.md frontmatter, PLAN.md/SUMMARY.md frontmatter, ROADMAP.md, VERIFICATION.md, config.json, agent-history.json, and phase directory listings to derive pipeline state.

GSD's state tracking is file-based (STATE.md, agent-history.json, current-agent-id.txt, PLAN/SUMMARY frontmatter). The daemon parses these into a database that serves as both a cache for current state and a historical record of execution timelines, metrics, and durations over time.

Stage derivation per phase follows GSD's file-presence logic: no directory = planned, CONTEXT.md = discussed, RESEARCH.md = researched, PLAN.md = planned, partial SUMMARY.md = executing, all SUMMARY.md = executed, VERIFICATION.md = verified (with pass/fail status).

Phase numbering supports decimals (2.1, 2.2) and milestones with continuous numbering (01–99). The timeline sorts numerically.

Theme: Dark navy Aurora design system from the non-planar project — CSS custom properties (tokens.css → theme.css → utilities.css → global.css), no Tailwind. Core palette: #040814, #0a1224, #0f1a31, #162543.

## Constraints

- **Architecture**: Daemon observes `.planning/` files only — never imports, embeds, or calls GSD
- **Deployment**: Remote server, accessed via browser over network
- **Auth**: Reverse proxy with OAuth2 (Caddy + oauth2-proxy or equivalent)
- **Theme**: Aurora dark navy palette from non-planar, CSS custom properties (no Tailwind)
- **PTY**: Browser terminals via xterm.js, PTY processes managed by daemon on server

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Observe, don't orchestrate | GSD already runs inside agent sessions; duplicating orchestration creates conflicts | — Pending |
| Database for state (not just file parsing) | Need historical data (timelines, metrics) that flat files don't preserve; also serves as fast cache | — Pending |
| Aurora theme from non-planar | Proven dark navy design system with comprehensive tokens; consistent visual identity | — Pending |
| Per-user project isolation | Team members work on different projects; shared view not needed | — Pending |
| Stack open to research | Axum/SvelteKit 5/pty-process mentioned but not locked — let research validate | — Pending |

---
*Last updated: 2026-03-06 after initialization*
