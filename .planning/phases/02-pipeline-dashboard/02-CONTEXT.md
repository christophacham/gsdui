# Phase 2: Pipeline Dashboard - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Pipeline visualization frontend with phase timeline, stage rails, wave swim lanes with plan cards, Aurora dark navy theme, agent routing configuration, and live WebSocket updates. This phase delivers everything users SEE in the Pipeline tab. Console and System tabs are separate phases.

</domain>

<decisions>
## Implementation Decisions

### Phase Timeline Interaction
- Click a phase chip navigates the main content area below the timeline to that phase's detail view (stage rail + wave lanes + plan cards)
- Timeline strip stays persistently visible at top across all detail views
- Rich phase chips: phase number, name, stage badge, progress bar (x/y plans), and duration — full status at a glance
- Default view: auto-select the currently executing phase on load; if none active, select the next pending; if all done, show the last phase
- Completed milestones collapse into a single group chip (e.g., "v1.0 ✓ — 8 phases"), click to expand; current milestone always expanded

### Plan Card Display
- Collapsed card: single row showing plan number, name (truncated), agent badge, status icon, step progress (3/5), commit count, and duration
- Expanded card: grows inline (other cards shift down), reveals ~10 lines of scrollable agent output stream, links to PLAN.md / Diff / SUMMARY.md, and "Jump to Console" button
- Output stream auto-scrolls to bottom on new content unless user has scrolled up
- File links (PLAN.md, SUMMARY.md) open as in-app read-only markdown-rendered panels; Diff renders with syntax-highlighted unified diff

### Display Configuration
- Dedicated settings panel (gear icon) for all display preferences:
  - Output line count (default 10, configurable)
  - Default card state (collapsed/expanded, auto-expand active card)
  - Visible stats toggle (steps, commits, duration, wave)
  - Auto-scroll behavior toggle
  - Timeline density (rich/medium/minimal chips)
  - Theme adjustments (font size, line spacing, contrast within Aurora palette)
  - Notification preferences (plan completion, errors, phase transitions, agent switches)
- Storage: browser local storage for immediate use, synced to server-side SQLite when auth is available (Phase 4)
- Claude has discretion on additional settings that emerge from the feature set

### Agent Routing Configuration
- Appears as a collapsible section within the phase detail view
- Shows full resolved cascade: project default → stage overrides → per-plan overrides
- Every plan shows its effective agent (resolved from cascade); overrides are visually distinct (bold/colored)
- Agent selection via dropdown with colored agent badges (Claude, Codex, Gemini) + "Inherit default" option to remove override
- Changes auto-save immediately with a brief toast confirmation; no explicit save button
- Routing writes to GSD config.json via REST API

### Wave Swim Lanes
- Horizontal rows per wave: plans within the same wave sit side by side (parallel execution)
- Waves stack vertically showing execution order (Wave 1 at top, Wave 2 below, etc.)
- When a wave has more plans than fit horizontally, the row scrolls with subtle scroll indicators
- Wave label at left edge of each row

### Dependency Arrows
- SVG connector lines from bottom of source plan card to top of dependent plan card
- Color-coded: green for satisfied dependencies, gray for pending, animated dash for in-progress
- Curved SVG paths (not straight lines)
- Clicking an arrow or plan card highlights the full transitive dependency chain (upstream and downstream); other cards dim

### Claude's Discretion
- SvelteKit project structure and component organization
- Exact spacing, typography scale, and animation timing
- Skeleton loading state design
- Reconnecting indicator design and placement
- Error state handling for failed WebSocket connections
- Markdown rendering library choice
- SVG arrow path algorithms and layout calculations
- Additional settings panel options beyond those specified
- Project sidebar layout and interaction details

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ProjectState` struct (`src/ws/messages.rs`): Full project state shape the frontend will consume — project, phases, plans, runs, agent sessions, verifications, config, parse errors
- `PhaseState` model (`src/db/models.rs`): phase_number, phase_name, goal, stage, plan_count, completed_plan_count — maps directly to phase chip display
- `PlanState` model: plan_number, plan_name, wave, depends_on, status, requirements — maps to plan card display
- `AgentSession` model: agent_type, phase_number, plan_number — drives agent badges on plan cards
- `ExecutionRun` model: duration_minutes, status, key_files — powers timing and commit stats
- `ProjectConfig` model: config_json — stores agent routing configuration

### Established Patterns
- REST API at `/api/v1/` prefix with project-scoped routes (`/projects/:id/state`, `/phases`, `/plans`)
- WebSocket at `/api/ws/state` with subscribe/snapshot/delta protocol
- All data models use serde Serialize/Deserialize — JSON wire format is stable
- Per-project subscription model — frontend subscribes to specific project IDs
- Delta updates via `StateChange` enum — frontend patches local state from deltas

### Integration Points
- Frontend consumes `GET /api/v1/projects/:id/state` for initial page load (before WebSocket connects)
- WebSocket snapshot provides full `ProjectState` on subscribe
- WebSocket deltas provide incremental `StateChange` updates
- `GET /api/v1/projects/:id/files/*path` serves raw file content for PLAN.md/SUMMARY.md rendering
- `GET /api/v1/projects` provides project list for sidebar
- `GET /api/v1/health` provides daemon diagnostics
- Agent routing config read/write via project config REST endpoints
- Static files served by Axum from `static/` directory — SvelteKit builds to this location

</code_context>

<specifics>
## Specific Ideas

- User explicitly wants "quite a bit of configuration" for display preferences — make the settings panel comprehensive, not minimal
- Strict error display preference carries forward from Phase 1: parse errors and connection issues should be visible, not hidden
- Full history preference carries forward: show all execution runs, agent timelines, and commit details — the rich data is available, surface it
- Notification preferences suggest the user wants to be informed of state changes without constantly watching — visual notifications for key events

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-pipeline-dashboard*
*Context gathered: 2026-03-06*
