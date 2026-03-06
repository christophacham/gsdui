# Architecture Patterns

**Domain:** Real-time developer dashboard with browser terminals (GSD Pipeline UI)
**Researched:** 2026-03-06

## Recommended Architecture

The system is a three-layer architecture: a Rust daemon (Axum) that watches files + manages PTYs + pushes state over WebSocket, a SvelteKit 5 SPA that renders the dashboard, and a reverse proxy (Caddy) that handles auth and TLS. The daemon is the single backend process -- it serves the SvelteKit static build, the REST API, and all WebSocket connections.

```
                          Internet
                             |
                       +-----+------+
                       |   Caddy    |  TLS termination
                       | oauth2-proxy  OAuth2 auth (forward_auth)
                       +-----+------+
                             |
                             | X-Forwarded-User header
                             |
                    +--------+--------+
                    |   Axum Daemon   |
                    |                 |
                    |  +-- Static ----+-- Serves SvelteKit build (ServeDir)
                    |  +-- REST ------+-- /api/projects, /api/phases, etc.
                    |  +-- WS /state -+-- JSON state updates (per-project)
                    |  +-- WS /pty ---+-- Binary PTY streams (per-session)
                    |                 |
                    |  +-- FileWatcher (notify + debounce)
                    |  +-- Parser (STATE.md, ROADMAP.md, PLAN.md, etc.)
                    |  +-- PTYManager (portable-pty sessions)
                    |  +-- SQLite (rusqlite, WAL mode)
                    |  +-- Broadcaster (tokio::broadcast channels)
                    +--------+--------+
                             |
                    .planning/ directories
                    (watched via inotify)
```

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| **Caddy** | TLS termination, OAuth2 forward_auth, WebSocket proxying | oauth2-proxy, Axum daemon |
| **oauth2-proxy** | OAuth2 flow with identity provider, sets user identity headers | Caddy (forward_auth), identity provider (GitHub/Google/etc.) |
| **Axum HTTP server** | Route requests, serve static files, upgrade WebSocket connections | All internal components |
| **FileWatcher** | Watch `.planning/` directories for changes, debounce events, dispatch to Parser | Parser (via tokio mpsc channel) |
| **Parser** | Read and parse GSD state files (STATE.md, ROADMAP.md, PLAN.md, config.json, etc.), derive pipeline state | FileWatcher (receives events), SQLite (writes state), Broadcaster (emits updates) |
| **SQLite store** | Persist parsed state, historical metrics, execution timelines, project/user registrations | Parser (writes), REST handlers (reads), WebSocket handlers (reads on connect) |
| **Broadcaster** | Fan-out state changes to connected WebSocket clients, scoped by user + project | Parser (receives updates), WebSocket handlers (subscribe) |
| **PTYManager** | Spawn PTY processes, manage lifecycle, resize, stream I/O | WebSocket /pty handlers (bidirectional I/O), portable-pty (PTY creation) |
| **SvelteKit SPA** | Render dashboard UI, manage client-side state via runes, connect to WebSocket endpoints | Axum (REST + WebSocket), xterm.js (terminal rendering) |

## Data Flow

### Flow 1: File Change to Dashboard Update

```
1. GSD agent writes to .planning/STATE.md (or PLAN.md, ROADMAP.md, etc.)
2. inotify fires event -> notify crate receives it
3. notify-debouncer-full coalesces events (200ms window)
4. Debounced event sent via tokio::mpsc to Parser task
5. Parser reads affected file(s), extracts frontmatter/content
6. Parser derives pipeline state (stage, progress, agent assignment)
7. Parser writes updated state to SQLite (INSERT/UPDATE)
8. Parser sends StateUpdate message to tokio::broadcast channel
9. Broadcaster filters: which connected clients care about this project?
10. Filtered JSON message sent to each relevant WebSocket /state connection
11. SvelteKit receives JSON, updates $state stores
12. Svelte reactivity propagates to components (pipeline, phase detail, cards)
```

