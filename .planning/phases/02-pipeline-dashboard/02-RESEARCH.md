# Phase 2: Pipeline Dashboard - Research

**Researched:** 2026-03-06
**Domain:** SvelteKit SPA frontend with real-time WebSocket state, Aurora dark theme, pipeline visualization
**Confidence:** HIGH

## Summary

Phase 2 builds a complete SvelteKit 2 single-page application that serves as the Pipeline Dashboard. The backend (Phase 1) is fully operational: Axum serves static files from `static/`, exposes REST APIs at `/api/v1/`, and pushes real-time state via WebSocket at `/api/v1/ws/state` with a subscribe/snapshot/delta protocol. The frontend must consume the `ProjectState` JSON shape (project, phases, plans map, execution runs, agent sessions, verifications, config, parse errors), render it as a phase timeline with stage rails and wave swim lanes, apply the Aurora dark navy theme, and keep everything live via WebSocket deltas.

The SvelteKit app builds to static files via `@sveltejs/adapter-static` in SPA fallback mode, outputting to the existing `static/` directory that Axum already serves. Svelte 5's rune-based reactivity ($state, $derived, $effect) replaces legacy stores and is well-suited for high-frequency WebSocket updates -- proxied arrays trigger surgical DOM updates without immutable copy overhead. The frontend is entirely client-rendered (no SSR) since the Axum backend handles all data serving.

**Primary recommendation:** Use SvelteKit 2 with adapter-static in SPA mode, Svelte 5 runes for all state management, CSS custom properties for the Aurora theme tokens, `marked` for markdown rendering, `@git-diff-view/svelte` for diff display, and hand-rolled SVG cubic Bezier paths for dependency arrows. Keep the SvelteKit project as a subdirectory (`frontend/`) with build output directed to `static/`.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Phase Timeline Interaction:**
- Click a phase chip navigates the main content area below the timeline to that phase's detail view (stage rail + wave lanes + plan cards)
- Timeline strip stays persistently visible at top across all detail views
- Rich phase chips: phase number, name, stage badge, progress bar (x/y plans), and duration -- full status at a glance
- Default view: auto-select the currently executing phase on load; if none active, select the next pending; if all done, show the last phase
- Completed milestones collapse into a single group chip (e.g., "v1.0 -- 8 phases"), click to expand; current milestone always expanded

**Plan Card Display:**
- Collapsed card: single row showing plan number, name (truncated), agent badge, status icon, step progress (3/5), commit count, and duration
- Expanded card: grows inline (other cards shift down), reveals ~10 lines of scrollable agent output stream, links to PLAN.md / Diff / SUMMARY.md, and "Jump to Console" button
- Output stream auto-scrolls to bottom on new content unless user has scrolled up
- File links (PLAN.md, SUMMARY.md) open as in-app read-only markdown-rendered panels; Diff renders with syntax-highlighted unified diff

**Display Configuration:**
- Dedicated settings panel (gear icon) for all display preferences
- Output line count (default 10, configurable), default card state, visible stats toggle, auto-scroll behavior, timeline density, theme adjustments, notification preferences
- Storage: browser local storage for immediate use, synced to server-side SQLite when auth is available (Phase 4)

**Agent Routing Configuration:**
- Collapsible section within the phase detail view
- Shows full resolved cascade: project default -> stage overrides -> per-plan overrides
- Every plan shows its effective agent (resolved from cascade); overrides are visually distinct
- Agent selection via dropdown with colored agent badges + "Inherit default" option
- Changes auto-save immediately with brief toast confirmation; routing writes to GSD config.json via REST API

**Wave Swim Lanes:**
- Horizontal rows per wave: plans within the same wave sit side by side (parallel execution)
- Waves stack vertically showing execution order (Wave 1 at top, Wave 2 below)
- When a wave has more plans than fit horizontally, the row scrolls with subtle scroll indicators

