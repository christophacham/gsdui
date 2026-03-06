# Feature Research

**Domain:** Developer Pipeline Dashboard / AI Agent Session Visualization
**Researched:** 2026-03-06
**Confidence:** HIGH

## Feature Landscape

This research surveys six feature dimensions across GitHub Actions, GitLab CI, Jenkins Blue Ocean, Buildkite, Vercel, Railway, Coolify, Portainer, and the emerging AI agent session management tools (agentsview, agent-deck, agent-of-empires). GSD Pipeline UI is unique in combining pipeline visualization, interactive browser terminals, and system monitoring -- most tools only cover one or two of these.

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete.

#### Pipeline / Workflow Visualization

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Pipeline list view with status indicators | Every CI/CD tool shows pipeline runs with pass/fail/running icons. Without this, users cannot orient themselves. GitLab, GitHub Actions, Buildkite all have this. | LOW | GSD equivalent: list of phases with status badges (planned/discussed/executing/verified) |
| Stage-level progress within a pipeline | GitLab shows stages as columns, Buildkite shows steps in a DAG. Users need to see where in the process things are. | MEDIUM | GSD has 4 fixed stages (Discuss/Plan/Execute/Verify) per phase -- simpler than arbitrary DAGs |
| Duration / timing information | Every CI dashboard shows how long each step took. Buildkite shows bar charts of build history with height = duration. Users expect to see "how long has this been running" and "how long did it take." | LOW | Parse from agent-history.json timestamps, store in DB for historical data |
| Color-coded status | Universal: green = passed, red = failed, yellow/amber = running, gray = pending. Jenkins Blue Ocean, GitLab, GitHub Actions all use this pattern. | LOW | Map to GSD states: planned (gray), executing (amber pulse), verified-pass (green), verified-fail (red) |
| Expandable detail view | Click a pipeline/phase to see more detail. Every tool does this -- GitLab expands to show job logs, Buildkite shows step details on hover/click. | MEDIUM | Phase detail = the 4-stage rail view with wave/plan cards |
| Build/execution history | GitLab CI analytics shows pipeline history over time. Buildkite shows last 30 builds as a bar chart. Users expect to see past runs, not just current state. | MEDIUM | Database stores historical execution records; display as timeline or list |

#### Terminal / Console

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Log output with ANSI color rendering | Vercel added ANSI color code rendering to build logs. Jenkins has the AnsiColor plugin. Users expect colored output, not raw escape sequences or stripped-to-plain-text. | MEDIUM | xterm.js handles this natively -- it is a full terminal emulator |
| Scrollback buffer / log history | Every CI tool lets you scroll back through build output. Portainer and Coolify both show container logs with scroll. | LOW | xterm.js supports configurable scrollback. Also persist logs to DB for post-session viewing |
| Auto-scroll to bottom (follow mode) | Standard in every log viewer. New output appears and view stays pinned to bottom until user scrolls up. | LOW | xterm.js addon or simple scroll behavior |
| Copy text from terminal output | Users need to copy error messages, stack traces, etc. Portainer console, VS Code terminal, all support text selection and copy. | LOW | xterm.js supports text selection natively; clipboard addon available |
| Multiple terminal tabs | VS Code has terminal tabs. Coolify has per-resource terminals. Portainer has per-container console. Users running multiple agents need multiple terminals. | MEDIUM | Tab management UI + one WebSocket/PTY per tab |
| Clear terminal / reset | Standard terminal feature. Every tool with a console has a clear button or Ctrl+L support. | LOW | xterm.js clear() API |

#### System Monitoring

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| CPU usage display | Railway, Portainer, every server dashboard shows CPU usage. Users hosting a daemon want to know if it is consuming resources. | MEDIUM | Server-side collection (sysinfo crate or /proc parsing), push via WebSocket |
| Memory usage display | Same as CPU -- universal in Railway, Portainer, Coolify dashboards. | LOW | Pair with CPU collection |
| Disk usage display | Railway shows disk usage trends. Important when .planning/ dirs and agent sessions accumulate data. | LOW | Pair with CPU collection |
| Service health status | Portainer shows container health. Railway shows service status. Users need to know if the daemon, database, and WebSocket server are healthy. | MEDIUM | Health check endpoints + status indicators |