### Flow 2: Terminal Session (PTY)

```
1. User clicks "New Terminal" or GSD agent starts (agent-session event)
2. Frontend opens WebSocket to /api/ws/pty?project=X&session=Y
3. Axum upgrades connection, authenticates via user header
4. PTYManager spawns new PTY (portable-pty) with shell in project directory
5. Two tokio tasks spawned per session:
   a. PTY stdout -> WebSocket binary frames (read PTY, write WS)
   b. WebSocket binary frames -> PTY stdin (read WS, write PTY)
6. xterm.js renders output, captures input
7. FitAddon resize -> frontend sends resize message -> PTYManager calls pty.resize()
8. On disconnect/exit: PTY killed, tasks dropped, session entry updated in DB
```

### Flow 3: Initial Page Load

```
1. Browser requests / -> Caddy -> oauth2-proxy verifies auth -> Axum
2. Axum ServeDir serves SvelteKit index.html (SPA fallback)
3. SPA loads, reads X-Forwarded-User from initial auth context
4. SPA calls GET /api/projects (user's registered projects)
5. SPA opens WebSocket /api/ws/state?projects=A,B,C
6. Server sends initial state snapshot (full pipeline state from SQLite)
7. Subsequent messages are incremental deltas
8. User navigates to Console tab -> opens PTY WebSocket connections as needed
```

## Component Detail

### FileWatcher Module

**Crate:** `notify` (v7+) with `notify-debouncer-full`

The FileWatcher manages one watcher per registered project directory. It watches `.planning/` recursively using inotify on Linux. Events are debounced (200ms default, configurable) to coalesce rapid multi-file writes that GSD agents produce.

**Key design decisions:**
- Use `notify-debouncer-full` over `notify-debouncer-mini` because we need event type information (create vs modify vs delete), not just path notifications
- Watch recursively from `.planning/` -- GSD writes to phase subdirectories (`.planning/phases/01-setup/`)
- Debounce per-file, not globally -- a change to `STATE.md` should not wait for `PLAN.md` writes to settle
- Send debounced events to Parser via `tokio::mpsc::channel` (bounded, backpressure-aware)

**Confidence:** HIGH (notify crate is the standard, 175M+ downloads, actively maintained)

### Parser Module

The Parser is a pure-function module that reads GSD file formats and produces structured state. It does NOT watch files -- it receives events from FileWatcher and reads on demand.

**Files parsed and what they produce:**

| File | Parse Method | Output |
|------|-------------|--------|
| `STATE.md` | YAML frontmatter extraction | Current phase, stage, active plans, agent assignments |
| `ROADMAP.md` | Markdown heading + checkbox parsing | Phase list with names, order, completion status |
| `config.json` | JSON deserialization | Agent routing config, project settings |
| `agent-history.json` | JSON deserialization | Historical agent sessions, timings |
| `current-agent-id.txt` | Plain text read | Currently active agent ID |
| `PLAN.md` (per phase) | YAML frontmatter + markdown | Plan details, status, assigned agent |
| `SUMMARY.md` (per phase) | YAML frontmatter | Execution results, duration, commits |
| `VERIFICATION.md` (per phase) | YAML frontmatter + content | Pass/fail status, test results |
| `CONTEXT.md` / `RESEARCH.md` | File existence check | Stage derivation (discussed/researched) |

