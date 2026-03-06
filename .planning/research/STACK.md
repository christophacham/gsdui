# Technology Stack

**Project:** GSD Pipeline UI
**Researched:** 2026-03-06
**Overall confidence:** HIGH

## Recommended Stack

### Backend Runtime & Framework

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Rust | 1.85+ (Edition 2024) | Language | Memory safety without GC, async performance for PTY multiplexing + WebSocket fan-out, single binary deployment. A dashboard daemon managing PTY sessions, file watchers, and WebSocket streams is exactly where Rust's zero-cost abstractions pay off. | HIGH |
| Axum | 0.8.x (latest 0.8.8) | HTTP/WebSocket framework | Tokio-native, built-in WebSocket support via `axum::extract::ws`, tower middleware ecosystem. 191M+ downloads, backed by tokio-rs team. The 0.8 release (Jan 2025) is production-stable. Actix-Web is the only real alternative but Axum's tower integration means middleware is reusable across the ecosystem. | HIGH |
| Tokio | 1.x | Async runtime | Required by Axum; also powers PTY I/O, file watching, and WebSocket streaming. The entire async Rust ecosystem gravitates to Tokio. No reason to consider alternatives. | HIGH |

### PTY Management

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| pty-process | 0.5.x | PTY spawning + I/O | Wraps `tokio::process::Command` with PTY allocation. Implements `AsyncRead`/`AsyncWrite` on the PTY handle, which maps directly to xterm.js WebSocket bridges. Enable the `async` feature flag for tokio integration. Lightweight, focused, maintained by doy (author of vt100-rust). | HIGH |

**Alternative considered: portable-pty (0.9.0)**
portable-pty (from the wezterm project) provides cross-platform PTY support including Windows ConPTY. However, this project targets Linux-only deployment (remote server), so portable-pty's cross-platform abstraction adds unnecessary complexity. pty-process is simpler and directly integrates with tokio's async model. Use portable-pty only if Windows support ever becomes a requirement.

### Database

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| SQLite | (bundled) | Persistent store | Single-file database, zero-ops, perfect for a daemon that runs on one server. No connection pool overhead, no separate service. Handles the read-heavy workload (dashboard queries) and moderate writes (state change events) with ease. | HIGH |
| SQLx | 0.8.x (latest 0.8.6) | Database driver | Async, compile-time checked queries, no DSL/ORM overhead. Use with `features = ["runtime-tokio", "sqlite"]` and the `sqlite` feature bundles SQLite into the binary (zero system dependency). Prefer raw SQL over an ORM -- the schema is simple (projects, phases, stages, plans, events, metrics). | HIGH |

**Alternatives considered:**
- **Rusqlite** -- Excellent for SQLite but synchronous only. Since everything else is async (Axum, tokio, PTY), mixing sync DB calls creates friction (spawn_blocking wrappers everywhere). SQLx is async-native.
- **Diesel** -- Heavy ORM with code generation. Overkill for ~8 tables. The compile-time query checking in SQLx provides similar safety without schema DSL.
- **SeaORM** -- Async ORM built on SQLx. Adds abstraction we don't need. This project benefits from direct SQL for flexibility with SQLite-specific features (WAL mode, JSON functions).
- **PostgreSQL** -- Would require a separate service. This is a single-server daemon, not a distributed system. SQLite in WAL mode handles concurrent reads fine.

### File Watching

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| notify | 8.x (latest 8.2.0) | Filesystem events | Cross-platform (uses inotify on Linux). Used by alacritty, cargo-watch, rust-analyzer, deno, mdBook. Mature, battle-tested. MSRV 1.85 aligns with Rust 2024 edition. | HIGH |
| notify-debouncer-mini | 0.7.x | Event debouncing | GSD writes multiple files in rapid succession during state transitions. Debouncing prevents redundant re-parses. The mini variant is lightweight; the full variant adds file ID tracking which is unnecessary here. | HIGH |

**Why not raw inotify?** The `inotify` crate is Linux-only and lower-level. `notify` wraps it with a sensible API and handles edge cases (recursive watching, event coalescing). No reason to go lower.

### Serialization & Data

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| serde | 1.x | Serialization framework | The Rust serialization standard. Every library in this stack uses it. | HIGH |
| serde_json | 1.x | JSON serialization | WebSocket messages, config files, API responses. | HIGH |
| toml | 0.8.x | TOML parsing | For parsing frontmatter from STATE.md, PLAN.md, SUMMARY.md files (GSD uses TOML frontmatter). | MEDIUM |
| gray_matter | 0.2.x | Frontmatter extraction | Extract TOML/YAML frontmatter from markdown files. Verify this crate is current -- may need to use a simple regex-based extractor instead. | LOW |