**Dependency Arrows:**
- SVG connector lines from bottom of source plan card to top of dependent plan card
- Color-coded: green for satisfied dependencies, gray for pending, animated dash for in-progress
- Curved SVG paths (not straight lines)
- Clicking an arrow or plan card highlights the full transitive dependency chain; other cards dim

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

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PIPE-01 | Horizontal phase timeline with status badges, names, progress bars | Phase timeline component using PhaseState data; CSS grid/flexbox horizontal layout with overflow scroll |
| PIPE-02 | Click phase to see 4-stage detail rail (Discuss/Plan/Execute/Verify) | PhaseStage enum maps to 4 GSD stages; stage rail component highlights current via PhaseState.stage |
| PIPE-03 | Wave swim lanes within execute stage, plan cards by dependency | PlanState.wave groups plans into rows; depends_on field drives layout ordering |
| PIPE-04 | Plan cards show agent badge, status, wave, step progress, commit stats | PlanState + ExecutionRun + AgentSession data combined per card; agent_type drives badge color |
| PIPE-05 | Expand plan card for live agent stream or jump to Console | Expanded card renders ~10 lines from WebSocket delta stream; "Jump to Console" links to Phase 3 tab |
| PIPE-06 | Plan cards show links to PLAN.md, diff, SUMMARY.md | Files API at /api/v1/projects/:id/files/*path; marked for markdown; @git-diff-view/svelte for diffs |
| PIPE-07 | Phase chips show decimal phase numbers sorted numerically | parseFloat() sorting; normalize_phase_number already handles "2.1" format in backend |
| PIPE-08 | Phases group by milestone with completed milestones collapsible | ProjectState includes project-level milestone info; group phases by milestone string |
| PIPE-09 | Dependency arrows between plans in wave visualization | SVG overlay with cubic Bezier paths; element position tracking via $effect |
| PIPE-10 | Agent routing config UI with cascade hierarchy | ProjectConfig.config_json stores routing; PUT/PATCH to project config REST endpoint |
| PIPE-11 | Pipeline updates in real-time via WebSocket | WebSocket client with subscribe/snapshot/delta; $state runes for reactive state |
| PIPE-12 | Duration and timing display on phase chips and plan cards | ExecutionRun.duration_minutes and timestamps; derived display strings |
| UI-01 | Dark navy Aurora theme with CSS custom properties | CSS custom properties on :root; #040814 -> #0a1224 -> #0f1a31 -> #162543 hierarchy |
| UI-02 | Color palette background hierarchy | Four-level depth system via --bg-base, --bg-surface, --bg-elevated, --bg-overlay tokens |
| UI-03 | Agent badges with distinct colors per agent type | Predefined color map: Claude (orange/amber), Codex (green), Gemini (blue/purple) |
| UI-04 | Top-level tab navigation: Pipeline, Console, System | Tab bar component; Console and System are placeholders until Phases 3 and 4 |
| UI-05 | Project sidebar with selection indicator and active execution badge | Sidebar fetches GET /api/v1/projects; selected project drives WebSocket subscription |
| UI-06 | Skeleton loading states while WebSocket connects | CSS animation skeleton placeholders matching each component's shape |
| UI-07 | Visible reconnecting indicator when WebSocket drops | Banner/overlay component triggered by WebSocket connection state |

</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| svelte | ^5.53 | UI framework | Runes provide fine-grained reactivity ideal for real-time dashboards; direct array mutation triggers surgical DOM updates |
| @sveltejs/kit | ^2.53 | Application framework | SPA mode via adapter-static; file-based routing; build tooling |
| @sveltejs/adapter-static | ^3.0 | Build adapter | Outputs static files to serve from Axum; SPA fallback mode |
| vite | ^6.0 | Build tool | Bundled with SvelteKit; fast HMR for development |
| typescript | ^5.7 | Type safety | Type the ProjectState, WebSocket messages, and component props |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| marked | ^17.0 | Markdown to HTML | Rendering PLAN.md and SUMMARY.md content fetched from files API |
| marked-highlight | ^2.2 | Syntax highlighting for markdown code blocks | Combined with marked for fenced code rendering |
| highlight.js | ^11.11 | Syntax highlighting engine | Powers marked-highlight for code blocks in markdown panels |
| @git-diff-view/svelte | ^1.0 | Unified diff rendering | Rendering diff view for plan changes with syntax highlighting |
| svelte-5-french-toast | latest | Toast notifications | Agent routing save confirmations, error notifications, state change alerts |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| marked | unified/remark | unified is more powerful but heavier; marked is simpler for read-only rendering of fetched markdown |
| @git-diff-view/svelte | Custom diff renderer with diff library | git-diff-view handles split/unified views, syntax highlighting, and edge cases; not worth hand-rolling |
| svelte-5-french-toast | svelte-toast | svelte-5-french-toast is purpose-built for Svelte 5 runes; svelte-toast may need adaptation |
| Hand-rolled SVG arrows | JointJS / D3 | Full graph libraries are overkill for dependency connectors between known card positions |
| CSS custom properties | Tailwind CSS | Aurora theme has specific tokens; CSS custom properties give exact control without utility class overhead |

**Installation:**
```bash
# From the frontend/ directory
npx sv create . --template minimal --types ts
npm install marked marked-highlight highlight.js @git-diff-view/svelte svelte-5-french-toast
npm install -D @sveltejs/adapter-static
```

## Architecture Patterns

### Recommended Project Structure
```
frontend/                    # SvelteKit project root
  src/
    app.html                 # HTML template
    app.css                  # Global Aurora theme tokens
    routes/
      +layout.svelte         # App shell: sidebar + tabs + reconnecting indicator
      +layout.ts             # export const ssr = false
      +page.svelte           # Pipeline tab (default view)
      console/
        +page.svelte         # Console tab placeholder (Phase 3)
      system/
        +page.svelte         # System tab placeholder (Phase 4)
    lib/
      stores/
        websocket.svelte.ts  # WebSocket connection manager with reconnection
        project.svelte.ts    # ProjectState reactive store ($state rune)
        settings.svelte.ts   # Display preferences (localStorage-backed)
      components/
        layout/
          Sidebar.svelte     # Project list sidebar
          TabBar.svelte      # Pipeline | Console | System tabs
          ReconnectBanner.svelte
        pipeline/
          PhaseTimeline.svelte      # Horizontal scrolling phase chips
          PhaseChip.svelte          # Individual phase chip with badge/progress
          MilestoneGroup.svelte     # Collapsible milestone group
          StageRail.svelte          # 4-stage Discuss/Plan/Execute/Verify rail
          WaveContainer.svelte      # Wave swim lanes container
          WaveLane.svelte           # Single wave row
          PlanCard.svelte           # Collapsed/expanded plan card
          PlanCardExpanded.svelte   # Expanded card content (output stream, links)
          DependencyArrows.svelte   # SVG overlay for dependency connectors
          AgentBadge.svelte         # Colored agent type badge
          StatusIcon.svelte         # Plan status indicator
        routing/
          AgentRoutingPanel.svelte  # Collapsible agent routing config
          AgentDropdown.svelte      # Agent selection with colored badges
        viewers/
          MarkdownPanel.svelte      # Read-only markdown rendered panel
          DiffPanel.svelte          # Syntax-highlighted unified diff
        shared/
          Skeleton.svelte           # Reusable skeleton loading placeholder
          ProgressBar.svelte        # Progress bar component
          Toast.svelte              # Toast notification wrapper
          SettingsPanel.svelte      # Gear icon settings drawer
      types/
        api.ts               # TypeScript interfaces matching Rust serde types
        websocket.ts          # WebSocket message types
      utils/
        phase-sort.ts         # Numeric sorting for decimal phase numbers
        duration.ts           # Duration formatting helpers
        dependency-graph.ts   # Dependency chain traversal (transitive highlight)
        svg-paths.ts          # Cubic Bezier path generation between elements
  svelte.config.js
  vite.config.ts
  tsconfig.json
  static/                     # Symlink or copy target -- build output goes to repo root static/
```

### Pattern 1: WebSocket Connection Manager with Runes
**What:** A `.svelte.ts` module managing WebSocket lifecycle, reconnection, and state distribution
**When to use:** Central real-time data source for all components
**Example:**
```typescript
// src/lib/stores/websocket.svelte.ts
// Source: Svelte 5 runes pattern for module-level state

type ConnectionStatus = 'connecting' | 'connected' | 'reconnecting' | 'disconnected';

class WebSocketManager {
  status = $state<ConnectionStatus>('disconnected');
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectDelay = 1000;
  private maxReconnectDelay = 30000;

  connect(projectIds: string[]) {
    this.status = 'connecting';
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.ws = new WebSocket(`${protocol}//${location.host}/api/v1/ws/state`);

    this.ws.onopen = () => {
      this.status = 'connected';
      this.reconnectDelay = 1000;
      // Send subscribe message (must be first message within 10s)
      this.ws!.send(JSON.stringify({
        type: 'subscribe',
        projects: projectIds
      }));
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      // Route to project store based on message type
      handleMessage(msg);
    };

    this.ws.onclose = () => {
      this.status = 'reconnecting';
      this.scheduleReconnect(projectIds);
    };
  }

  private scheduleReconnect(projectIds: string[]) {
    this.reconnectTimer = setTimeout(() => {
      this.connect(projectIds);
    }, this.reconnectDelay);
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
  }

  disconnect() {
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
    this.status = 'disconnected';
  }
}

export const wsManager = new WebSocketManager();
```

### Pattern 2: Project State with Fine-Grained Reactive Updates
**What:** A reactive class holding the full ProjectState, updated surgically from WebSocket deltas
**When to use:** All pipeline components derive their data from this store
**Example:**
```typescript
// src/lib/stores/project.svelte.ts
import type { ProjectState, StateChange } from '$lib/types/api';

class ProjectStore {
  state = $state<ProjectState | null>(null);
  loading = $state(true);

  applySnapshot(data: ProjectState) {
    this.state = data;
    this.loading = false;
  }

  applyDelta(change: StateChange) {
    if (!this.state) return;
    switch (change.type) {
      case 'PhaseUpdated': {
        const phase = this.state.phases.find(p => p.phase_number === change.data.phase_number);
        if (phase) phase.stage = change.data.stage;
        break;
      }
      case 'PlanUpdated': {
        const plans = this.state.plans[change.data.phase_number];
        const plan = plans?.find(p => p.plan_number === change.data.plan_number);
        if (plan) plan.status = change.data.status;
        break;
      }
      // ... other delta types
    }
  }
}

export const projectStore = new ProjectStore();
```

### Pattern 3: CSS Custom Properties Aurora Theme
**What:** A pure CSS token system with the Aurora dark navy palette
**When to use:** Every component references theme tokens, never raw color values
**Example:**
```css
/* src/app.css */
:root {
  /* Background hierarchy (depth levels) */
  --bg-base: #040814;
  --bg-surface: #0a1224;
  --bg-elevated: #0f1a31;
  --bg-overlay: #162543;

  /* Foreground */
  --fg-primary: #e0e6f0;
  --fg-secondary: #94a3b8;
  --fg-muted: #64748b;
  --fg-accent: #7dd3fc;

  /* Agent colors */
  --agent-claude: #f59e0b;
  --agent-codex: #22c55e;
  --agent-gemini: #8b5cf6;

  /* Status colors */
  --status-done: #22c55e;
  --status-working: #3b82f6;
  --status-pending: #64748b;
  --status-failed: #ef4444;

  /* Dependency arrow colors */
  --dep-satisfied: #22c55e;
  --dep-pending: #64748b;
  --dep-in-progress: #3b82f6;

  /* Borders and surfaces */
  --border-subtle: #1e293b;
  --border-focus: #7dd3fc;

  /* Typography */
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  --text-xs: 0.75rem;
  --text-sm: 0.875rem;
  --text-base: 1rem;
  --text-lg: 1.125rem;

  /* Spacing scale */
  --space-1: 0.25rem;
  --space-2: 0.5rem;
  --space-3: 0.75rem;
  --space-4: 1rem;
  --space-6: 1.5rem;
  --space-8: 2rem;

  /* Transitions */
  --transition-fast: 150ms ease;
  --transition-normal: 250ms ease;
}
```

### Pattern 4: SVG Dependency Arrow Paths
**What:** Cubic Bezier curves connecting plan cards across wave lanes
**When to use:** PIPE-09 dependency visualization
**Example:**
```typescript
// src/lib/utils/svg-paths.ts
// Generates a curved SVG path from the bottom-center of source to top-center of target

