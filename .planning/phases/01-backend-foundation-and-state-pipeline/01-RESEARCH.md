# Phase 1: Backend Foundation and State Pipeline - Research

**Researched:** 2026-03-06
**Domain:** Rust daemon with file watching, GSD state parsing, SQLite persistence, WebSocket streaming, REST API
**Confidence:** HIGH

## Summary

Phase 1 builds the complete backend: an Axum daemon that watches `.planning/` directories via inotify, parses all GSD state files into a SQLite database, and pushes live state updates to connected WebSocket clients. This is a greenfield Rust project -- no existing code. The phase covers 16 requirements (STATE-01 through STATE-15 plus INFRA-01) and delivers three plans: server skeleton with database, file watcher with parsers, and WebSocket/REST endpoints.

The core technical challenges are: (1) correctly parsing 7 different GSD file formats with YAML frontmatter, markdown structure, and JSON, (2) debouncing rapid file changes from GSD agents without losing events or reading half-written files, (3) designing a WebSocket snapshot+delta protocol that handles reconnection correctly from day one, and (4) structuring the data pipeline so that database writes do not bottleneck real-time WebSocket broadcasts.

**Primary recommendation:** Use Axum 0.8.8 with tokio, SQLx 0.8.6 for async SQLite with embedded migrations, notify 8.2.0 with a custom debouncer (not notify-debouncer-mini or notify-debouncer-full -- use manual debouncing via tokio for finer control over per-file timing within the 50-100ms range), and `serde_yml` (not the deprecated `serde_yaml`) for YAML frontmatter parsing. Write a simple ~30-line frontmatter extractor rather than depending on `yaml-front-matter` (which depends on deprecated `serde_yaml ^0.8`) or `gray_matter` (LOW confidence). Decouple the broadcast hot path from database writes -- broadcast from in-memory parsed state, persist to SQLite asynchronously.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Parsing Resilience:**
- Mid-write files: use last-known-good state until new write is complete and parseable (never show partial/broken data)
- Unknown frontmatter fields or unexpected structure: treat as parse error and surface to user as a warning badge on the project (strict mode)
- Missing critical files (no ROADMAP.md, STATE.md deleted): show project in sidebar with 'incomplete' state badge, parse whatever is available
- Unparseable files (corrupted, wrong encoding, invalid YAML): surface as file-level error badge on the specific phase/plan, log error in daemon

**Historical Data Depth:**
- Plan-level timing: store start/end timestamps and duration per plan (from SUMMARY.md frontmatter)
- Full agent timeline: record every agent switch/handoff within a plan, not just the final agent (parsed from agent-history.json)
- Keep all execution runs: every re-execution attempt stored with timestamps, old runs marked as superseded
- Full commit details: parse individual commit hashes and messages from SUMMARY.md body, not just counts

**Bootstrap Behavior:**
- First watch of existing project: full reconstruction of all history from .planning/ files (completed phases, durations, verification results)
- Daemon restart: reconcile database against current files on startup, update any differences from changes made while daemon was down
- New project registration: immediate watch + full initial parse, project appears in UI within seconds
- Lost project directory: mark as 'offline' with badge in sidebar, preserve all historical data in DB, periodically check if directory reappears

**State Update Granularity:**
- WebSocket push model: targeted deltas only (e.g., 'phase 2 stage moved to Execute', 'plan 02-01 status: working'), frontend applies patches to local state
- Rapid file changes: debounce + batch over 50-100ms window, send one batched update per burst
- Subscription model: per-project subscriptions, client sends subscribe message for project(s) of interest
- Initial snapshot: current state + recent execution history (not just latest state)

**Data Retention:**
- Configurable retention period with 180-day default
- Global default with per-project override capability
- Pruning action: hard delete from SQLite (original .planning/ files remain on disk as backup)

**REST API Surface:**
- Serve file content via API: REST endpoints return raw markdown for PLAN.md, SUMMARY.md, VERIFICATION.md (required for remote browser access)
- Historical queries: time range + filters (date range, phase, plan, agent, status) via standard REST query params
- Full CRUD for project management: register, unregister, configure projects through REST endpoints
- API versioning from day one: /api/v1/ prefix on all routes

**Daemon Health Reporting:**
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

### Deferred Ideas (OUT OF SCOPE)