**Stage derivation logic (critical -- mirrors GSD's own logic):**
```
no directory          -> Planned
CONTEXT.md exists     -> Discussed
RESEARCH.md exists    -> Researched
PLAN.md exists        -> Planned (execution-ready)
SUMMARY.md (partial)  -> Executing
SUMMARY.md (complete) -> Executed
VERIFICATION.md       -> Verified (read pass/fail from frontmatter)
```

**Design decisions:**
- Parser is stateless -- given a file path and contents, it returns structured data
- Frontmatter parsing via a lightweight YAML parser (e.g., `serde_yaml` or `yaml-rust2`)
- Markdown heading/checkbox parsing via simple regex or `pulldown-cmark` (no need for full AST)
- Error handling: malformed files produce warnings, not crashes -- partial state is better than no state

**Confidence:** HIGH (file formats are well-defined by GSD, parsing is straightforward)

### SQLite Store

**Crate:** `rusqlite` with WAL mode enabled

SQLite is the right choice here because:
1. Single daemon process -- no need for client-server DB
2. Zero configuration, embedded in the binary
3. WAL mode gives concurrent reads while writing (one writer is fine -- only the Parser writes)
4. Excellent for read-heavy workloads (WebSocket handlers read on connect, REST reads for API)
5. No additional service to deploy or manage

**Schema domains:**

```sql
-- User and project registration
users (id, external_id, display_name, created_at)
projects (id, user_id, name, path, created_at, last_seen_at)

-- Current pipeline state (cache of parsed files)
pipeline_state (project_id, phase_number, phase_name, stage, progress_pct, updated_at)
plan_state (project_id, phase_number, plan_name, status, agent, commits, updated_at)

-- Agent configuration
agent_config (project_id, scope, scope_id, agent_type, model, created_at)

-- Historical records
execution_history (id, project_id, phase_number, plan_name, agent, started_at, ended_at, duration_ms, status)
agent_sessions (id, project_id, agent_id, agent_type, started_at, ended_at)

-- PTY sessions
pty_sessions (id, user_id, project_id, label, pid, started_at, ended_at, exit_code)
```

**Key settings:**
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

**Confidence:** HIGH (SQLite WAL for embedded daemons is a well-proven pattern)

### WebSocket Protocol

Two separate WebSocket endpoints with different serialization:

#### `/api/ws/state` -- JSON text frames (pipeline state)

```typescript
// Client -> Server
{ "type": "subscribe", "projects": ["project-uuid-1", "project-uuid-2"] }
{ "type": "unsubscribe", "projects": ["project-uuid-1"] }

// Server -> Client
{ "type": "snapshot", "project": "uuid", "data": { /* full pipeline state */ } }
{ "type": "delta", "project": "uuid", "changes": [
  { "kind": "phase_update", "phase": 3, "stage": "executing", "progress": 45 },
  { "kind": "plan_update", "phase": 3, "plan": "implement-auth", "status": "running", "agent": "claude" }
]}
{ "type": "agent_event", "project": "uuid", "agent_id": "abc123", "event": "session_started" }
```

Rationale for JSON text frames: State updates are infrequent (triggered by file changes, seconds apart), human-readable for debugging, small payloads (<1KB typically). Binary encoding would add complexity without meaningful benefit.

#### `/api/ws/pty` -- Binary frames (terminal I/O)

```
Binary frame format (server -> client and client -> server):
  [1 byte: message type] [payload]

Message types:
  0x01 = PTY data (payload = raw terminal bytes)
  0x02 = Resize (payload = 2x uint16 LE: cols, rows)
  0x03 = Ping/pong (keepalive)
  0x04 = Exit (payload = 1 byte exit code)
```

Rationale for binary frames: Terminal data is high-frequency (every keystroke, every output byte), already binary (ANSI escape sequences, UTF-8 bytes), and benefits from minimal framing overhead. The 1-byte type prefix is sufficient because PTY connections are 1:1 (no multiplexing needed per-connection -- each terminal tab opens its own WebSocket).

**Key decision: Separate WebSocket connections per terminal, NOT multiplexed.** Multiplexing adds protocol complexity (channel IDs, demuxing logic) with no real benefit -- browsers handle many concurrent WebSocket connections efficiently, and separate connections give independent flow control and clean lifecycle management (close one terminal without affecting others).

**Confidence:** HIGH (pattern proven by VS Code remote terminals, Jupyter, Proxmox)

### PTYManager

**Crate:** `portable-pty` (from wezterm project)

The PTYManager owns all PTY processes and their associated tokio tasks. It is an actor-like component accessed via a `tokio::mpsc` command channel.

**Responsibilities:**
- Spawn new PTY with specified shell and working directory
- Track session ID -> PTY mapping
- Handle resize requests (relay to PTY)
- Detect PTY exit and notify connected WebSocket
- Clean up orphaned PTYs (no connected client for >30s)
- Enforce per-user PTY limits (prevent resource exhaustion)

**Architecture:**
```rust
struct PtyManager {
    sessions: HashMap<SessionId, PtySession>,
    cmd_rx: mpsc::Receiver<PtyCommand>,
}

struct PtySession {
    pty: Box<dyn MasterPty>,  // portable-pty master
    child: Box<dyn Child>,     // spawned process
    ws_tx: mpsc::Sender<Vec<u8>>,  // send to WebSocket
    created_at: Instant,
    user_id: UserId,
    project_id: ProjectId,
}

enum PtyCommand {
    Spawn { user_id, project_id, shell, cwd, reply: oneshot::Sender<SessionId> },
    Write { session_id, data: Vec<u8> },
    Resize { session_id, cols: u16, rows: u16 },
    Kill { session_id },
}
```

**Flow control:** The PTY read loop uses a bounded channel to the WebSocket sender. If the WebSocket cannot keep up (slow client), the channel applies backpressure on the PTY read. This prevents unbounded memory growth from fast-output commands (e.g., `find /`).

**Confidence:** MEDIUM (portable-pty is well-maintained but async integration requires careful task management; pty-process is an alternative if portable-pty causes issues)

### Broadcaster (State Fan-Out)

**Mechanism:** `tokio::broadcast` channel per project

When a user subscribes to project updates, the WebSocket handler clones the broadcast receiver for that project. The Parser sends updates to the broadcast sender after writing to SQLite.

```rust
struct Broadcaster {
    channels: RwLock<HashMap<ProjectId, broadcast::Sender<StateUpdate>>>,
}

impl Broadcaster {
    fn subscribe(&self, project_id: &ProjectId) -> broadcast::Receiver<StateUpdate> {
        // Get or create channel for this project
        let channels = self.channels.read().await;
        channels.get(project_id).unwrap().subscribe()
    }

    fn publish(&self, project_id: &ProjectId, update: StateUpdate) {
        if let Some(tx) = self.channels.read().await.get(project_id) {
            let _ = tx.send(update); // Ignore error if no receivers
        }
    }
}
```

Why `broadcast` over `watch`: `watch` only keeps the latest value, losing intermediate state changes. `broadcast` delivers every update in order. With the low frequency of file-change events (seconds between updates), the broadcast buffer (capacity 64) will never overflow.

**Confidence:** HIGH (tokio broadcast is the standard pattern for WebSocket fan-out)

### Axum AppState

```rust
struct AppState {
    db: SqlitePool,              // r2d2 or deadpool connection pool
    broadcaster: Arc<Broadcaster>,
    pty_manager: mpsc::Sender<PtyCommand>,
    file_watcher: Arc<FileWatcherHandle>,  // for add/remove project watches
}
```

AppState is wrapped in `Arc` and passed via Axum's `.with_state()`. All fields are cheaply cloneable (Senders, Arcs). No Mutex on AppState itself -- contention is pushed to individual components.

### SvelteKit Frontend

**Build:** SvelteKit with `adapter-static` (SPA mode with fallback page)
**Served by:** Axum via `tower-http::ServeDir` with `.not_found_service(ServeFile::new("index.html"))`

#### Component Hierarchy

```
App.svelte
+-- Layout.svelte
    +-- Sidebar.svelte                    # Project list, user info
    |   +-- ProjectListItem.svelte        # Per-project entry with status dot
    |   +-- ProjectRegistrationModal.svelte
    |
    +-- TabBar.svelte                     # Pipeline | Console | System
    |
    +-- PipelineTab.svelte                # Tab content area
    |   +-- PhaseTimeline.svelte          # Horizontal scrolling timeline
    |   |   +-- PhaseNode.svelte          # Single phase circle/card
    |   +-- PhaseDetail.svelte            # Selected phase expanded view
    |   |   +-- StageRail.svelte          # 4-stage progress (Discuss->Plan->Execute->Verify)
    |   |   +-- WaveLane.svelte           # Execution wave container
    |   |       +-- PlanCard.svelte       # Individual plan with status, agent, progress
    |   +-- AgentConfigPanel.svelte       # Agent routing configuration
    |
    +-- ConsoleTab.svelte
    |   +-- TerminalTabBar.svelte         # Sub-tabs for each terminal session
    |   +-- TerminalPane.svelte           # xterm.js instance wrapper
    |       +-- (xterm.js Terminal)       # Third-party, not a Svelte component
    |
    +-- SystemTab.svelte
        +-- HostMetrics.svelte            # CPU, memory, disk
        +-- ServiceHealth.svelte          # Daemon status, uptime
        +-- LogViewer.svelte              # Scrolling log output
```

#### State Management (Svelte 5 Runes)

```typescript
// lib/stores/pipeline.svelte.ts
class PipelineStore {
    projects = $state<Map<string, ProjectState>>(new Map());
    activeProject = $state<string | null>(null);

    activePhases = $derived(
        this.activeProject
            ? this.projects.get(this.activeProject)?.phases ?? []
            : []
    );

    selectedPhase = $state<number | null>(null);

    phaseDetail = $derived(
        this.selectedPhase !== null
            ? this.activePhases.find(p => p.number === this.selectedPhase)
            : null
    );

    applyDelta(projectId: string, changes: StateChange[]) {
        // Surgical updates -- Svelte 5 proxy tracks which fields changed
        const project = this.projects.get(projectId);
        if (!project) return;
        for (const change of changes) {
            switch (change.kind) {
                case 'phase_update':
                    const phase = project.phases.find(p => p.number === change.phase);
                    if (phase) { phase.stage = change.stage; phase.progress = change.progress; }
                    break;
                case 'plan_update':
                    // ... similar surgical update
                    break;
            }
        }
    }
}

export const pipeline = new PipelineStore();
```

```typescript
// lib/stores/websocket.svelte.ts
class WebSocketManager {
    stateWs = $state<WebSocket | null>(null);
    connected = $state(false);
    reconnectAttempts = $state(0);

    connect(projects: string[]) {
        const ws = new WebSocket(`wss://${location.host}/api/ws/state`);
        ws.onopen = () => {
            this.connected = true;
            this.reconnectAttempts = 0;
            ws.send(JSON.stringify({ type: 'subscribe', projects }));
        };
        ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            if (msg.type === 'snapshot') pipeline.setSnapshot(msg.project, msg.data);
            if (msg.type === 'delta') pipeline.applyDelta(msg.project, msg.changes);
        };
        ws.onclose = () => { this.connected = false; this.scheduleReconnect(); };
    }

    scheduleReconnect() {
        const delay = Math.min(1000 * 2 ** this.reconnectAttempts, 30000);
        setTimeout(() => { this.reconnectAttempts++; this.connect(/*...*/); }, delay);
    }
}
```

**Key Svelte 5 pattern:** Use class-based stores with `$state` fields instead of Svelte 4 writable stores. Classes give us methods (applyDelta, connect) co-located with state. `$derived` computes views. `$effect` handles side effects (WebSocket lifecycle in component onMount).

**Confidence:** HIGH (Svelte 5 runes are stable, class-based stores are the recommended pattern)

### Authentication Layer

**Architecture:** Caddy + oauth2-proxy as a forward_auth middleware

```
Browser -> Caddy (TLS) -> forward_auth to oauth2-proxy -> Axum daemon
```

Caddy's `forward_auth` directive sends a sub-request to oauth2-proxy's `/oauth2/auth` endpoint. If authenticated, oauth2-proxy returns 202 with `X-Forwarded-User` and `X-Forwarded-Email` headers. Caddy copies these headers to the proxied request to Axum. If unauthenticated, oauth2-proxy returns 401 and Caddy redirects to the OAuth2 sign-in flow.

**Why this pattern:**
1. Auth is completely decoupled from the application -- Axum never handles OAuth flows
2. WebSocket connections go through the same auth check (Caddy forward_auth applies to upgrade requests)
3. Any OAuth2 provider works (GitHub, Google, GitLab, etc.) -- just configure oauth2-proxy
4. Adding/removing users is an identity provider concern, not an application concern

**Axum's role in auth:**
- Trust `X-Forwarded-User` header (only accept from Caddy's internal network)
- Look up or create user in SQLite on first request
- Scope all API responses and WebSocket subscriptions to authenticated user

```rust
// Axum extractor
struct AuthenticatedUser {
    id: UserId,
    external_id: String,
    display_name: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser {
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user_header = parts.headers.get("X-Forwarded-User")
            .ok_or(StatusCode::UNAUTHORIZED)?;
        // Look up in DB, create if new
    }
}
```

**Confidence:** HIGH (Caddy + oauth2-proxy forward_auth is a well-documented, production-proven pattern)

### Multi-User Isolation

Every database query and WebSocket subscription is scoped by `user_id`. A user only sees projects they have explicitly registered.

**Registration flow:**
1. User provides a filesystem path via the UI
2. Backend validates: path exists, contains `.planning/` directory, is not already registered by this user
3. Backend creates `projects` record, starts FileWatcher for that path
4. User sees the project appear in their sidebar

**Scoping enforcement:**
- REST API: All queries include `WHERE user_id = ?`
- WebSocket /state: Broadcaster only sends updates for user's own projects
- WebSocket /pty: PTYManager checks project ownership before spawning
- No project UUIDs are guessable -- they are server-generated

**Confidence:** HIGH (standard per-user scoping, no novel patterns needed)

## Patterns to Follow

### Pattern 1: Actor-Style Component Communication

**What:** Components (FileWatcher, Parser, PTYManager, Broadcaster) communicate via `tokio::mpsc` channels, not shared mutable state.

**When:** Always, for all inter-component communication within the daemon.

**Why:** Eliminates lock contention, makes data flow explicit, each component runs as an independent tokio task that owns its state.

```rust
// Spawn components as independent tasks
let (file_tx, file_rx) = mpsc::channel::<FileEvent>(256);
let (pty_tx, pty_rx) = mpsc::channel::<PtyCommand>(64);