interface Point { x: number; y: number; }

export function cubicConnectorPath(from: Point, to: Point): string {
  // Vertical offset for control points (creates a gentle curve)
  const dy = Math.abs(to.y - from.y);
  const controlOffset = Math.max(dy * 0.4, 30);

  const cx1 = from.x;
  const cy1 = from.y + controlOffset;
  const cx2 = to.x;
  const cy2 = to.y - controlOffset;

  return `M ${from.x} ${from.y} C ${cx1} ${cy1}, ${cx2} ${cy2}, ${to.x} ${to.y}`;
}
```

### Anti-Patterns to Avoid
- **Using Svelte stores ($writable) instead of runes:** Svelte 5 runes are the standard; legacy stores are deprecated for new code. Use `$state` in `.svelte.ts` files for shared state.
- **Global module-level $state without class wrapper:** Bare `let x = $state(0)` in modules loses reactivity when destructured. Wrap in a class or use getter/setter objects.
- **Re-rendering entire plan card lists on any delta:** Use keyed `{#each}` blocks with plan IDs so Svelte updates only the changed card.
- **Polling REST API instead of WebSocket:** The backend pushes deltas in real-time; polling adds latency and load.
- **Hardcoded colors instead of CSS custom properties:** Every color must reference a token for theme consistency and future theme switching.
- **SSR configuration in SPA mode:** This app is pure client-side. Do not add +page.server.ts or +layout.server.ts files -- they cause adapter-static to fail.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Markdown rendering | Custom parser | `marked` + `marked-highlight` + `highlight.js` | Markdown has edge cases (nested lists, HTML entities, GFM tables); marked handles them all |
| Diff visualization | Custom unified diff parser + renderer | `@git-diff-view/svelte` | Split/unified views, syntax highlighting, line numbers, virtual scrolling -- dozens of edge cases |
| Toast notifications | Custom toast system | `svelte-5-french-toast` | Animation, stacking, auto-dismiss, accessibility -- well-tested |
| WebSocket reconnection | Naive reconnect loop | Exponential backoff with jitter in WebSocket manager class | Must handle: backoff, max delay, clean teardown on navigate, snapshot re-request on reconnect |
| Phase number sorting | String sort | `parseFloat()` comparison | "2.1" must sort between "2" and "3"; string sort puts "10" before "2" |

**Key insight:** The backend already provides all data in well-structured JSON. The frontend complexity is in rendering fidelity (theme, layout, interactions) and real-time update management -- not in data transformation.

## Common Pitfalls

### Pitfall 1: SvelteKit Build Output Location
**What goes wrong:** SvelteKit defaults to building output in its own `build/` directory, but Axum serves from `static/`
**Why it happens:** adapter-static has its own default output path
**How to avoid:** Configure adapter-static with `pages: '../../static'` and `assets: '../../static'` (relative to frontend/) to output directly to the repo root `static/` directory. The existing `static/index.html` placeholder gets replaced by the build.
**Warning signs:** Axum serves the old placeholder page instead of the SvelteKit app

### Pitfall 2: WebSocket Subscribe Timeout
**What goes wrong:** Client connects but doesn't send Subscribe within 10 seconds, server drops connection
**Why it happens:** Backend enforces `wait_for_subscribe` with 10-second timeout (see `src/ws/mod.rs` line 253)
**How to avoid:** Send Subscribe message immediately in the WebSocket `onopen` handler, before any other logic
**Warning signs:** WebSocket connections silently closing with "subscribe_required" error

### Pitfall 3: Stale State After Reconnection
**What goes wrong:** After WebSocket reconnect, UI shows stale data because deltas were missed during disconnection
**Why it happens:** Deltas are incremental; missing any creates divergence
**How to avoid:** On reconnect, the server automatically sends a fresh snapshot (the subscribe flow always starts with snapshot). The frontend must reset its state to the new snapshot, not merge it.
**Warning signs:** Phase or plan statuses not matching what the backend reports via REST API

### Pitfall 4: Svelte 5 Rune Reactivity Loss in Modules
**What goes wrong:** State declared in `.ts` files doesn't trigger reactivity
**Why it happens:** Svelte runes require the `.svelte.ts` extension for module files that use `$state`, `$derived`, `$effect`
**How to avoid:** Always use `.svelte.ts` extension for files containing runes. Plain `.ts` files are for types, utilities, and pure functions only.
**Warning signs:** Components don't update when shared state changes

### Pitfall 5: SVG Arrow Position Drift
**What goes wrong:** Dependency arrows don't align with plan cards after layout changes
**Why it happens:** Card positions change on expand/collapse, window resize, or content updates, but SVG paths use stale coordinates
**How to avoid:** Use `$effect` to recalculate arrow paths whenever card positions change. Use `ResizeObserver` on the wave container. Debounce recalculation to avoid layout thrashing.
**Warning signs:** Arrows pointing to wrong cards or floating in space

### Pitfall 6: Array Mutation Gotcha in Svelte 5
**What goes wrong:** Pushing to a `$state` array doesn't trigger updates
**Why it happens:** Svelte 5 proxies arrays but only if the array itself is the $state value (not a nested property accessed through a non-proxied path)
**How to avoid:** Ensure the root ProjectState is `$state` so all nested arrays and objects are proxied. Mutate in place (push, splice) rather than reassigning.
**Warning signs:** Adding plans or phases doesn't cause re-render

### Pitfall 7: CORS and WebSocket Protocol Mismatch in Dev
**What goes wrong:** Vite dev server runs on port 5173, but Axum API is on port 3000 -- CORS blocks requests
**Why it happens:** Different origins during development
**How to avoid:** Configure Vite proxy in `vite.config.ts` to forward `/api/` to `http://localhost:3000`. The backend already has `CorsLayer::permissive()` but the WebSocket upgrade needs the proxy.
**Warning signs:** Network errors in browser console for API calls during development

## Code Examples

### WebSocket Message Types (TypeScript matching Rust serde)
```typescript
// src/lib/types/api.ts
// Source: Matches src/ws/messages.rs and src/db/models.rs exactly

export interface Project {
  id: string;
  name: string;
  path: string;
  status: string;
  retention_days: number | null;
  created_at: string;
  last_seen_at: string;
}

export interface PhaseState {
  id: number;
  project_id: string;
  phase_number: string;
  phase_name: string;
  goal: string | null;
  depends_on: string | null;
  stage: string;  // planned | discussed | researched | planned_ready | executing | executed | verified
  status: string | null;
  requirements: string | null;
  plan_count: number | null;
  completed_plan_count: number | null;
  updated_at: string;
}

export interface PlanState {
  id: number;
  project_id: string;
  phase_number: string;
  plan_number: string;
  plan_name: string | null;
  wave: number | null;
  depends_on: string | null;
  plan_type: string | null;
  status: string;  // pending | working | done | failed
  requirements: string | null;
  files_modified: string | null;
  updated_at: string;
}

export interface ExecutionRun {
  id: number;
  project_id: string;
  phase_number: string;
  plan_number: string;
  run_number: number;
  superseded: number;
  started_at: string | null;
  completed_at: string | null;
  duration_minutes: number | null;
  status: string | null;
  key_files_created: string | null;
  key_files_modified: string | null;
  requirements_completed: string | null;
  created_at: string;
}

export interface AgentSession {
  id: number;
  project_id: string;
  agent_id: string | null;
  agent_type: string | null;
  phase_number: string | null;
  plan_number: string | null;
  started_at: string | null;
  ended_at: string | null;
  created_at: string;
}

export interface VerificationResult {
  id: number;
  project_id: string;
  phase_number: string;
  status: string;  // passed | gaps_found | human_needed
  score: string | null;
  verified_at: string | null;
  created_at: string;
}

export interface ProjectConfig {
  id: number;
  project_id: string;
  config_json: string;
  updated_at: string;
}

export interface ParseError {
  id: number;
  project_id: string;
  file_path: string;
  error_message: string;
  severity: string;  // warning | error
  occurred_at: string;
  resolved_at: string | null;
}

export interface ProjectState {
  project: Project;
  phases: PhaseState[];
  plans: Record<string, PlanState[]>;  // keyed by phase_number
  recent_runs: ExecutionRun[];
  agent_sessions: AgentSession[];
  verifications: Record<string, VerificationResult>;  // keyed by phase_number
  config: ProjectConfig | null;
  parse_errors: ParseError[];
}

// WebSocket server-to-client messages (tagged union via "type" field)
export type WsMessage =
  | { type: 'snapshot'; project: string; data: ProjectState }
  | { type: 'delta'; project: string; changes: StateChange[] }
  | { type: 'health'; uptime_secs: number; db_size_bytes: number; ws_client_count: number; watcher_queue_depth: number; memory_usage_bytes: number; per_project_status: Record<string, ProjectWatcherStatus> }
  | { type: 'error'; message: string; code: string };

export type StateChange =
  | { type: 'PhaseUpdated'; data: { phase_number: string; stage: string } }
  | { type: 'PlanUpdated'; data: { phase_number: string; plan_number: string; status: string } }
  | { type: 'VerificationUpdated'; data: { phase_number: string; status: string } }
  | { type: 'ConfigUpdated' }
  | { type: 'AgentHistoryUpdated'; data: { session_count: number } }
  | { type: 'ProjectStateUpdated'; data: { status: string | null } }
  | { type: 'ParseError'; data: { file_path: string; error: string } };

export interface ProjectWatcherStatus {
  active: boolean;
  watched_paths: number;
  last_event_at: string | null;
  error_count: number;
}

// Client-to-server messages
export type ClientMessage =
  | { type: 'subscribe'; projects: string[] }
  | { type: 'unsubscribe'; projects: string[] };
```

### Vite Proxy Configuration for Development
```typescript
// frontend/vite.config.ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
        ws: true  // Important: proxy WebSocket upgrades too
      }
    }
  }
});
```

### SvelteKit SPA Adapter Configuration
```javascript
// frontend/svelte.config.js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: '../static',    // Output to repo root static/ directory
      assets: '../static',
      fallback: 'index.html',
      strict: false
    }),
    paths: {
      base: ''  // Served from root
    }
  }
};

export default config;
```

### Phase Number Numeric Sort
```typescript
// src/lib/utils/phase-sort.ts
export function sortPhaseNumbers(phases: { phase_number: string }[]): typeof phases {
  return [...phases].sort((a, b) => {
    return parseFloat(a.phase_number) - parseFloat(b.phase_number);
  });
}
```

### Skeleton Loading Component Pattern
```svelte
<!-- src/lib/components/shared/Skeleton.svelte -->
<script lang="ts">
  interface Props {
    width?: string;
    height?: string;
    rounded?: boolean;
  }
  let { width = '100%', height = '1rem', rounded = false }: Props = $props();
</script>

<div
  class="skeleton"
  class:rounded
  style:width={width}
  style:height={height}
></div>

<style>
  .skeleton {
    background: linear-gradient(
      90deg,
      var(--bg-elevated) 25%,
      var(--bg-overlay) 50%,
      var(--bg-elevated) 75%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
    border-radius: 4px;
  }
  .skeleton.rounded {
    border-radius: 50%;
  }
  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
```

### Reconnecting Indicator Pattern
```svelte
<!-- src/lib/components/layout/ReconnectBanner.svelte -->
<script lang="ts">
  import { wsManager } from '$lib/stores/websocket.svelte';

  const isReconnecting = $derived(
    wsManager.status === 'reconnecting' || wsManager.status === 'connecting'
  );
</script>

{#if isReconnecting}
  <div class="reconnect-banner" role="alert">
    <span class="pulse-dot"></span>
    Reconnecting to server...
  </div>
{/if}

<style>
  .reconnect-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 1000;
    padding: var(--space-2) var(--space-4);
    background: var(--status-working);
    color: white;
    text-align: center;
    font-size: var(--text-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
  }
  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: white;
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Svelte stores ($writable/$readable) | Svelte 5 runes ($state/$derived/$effect) | Svelte 5.0 (Oct 2024) | Runes are the standard; stores still work but are legacy. Use runes for all new code. |
| .svelte files only for reactivity | .svelte.ts modules for shared state | Svelte 5.0 | Shared state no longer needs components or store boilerplate |
| SvelteKit adapter-node for SSR | adapter-static SPA mode for Axum-served apps | SvelteKit 2.0 | No Node.js server needed; Axum serves static build output |
| createEventDispatcher for events | Callback props ($props) | Svelte 5.0 | Use callback props instead of custom events |
| slot for component children | {@render children()} snippet | Svelte 5.0 | Snippets replace slots for component composition |

**Deprecated/outdated:**
- `$:` reactive declarations: replaced by `$derived` and `$effect`
- `export let` for props: replaced by `$props()`
- `<slot>`: replaced by `{@render children()}` snippet pattern
- `createEventDispatcher()`: replaced by callback props
- Svelte stores for new code: use runes in `.svelte.ts` files

## Open Questions

1. **Agent routing config REST endpoint**
   - What we know: `ProjectConfig.config_json` stores the full config; `GET /api/v1/projects/:id/state` returns it; the `config.json` watcher triggers `ConfigUpdated` deltas
   - What's unclear: There is no PUT/PATCH endpoint specifically for updating `config_json` in the current REST API. The `update_project` endpoint only handles `name` and `retention_days`.
   - Recommendation: The planner should include a task to add a `PUT /api/v1/projects/:id/config` endpoint that accepts the config JSON, writes it to the project's `.planning/config.json` file, and lets the file watcher propagate the change. This is a small backend addition needed for PIPE-10.

2. **Live agent output stream for expanded plan cards**
   - What we know: PIPE-05 requires ~10 lines of scrollable agent output in expanded cards. The backend currently sends `StateChange` deltas (status changes, not output streams).
   - What's unclear: There is no agent output streaming mechanism in the Phase 1 backend. Agent output is a Phase 3 concept (terminal/PTY system).
   - Recommendation: For Phase 2, the expanded card output area should show the plan's execution log summary (from SUMMARY.md) or a "Live output available in Console tab" placeholder. Full streaming comes with Phase 3 integration. The "Jump to Console" button can be rendered as disabled with a tooltip until Phase 3 is complete.

3. **Milestone information source**
   - What we know: PIPE-08 requires milestone grouping. STATE.md frontmatter has `milestone` and `milestone_name` fields that the parser reads.
   - What's unclear: The parsed milestone data is stored in the `projects` table (not directly exposed per phase), and phases don't have individual milestone fields in the DB.
   - Recommendation: Use the project-level milestone from STATE.md frontmatter for now. If per-phase milestone grouping is needed, it can be derived from the ROADMAP.md structure or added as a parser enhancement in a backend micro-task.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Vitest + @testing-library/svelte (frontend unit/component tests) |
| Config file | `frontend/vitest.config.ts` -- Wave 0 creation needed |
| Quick run command | `cd frontend && npx vitest run --reporter=verbose` |
| Full suite command | `cd frontend && npx vitest run && npx svelte-check --tsconfig ./tsconfig.json` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PIPE-01 | Phase timeline renders phases with badges and progress | component | `cd frontend && npx vitest run src/lib/components/pipeline/PhaseTimeline.test.ts -x` | -- Wave 0 |
| PIPE-02 | Stage rail highlights current stage | component | `cd frontend && npx vitest run src/lib/components/pipeline/StageRail.test.ts -x` | -- Wave 0 |
| PIPE-03 | Plans grouped into wave lanes by wave number | component | `cd frontend && npx vitest run src/lib/components/pipeline/WaveContainer.test.ts -x` | -- Wave 0 |
| PIPE-04 | Plan card displays all required fields | component | `cd frontend && npx vitest run src/lib/components/pipeline/PlanCard.test.ts -x` | -- Wave 0 |
| PIPE-05 | Expanded card shows output area and jump link | component | `cd frontend && npx vitest run src/lib/components/pipeline/PlanCardExpanded.test.ts -x` | -- Wave 0 |
| PIPE-06 | File links fetch and render markdown/diff | integration | `cd frontend && npx vitest run src/lib/components/viewers/MarkdownPanel.test.ts -x` | -- Wave 0 |
| PIPE-07 | Decimal phase numbers sort correctly | unit | `cd frontend && npx vitest run src/lib/utils/phase-sort.test.ts -x` | -- Wave 0 |
| PIPE-08 | Milestone grouping collapses completed | component | `cd frontend && npx vitest run src/lib/components/pipeline/MilestoneGroup.test.ts -x` | -- Wave 0 |
| PIPE-09 | Dependency arrows render between correct cards | unit | `cd frontend && npx vitest run src/lib/utils/svg-paths.test.ts -x` | -- Wave 0 |
| PIPE-10 | Agent routing cascade displays and saves | component | `cd frontend && npx vitest run src/lib/components/routing/AgentRoutingPanel.test.ts -x` | -- Wave 0 |
| PIPE-11 | WebSocket state updates trigger UI re-render | unit | `cd frontend && npx vitest run src/lib/stores/websocket.svelte.test.ts -x` | -- Wave 0 |
| PIPE-12 | Duration formatting correct for various inputs | unit | `cd frontend && npx vitest run src/lib/utils/duration.test.ts -x` | -- Wave 0 |
| UI-01 | Aurora theme tokens applied globally | manual-only | Visual inspection -- CSS custom properties on :root | N/A |
| UI-02 | Background hierarchy correct per depth | manual-only | Visual inspection -- four depth levels distinguishable | N/A |
| UI-03 | Agent badges show correct colors | component | `cd frontend && npx vitest run src/lib/components/pipeline/AgentBadge.test.ts -x` | -- Wave 0 |
| UI-04 | Tab navigation between Pipeline/Console/System | component | `cd frontend && npx vitest run src/lib/components/layout/TabBar.test.ts -x` | -- Wave 0 |
| UI-05 | Sidebar lists projects and indicates selection | component | `cd frontend && npx vitest run src/lib/components/layout/Sidebar.test.ts -x` | -- Wave 0 |
| UI-06 | Skeleton states shown during loading | component | `cd frontend && npx vitest run src/lib/components/shared/Skeleton.test.ts -x` | -- Wave 0 |
| UI-07 | Reconnecting indicator visible on WS drop | component | `cd frontend && npx vitest run src/lib/components/layout/ReconnectBanner.test.ts -x` | -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cd frontend && npx vitest run --reporter=verbose`
- **Per wave merge:** `cd frontend && npx vitest run && npx svelte-check --tsconfig ./tsconfig.json`
- **Phase gate:** Full suite green + visual inspection of all UI requirements before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `frontend/vitest.config.ts` -- Vitest configuration for SvelteKit component testing
- [ ] `frontend/src/lib/test-utils.ts` -- Shared test utilities (mock WebSocket, mock ProjectState factory)
- [ ] Framework install: `npm install -D vitest @testing-library/svelte jsdom @sveltejs/vite-plugin-svelte` in frontend/
- [ ] `frontend/tsconfig.json` -- TypeScript config (auto-generated by SvelteKit scaffold but may need vitest paths)

## Sources

### Primary (HIGH confidence)
- Svelte 5 official docs (svelte.dev/docs/svelte/$state) -- runes API, migration guide
- SvelteKit official docs (svelte.dev/docs/kit/single-page-apps) -- SPA configuration, adapter-static
- SvelteKit official docs (svelte.dev/docs/kit/adapter-static) -- static adapter configuration
- Existing codebase: `src/ws/messages.rs`, `src/db/models.rs`, `src/api/` -- exact API shapes and WebSocket protocol

### Secondary (MEDIUM confidence)
- [Mainmatter: Runes and Global State](https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/) -- do's and don'ts for module-level $state
- [Real-world Svelte 5: High-frequency real-time data](https://dev.to/polliog/real-world-svelte-5-handling-high-frequency-real-time-data-with-runes-3i2f) -- WebSocket + runes patterns
- [Dave Snider: Svelte 5 theming with vanilla CSS](https://www.davesnider.com/posts/svelte-theme) -- CSS custom properties approach
- [svelte-axum-project template](https://github.com/jbertovic/svelte-axum-project) -- Svelte + Axum integration patterns
- [marked.js documentation](https://marked.js.org/) -- markdown rendering API
- [git-diff-view GitHub](https://github.com/MrWangJustToDo/git-diff-view) -- diff rendering component
- [MDN SVG Paths tutorial](https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorials/SVG_from_scratch/Paths) -- cubic Bezier path syntax

### Tertiary (LOW confidence)
- svelte-5-french-toast -- referenced but not verified via official docs; may need version check during implementation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- SvelteKit 2 + Svelte 5 + adapter-static is the documented approach for this architecture; versions verified via npm
- Architecture: HIGH -- Project structure follows SvelteKit conventions; data shapes verified against actual Rust source code
- Pitfalls: HIGH -- WebSocket protocol details verified against actual `src/ws/mod.rs`; rune patterns verified against official docs and community articles
- Open questions: MEDIUM -- Agent routing endpoint gap and live output limitation are real; recommendations are sound but need planner confirmation

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable ecosystem; SvelteKit 2 and Svelte 5 are mature)