#### Multi-Project Management

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Project list / sidebar | Every multi-project tool (Vercel, Railway, Coolify, GitLab) has a sidebar or project switcher. Users with multiple GSD projects need to navigate between them. | MEDIUM | Sidebar with project list, active project highlighted |
| Project status at-a-glance | GitLab Operations Dashboard shows project health summary. Buildkite shows pipeline health per pipeline. Users scanning their project list need quick status indicators. | LOW | Badge/icon per project showing overall state (idle, active, error) |
| Search / filter projects | Buildkite has pipeline search and tag filtering. Vercel has project search in sidebar. Necessary when project count exceeds ~10. | LOW | Text filter on project list |

#### Real-Time Updates

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Live status updates without page refresh | Every modern CI dashboard updates in real-time. GitHub Actions shows running workflows with live status. Railway uses real-time logs. | HIGH | WebSocket infrastructure: inotify -> daemon parse -> DB update -> WebSocket broadcast -> UI reactivity |
| Connection status indicator | Users need to know if they are connected to the live feed or viewing stale data. Standard in WebSocket-based dashboards. | LOW | Small indicator dot (green = connected, red = disconnected, yellow = reconnecting) |
| Automatic reconnection | WebSocket connections drop. Every production WebSocket app handles reconnection with exponential backoff. | MEDIUM | Client-side reconnection logic with state resync on reconnect |

### Differentiators (Competitive Advantage)