None -- discussion stayed within phase scope

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STATE-01 | Daemon watches `.planning/` directories recursively via inotify with debouncing (50-100ms) | notify 8.2.0 with custom tokio-based debouncer; see Architecture Patterns section for debounce design |
| STATE-02 | Parser extracts state from STATE.md frontmatter (status, progress, milestone, timestamps) | YAML frontmatter via serde_yml; STATE.md has `---` delimited YAML frontmatter with gsd_state_version, milestone, status, progress fields |
| STATE-03 | Parser extracts phase list from ROADMAP.md (names, goals, dependencies, completion) | Markdown parsing via regex; ROADMAP.md uses `### Phase N: Name` headings with `**Goal**:`, `**Depends on**:`, checkbox lists |
| STATE-04 | Parser extracts plan metadata from PLAN.md frontmatter (wave, depends_on, type, requirements) | YAML frontmatter via serde_yml; PLAN.md has `---` delimited frontmatter with phase, plan, wave, depends_on, files_modified, requirements, must_haves fields |
| STATE-05 | Parser extracts completion data from SUMMARY.md frontmatter (duration, key-files, requirements-completed) | YAML frontmatter via serde_yml; SUMMARY.md has `---` delimited frontmatter with duration, completed, key-files, requirements-completed, provides, affects fields |
| STATE-06 | Parser extracts commit counts from SUMMARY.md body (Task Commits section) | Markdown parsing via regex; `## Task Commits` section has numbered list items with backtick-wrapped commit hashes |
| STATE-07 | Parser derives stage per phase from file presence (CONTEXT.md, RESEARCH.md, PLAN.md, SUMMARY.md, VERIFICATION.md) | File existence checks via std::fs or tokio::fs; stage derivation logic documented in Architecture Patterns |
| STATE-08 | Parser extracts verification status from VERIFICATION.md frontmatter (passed/gaps_found/human_needed, score) | YAML frontmatter via serde_yml; VERIFICATION.md has `---` delimited frontmatter with status (enum), score fields |
| STATE-09 | Parser reads config.json for workflow settings and agent routing defaults | JSON parsing via serde_json; config.json is standard JSON with mode, granularity, workflow, planning, parallelization, gates, safety fields |
| STATE-10 | Parser reads agent-history.json for agent execution timeline | JSON parsing via serde_json; agent-history.json contains array of agent session records with timestamps |
| STATE-11 | SQLite database caches current parsed state for fast queries and page loads | SQLx 0.8.6 with sqlite feature; WAL mode; schema design in Architecture Patterns section |
| STATE-12 | Database stores historical execution metrics (durations, timelines) that persist across file changes | Separate history tables from current-state tables; see schema design |
| STATE-13 | WebSocket endpoint pushes state updates with snapshot + delta protocol | Axum WebSocket via axum::extract::ws; tokio::broadcast for fan-out; JSON text frames; see WebSocket Protocol section |
| STATE-14 | WebSocket reconnection restores full state via snapshot (no stale UI after network blip) | On connect/reconnect: query SQLite for full current state, send as snapshot message before subscribing to broadcast |
| STATE-15 | REST API serves project list, current state, and historical metrics | Axum Router with /api/v1/ prefix; JSON responses via serde_json; SQLite queries for data |
| INFRA-01 | Axum daemon serves SvelteKit static assets, REST API, and WebSocket endpoints from single binary | tower-http ServeDir for static files; Axum Router nesting /api routes; WebSocket upgrade handlers; no SvelteKit build in Phase 1 (placeholder static serving) |

</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8.8 | HTTP/WebSocket framework | Tokio-native, built-in WebSocket via `axum::extract::ws`, tower middleware. 191M+ downloads. Backed by tokio-rs team. |
| tokio | 1.x | Async runtime | Required by Axum. Powers all async I/O: file watching, WebSocket, database. No alternative in this ecosystem. |
| sqlx | 0.8.6 | Async SQLite driver | Async-native, compile-time checked queries, `sqlite` feature bundles libsqlite3 into binary. Zero system dependency. |
| notify | 8.2.0 | Filesystem events | Cross-platform (inotify on Linux). Used by cargo-watch, rust-analyzer, deno. MSRV 1.85. |
| serde | 1.x | Serialization framework | Rust standard. Every library in this stack uses it. |
| serde_json | 1.x | JSON parsing/serialization | WebSocket messages, config.json, agent-history.json, API responses. |
| serde_yml | 0.0.12+ | YAML parsing | Replacement for deprecated serde_yaml. Fork by sebastienrousseau, compatible API. For YAML frontmatter in STATE.md, PLAN.md, SUMMARY.md, VERIFICATION.md. |
| tower-http | 0.6.x | HTTP middleware | ServeDir (static files), CorsLayer, TraceLayer. Part of tower ecosystem that Axum is built on. |
| tower | 0.5.x | Service abstraction | Foundation for middleware. Required by tower-http and Axum. |
| tracing | 0.1.x | Structured logging | Tokio-ecosystem standard. Span-based tracing for debugging async tasks. |
| tracing-subscriber | 0.3.x | Log output | Provides fmt subscriber. Use EnvFilter for runtime log level control. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1.x | Unique identifiers | Project IDs, session IDs. Use v4 (random) with serde feature. |
| chrono | 0.4.x | Date/time handling | Timestamps for historical records, duration calculations. Use serde feature. |
| regex | 1.x | Pattern matching | Parsing ROADMAP.md markdown structure, SUMMARY.md commit sections. Not for frontmatter. |
| tokio-util | 0.7.x | Async utilities | CancellationToken for graceful shutdown of watcher tasks. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| serde_yml | serde_yaml 0.9 | serde_yaml is deprecated and archived. serde_yml is the maintained fork. |
| Custom frontmatter extractor | yaml-front-matter crate | yaml-front-matter depends on deprecated serde_yaml ^0.8. Custom extractor is ~30 lines. |
| Custom frontmatter extractor | gray_matter crate | gray_matter is LOW confidence, not widely adopted. Custom is more reliable. |
| Custom debouncer (tokio) | notify-debouncer-mini | Mini doesn't provide event type info (create/modify/delete needed for stage derivation). |
| Custom debouncer (tokio) | notify-debouncer-full | Full's debounce timing is not easily tunable to the 50-100ms range. Custom gives precise control. |
| SQLx | rusqlite | rusqlite is sync-only. Mixing sync DB calls with async Axum requires spawn_blocking wrappers everywhere. |
| SQLx | diesel | Heavy ORM with code generation. Overkill for ~10 tables. |