tokio::spawn(file_watcher::run(file_tx));
tokio::spawn(parser::run(file_rx, db.clone(), broadcaster.clone()));
tokio::spawn(pty_manager::run(pty_rx));
```

### Pattern 2: Snapshot + Delta WebSocket Protocol

**What:** On WebSocket connect, send full state snapshot. After that, send only deltas.

**When:** For the state WebSocket endpoint.

**Why:** Client always has consistent state. Reconnection is simple (just get a new snapshot). Deltas keep bandwidth low during normal operation.

### Pattern 3: SPA Served by Backend

**What:** Build SvelteKit as a static SPA, serve it from Axum via `tower-http::ServeDir`.

**When:** Always -- do not run SvelteKit as a separate server process.

**Why:** Single binary deployment. No CORS issues. WebSocket URLs are same-origin. One port to expose through Caddy.

```rust
let spa = ServeDir::new("./frontend/build")
    .not_found_service(ServeFile::new("./frontend/build/index.html"));

let app = Router::new()
    .nest("/api", api_routes)
    .fallback_service(spa);
```

### Pattern 4: Debounced File Processing Pipeline

**What:** FileWatcher debounces -> Parser processes -> DB writes -> Broadcast publishes

**When:** For all GSD file state observation.

**Why:** GSD agents write multiple files in rapid succession. Without debounce, the parser would process intermediate states. A 200ms debounce window catches most multi-file atomic operations.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Polling for File Changes

**What:** Using a timer to periodically scan `.planning/` directories for changes.

**Why bad:** Wastes CPU, introduces latency (up to poll interval), doesn't scale with project count.

**Instead:** Use inotify via the `notify` crate. Event-driven, near-zero CPU when idle, instant detection.

### Anti-Pattern 2: Sharing PTY Across WebSocket Connections

**What:** Multiple browser tabs or users attached to the same PTY process.

**Why bad:** Input conflicts (whose keystrokes reach the terminal?), resize conflicts (different tab sizes), security boundary violation.

**Instead:** One PTY per WebSocket connection. If a user opens the same project terminal in two tabs, they get two independent PTY sessions.

### Anti-Pattern 3: Multiplexing All Terminals Over One WebSocket

**What:** Using a single WebSocket connection with channel IDs to carry multiple terminal sessions.

**Why bad:** One slow terminal blocks all terminals (head-of-line blocking). Complex protocol. Resize/close logic must be multiplexed. Browser can handle many WebSocket connections just fine.

**Instead:** One WebSocket per terminal session. Clean lifecycle, independent flow control, simpler protocol.

### Anti-Pattern 4: Using the Daemon to Orchestrate GSD

**What:** Daemon imports GSD code, calls GSD functions, or writes to GSD state files.

**Why bad:** GSD runs inside agent sessions (Claude Code, Codex). If the daemon also writes state files, you get race conditions and conflicting state. The daemon's purpose is observation, not control.

**Instead:** The daemon is read-only with respect to `.planning/` files. Users interact with GSD through terminal sessions (PTY). The daemon watches the results.

### Anti-Pattern 5: PostgreSQL for a Single-Daemon Deployment

**What:** Running a PostgreSQL server alongside the daemon for state storage.

**Why bad:** Additional service to deploy, configure, and maintain. Connection management overhead. Backup complexity. All for a single-writer workload that SQLite handles perfectly.

**Instead:** SQLite with WAL mode. Embedded, zero-config, excellent single-writer performance, concurrent reads for WebSocket handlers.

## Scalability Considerations

| Concern | 1-5 projects | 10-50 projects | 100+ projects |
|---------|-------------|----------------|---------------|
| **File watchers** | One watcher per project, trivial | inotify watch limit may need raising (`fs.inotify.max_user_watches`) | Consider watch consolidation or polling fallback |
| **WebSocket connections** | Handful of connections, no concern | Dozens of connections, still fine for Axum | May need connection limits per user |
| **PTY processes** | 5-10 PTYs, negligible resources | 20-50 PTYs, ~50MB RSS each | Enforce per-user PTY limits, idle timeout cleanup |
| **SQLite writes** | Sparse, sub-millisecond | Still fine -- single writer, WAL mode | Monitor WAL file size, tune checkpoint interval |
| **Broadcast channels** | One per project, minimal memory | 50 channels, still trivial | Lazy channel creation/cleanup for inactive projects |

The architecture is designed for the 1-50 project range (a developer or small team). For 100+ projects, the single-daemon model still works but needs resource limits and monitoring. True multi-tenant SaaS would require architectural changes (PostgreSQL, horizontal scaling) that are explicitly out of scope.

## Suggested Build Order

Based on dependency analysis, components should be built in this order:

```
Phase 1: Foundation
  SQLite schema + migrations
  Axum server skeleton (AppState, routing, static file serving)
  SvelteKit project setup (adapter-static, Aurora theme tokens)