Features that set GSD Pipeline UI apart. Not expected in generic dashboards, but high-value for the GSD use case.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Interactive PTY sessions (bidirectional terminal) | Most CI dashboards show read-only logs. Coolify and Portainer offer container exec. But GSD needs full interactive PTY where users respond to agent prompts and inject commands. This is the core interaction model -- not just viewing output but participating in agent sessions. | HIGH | node-pty (or Rust pty-process) on server, xterm.js on client, bidirectional WebSocket relay. Security boundary critical. |
| Agent session auto-creation of console tabs | When GSD spawns an agent, a new Console sub-tab appears automatically. No other CI tool does this because they don't have interactive agent sessions. Agent-deck and agent-of-empires do this in TUI context -- GSD brings it to the browser. | HIGH | Requires daemon to detect new agent sessions (via current-agent-id.txt or agent-history.json changes) and push new tab creation events to connected clients |
| Multi-agent routing visualization | Show which AI agent (Claude, Codex, Gemini) is assigned to which stage/plan, with project-default -> stage-override -> plan-override hierarchy. No existing tool visualizes this routing because multi-agent AI pipelines are novel. | MEDIUM | UI for displaying and potentially configuring agent assignments per stage/plan. Reads from config.json. |
| Wave/plan card visualization | GSD's unique model: phases contain stages, stages execute in waves, waves contain plan cards. This is domain-specific visualization that no generic CI tool has. Cards show agent, status, progress, commits. | HIGH | Custom component: plan cards arranged in wave lanes within stage rails. Requires parsing PLAN.md/SUMMARY.md frontmatter. |
| Observe-only architecture (no orchestration) | Unlike Coolify/Railway which deploy and manage services, GSD UI only watches .planning/ files. This means zero risk of the dashboard interfering with agent execution. The daemon cannot break your pipeline. Users trust it. | LOW | Architectural constraint, not a feature to build. But it shapes everything -- no write APIs to .planning/ files from the dashboard. |
| File-change-driven state derivation | State derived from file presence/contents (CONTEXT.md = discussed, PLAN.md = planned, etc.) rather than explicit API calls. Novel approach -- most dashboards receive state via API webhooks or polling. GSD derives state from filesystem semantics. | HIGH | inotify/notify watcher + parser for each file type (STATE.md frontmatter, PLAN.md frontmatter, VERIFICATION.md, etc.). Must handle rapid file changes and partial writes gracefully. |
| Per-user project isolation | Unlike shared team dashboards (GitLab, Buildkite), each user registers their own project folders and only sees their own. This is a personal productivity dashboard, not a team collaboration tool. | MEDIUM | Auth + per-user project registry in DB. Projects stored per-user, not globally visible. |
| Historical execution analytics | Agent-deck tracks session history. Railway tracks deployment metrics over time. GSD can track execution timelines, agent durations, verification pass rates, and phase completion velocity over time -- becoming a performance analysis tool, not just a live dashboard. | MEDIUM | Database accumulates historical records. Chart/graph views for trends. Deferred to later phases. |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem good but create problems for the GSD Pipeline UI.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Pipeline editing / orchestration from UI | Users may want to edit ROADMAP.md, reorder phases, or trigger GSD commands from a pipeline view button. Coolify and Railway let you deploy from their UI. | Violates the "observe, don't orchestrate" constraint. GSD runs inside agent sessions. Adding write-back to .planning/ files creates race conditions with running agents and splits the source of truth. | Users kick off GSD from Console terminals. The pipeline view is read-only. |
| Drag-and-drop pipeline builder | Jenkins Blue Ocean has a visual pipeline editor. GitLab has a pipeline editor with visualization tab. Tempting to build for GSD roadmaps. | GSD roadmaps are markdown files with specific formatting. A visual editor would need to round-trip Markdown faithfully, handle frontmatter, and stay in sync with agent modifications. Enormous complexity for marginal value. | Edit ROADMAP.md in Console terminal via your preferred editor or agent. |
| Real-time collaborative editing | Google Docs-style multi-user editing of plans or roadmaps. Seems modern and useful. | GSD files are owned by running agents. Two humans editing the same PLAN.md while an agent is executing creates unresolvable conflicts. CRDT complexity is massive for this use case. | Per-user project isolation. Each user works in their own project space. |
| Mobile-responsive terminal | Users may want to check pipelines from mobile. Terminal interaction on mobile is painful. | xterm.js on mobile is a terrible experience -- no keyboard shortcuts, tiny text, touch-based text selection is broken. Building for mobile terminals wastes effort on an unusable experience. | Pipeline status view can be mobile-responsive (read-only badges/cards). Terminal tabs are desktop-only. |
| Embedded code editor / IDE | VS Code has integrated terminals. Why not add an editor to GSD UI? | Scope explosion. Code editors are a solved problem (VS Code, Zed, etc.). Building even a basic one is months of work. Users already have editors open alongside the dashboard. | Console terminals can run vim/nano. Link to open files in external editor. |
| Notification system (email/Slack) | Railway and Buildkite send alerts. Users may want notifications when phases complete or fail. | Requires outbound integrations, message formatting, notification preferences, delivery reliability, unsubscribe logic. Significant infrastructure for a v1. | Webhook endpoint for external integration. Users can pipe to Slack/email via their own automation. |
| Role-based access control (RBAC) | Enterprise dashboards have admin/viewer/editor roles. | This is a personal productivity tool, not a team platform. Per-user isolation is sufficient. RBAC adds UI complexity (role management screens) and auth complexity for no user value in this context. | Per-user project isolation. Auth handled by reverse proxy (oauth2-proxy). |
| Plugin / extension system | Jenkins has a plugin marketplace. Buildkite has plugins. Extensibility seems forward-looking. | Plugin systems are architecture-defining decisions that constrain everything. Building one before the core product is stable is premature optimization. The surface area for security issues is enormous. | Stable API first. Plugin system is a v3+ consideration if there is demand. |

## Feature Dependencies