### Observability

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tracing | 0.1.x | Structured logging/tracing | Tokio-ecosystem standard. Span-based tracing is critical for debugging async PTY sessions and WebSocket connections. Replaces `log` crate. | HIGH |
| tracing-subscriber | 0.3.x | Log output formatting | Provides `fmt` subscriber for console output. Use `EnvFilter` for runtime log level control. | HIGH |

### HTTP Middleware (tower-http)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tower-http | 0.6.x | HTTP middleware | Part of the tower ecosystem. Provides `ServeDir` (static file serving with SPA fallback), `CorsLayer`, `CompressionLayer`, `TraceLayer`. Axum is built on tower, so this is the natural middleware layer. | HIGH |
| tower | 0.5.x | Service abstraction | Foundation for middleware composition. Required by tower-http and Axum. | HIGH |

---

### Frontend Framework

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Svelte | 5.x (latest ~5.45) | UI framework | Compiled reactivity (no virtual DOM) means lower overhead for the high-frequency state updates this dashboard needs. Runes system ($state, $derived, $effect) provides explicit, predictable reactivity that's ideal for WebSocket-driven state. Smaller bundle than React (~4KB vs ~45KB for equivalent app). The dashboard doesn't need React's ecosystem breadth -- it needs fast updates and small code. | HIGH |
| SvelteKit | 2.x (latest ~2.53) | App framework | File-based routing, build tooling, dev server. Use **adapter-static** in SPA mode (ssr=false) because all data comes via WebSocket from the Axum backend -- there is no server-side rendering need. The static build outputs plain HTML/JS/CSS that Axum serves via tower-http's ServeDir. | HIGH |
| Vite | 6.x | Build tool | Bundled with SvelteKit. Fast HMR in development, optimized production builds. | HIGH |

**Why SPA mode with adapter-static?**
This dashboard has zero SEO requirements and all data arrives via WebSocket. SSR would mean running a Node.js server alongside the Rust daemon -- unnecessary complexity. SPA mode means SvelteKit builds to static assets, the Rust daemon serves them via `ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"))`, and all routing happens client-side. One process, one binary (plus static assets).

**Why not React?** React's ecosystem is larger but this project doesn't need it. No third-party component library dependencies, no complex form handling, no RSC. The dashboard is bespoke UI with custom visualizations. Svelte's compiled output and smaller bundle are meaningful advantages. React's virtual DOM diffing is overhead you pay for on every WebSocket state update.

**Why not SolidJS?** Similar performance profile to Svelte but smaller ecosystem and community. Svelte has xterm-svelte (xterm.js wrapper), proven SPA patterns, and broader adoption momentum.

### Terminal Emulation (Frontend)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| @xterm/xterm | 6.0.x | Terminal rendering | The standard for browser terminal emulation. Used by VS Code, Theia, Codesandbox. GPU-accelerated renderer. Full ANSI escape code support, mouse events, Unicode. Migrated to @xterm scope (old `xterm` package is deprecated). | HIGH |
| @xterm/addon-fit | 6.x | Terminal resizing | Auto-fits terminal to container. Required for responsive layouts. | HIGH |
| @xterm/addon-webgl | 6.x | GPU rendering | WebGL-based renderer for better performance with high-throughput PTY output (build logs, test output). Falls back to canvas if WebGL unavailable. | HIGH |
| @xterm/addon-web-links | 6.x | Clickable links | Auto-detects URLs in terminal output. Small quality-of-life addition. | MEDIUM |

**Svelte integration approach:** Use `@battlefieldduck/xterm-svelte` as a reference but write a thin custom Svelte 5 wrapper component. The wrapper is ~50 lines and avoids a dependency on a third-party library that may lag behind @xterm/xterm 6.x. The component needs: mount terminal on element, connect to WebSocket, handle resize events, teardown on unmount. Svelte 5's `$effect` makes this clean.

### Styling

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| CSS Custom Properties | -- | Theming | PROJECT.md mandates Aurora dark navy theme via CSS tokens. Custom properties enable runtime theme switching and component-scoped overrides. No Tailwind per project constraint. | HIGH |
| Svelte scoped styles | -- | Component styling | Svelte's built-in `<style>` blocks scope CSS to components automatically. Combined with global token variables, this eliminates the need for any CSS framework. | HIGH |

**Theme architecture:** `tokens.css` (design tokens: colors, spacing, typography) -> `theme.css` (semantic mappings: --surface, --text, --accent) -> `utilities.css` (utility classes for common patterns) -> `global.css` (resets, base styles). Components reference semantic tokens via `var(--surface-primary)`.

### WebSocket Protocol

| Technology | Purpose | Why | Confidence |
|------------|---------|-----|------------|
| Native WebSocket API (browser) | Client-side WS | No library needed. The browser WebSocket API is sufficient. Wrap in a Svelte store with reconnection logic. | HIGH |
| axum::extract::ws (server) | Server-side WS | Built into Axum, uses tungstenite under the hood. No additional dependency. | HIGH |