**Installation (Cargo.toml):**
```toml
[package]
name = "gsdui"
version = "0.1.0"
edition = "2024"

[dependencies]
# Web framework
axum = { version = "0.8", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["fs", "cors", "compression-gzip", "trace"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }

# File watching
notify = "8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yml = "0.0.12"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
regex = "1"
tokio-util = "0.7"
```

## Architecture Patterns

### Recommended Project Structure
```
gsdui/
+-- Cargo.toml
+-- build.rs                    # cargo:rerun-if-changed=migrations
+-- migrations/
|   +-- 001_initial_schema.sql
+-- src/
|   +-- main.rs                 # Entry point, server startup, graceful shutdown
|   +-- config.rs               # CLI args, env vars, daemon configuration
|   +-- state.rs                # AppState struct (shared across handlers)
|   +-- db/
|   |   +-- mod.rs              # Database pool initialization, WAL mode setup
|   |   +-- schema.rs           # Query functions (reads and writes)
|   |   +-- models.rs           # Database row types (FromRow structs)
|   +-- watcher/
|   |   +-- mod.rs              # FileWatcher: notify setup, event dispatch
|   |   +-- debounce.rs         # Custom tokio-based per-file debouncer
|   +-- parser/
|   |   +-- mod.rs              # Parse dispatcher (route file events to parsers)
|   |   +-- frontmatter.rs      # YAML frontmatter extractor (~30 lines)
|   |   +-- state_md.rs         # STATE.md parser
|   |   +-- roadmap.rs          # ROADMAP.md parser (markdown structure)
|   |   +-- plan.rs             # PLAN.md parser (frontmatter)
|   |   +-- summary.rs          # SUMMARY.md parser (frontmatter + commit section)
|   |   +-- verification.rs     # VERIFICATION.md parser (frontmatter)
|   |   +-- config_json.rs      # config.json parser
|   |   +-- agent_history.rs    # agent-history.json parser
|   |   +-- stage.rs            # Stage derivation from file presence
|   +-- broadcast.rs            # Broadcaster: tokio::broadcast per project
|   +-- ws/
|   |   +-- mod.rs              # WebSocket handler, subscribe/unsubscribe
|   |   +-- messages.rs         # Message types (Snapshot, Delta, Health, Subscribe)
|   +-- api/
|   |   +-- mod.rs              # Router setup with /api/v1/ prefix
|   |   +-- projects.rs         # CRUD for project registration
|   |   +-- state_api.rs        # Current state queries
|   |   +-- history.rs          # Historical metrics queries
|   |   +-- files.rs            # Raw file content endpoints
|   |   +-- health.rs           # Health/diagnostics endpoint
+-- static/                     # Placeholder for SvelteKit build (Phase 2)
|   +-- index.html              # Minimal placeholder page
```

### Pattern 1: Actor-Style Component Communication via Channels

**What:** Components communicate via `tokio::mpsc` channels, not shared mutable state. Each component runs as an independent tokio task.

**When to use:** All inter-component communication (FileWatcher -> Parser, Parser -> DB writer, Parser -> Broadcaster).

**Example:**
```rust
// Source: tokio channels documentation + Axum chat example pattern
use tokio::sync::{mpsc, broadcast};

// File events from watcher to parser
let (file_tx, file_rx) = mpsc::channel::<FileEvent>(256);

// State updates from parser to broadcaster (and async DB writer)
let (state_tx, _) = broadcast::channel::<StateUpdate>(128);

// Spawn independent tasks
tokio::spawn(watcher::run(file_tx, cancel_token.clone()));
tokio::spawn(parser::run(file_rx, state_tx.clone(), db_pool.clone()));
```

### Pattern 2: Custom Per-File Debouncer Using Tokio

**What:** Instead of using notify-debouncer-mini/full, implement a custom debouncer using `tokio::time::sleep` and a `HashMap<PathBuf, JoinHandle>` that resets the timer on each new event for the same path.

**When to use:** For all file change events from notify.