```
[WebSocket Infrastructure]
    |
    +-- [Live pipeline status updates]
    |       +-- [Pipeline list view with status]
    |       +-- [Phase detail / stage rail view]
    |       +-- [Wave/plan card updates]
    |
    +-- [Terminal streaming]
    |       +-- [Interactive PTY sessions]
    |       +-- [Agent session auto-tab creation]
    |       +-- [Multiple terminal tabs]
    |
    +-- [System metrics streaming]
            +-- [CPU/Memory/Disk displays]
            +-- [Service health status]

[File Watcher (inotify)]
    +-- [State parser (ROADMAP.md, STATE.md, PLAN.md, etc.)]
    |       +-- [Stage derivation logic]
    |       +-- [Pipeline list view with status]
    |       +-- [Phase detail / stage rail view]
    |       +-- [Wave/plan card visualization]
    |
    +-- [Agent session detection]
            +-- [Agent session auto-tab creation]
            +-- [Multi-agent routing visualization]

[Database (parsed cache + history)]
    +-- [Execution history view]
    +-- [Historical analytics]
    +-- [Build/execution history list]

[Authentication (reverse proxy)]
    +-- [Per-user project isolation]
    +-- [Project list/sidebar]

[xterm.js + PTY process management]
    +-- [Interactive PTY sessions]
    +-- [ANSI color rendering]
    +-- [Multiple terminal tabs]
    +-- [Scrollback / log history]
```

### Dependency Notes

- **WebSocket Infrastructure is foundational:** Pipeline updates, terminal streaming, and system metrics all flow through WebSockets. This must be built first and built well. Without it, nothing is real-time.
- **File Watcher + State Parser unlocks Pipeline views:** The pipeline visualization is entirely dependent on correctly parsing GSD's file-based state. The parser must handle all file types (STATE.md, PLAN.md, SUMMARY.md, ROADMAP.md, VERIFICATION.md, config.json, agent-history.json) and must be robust against partial writes and rapid changes.
- **PTY management is independent of pipeline visualization:** Terminal tabs can work before the pipeline view is complete. These are parallel workstreams.
- **Authentication is a prerequisite for multi-user features:** Per-user project isolation requires knowing who the user is. Auth must be in place before project registration.
- **Database enables history but is not required for live state:** Live state can flow through WebSockets from the file watcher. The database adds persistence, history, and query capability but the live dashboard could function (degraded) without it for pure real-time viewing. However, the database also serves as the cache layer, so in practice it should be early.

## MVP Definition

### Launch With (v1)

Minimum viable product -- what's needed to validate the core value proposition ("see what GSD is doing and interact with it from the browser").

- [ ] **Pipeline list view** with phase names, status badges, and timing -- validates that file-watching and state parsing work
- [ ] **Phase detail view** with 4-stage rail (Discuss/Plan/Execute/Verify) -- validates the GSD-specific visualization model
- [ ] **Single interactive terminal** (Console tab with one xterm.js PTY) -- validates that users can run GSD commands and respond to prompts from the browser
- [ ] **WebSocket live updates** -- validates real-time state propagation from filesystem to browser
- [ ] **File watcher + state parser** for core GSD files (STATE.md, ROADMAP.md, phase directories) -- the engine that drives everything
- [ ] **Single-project mode** -- defer multi-project to reduce v1 scope; validate with one project first
- [ ] **Database for state cache** -- parsed state stored for fast queries and page loads

### Add After Validation (v1.x)

Features to add once the core loop (watch files -> show pipeline -> interact via terminal) is working.

- [ ] **Multiple terminal tabs** with agent session auto-creation -- when users start running multi-agent workflows
- [ ] **Wave/plan card visualization** within stage rails -- adds depth to pipeline view
- [ ] **Multi-project sidebar** with project registration -- when users want to monitor multiple GSD projects
- [ ] **Per-user project isolation** with auth integration -- when multiple users share the server
- [ ] **System tab** with host metrics (CPU/Memory/Disk) -- when users want to monitor the daemon host
- [ ] **Build/execution history** -- when users want to review past runs
- [ ] **Service health indicators** -- when users want to confirm daemon components are healthy
- [ ] **Agent routing visualization** -- when multi-agent configuration becomes common
- [ ] **Connection status indicator** and automatic reconnection -- production hardening

### Future Consideration (v2+)

Features to defer until the product is validated and stable.