**Message format:** JSON over WebSocket. Define a typed message enum in Rust (serde-serialized) and TypeScript types on the frontend. Message types: `StateUpdate`, `TerminalOutput`, `TerminalInput`, `TerminalResize`, `Metrics`, `Subscribe`, `Unsubscribe`. Keep messages granular -- send diffs, not full state snapshots, to minimize bandwidth.

---

### Infrastructure & Deployment

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Caddy | 2.x | Reverse proxy + TLS | Automatic HTTPS via Let's Encrypt, simple Caddyfile syntax, built-in `forward_auth` directive for oauth2-proxy integration. ~15% the config of equivalent nginx setup. Performance difference vs nginx is negligible below 50k req/sec (this dashboard serves <100 concurrent users). | HIGH |
| oauth2-proxy | 7.x | Authentication | Handles OAuth2 flow with GitHub/Google/etc. Caddy's `forward_auth` checks each request against oauth2-proxy. Proven pattern, well-documented integration. Passes user identity via `X-Forwarded-User` header to the Axum backend. | HIGH |
| systemd | -- | Process management | Daemon runs as a systemd service. Provides restart-on-crash, log aggregation (journald), and resource limits. No need for Docker unless the deployment environment requires it. | HIGH |

**Why Caddy over nginx?** For this project specifically: automatic HTTPS with zero config, native `forward_auth` for oauth2-proxy (nginx requires `auth_request` module configuration), and the Caddyfile is dramatically simpler to maintain. nginx wins on raw throughput at scale, but this dashboard doesn't need it.

**Why not Traefik?** Traefik is designed for dynamic container orchestration (Docker/Kubernetes service discovery). This is a single-server deployment. Caddy is simpler for static reverse proxy configs.

---

## Development Tools

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| cargo-watch | latest | Auto-rebuild on save | `cargo watch -x run` for backend development. | HIGH |
| sqlx-cli | 0.8.x | Database migrations | `sqlx migrate run` for schema management. Compile-time query checking requires `DATABASE_URL` or `sqlx prepare` for offline mode. | HIGH |
| just | latest | Task runner | Replaces Makefiles with a simpler syntax. Define tasks like `dev`, `build`, `migrate`, `deploy`. Cross-platform. | MEDIUM |

---

## Complete Dependency Summary

### Rust (Cargo.toml)

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
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }

# PTY management
pty-process = { version = "0.5", features = ["async"] }

# File watching
notify = "8"
notify-debouncer-mini = "0.7"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### Frontend (package.json)

```json
{
  "devDependencies": {
    "@sveltejs/adapter-static": "^3.0",
    "@sveltejs/kit": "^2.0",
    "svelte": "^5.0",
    "vite": "^6.0",
    "typescript": "^5.0",
    "@sveltejs/vite-plugin-svelte": "^5.0"
  },
  "dependencies": {
    "@xterm/xterm": "^6.0",
    "@xterm/addon-fit": "^0.10",
    "@xterm/addon-webgl": "^0.18",
    "@xterm/addon-web-links": "^0.11"
  }
}
```

**Note on xterm addon versions:** The addon versions above are estimates based on the @xterm/xterm 6.0 migration. Verify exact compatible versions from the xterm.js releases page before installing. The addons must match the major version of @xterm/xterm.

### Infrastructure

```bash
# Caddy (via package manager or official binary)
caddy version  # 2.x

# oauth2-proxy (via binary release)
oauth2-proxy version  # 7.x

# SQLite CLI (for debugging, optional)
sqlite3 --version
```

---

## Alternatives Considered (Summary)

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Backend language | Rust | Go | Go is simpler but lacks Rust's type safety for complex state parsing. Node.js would work but PTY management is cleaner in Rust. |
| Web framework | Axum 0.8 | Actix-Web 4 | Actix is slightly faster in benchmarks but Axum's tower ecosystem and tokio-native design are more composable. |
| PTY crate | pty-process | portable-pty | portable-pty adds Windows support we don't need. pty-process has direct tokio AsyncRead/AsyncWrite. |
| Database | SQLite via SQLx | PostgreSQL | PostgreSQL requires a separate service. SQLite is zero-ops for single-server deployment. |
| DB driver | SQLx | Rusqlite | Rusqlite is sync-only. SQLx is async-native, aligning with the rest of the stack. |
| DB driver | SQLx | Diesel | Diesel's ORM/DSL is overkill for ~8 simple tables. |
| Frontend | Svelte 5 | React 19 | React's virtual DOM is overhead for WebSocket-driven updates. Svelte compiles away, smaller bundles. |
| Frontend | Svelte 5 | SolidJS | Similar perf but smaller ecosystem. No xterm.js wrapper, less community momentum. |
| CSS | Custom Properties | Tailwind CSS | Project constraint: no Tailwind. Aurora theme uses design tokens. |
| Reverse proxy | Caddy 2 | nginx | Caddy has automatic HTTPS, simpler config, native forward_auth. nginx wins on raw throughput but that's irrelevant at this scale. |
| Reverse proxy | Caddy 2 | Traefik | Traefik is for container orchestration. Overkill for single-server. |
| Process mgmt | systemd | Docker | Docker adds a layer. systemd is already on the server. Use Docker only if deployment policy requires it. |