**Why custom:** The user requires 50-100ms debounce (much shorter than notify-debouncer-full's typical 2s default), per-file debouncing (not global), and we need to batch multiple path changes into a single update burst. Custom gives precise control.

**Example:**
```rust
// Source: custom pattern based on tokio documentation
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

struct Debouncer {
    pending: HashMap<PathBuf, tokio::task::JoinHandle<()>>,
    output: mpsc::Sender<Vec<PathBuf>>,
    delay: Duration,
}

impl Debouncer {
    fn handle_event(&mut self, path: PathBuf) {
        // Cancel existing timer for this path
        if let Some(handle) = self.pending.remove(&path) {
            handle.abort();
        }
        let tx = self.output.clone();
        let delay = self.delay;
        let p = path.clone();
        // Start new timer
        let handle = tokio::spawn(async move {
            sleep(delay).await;
            let _ = tx.send(vec![p]).await;
        });
        self.pending.insert(path, handle);
    }
}
```

### Pattern 3: Snapshot + Delta WebSocket Protocol

**What:** On WebSocket connect, send full state snapshot from SQLite. After that, send only targeted deltas via broadcast channel.

**When to use:** For the `/api/v1/ws/state` endpoint.

**Why:** Client always has consistent state. Reconnection is simple (new snapshot). Deltas keep bandwidth low. This is a locked user decision.

**Example message types:**
```rust
// Source: user decisions + Axum WebSocket patterns
#[derive(Serialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "snapshot")]
    Snapshot {
        project: String,
        data: ProjectState,
    },
    #[serde(rename = "delta")]
    Delta {
        project: String,
        changes: Vec<StateChange>,
    },
    #[serde(rename = "health")]
    Health {
        uptime_secs: u64,
        db_size_bytes: u64,
        ws_client_count: u32,
        watcher_queue_depth: u32,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { projects: Vec<String> },
    #[serde(rename = "unsubscribe")]
    Unsubscribe { projects: Vec<String> },
}
```

### Pattern 4: Decoupled Broadcast from Database Writes

**What:** The parser broadcasts state changes to WebSocket clients from in-memory parsed state, then writes to SQLite asynchronously. The database is NOT in the hot path for real-time updates.

**When to use:** Always. This prevents the database write serialization from delaying WebSocket broadcasts.

**Why:** SQLite serializes all writes (even in WAL mode). If every file change triggers a synchronous DB write before broadcasting, updates lag. By broadcasting first and persisting second, real-time updates are near-instant.

**Example:**
```rust
// In parser task:
async fn process_file_change(
    path: &Path,
    broadcast_tx: &broadcast::Sender<StateUpdate>,
    db_pool: &SqlitePool,
) {
    let parsed = parse_file(path).await;

    // 1. Broadcast immediately (hot path)
    let _ = broadcast_tx.send(StateUpdate::from(&parsed));

    // 2. Persist to database (cold path, can batch)
    if let Err(e) = db::write_state(&db_pool, &parsed).await {
        tracing::error!("DB write failed for {}: {}", path.display(), e);
    }
}
```

### Pattern 5: YAML Frontmatter Extraction (Custom, No External Crate)

**What:** A simple function that splits `---`-delimited YAML frontmatter from markdown content and deserializes it using `serde_yml`.

**Why custom:** The `yaml-front-matter` crate depends on deprecated `serde_yaml ^0.8`. The `gray_matter` crate has LOW confidence. The extraction logic is trivially simple (~30 lines).

**Example:**
```rust
use serde::de::DeserializeOwned;

pub struct Document<T> {
    pub metadata: T,
    pub content: String,
}

pub fn parse_frontmatter<T: DeserializeOwned>(input: &str) -> Result<Document<T>, ParseError> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return Err(ParseError::NoFrontmatter);
    }

    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")
        .ok_or(ParseError::UnclosedFrontmatter)?;

    let yaml_str = &after_first[..end];
    let content_start = end + 4; // skip \n---
    let content = after_first[content_start..].trim_start().to_string();

    let metadata: T = serde_yml::from_str(yaml_str)
        .map_err(ParseError::YamlError)?;

    Ok(Document { metadata, content })
}
```

### Pattern 6: GSD Stage Derivation from File Presence

**What:** Determine the current stage of a phase by checking which files exist in the phase directory. This mirrors GSD's own logic.

**When to use:** After any file change in a phase directory.

**Stage derivation rules (from GSD source):**
```
no phase directory          -> Planned (phase exists in ROADMAP but no directory)
CONTEXT.md exists           -> Discussed
RESEARCH.md exists          -> Researched
PLAN.md exists              -> Planned (execution-ready)
SUMMARY.md exists (partial) -> Executing (some plans have summaries, not all)
All SUMMARY.md complete     -> Executed (every plan has a summary)
VERIFICATION.md exists      -> Verified (read pass/fail from frontmatter)
```

**Implementation note:** "Partial SUMMARY.md" means some `{phase}-{plan}-SUMMARY.md` files exist but not all expected plans (from ROADMAP.md plan list) have summaries. The parser must cross-reference the plan list from ROADMAP.md against existing SUMMARY.md files.

### Anti-Patterns to Avoid

- **Parsing on raw inotify MODIFY events without debounce:** MODIFY fires mid-write. Always debounce 50-100ms and validate parsed output before storing. Discard empty/null results, retain last-known-good state.
- **Synchronous database writes in the broadcast path:** Puts SQLite write lock in the hot path. Broadcast from in-memory state, write to DB asynchronously.
- **Global debounce instead of per-file debounce:** A change to `STATE.md` should not wait for `PLAN.md` writes to settle. Each file path gets its own debounce timer.
- **Single broadcast channel for all projects:** Wastes bandwidth (clients receive updates for unsubscribed projects). Use one `broadcast::Sender` per project, clients subscribe only to their projects.
- **Polling for file changes:** Wastes CPU, introduces latency. Use inotify via notify crate. Reserve polling only as a fallback when inotify watch registration fails.
- **Holding database read transactions open during WebSocket sends:** Prevents WAL checkpointing. Query, close transaction, then send data over WebSocket.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP routing + WebSocket upgrade | Custom HTTP server | Axum 0.8 | Handles routing, extractors, middleware, WebSocket upgrade, tower integration. Hundreds of edge cases. |
| inotify event handling | Raw `inotify` crate | `notify` 8.2 | Handles recursive watching, event coalescing, cross-platform abstraction. Tested by cargo-watch, rust-analyzer. |
| SQLite connection pooling | Manual connection management | SQLx pool | Handles async connection lifecycle, WAL mode setup, busy timeout, prepared statement caching. |
| Async runtime | Custom event loop | tokio | Powers everything. Channels, timers, spawning, I/O. No alternative. |
| JSON serialization | Manual JSON building | serde_json | Type-safe, derived serialization. No reason to ever build JSON strings manually. |
| YAML deserialization | Regex-based YAML parser | serde_yml | YAML is deceptively complex (anchors, multiline strings, type coercion). Use a real parser. |
| UUID generation | Custom ID scheme | uuid crate v4 | Cryptographically random, standard format, serde integration. |

**Key insight:** The frontmatter extraction (splitting `---` delimiters) is trivially simple enough to hand-roll. The YAML parsing within that frontmatter is NOT -- use serde_yml. The markdown structure parsing (ROADMAP.md headings, SUMMARY.md commit sections) is regex-appropriate since we control the format and it's well-defined.

## Common Pitfalls

### Pitfall 1: File Watcher Race Conditions (Half-Written Files)

**What goes wrong:** inotify fires MODIFY mid-write. Parser reads half-written STATE.md, gets empty/invalid frontmatter. Database records broken state that propagates to WebSocket clients, causing UI flicker.

**Why it happens:** File writes are not atomic. GSD agents write in stages (truncate, write partial, flush, close). Editors use different patterns (vim: write-to-temp-then-rename, VS Code: atomic write).

**How to avoid:**
1. Debounce per-file at 50-100ms (user-specified range)
2. Validate parsed output: if frontmatter parses to empty/null, discard and retain last-known-good state (locked user decision)
3. Handle both MODIFY and CLOSE_WRITE events; prefer CLOSE_WRITE when available
4. For rename-based writes (common in editors), handle MOVED_TO events

**Warning signs:** Pipeline UI flickers, database contains null fields that self-correct, intermittent parse errors in logs.

### Pitfall 2: inotify Watch Exhaustion

**What goes wrong:** Each subdirectory in `.planning/` requires a separate inotify watch. With many projects, the per-user watch limit is hit. New watches silently fail.

**Why it happens:** Default `max_user_watches` is 8192 on older kernels. The notify crate may log errors at debug level, not surfacing them clearly.

**How to avoid:**
1. On startup, check `/proc/sys/fs/inotify/max_user_watches` and log warning if below 65536
2. Watch only `.planning/` root and known subdirectory patterns, not entire project trees
3. Handle watch registration errors explicitly -- surface to user as project-level error
4. Consider polling fallback for directories where inotify fails

**Warning signs:** "No space left on device" errors (confusing errno for inotify exhaustion), some projects stop updating.

### Pitfall 3: Database as Bottleneck in Hot Path

**What goes wrong:** Every file change triggers synchronous DB write then broadcast. SQLite serializes all writes. During active GSD execution, dozens of files change per second. DB becomes the bottleneck, state updates lag seconds behind actual changes.

**Why it happens:** SQLite has a single writer even in WAL mode. Developers test with single-project scenarios and never see contention.

**How to avoid:**
1. Broadcast from in-memory parsed state, NOT from DB write completion
2. Batch DB writes in 50-100ms windows (same debounce window as file events)
3. Set `busy_timeout = 5000` for brief contention handling
4. Keep read transactions short-lived to prevent WAL checkpoint blocking
5. Monitor WAL file size -- if it grows beyond a few MB, investigate

**Warning signs:** Updates lag >500ms behind file changes, "database is locked" errors, WAL file grows to hundreds of MB.

### Pitfall 4: WebSocket Reconnection State Loss

**What goes wrong:** Network blips cause WebSocket drop. Client reconnects but has stale state. Without resync, UI shows outdated information.

**Why it happens:** WebSocket is fire-and-forget. Messages during disconnection are lost. Reconnection without state resync gives false confidence.

**How to avoid:**
1. On connect/reconnect, always send full state snapshot from SQLite (locked user decision)
2. Snapshot includes current state + recent execution history (locked user decision)
3. Display visible "Reconnecting..." indicator during disconnection (future frontend concern, but protocol must support it)

**Warning signs:** "UI shows wrong state after laptop wake" reports, no visual indication during disconnection.

### Pitfall 5: serde_yaml Dependency Chain

**What goes wrong:** Using `yaml-front-matter` or `gray_matter` pulls in deprecated `serde_yaml` which receives no security updates and may have breaking changes or vulnerabilities.

**Why it happens:** These crates were written against `serde_yaml` before its deprecation in March 2024. They haven't been updated.

**How to avoid:** Use `serde_yml` directly with a custom frontmatter extractor. The extraction logic is ~30 lines. Never depend on `serde_yaml`.

**Warning signs:** Cargo warning about deprecated dependencies, build failures as serde_yaml becomes incompatible with newer Rust editions.

## Code Examples

### Axum Server Skeleton with State

```rust
// Source: Axum 0.8 documentation + chat example
use axum::{Router, routing::get};
use std::sync::Arc;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

struct AppState {
    db: SqlitePool,
    broadcaster: Broadcaster,
    // file_watcher_handle for adding/removing watches
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("gsdui=debug,tower_http=info")
        .init();

    let db = SqlitePool::connect("sqlite:data/gsdui.db?mode=rwc")
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // Enable WAL mode
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&db).await.unwrap();
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&db).await.unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&db).await.unwrap();
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(&db).await.unwrap();

    let state = Arc::new(AppState {
        db,
        broadcaster: Broadcaster::new(),
    });

    let app = Router::new()
        .nest("/api/v1", api_routes())
        .route("/api/v1/ws/state", get(ws_handler))
        .fallback_service(
            tower_http::services::ServeDir::new("static")
                .not_found_service(
                    tower_http::services::ServeFile::new("static/index.html")
                )
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
```

### WebSocket Handler with Broadcast

```rust
// Source: Axum 0.8 WebSocket docs + chat example pattern
use axum::extract::ws::{WebSocket, WebSocketUpgrade, Message};
use axum::extract::State;
use axum::response::Response;
use std::sync::Arc;

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for subscribe message from client
    let subscribed_projects = match receiver.next().await {
        Some(Ok(Message::Text(text))) => {
            // Parse subscribe message, extract project IDs
            parse_subscribe(&text)
        }
        _ => return, // Invalid first message, close
    };

    // Send initial snapshot for each subscribed project
    for project_id in &subscribed_projects {
        let snapshot = state.db.get_project_state(project_id).await;
        let msg = WsMessage::Snapshot {
            project: project_id.clone(),
            data: snapshot,
        };
        let json = serde_json::to_string(&msg).unwrap();
        if sender.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    // Subscribe to broadcast channels
    let mut rx = state.broadcaster.subscribe(&subscribed_projects);

    // Spawn send task (broadcast -> client)
    let mut send_task = tokio::spawn(async move {
        while let Ok(update) = rx.recv().await {
            let json = serde_json::to_string(&update).unwrap();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Spawn receive task (client -> server, for unsubscribe etc.)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // Handle unsubscribe, ping/pong
        }
    });

    // If either task finishes, abort the other
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
```

### SQLite Schema (Initial Migration)

```sql
-- Source: user decisions on historical data depth + data retention
-- migrations/001_initial_schema.sql

-- Projects registered for monitoring
CREATE TABLE projects (
    id TEXT PRIMARY KEY,              -- UUID
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,        -- Filesystem path to project root
    status TEXT NOT NULL DEFAULT 'active',  -- active, offline
    retention_days INTEGER DEFAULT 180,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Current parsed state per phase (cache of parsed files, replaced on each parse)
CREATE TABLE phase_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,       -- "01", "02", "02.1" (supports decimals)
    phase_name TEXT NOT NULL,
    goal TEXT,
    depends_on TEXT,                  -- JSON array of phase numbers
    stage TEXT NOT NULL DEFAULT 'planned',  -- planned, discussed, researched, planned_ready, executing, executed, verified
    status TEXT,                      -- from ROADMAP.md progress table
    requirements TEXT,               -- JSON array of requirement IDs
    plan_count INTEGER DEFAULT 0,
    completed_plan_count INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, phase_number)
);

-- Current parsed state per plan (cache of parsed PLAN.md/SUMMARY.md)
CREATE TABLE plan_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    plan_number TEXT NOT NULL,        -- "01", "02"
    plan_name TEXT,
    wave INTEGER,
    depends_on TEXT,                  -- JSON array of plan IDs
    plan_type TEXT DEFAULT 'execute',
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, working, done, failed
    requirements TEXT,               -- JSON array of requirement IDs
    files_modified TEXT,             -- JSON array
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, phase_number, plan_number)
);

-- Execution history (persists across file changes, supports "keep all runs")
CREATE TABLE execution_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    plan_number TEXT NOT NULL,
    run_number INTEGER NOT NULL DEFAULT 1,
    superseded INTEGER NOT NULL DEFAULT 0,  -- 1 if a later run exists
    started_at TEXT,
    completed_at TEXT,
    duration_minutes REAL,
    status TEXT,                      -- completed, failed, in_progress
    key_files_created TEXT,           -- JSON array
    key_files_modified TEXT,          -- JSON array
    requirements_completed TEXT,      -- JSON array
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Individual commit records from SUMMARY.md Task Commits section
CREATE TABLE commits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_run_id INTEGER NOT NULL REFERENCES execution_runs(id) ON DELETE CASCADE,
    task_number INTEGER NOT NULL,
    task_name TEXT,
    commit_hash TEXT,
    commit_type TEXT,                 -- feat, fix, test, refactor, docs
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Agent session history from agent-history.json
CREATE TABLE agent_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    agent_id TEXT,
    agent_type TEXT,                  -- claude, codex, gemini
    phase_number TEXT,
    plan_number TEXT,
    started_at TEXT,
    ended_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Verification results from VERIFICATION.md
CREATE TABLE verification_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    phase_number TEXT NOT NULL,
    status TEXT NOT NULL,             -- passed, gaps_found, human_needed
    score TEXT,                       -- "N/M"
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id, phase_number)
);

-- Parse errors (accumulated with timestamps, queryable)
CREATE TABLE parse_errors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    error_message TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',  -- warning, error
    occurred_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
);

-- Project configuration cache (from config.json)
CREATE TABLE project_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    config_json TEXT NOT NULL,        -- Full config.json content
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(project_id)
);

-- Indexes for common query patterns
CREATE INDEX idx_phase_state_project ON phase_state(project_id);
CREATE INDEX idx_plan_state_project ON plan_state(project_id, phase_number);
CREATE INDEX idx_execution_runs_project ON execution_runs(project_id, phase_number, plan_number);
CREATE INDEX idx_agent_sessions_project ON agent_sessions(project_id);
CREATE INDEX idx_parse_errors_project ON parse_errors(project_id, resolved_at);
CREATE INDEX idx_commits_run ON commits(execution_run_id);
```

### Notify File Watcher Setup

```rust
// Source: notify 8.2.0 docs
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

pub async fn watch_project(
    path: &Path,
    event_tx: mpsc::Sender<(String, Event)>,
    project_id: String,
) -> Result<RecommendedWatcher, notify::Error> {
    let planning_dir = path.join(".planning");

    let tx = event_tx.clone();
    let pid = project_id.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            if let Ok(event) = result {
                let _ = tx.blocking_send((pid.clone(), event));
            }
        },
        Config::default(),
    )?;

    watcher.watch(&planning_dir, RecursiveMode::Recursive)?;

    tracing::info!(
        project_id = %project_id,
        path = %planning_dir.display(),
        "Started watching project"
    );

    Ok(watcher)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `serde_yaml` for YAML parsing | `serde_yml` (maintained fork) | March 2024 | Must use serde_yml. serde_yaml is archived and receives no updates. |
| `yaml-front-matter` crate | Custom frontmatter extractor + serde_yml | 2024+ | yaml-front-matter depends on serde_yaml ^0.8 (very old). Write ~30 lines instead. |
| `notify-debouncer-mini` 0.4 | notify 8.2 + custom tokio debouncer | 2025 | Custom debouncer gives precise 50-100ms timing and per-file control. |
| Axum 0.7 | Axum 0.8.8 | January 2025 | 0.8 has improved WebSocket API, tower 0.5 compatibility. |
| Rust Edition 2021 | Rust Edition 2024 (1.85+) | February 2025 | New edition with improved ergonomics. |

**Deprecated/outdated:**
- `serde_yaml`: Archived March 2024. Use `serde_yml` instead.
- `yaml-front-matter`: Depends on deprecated `serde_yaml ^0.8`. Do not use.
- `gray_matter`: LOW confidence, not widely adopted. Use custom extractor.
- `notify-debouncer-mini` / `notify-debouncer-full`: These work but don't give the precise timing control needed. Custom tokio-based debouncer is better for this use case.

## Open Questions

1. **GSD Frontmatter Format: YAML vs TOML**
   - What we know: STATE.md uses YAML frontmatter (`---` delimiters). PLAN.md and SUMMARY.md templates show `---` delimiters (YAML). The project-level STACK.md research mentions `toml` crate for "TOML frontmatter" but actual GSD templates all use YAML.
   - What's unclear: The project research listed `toml` crate for "parsing frontmatter from STATE.md" but STATE.md actually uses YAML. Is there any GSD file that uses TOML frontmatter?
   - Recommendation: Use YAML (serde_yml) for all frontmatter. This matches the actual file templates. If TOML frontmatter is encountered, add support later. Do NOT include the `toml` crate in initial dependencies.

2. **agent-history.json Schema**
   - What we know: The file exists and contains agent session records with timestamps. User wants "full agent timeline" with every switch/handoff.
   - What's unclear: The exact JSON schema of agent-history.json. No template was found in GSD templates.
   - Recommendation: Implement a flexible parser that handles unknown fields gracefully. Define a best-guess struct based on what we know (agent_id, agent_type, started_at, ended_at, phase, plan), but use `serde_json::Value` fallback for unknown structure. Log warnings for unrecognized fields per user's strict mode decision.

3. **STATE.md Frontmatter vs Body Parsing**
   - What we know: STATE.md has YAML frontmatter with structured fields (gsd_state_version, milestone, status, progress) AND a markdown body with "Current Position", "Performance Metrics" sections.
   - What's unclear: Does the parser need to extract information from both frontmatter AND body? The body contains metrics, decisions, blockers that are not in frontmatter.
   - Recommendation: Parse frontmatter for primary state. Parse body sections (Current Position, Performance Metrics) via regex for supplementary data. This matches the "comprehensive historical data" user decision.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework + cargo test |
| Config file | none -- Cargo.toml `[dev-dependencies]` section |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo test` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STATE-01 | File watcher detects changes within 100ms | integration | `cargo test --test watcher_integration` | Wave 0 |
| STATE-02 | STATE.md frontmatter parsed correctly | unit | `cargo test parser::state_md::tests` | Wave 0 |
| STATE-03 | ROADMAP.md parsed into phase list | unit | `cargo test parser::roadmap::tests` | Wave 0 |
| STATE-04 | PLAN.md frontmatter parsed correctly | unit | `cargo test parser::plan::tests` | Wave 0 |
| STATE-05 | SUMMARY.md frontmatter parsed correctly | unit | `cargo test parser::summary::tests` | Wave 0 |
| STATE-06 | SUMMARY.md commit extraction works | unit | `cargo test parser::summary::tests::commits` | Wave 0 |
| STATE-07 | Stage derivation from file presence | unit | `cargo test parser::stage::tests` | Wave 0 |
| STATE-08 | VERIFICATION.md parsed correctly | unit | `cargo test parser::verification::tests` | Wave 0 |
| STATE-09 | config.json parsed correctly | unit | `cargo test parser::config_json::tests` | Wave 0 |
| STATE-10 | agent-history.json parsed correctly | unit | `cargo test parser::agent_history::tests` | Wave 0 |
| STATE-11 | SQLite stores parsed state correctly | integration | `cargo test --test db_integration` | Wave 0 |
| STATE-12 | Historical metrics persist across changes | integration | `cargo test --test db_integration::history` | Wave 0 |
| STATE-13 | WebSocket snapshot + delta protocol | integration | `cargo test --test ws_integration` | Wave 0 |
| STATE-14 | Reconnection restores fresh snapshot | integration | `cargo test --test ws_integration::reconnect` | Wave 0 |
| STATE-15 | REST API returns project list and state | integration | `cargo test --test api_integration` | Wave 0 |
| INFRA-01 | Single binary serves static + API + WS | integration | `cargo test --test server_integration` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --lib` (unit tests only, fast)
- **Per wave merge:** `cargo test` (full suite including integration)
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/watcher_integration.rs` -- covers STATE-01 (file watcher timing)
- [ ] `tests/db_integration.rs` -- covers STATE-11, STATE-12 (database persistence)
- [ ] `tests/ws_integration.rs` -- covers STATE-13, STATE-14 (WebSocket protocol)
- [ ] `tests/api_integration.rs` -- covers STATE-15, INFRA-01 (REST API + server)
- [ ] Unit test modules in each `src/parser/*.rs` file -- covers STATE-02 through STATE-10
- [ ] Test fixtures: sample `.planning/` directory with all GSD file types for parser testing
- [ ] `[dev-dependencies]` in Cargo.toml: `tokio-test`, `tempfile`, `axum-test` (or `reqwest` for HTTP testing)

## Sources

### Primary (HIGH confidence)
- [Axum 0.8.8 docs.rs](https://docs.rs/axum/latest/axum/) -- WebSocket API, Router, extractors
- [Axum WebSocket example](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) -- Handler patterns
- [Axum chat example](https://github.com/tokio-rs/axum/blob/main/examples/chat/src/main.rs) -- Broadcast + WebSocket fan-out
- [SQLx 0.8.6 docs.rs](https://docs.rs/sqlx/latest/sqlx/) -- Async SQLite, migrations
- [sqlx::migrate!() macro](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html) -- Embedded migrations
- [notify 8.2.0 docs.rs](https://docs.rs/notify/latest/notify/) -- File watching API
- [notify-debouncer-full 0.7.0 docs.rs](https://docs.rs/notify-debouncer-full/latest/notify_debouncer_full/) -- Debouncer API (reference only, using custom)
- [tokio::sync::broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html) -- Channel for WebSocket fan-out
- [SQLite WAL mode](https://sqlite.org/wal.html) -- Write concurrency, checkpoint behavior
- [GSD templates](file:///home/sophia/.claude/get-shit-done/templates/) -- Authoritative GSD file format definitions (state.md, roadmap.md, summary.md, verification-report.md, phase-prompt.md, config.json)

### Secondary (MEDIUM confidence)
- [serde_yml GitHub](https://github.com/sebastienrousseau/serde_yml) -- serde_yaml replacement
- [serde_yaml deprecation discussion](https://users.rust-lang.org/t/serde-yaml-deprecation-alternatives/108868) -- Community discussion of alternatives
- [yaml-front-matter crate](https://docs.rs/yaml-front-matter/latest/yaml_front_matter/) -- API reference (NOT using due to deprecated dependency)
- [File watcher with debouncing in Rust (OneUptime, 2026)](https://oneuptime.com/blog/post/2026-01-25-file-watcher-debouncing-rust/view) -- Custom debounce patterns
- [Axum WebSocket state sharing discussion](https://github.com/tokio-rs/axum/discussions/3043) -- State management patterns

### Tertiary (LOW confidence)
- agent-history.json schema -- no authoritative template found; parser must be flexible

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all crates verified on crates.io/docs.rs with current versions. serde_yaml deprecation identified and mitigated.
- Architecture: HIGH -- patterns proven by Axum official examples (chat example), project-level research validates approach
- Pitfalls: HIGH -- file watcher race conditions, DB bottleneck, WebSocket reconnection all documented with specific prevention strategies from project-level pitfalls research
- GSD file formats: HIGH -- verified against actual GSD template files in ~/.claude/get-shit-done/templates/

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable crate ecosystem, 30-day validity)

---
*Phase: 01-backend-foundation-and-state-pipeline*
*Researched: 2026-03-06*