- [ ] **Historical execution analytics** (charts, trends, velocity metrics) -- requires significant data accumulation
- [ ] **Search across projects and executions** -- useful at scale
- [ ] **Webhook endpoint** for external notification integrations -- user demand driven
- [ ] **Log persistence and search** -- full-text search across past terminal sessions
- [ ] **Keyboard shortcuts / vim-style navigation** -- power user feature (agentsview uses this)
- [ ] **Export/share execution reports** -- agentsview supports HTML export and GitHub Gist publishing
- [ ] **Mobile-responsive pipeline status** (read-only, no terminal) -- if mobile access demand emerges

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Pipeline list view with status | HIGH | LOW | P1 |
| Phase detail / 4-stage rail | HIGH | MEDIUM | P1 |
| Interactive PTY terminal (single) | HIGH | HIGH | P1 |
| WebSocket live updates | HIGH | HIGH | P1 |
| File watcher + state parser | HIGH | HIGH | P1 |
| Database state cache | HIGH | MEDIUM | P1 |
| ANSI color rendering | HIGH | LOW | P1 (free with xterm.js) |
| Color-coded status indicators | HIGH | LOW | P1 |
| Duration / timing display | MEDIUM | LOW | P1 |
| Auto-scroll / follow mode | MEDIUM | LOW | P1 |
| Scrollback buffer | MEDIUM | LOW | P1 |
| Multiple terminal tabs | HIGH | MEDIUM | P2 |
| Agent session auto-tab creation | HIGH | HIGH | P2 |
| Wave/plan card visualization | HIGH | HIGH | P2 |
| Multi-project sidebar | HIGH | MEDIUM | P2 |
| Per-user project isolation | MEDIUM | MEDIUM | P2 |
| System metrics (CPU/Memory/Disk) | MEDIUM | MEDIUM | P2 |
| Build/execution history | MEDIUM | MEDIUM | P2 |
| Service health status | MEDIUM | LOW | P2 |
| Agent routing visualization | MEDIUM | MEDIUM | P2 |
| Connection status / auto-reconnect | MEDIUM | LOW | P2 |
| Search / filter projects | LOW | LOW | P2 |
| Historical analytics | MEDIUM | HIGH | P3 |
| Webhook endpoint | LOW | MEDIUM | P3 |
| Log persistence + search | MEDIUM | HIGH | P3 |
| Keyboard shortcuts | LOW | LOW | P3 |
| Export/share reports | LOW | MEDIUM | P3 |
| Mobile-responsive status | LOW | MEDIUM | P3 |

**Priority key:**
- P1: Must have for launch (validates core value proposition)
- P2: Should have, add in subsequent phases (completes the product vision)
- P3: Nice to have, future consideration (respond to user demand)

## Competitor Feature Analysis

| Feature | GitHub Actions | GitLab CI | Buildkite | Vercel / Railway | Coolify / Portainer | Agent Tools (agentsview, agent-deck) | GSD Pipeline UI |
|---------|---------------|-----------|-----------|-----------------|---------------------|--------------------------------------|-----------------|
| Pipeline visualization | Workflow graph with job nodes | Stage columns with job cards | DAG canvas with step nodes | Deployment list (no DAG) | Deployment list | Session list | Phase timeline with stage rails and wave lanes |
| Real-time updates | Live status polling | WebSocket live updates | Live build status | Live deployment logs | Live container logs | File watcher + SSE (agentsview) | WebSocket from inotify file watcher |
| Interactive terminal | None (read-only logs) | Web IDE terminal | None (read-only logs) | None | Container exec console | tmux session manager (agent-deck) | Full interactive PTY with bidirectional I/O |
| Log rendering | ANSI color support | ANSI color support | Rich logs with emoji | ANSI color rendering | Container log output | Conversation rendering | xterm.js full terminal emulation |
| Multi-project | Org-level workflow view | Operations Dashboard | Pipeline bookmarks + tags | Project list sidebar | Project/environment tree | Multi-session TUI | Per-user project sidebar |
| System metrics | None | None | None | CPU/Memory/Disk/Network (Railway) | Container stats (Portainer) | None | Host metrics (CPU/Memory/Disk) |
| Agent management | Self-hosted runner admin | Runner fleet management | Agent queue management | N/A | N/A | Session status detection | Agent routing config (project/stage/plan level) |
| History / analytics | Workflow run history | CI/CD analytics (P50/P95 duration, failure rate) | Pipeline metrics (reliability, speed) | Deployment history | Deployment history | Activity heatmaps, velocity metrics (agentsview) | Execution history with duration tracking |
| Auth model | GitHub OAuth | GitLab auth | SSO/SAML | Team-based | Team/user roles | Local only | Reverse proxy OAuth2 (per-user isolation) |