---

## Architecture Implications

1. **Single binary + static assets**: Rust daemon compiled to one binary. SvelteKit builds to static HTML/JS/CSS. Deployment is: copy binary + static folder + run.
2. **No Node.js runtime in production**: SvelteKit adapter-static means Node is dev-only (build tool). Production runs Rust + Caddy.
3. **SQLite bundled in binary**: Using SQLx's `sqlite` feature bundles libsqlite3. Zero system dependencies beyond libc.
4. **WebSocket as primary data channel**: REST API only for initial page load (project list, current state snapshot). All live updates via WebSocket subscriptions.

---

## Version Verification Notes

| Technology | Version Claimed | Verification Source | Date Verified |
|------------|----------------|---------------------|---------------|
| Axum | 0.8.8 | docs.rs/crate/axum/latest, crates.io | 2026-03-06 |
| Rust 2024 Edition | 1.85+ | blog.rust-lang.org/2025/02/20 | 2026-03-06 |
| Svelte | 5.45+ | svelte.dev/blog (Dec 2025 update) | 2026-03-06 |
| SvelteKit | 2.53+ | npmjs.com/@sveltejs/kit | 2026-03-06 |
| @xterm/xterm | 6.0.0 | npmjs.com/@xterm/xterm | 2026-03-06 |
| SQLx | 0.8.6 | docs.rs/crate/sqlx/latest | 2026-03-06 |
| pty-process | 0.5.3 | crates.io/crates/pty-process | 2026-03-06 |
| notify | 8.2.0 | docs.rs/crate/notify/latest | 2026-03-06 |
| portable-pty | 0.9.0 | docs.rs/crate/portable-pty/latest | 2026-03-06 |

---

## Sources

- [Axum 0.8.0 Announcement (tokio.rs)](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) -- HIGH confidence
- [Axum docs.rs](https://docs.rs/axum/latest/axum/) -- HIGH confidence
- [Axum GitHub](https://github.com/tokio-rs/axum) -- HIGH confidence
- [Rust 1.85 / Edition 2024 announcement](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/) -- HIGH confidence
- [SvelteKit docs -- adapter-static](https://svelte.dev/docs/kit/adapter-static) -- HIGH confidence
- [SvelteKit docs -- SPA mode](https://svelte.dev/docs/kit/single-page-apps) -- HIGH confidence
- [Svelte 5 release announcement](https://svelte.dev/blog/svelte-5-is-alive) -- HIGH confidence
- [@xterm/xterm npm](https://www.npmjs.com/package/@xterm/xterm) -- HIGH confidence
- [xterm.js GitHub](https://github.com/xtermjs/xterm.js/) -- HIGH confidence
- [pty-process crates.io](https://crates.io/crates/pty-process) -- HIGH confidence
- [pty-process GitHub](https://github.com/doy/pty-process) -- HIGH confidence
- [SQLx GitHub](https://github.com/launchbadge/sqlx) -- HIGH confidence
- [notify crate docs.rs](https://docs.rs/notify/latest/notify/) -- HIGH confidence
- [notify-debouncer-mini docs.rs](https://docs.rs/notify-debouncer-mini/latest/notify_debouncer_mini/) -- HIGH confidence
- [tower-http docs.rs](https://docs.rs/tower-http/latest/tower_http/) -- HIGH confidence
- [Caddy forward_auth docs](https://caddyserver.com/docs/caddyfile/directives/forward_auth) -- HIGH confidence
- [oauth2-proxy Caddy integration](https://oauth2-proxy.github.io/oauth2-proxy/next/configuration/integrations/caddy/) -- HIGH confidence
- [xterm-svelte](https://github.com/BattlefieldDuck/xterm-svelte) -- MEDIUM confidence (third-party wrapper)
- [Rust ORMs comparison 2026](https://aarambhdevhub.medium.com/rust-orms-in-2026-diesel-vs-sqlx-vs-seaorm-vs-rusqlite-which-one-should-you-actually-use-706d0fe912f3) -- MEDIUM confidence
- [Rust web frameworks comparison 2026](https://aarambhdevhub.medium.com/rust-web-frameworks-in-2026-axum-vs-actix-web-vs-rocket-vs-warp-vs-salvo-which-one-should-you-2db3792c79a2) -- MEDIUM confidence