Phase 2: State Pipeline
  FileWatcher (notify + debounce)
  Parser (all GSD file formats)
  SQLite read/write for pipeline state
  Broadcaster (tokio::broadcast)
  WebSocket /state endpoint
  REST API for projects + state

Phase 3: Frontend Dashboard
  WebSocket client (connect, subscribe, reconnect)
  Pipeline store ($state, $derived)
  Sidebar + project list
  Phase timeline component
  Phase detail + stage rail
  Plan cards

Phase 4: Terminal System
  PTYManager (portable-pty spawn, lifecycle)
  WebSocket /pty endpoint (binary frames)
  TerminalPane component (xterm.js + fit addon)
  Console tab with sub-tabs
  Agent session auto-tab creation

Phase 5: System + Polish
  System tab (host metrics, service health, logs)
  Agent configuration panel
  Multi-user isolation testing
  Auth integration (Caddy + oauth2-proxy)
  Deployment packaging
```

**Ordering rationale:**
- Phase 1 before all: Every component depends on the DB schema and server skeleton
- Phase 2 before Phase 3: Frontend needs WebSocket data to render anything meaningful
- Phase 3 before Phase 4: Pipeline view is the primary interface; terminals are secondary
- Phase 4 can partially overlap Phase 3: PTYManager backend is independent of pipeline frontend
- Phase 5 last: System tab and auth are important but not core product value

## Sources

- [Axum WebSocket documentation](https://docs.rs/axum/latest/axum/extract/ws/index.html) -- HIGH confidence
- [Axum examples: WebSocket server](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) -- HIGH confidence
- [Axum examples: static file server](https://github.com/tokio-rs/axum/blob/main/examples/static-file-server/src/main.rs) -- HIGH confidence
- [tokio::sync::broadcast documentation](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html) -- HIGH confidence
- [notify crate (file watching)](https://docs.rs/notify/) -- HIGH confidence
- [notify-debouncer-full](https://docs.rs/notify-debouncer-full) -- HIGH confidence
- [portable-pty crate](https://docs.rs/portable-pty) -- HIGH confidence
- [rusqlite](https://github.com/rusqlite/rusqlite) -- HIGH confidence
- [SQLite WAL mode](https://sqlite.org/wal.html) -- HIGH confidence
- [SQLite appropriate uses](https://sqlite.org/whentouse.html) -- HIGH confidence
- [xterm.js](https://xtermjs.org/) -- HIGH confidence
- [xterm.js flow control guide](https://xtermjs.org/docs/guides/flowcontrol/) -- HIGH confidence
- [SvelteKit adapter-static / SPA mode](https://svelte.dev/docs/kit/adapter-static) -- HIGH confidence
- [Svelte 5 runes introduction](https://svelte.dev/blog/runes) -- HIGH confidence
- [Caddy forward_auth directive](https://caddyserver.com/docs/caddyfile/directives/forward_auth) -- HIGH confidence
- [oauth2-proxy Caddy integration](https://oauth2-proxy.github.io/oauth2-proxy/next/configuration/integrations/caddy/) -- HIGH confidence
- [Svelte 5 global state patterns (Mainmatter, 2025)](https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/) -- MEDIUM confidence
- [Real-world Svelte 5 high-frequency data with runes](https://dev.to/polliog/real-world-svelte-5-handling-high-frequency-real-time-data-with-runes-3i2f) -- MEDIUM confidence
- [Building real-time WebSocket app with SvelteKit (Inngest)](https://www.inngest.com/blog/building-a-realtime-websocket-app-using-sveltekit) -- MEDIUM confidence
- [Rust state machine patterns (Hoverbear)](https://hoverbear.org/blog/rust-state-machine-pattern/) -- MEDIUM confidence
- [File watcher with debouncing in Rust (OneUptime, 2026)](https://oneuptime.com/blog/post/2026-01-25-file-watcher-debouncing-rust/view) -- MEDIUM confidence