### Key Takeaways from Competitor Analysis

1. **Interactive terminal is the biggest differentiator.** GitHub Actions, GitLab CI, and Buildkite all show read-only logs. Coolify and Portainer offer container exec but not full PTY sessions with agent interaction. Only the TUI-based agent tools (agent-deck, agent-of-empires) offer comparable interactivity, but they are terminal-only, not browser-based.

2. **File-based state derivation is novel.** Every other tool receives state through APIs, webhooks, or database writes. GSD's approach of deriving pipeline state from filesystem semantics (.planning/ file presence and contents) is unique. This is both a strength (zero integration code) and a risk (parser complexity, partial write handling).

3. **Wave/plan card model has no precedent.** CI tools use stages and jobs. GSD uses phases, stages, waves, and plans. The visualization must be custom -- no existing component library has this layout. This is the highest-complexity custom UI work.

4. **Agent routing is a new category.** No CI tool has multi-AI-agent routing with fallback hierarchies. This is genuinely novel and will need its own UX patterns.

5. **The emerging agent session tools validate the market.** agentsview, agent-deck, agent-of-empires, and agent-view all launched in 2025-2026. There is clear demand for tools that visualize and manage AI coding agent sessions. GSD Pipeline UI goes further by combining this with pipeline visualization and system monitoring.

## Sources

- [Buildkite Dashboard Walkthrough](https://buildkite.com/docs/pipelines/dashboard-walkthrough)
- [Buildkite Pipeline Metrics](https://building.buildkite.com/new-in-buildkite-pipeline-metrics-b5e7bf187272)
- [Buildkite DAG Canvas Visualization](https://buildkite.com/resources/blog/visualize-your-ci-cd-pipeline-on-a-canvas/)
- [GitLab CI/CD Analytics](https://docs.gitlab.com/user/analytics/ci_cd_analytics/)
- [GitLab Pipeline Editor](https://docs.gitlab.com/ci/pipeline_editor/)
- [GitLab Environments Dashboard](https://docs.gitlab.com/ci/environments/environments_dashboard/)
- [GitLab Operations Dashboard](https://docs.gitlab.com/user/operations_dashboard/)
- [Railway Observability Dashboard](https://docs.railway.com/observability)
- [Railway Features](https://railway.com/features)
- [Vercel New Dashboard UX](https://medium.com/design-bootcamp/vercels-new-dashboard-ux-what-it-teaches-us-about-developer-centric-design-93117215fe31)
- [Vercel ANSI Color Code Rendering](https://vercel.com/changelog/build-and-function-logs-now-render-ansi-color-codes-nicely)
- [Coolify Terminal Documentation](https://coolify.io/docs/knowledge-base/internal/terminal)
- [Coolify Introduction](https://coolify.io/docs/get-started/introduction)
- [Portainer Dashboard Documentation](https://docs.portainer.io/user/docker/dashboard)
- [Portainer Features](https://www.portainer.io/features)
- [Jenkins Blue Ocean](https://www.jenkins.io/doc/book/blueocean/)
- [xterm.js](https://xtermjs.org/)
- [xterm.js GitHub - Multiple Terminals](https://github.com/xtermjs/xterm.js/issues/4379)
- [agentsview - AI Agent Session Viewer](https://github.com/wesm/agentsview)
- [agent-deck - AI Agent Terminal Session Manager](https://github.com/asheshgoplani/agent-deck)
- [agent-of-empires - Multi-Agent Terminal Manager](https://github.com/njbrake/agent-of-empires)
- [VS Code Multi-Agent Development](https://code.visualstudio.com/blogs/2026/02/05/multi-agent-development)

---
*Feature research for: Developer Pipeline Dashboard / AI Agent Session Visualization*
*Researched: 2026-03-06*
