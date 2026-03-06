# Pitfalls Research

**Domain:** Real-time developer dashboard with file-watching pipeline, browser terminals, and WebSocket streaming
**Researched:** 2026-03-06
**Confidence:** HIGH (multiple sources corroborate across all critical areas)

## Critical Pitfalls

### Pitfall 1: File Watcher Race Conditions Corrupt Parsed State

**What goes wrong:**
The daemon watches `.planning/` files via inotify and parses them into a database. File writes are not atomic -- editors and GSD agents write files in stages (truncate, write partial, flush, close). The daemon receives a `MODIFY` event mid-write and reads a half-written or empty file. STATE.md frontmatter is parsed as empty/invalid. The database records a "no state" entry that propagates through WebSocket to all connected clients, causing the pipeline UI to flicker or show incorrect state before self-correcting on the next event.

**Why it happens:**
inotify fires on every write syscall, not on "file is done being written." Developers assume MODIFY means "file is ready to read." GSD agents (Claude Code, Codex) may write files through multiple write() calls, and text editors use write-to-temp-then-rename patterns that generate CREATE + MOVE_TO events rather than MODIFY. Different writers produce different event sequences for the same logical operation.

**How to avoid:**
- Debounce file events: accumulate events for 50-100ms after the last event on a given path before triggering a parse. This catches multi-syscall writes.
- Validate parsed output: if STATE.md parses to empty/null frontmatter, discard the parse result and retain the previous known-good state. Never store a "worse" state without confirmation.
- Handle both MODIFY and CLOSE_WRITE: prefer CLOSE_WRITE (inotify IN_CLOSE_WRITE) as it fires after the writer closes the file descriptor, which is more reliable for catching complete writes.
- For rename-based writes (common in editors), watch for MOVED_TO events, not just MODIFY.

**Warning signs:**
- Pipeline UI flickering between states on file saves
- Database contains entries with null/empty fields that later self-correct
- Intermittent "file not found" errors in parse logs despite file existing
- State "downgrades" (e.g., phase goes from "executing" back to "planned" momentarily)

**Phase to address:**
Phase 1 (daemon foundation). File watching is the root of the entire data pipeline. Getting event handling wrong here poisons every downstream component. This must be solid before building UI on top of it.

---

### Pitfall 2: PTY Process Leaks and Zombie Accumulation

**What goes wrong:**
Each Console tab spawns a PTY process on the server. When users close browser tabs, navigate away, lose network connectivity, or the WebSocket drops, the PTY process keeps running on the server with no client consuming its output. Over time, dozens of orphaned shell processes accumulate, consuming PIDs, file descriptors, and memory. Eventually the server hits process limits or OOM. This is exacerbated by agent sessions that auto-create Console sub-tabs -- if a GSD run spawns 5 agent sessions, that is 5 PTY processes that must all be properly lifecycle-managed.

**Why it happens:**
WebSocket disconnection does not automatically signal the PTY process to terminate. The browser `beforeunload` event is unreliable (especially on mobile, tab crashes, or network drops). Developers implement the "happy path" (user clicks close button) but not the "unhappy path" (connection drops silently). Child processes spawned within the PTY (vim, make, long-running builds) survive even if the parent shell is killed with SIGTERM.

**How to avoid:**
- Implement server-side PTY reaping: use WebSocket heartbeat/ping-pong (every 15-30 seconds). If 3 consecutive pongs are missed, kill the PTY process group (not just the shell PID -- use `kill(-pgid, SIGTERM)` to kill the entire process group).
- Set a session inactivity timeout (e.g., 30 minutes of no input kills the PTY).
- On daemon startup, scan for and kill orphaned PTY processes from previous daemon runs (store PTY PIDs in the database, clean up on restart).
- Use `SIGKILL` as a fallback 5 seconds after `SIGTERM` for processes that trap SIGTERM.
- Track PTY count per user and enforce limits (e.g., max 10 concurrent PTYs).

**Warning signs:**
- `ps aux | grep pts` shows growing list of shell processes
- File descriptor count (`/proc/<daemon-pid>/fd/`) grows over time
- Server memory usage climbs steadily over days
- "Too many open files" errors in daemon logs

**Phase to address:**
Phase where Console/PTY is implemented. This must be part of the initial PTY implementation, not bolted on later. The reaping logic is tightly coupled to the WebSocket lifecycle and PTY spawn code.

---

### Pitfall 3: No Flow Control Between PTY and xterm.js (Output Flooding)

**What goes wrong:**
A command like `find / -type f`, `cat` on a large file, or a verbose build log produces output at hundreds of MB/s from the PTY. This data traverses: PTY -> daemon -> WebSocket -> browser -> xterm.js. xterm.js processes input at 5-35 MB/s. Without flow control, the WebSocket send buffer grows unbounded on the server side, the browser's JS event loop is starved processing terminal writes, the page becomes unresponsive, and eventually the browser tab crashes or the server runs out of memory buffering unsent WebSocket frames.

**Why it happens:**
The naive implementation is `pty.on('data', chunk => ws.send(chunk))` -- a direct pipe with no backpressure. WebSockets have no built-in backpressure mechanism (unlike TCP streams). xterm.js has a 50MB hard buffer limit before it starts discarding data, but by that point the user experience is already destroyed. Developers test with normal interactive commands and never stress-test with high-throughput output.

**How to avoid:**
- Implement watermark-based flow control between the PTY and WebSocket: track bytes pending in the xterm.js write buffer via write callbacks. When pending bytes exceed HIGH watermark (128KB-500KB), send a pause message to the server. When pending drops below LOW watermark (16KB), send resume.
- On the server side, when the client signals pause, call `pty.pause()` to apply OS-level backpressure to the PTY (this actually blocks the writing process via TTY flow control).
- Set a server-side WebSocket send buffer limit. If the buffer exceeds a threshold (e.g., 10MB), start dropping older frames rather than accumulating unboundedly.
- Consider rate-limiting output on the server side: buffer PTY output and flush at most every 16ms (one frame) to avoid overwhelming the WebSocket transport.

**Warning signs:**
- Browser tab becomes unresponsive during `cat large_file.log`
- Server memory spikes during high-output commands
- xterm.js logs "write buffer exceeded" warnings
- WebSocket connection drops under load (browser kills unresponsive tab)

**Phase to address:**
Phase where Console/PTY is implemented. Flow control must be baked into the PTY-to-WebSocket bridge from day one. Retrofitting backpressure into a working pipe is a significant refactor because it changes the entire data flow from push to push-with-pause.

---

### Pitfall 4: WebSocket Reconnection Loses Real-Time State

**What goes wrong:**
The user's network blips for 5 seconds (laptop sleep, WiFi handoff, VPN reconnect). The WebSocket drops. The client reconnects. But the client now has stale state -- pipeline phases may have progressed, agent sessions may have started or finished, terminal output was produced while disconnected. The UI shows outdated information with no indication that it is stale. Users make decisions based on incorrect state (e.g., thinking a phase is still running when it completed).

**Why it happens:**
WebSocket is a stateful, fire-and-forget protocol. Messages sent while the client is disconnected are gone -- there is no replay buffer in the WebSocket protocol itself. Developers implement reconnection (exponential backoff, auto-reconnect) but forget that reconnection without state resynchronization is worse than no reconnection at all, because it gives the user false confidence that data is current.

**How to avoid:**
- On reconnect, the client must request a full state snapshot from the server (all pipeline state, all active sessions, current phase/stage for each project). The server sends this as the first message after WebSocket handshake.
- Assign monotonically increasing sequence numbers to all state-change messages. On reconnect, the client sends its last-seen sequence number. The server replays missed messages if they are within a buffer window, or sends a full snapshot if too many were missed.
- Display a visible "Reconnecting..." banner during disconnection and a brief "Resyncing..." indicator after reconnect until the full snapshot is applied.
- For terminal sessions: buffer the last N lines (e.g., 1000) of each PTY's output server-side. On reconnect, replay the buffer so the user sees recent context. Do NOT attempt to replay the entire terminal history -- that is unbounded.

**Warning signs:**
- QA reports "UI shows wrong state after laptop wake"
- Users report pipeline status not matching actual file system state
- No visual indication during WebSocket disconnection
- Terminal tabs show blank content after reconnect

**Phase to address:**
Phase where WebSocket infrastructure is built. The reconnection-with-resync protocol must be designed into the message format from the start. Sequence numbers and snapshot endpoints are architectural decisions that are painful to add retroactively.

---

### Pitfall 5: Terminal Escape Sequence Injection via Untrusted Output

**What goes wrong:**
GSD agent output, log files, or command output contains malicious or unexpected ANSI escape sequences. These sequences can: change the terminal title (leaking information via `onTitleChange`), create invisible clickable links overlaying the terminal (href parser abuse), or in older xterm.js versions, trigger responses that execute in the embedding page context. In a multi-user system, if one user's agent session produces crafted output that another user later views in a shared log, this becomes a cross-user attack vector.

**Why it happens:**
xterm.js by design interprets escape sequences -- that is its job as a terminal emulator. Developers treat terminal output as "just text" and pipe it directly from PTY to browser. The trust boundary between "data from a PTY process" and "code rendered in a browser" is invisible and easy to overlook. GSD agents may process untrusted content (git diffs, file contents, user-provided descriptions) that gets echoed to the terminal.

**How to avoid:**
- Never expose xterm.js's `onTitleChange` event to the embedding page unless you sanitize the title value.
- Disable or do not use the href/linkifier addon in security-sensitive contexts, or sanitize URLs before making them clickable.
- Keep xterm.js updated -- security vulnerabilities (like the DCS response injection) are patched in newer versions.
- For log viewing (as opposed to interactive PTY), strip or sanitize escape sequences server-side before sending to the client. Only allow a whitelist of safe sequences (basic colors, cursor movement).
- Ensure each user's terminal sessions are strictly isolated -- User A must never see raw terminal output from User B's PTY.

**Warning signs:**
- Terminal title changes unexpectedly when viewing certain output
- Clickable links appear in terminal output that should not have them
- Security audit flags xterm.js version as outdated
- User reports seeing "garbled" output (often escape sequences being partially interpreted)

**Phase to address:**
Phase where Console/PTY is implemented, with security hardening revisited in a later security-focused phase. The isolation between users must be enforced at the data layer (database queries, WebSocket subscription topics), not just the UI layer.

---

### Pitfall 6: inotify Watch Exhaustion on Multi-Project Systems

**What goes wrong:**
Each registered project has a `.planning/` directory tree that needs monitoring. inotify requires a separate watch for each subdirectory (it does not support recursive watching natively). A single GSD project can have 20+ phase directories, each with subdirectories. With 10 users, each registering 5 projects, that is 1000+ inotify watches just for `.planning/` trees. The daemon competes with other system tools (VS Code, IDE file watchers, systemd) for the same per-user inotify watch pool. When the limit is hit, new watches silently fail to register and file changes go undetected.

**Why it happens:**
The default `max_user_watches` is 8192 on older kernels (kernels since 5.11 auto-scale to ~1M based on RAM, but many servers run older kernels or have custom limits). Developers test with 1-2 projects and never hit the limit. The failure mode is silent -- `inotify_add_watch` returns an error but the daemon may not surface it clearly. Higher-level libraries (like Rust's `notify` crate) may swallow the error or log it at debug level.

**How to avoid:**
- On daemon startup, check `cat /proc/sys/fs/inotify/max_user_watches` and log a warning if it is below a safe threshold (e.g., 65536). Document the requirement to set it higher.
- Watch only the specific directories that matter (`.planning/` root plus known subdirectory patterns), not entire project trees.
- Handle watch registration errors explicitly -- if `inotify_add_watch` fails, log an error and notify the user that their project cannot be monitored.
- Consider using `fanotify` (Linux 2.6.37+) with filesystem-level marks instead of per-directory watches, though this requires `CAP_SYS_ADMIN` or `CAP_DAC_READ_SEARCH`.
- Alternatively, use a polling fallback: if inotify watch registration fails, fall back to polling that directory every 2-5 seconds.

**Warning signs:**
- Daemon logs show "No space left on device" errors (the confusing errno for inotify exhaustion)
- Some projects stop updating in the UI while others work fine
- Adding a new project registration causes existing watches to break
- `cat /proc/sys/fs/inotify/max_user_watches` shows a low value

**Phase to address:**
Phase 1 (daemon foundation). Must be addressed alongside file watching implementation. The polling fallback should be designed into the watcher abstraction from the start.

---

### Pitfall 7: Database as Bottleneck in the Hot Path

**What goes wrong:**
The pipeline is: file change -> parse -> database write -> WebSocket broadcast. Every file change triggers a database write. During active GSD execution, dozens of files change per second across multiple projects (STATE.md updates, PLAN.md writes, agent-history.json appends, SUMMARY.md updates). If each file event triggers a synchronous database write followed by a WebSocket broadcast, the database becomes the bottleneck. With SQLite, write serialization means all concurrent file events queue behind a single writer. Parse latency + DB write latency + broadcast latency compounds, causing state updates to lag seconds behind actual file changes.

**Why it happens:**
SQLite serializes all writes -- only one writer at a time, even in WAL mode. Developers test with single-project, single-phase scenarios and never see contention. The "database as cache" design pattern implies writes should be fast, but transactional writes with schema validation are not free. Checkpointing in WAL mode can stall if long-running read transactions prevent WAL file truncation, causing the WAL file to grow without bound.

**How to avoid:**
- Batch database writes: accumulate parsed state changes for 50-100ms, then write them all in a single transaction. This amortizes transaction overhead and reduces write lock contention.
- Separate the hot path from the cold path: WebSocket broadcasts should be triggered by the parsed state change, not by the database write completing. The database write can be async/eventual. The in-memory parsed state is the source of truth for real-time updates; the database is for persistence and history.
- Use WAL mode with `busy_timeout` set to 5000ms to handle brief contention.
- Ensure read transactions are short-lived -- never hold a read transaction open while waiting for WebSocket acknowledgment or client responses.
- Monitor WAL file size. If it grows beyond a few MB, investigate long-held read transactions preventing checkpointing.

**Warning signs:**
- Pipeline UI updates lag noticeably behind actual file changes (>500ms delay)
- SQLite "database is locked" errors in daemon logs
- WAL file (`.planning/gsd.db-wal`) grows to hundreds of MB
- CPU spikes during multi-project GSD execution correlate with DB write patterns

**Phase to address:**
Phase 1-2 (daemon data pipeline). The decision to batch writes and decouple WebSocket broadcasts from DB writes is an architectural decision that must be made when designing the parse-to-broadcast pipeline. Changing this later requires refactoring the entire data flow.

---

### Pitfall 8: xterm.js FitAddon Resize Chaos

**What goes wrong:**
The terminal does not properly resize when the browser window changes size, when the Console panel is shown/hidden, or when switching between Console sub-tabs. The FitAddon calculates incorrect dimensions (especially width collapsing to 1 column), the PTY on the server is not notified of the new size, and programs like vim/tmux running inside the PTY render garbled output because their assumed terminal dimensions do not match reality. Resizing triggers a cascade: browser resize event -> FitAddon.fit() -> terminal.resize() -> WebSocket message to server -> PTY resize -> program redraws. Any break in this chain causes visual corruption.

**Why it happens:**
FitAddon.fit() measures the terminal container's DOM dimensions to calculate rows/columns. If the container is hidden (display:none), has zero dimensions (during tab switching), or is mid-animation, FitAddon returns garbage values. The resize event fires multiple times during a window drag, each triggering a PTY resize, and the PTY resize is asynchronous -- data written between the resize request and the PTY acknowledging it may be formatted for the wrong dimensions. Multiple open issues in the xterm.js repository span years without resolution.

**How to avoid:**
- Debounce resize events (100-200ms) before calling FitAddon.fit().
- Guard against zero/negative dimensions: if FitAddon.proposeDimensions() returns cols < 2 or rows < 2, skip the resize.
- Only call fit() when the terminal container is visible and has non-zero dimensions. Use IntersectionObserver or visibility checks before fitting.
- When switching tabs, delay fit() until after the tab's CSS transition completes and the container has its final layout.
- Send resize dimensions to the server and await confirmation before assuming the PTY is at the new size.

**Warning signs:**
- Terminal content wraps incorrectly after browser resize
- vim/tmux shows garbled display after tab switching
- Terminal width becomes 1 column when console panel is toggled
- Resize flicker when opening/closing sidebar

**Phase to address:**
Phase where Console/terminal UI is implemented. The fit/resize logic must be considered from the initial terminal component design. It interacts with layout (tab switching, panel visibility) and requires coordination with the WebSocket resize message protocol.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Direct PTY-to-WebSocket pipe (no flow control) | Simpler implementation, works for interactive use | Browser crashes on high-output commands, memory leaks on server | Never -- flow control takes a day to implement upfront, weeks to retrofit |
| Polling `.planning/` files instead of inotify | Cross-platform, simpler code | Higher CPU usage, 1-5s latency on state updates, poor UX for "real-time" | As a fallback when inotify watches are exhausted, not as primary mechanism |
| Storing all state in memory (no database) | Faster development, no schema design needed | No historical data, no metrics over time, state lost on daemon restart | Never for this project -- historical execution data is a core requirement |
| Single WebSocket topic for all updates | Simple broadcast, no topic management | Every client receives every update for every project/user. Wastes bandwidth, breaks per-user isolation | Only in single-user MVP. Must be scoped to per-user topics before multi-user support |
| Parsing files on every inotify event (no debounce) | Immediate state reflection | Wasted CPU parsing half-written files, database churn from intermediate states | Never -- 50ms debounce is trivial to add and prevents cascading issues |
| Hardcoded terminal dimensions (80x24) | Skip resize complexity | Unusable on non-standard displays, broken UX on mobile/tablets, unfixable without refactor | Only for initial PTY bring-up testing, must add FitAddon before any demo |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| xterm.js + WebSocket | Sending raw binary PTY output over text WebSocket frames | Use binary WebSocket frames for PTY data. Text frames require UTF-8 validity; raw PTY output can contain arbitrary bytes that break text frame parsing |
| inotify + file parsing | Parsing on MODIFY event (fires mid-write) | Parse on IN_CLOSE_WRITE event, or debounce MODIFY events by 50-100ms. Validate parsed output before storing |
| SQLite + multi-threaded daemon | Opening one connection and sharing across threads | Use a connection pool (one connection per thread in WAL mode), or serialize all DB access through a dedicated writer thread/channel |
| PTY + WebSocket resize | Sending resize only from client to server | Must be bidirectional: client sends new dimensions after FitAddon.fit(), server acknowledges. Server may also need to send initial dimensions on session creation |
| OAuth2 proxy + WebSocket | Assuming OAuth2 proxy handles WebSocket upgrade transparently | Many OAuth2 proxies (including oauth2-proxy) need explicit configuration for WebSocket passthrough. Test WebSocket connections through the full auth proxy chain early |
| xterm.js addons (FitAddon, WebLinksAddon) | Loading all addons at terminal creation time | Load addons after terminal.open() is called and the terminal is attached to a visible DOM element. Loading before open() causes silent failures or incorrect initialization |
| Daemon + GSD state files | Assuming file format is stable across GSD versions | GSD is evolving software. Pin the expected frontmatter schema version. Implement graceful degradation when encountering unknown fields rather than crashing the parser |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-rendering entire pipeline visualization on every state update | UI feels sluggish, high CPU in browser during active execution | Use fine-grained reactivity (Svelte stores per-component, not one global store). Only update the specific plan card/phase that changed | >20 state updates per second across multiple phases |
| Unbounded terminal scrollback buffer | Browser memory grows without limit per terminal tab, page crashes after hours | Set scrollback limit (e.g., 5000-10000 lines). Default xterm.js scrollback is 1000 which is reasonable; do not set to 999999 | >50000 lines of output in a single terminal session |
| Broadcasting full state objects on every change | WebSocket bandwidth grows linearly with state size, client GC pressure from large JSON parse/serialize cycles | Send diffs/patches instead of full state. Include only changed fields. Use binary serialization (MessagePack/CBOR) for high-frequency updates | >10 simultaneous projects with frequent state changes |
| Synchronous file parsing in the inotify event handler | File watcher event queue backs up, events are dropped, state updates lag | Parse files in a separate task/thread. The inotify event handler should only enqueue work, not do it | >50 file events per second during heavy GSD execution |
| Creating a new WebSocket connection per terminal tab | Connection count grows linearly with open tabs, hits browser connection limits (6 per domain for HTTP/1.1) | Multiplex all terminal I/O and state updates over a single WebSocket connection using message framing (type + session_id + payload) | >6 terminal tabs open simultaneously |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| PTY processes run as the daemon's user without additional sandboxing | Users can execute arbitrary commands on the server with daemon privileges -- `rm -rf /`, read other users' files, access daemon's database | Run PTY processes as the authenticated user (via `su` or `setuid`), or in isolated namespaces/containers. At minimum, use `chroot` to restrict filesystem access to the user's registered project directories |
| WebSocket connection not validated against authenticated session | After initial OAuth2 authentication, the WebSocket upgrade may not carry session tokens, allowing unauthenticated WebSocket connections | Validate auth token on WebSocket upgrade handshake. Re-validate periodically (tokens expire). Reject WebSocket connections that cannot prove authentication |
| Per-user project isolation enforced only in the UI | JavaScript checks `user.projects` before rendering, but the WebSocket subscription or REST API returns all projects' data | Enforce isolation at the data layer: database queries filter by user_id, WebSocket subscriptions are scoped to the user's project set, PTY processes are scoped to user's directories |
| Terminal input is not rate-limited | A compromised or malicious client can flood the PTY with input faster than the shell can process, potentially causing DoS or enabling timing-based attacks | Rate-limit WebSocket messages from client. Cap input at a reasonable rate (e.g., 10KB/s of raw input). Log and alert on anomalous input patterns |
| Daemon parses `.planning/` files from arbitrary registered paths | A user registers `/etc/` or `/home/otheruser/` as a "project folder," and the daemon reads sensitive files | Validate registered project paths: must be within user's home directory, must actually contain a `.planning/` directory, must not be symlinks to sensitive locations |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No loading/skeleton states during initial data fetch | User sees empty pipeline, empty sidebar, thinks the app is broken, clicks away | Show skeleton UI immediately, populate as data arrives. Show "Loading projects..." with spinner in sidebar |
| Terminal tab auto-scrolls aggressively during output, preventing scroll-back reading | User tries to read earlier output while a build is running, keeps getting yanked to bottom | Only auto-scroll if user is already at the bottom. If user has scrolled up, show a "Jump to bottom" button and pause auto-scroll. Resume auto-scroll when user clicks it |
| Dark navy theme with insufficient contrast on status indicators | Users cannot distinguish between "running" (blue), "planned" (darker blue), and "background" on the dark navy palette (#040814 base) | Use the Aurora palette's accent colors (greens, ambers, reds) for status, not variations of the base navy. Test all status colors against the background at WCAG AA (4.5:1 minimum) |
| Pipeline view shows all phases at same visual weight | User cannot quickly identify which phase is currently active vs. completed vs. upcoming | Visually emphasize the active phase (glow, border, scale). Dim completed phases. Use opacity or desaturation for upcoming phases |
| Console tab switching feels broken when xterm.js is hidden/shown | Terminal content disappears, reappears garbled, or shows at wrong dimensions when switching between sub-tabs | Detach the terminal from DOM when hidden (do not use `display:none`) and reattach + fit when shown. Or keep terminals attached but in a stacking context where only the active one is visible |

## "Looks Done But Isn't" Checklist

- [ ] **File watching:** Often missing CLOSE_WRITE/MOVED_TO handling -- verify state parses correctly when files are saved by vim (uses rename), VS Code (uses atomic write), and direct echo/redirect
- [ ] **PTY sessions:** Often missing process group cleanup -- verify that killing a PTY session also kills child processes (run `sleep 9999 &` in a terminal, close the tab, check if the sleep process is gone)
- [ ] **WebSocket reconnection:** Often missing state resynchronization -- verify by disconnecting WiFi for 10 seconds during active GSD execution, reconnecting, and checking that all state is current
- [ ] **Flow control:** Often missing the server-side pause -- verify by running `yes` in a terminal and confirming the browser tab stays responsive and server memory does not grow
- [ ] **Multi-user isolation:** Often missing data-layer enforcement -- verify by logging in as User B and checking that User A's projects, terminal sessions, and state updates are completely invisible (including WebSocket messages)
- [ ] **Terminal resize:** Often missing hidden-container guard -- verify by switching to a different tab, resizing the browser window, switching back, and confirming the terminal renders correctly
- [ ] **Dark theme accessibility:** Often missing contrast testing -- verify all text, status indicators, and interactive elements meet WCAG AA contrast ratios (4.5:1 for normal text, 3:1 for large text) against the #040814 / #0a1224 backgrounds
- [ ] **Database persistence:** Often missing WAL checkpoint management -- verify by running the daemon under load for 24 hours and confirming the WAL file does not grow beyond a few MB
- [ ] **OAuth2 + WebSocket:** Often missing token refresh during long sessions -- verify by setting a short token TTL (e.g., 5 minutes), keeping a session open for 10 minutes, and confirming the WebSocket session is re-authenticated or gracefully terminated

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| File watcher race conditions | LOW | Add debouncing and validation. Existing parsed data in DB is likely fine since bad parses are transient. Rebuild DB from files if needed via a one-time re-parse command |
| PTY process leaks | LOW | Kill orphaned processes (`pkill -f` matching PTY process patterns). Add reaping logic. Restart daemon with cleanup-on-startup |
| Missing flow control | MEDIUM | Requires refactoring PTY-to-WebSocket bridge to add pause/resume protocol. Server and client must both change. ~2-3 days of work if interfaces are clean |
| WebSocket reconnection without resync | HIGH | Requires designing a state snapshot protocol, adding sequence numbers to all messages, and implementing server-side replay buffer. Touches every message type. ~1 week |
| Escape sequence injection | MEDIUM | Add server-side sanitization filter for non-PTY output (logs, historical data). PTY output cannot be filtered without breaking terminal functionality, so isolation is the defense |
| inotify watch exhaustion | LOW | Increase system limit (`sysctl fs.inotify.max_user_watches=524288`). Add fallback polling. ~1 day |
| Database bottleneck | HIGH | Requires architectural change to decouple DB writes from the hot path. Add in-memory state layer, batch writes, make broadcasts independent of DB. Touches core data flow. ~1 week |
| FitAddon resize issues | LOW-MEDIUM | Add dimension guards and debouncing. Most fixes are localized to the terminal component. ~1-2 days |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| File watcher race conditions | Phase 1 (Daemon foundation) | Save a file mid-GSD-execution and confirm UI does not flicker. Run automated test that writes partial file content and verifies parser discards it |
| PTY process leaks | Phase 2-3 (Console/PTY implementation) | Run `ps aux | grep pts` after closing 10 terminal tabs. Count should return to baseline. Monitor over 8-hour soak test |
| No flow control (output flooding) | Phase 2-3 (Console/PTY implementation) | Run `yes`, `find /`, `cat /dev/urandom | xxd` in browser terminal. Browser must stay responsive. Server memory must not grow |
| WebSocket reconnection state loss | Phase 1-2 (WebSocket infrastructure) | Disconnect network for 30s during active execution, reconnect, verify all pipeline state matches actual file state within 2 seconds |
| Terminal escape sequence injection | Phase 2-3 (Console) + Phase N (Security hardening) | Craft a file with malicious escape sequences, `cat` it in the terminal, verify no title changes or unexpected links appear. Verify User A cannot see User B's terminal output |
| inotify watch exhaustion | Phase 1 (Daemon foundation) | Register 20 projects, verify all are monitored. Artificially lower max_user_watches, verify daemon logs warning and falls back to polling |
| Database bottleneck | Phase 1-2 (Data pipeline) | Run GSD execution across 3 projects simultaneously. UI update latency must stay under 200ms. WAL file must stay under 10MB |
| FitAddon resize chaos | Phase 2-3 (Console UI) | Resize browser window while terminal is active. Switch between terminal tabs. Verify dimensions are correct and content is not garbled |

## Sources

- [xterm.js Flow Control Guide](https://xtermjs.org/docs/guides/flowcontrol/) -- Official documentation on backpressure and watermark patterns
- [xterm.js Security Guide](https://xtermjs.org/docs/guides/security/) -- Official security considerations for embedding terminals
- [xterm.js Security Pitfalls Issue #2443](https://github.com/xtermjs/xterm.js/issues/2443) -- Community discussion of trust boundaries
- [Teleport: xterm.js DCS Vulnerability](https://goteleport.com/blog/xterm-js-vulnerability-affects-vs-code-users/) -- Real-world escape sequence injection vulnerability
- [FitAddon Resize Issues](https://github.com/xtermjs/xterm.js/issues/4841) -- Ongoing dimension calculation problems
- [Terminal Resize Roundtrip Issue #1914](https://github.com/xtermjs/xterm.js/issues/1914) -- Race condition in resize chain
- [inotify Linux Manual](https://man7.org/linux/man-pages/man7/inotify.7.html) -- Authoritative reference on event coalescing, non-recursive watching, limits
- [inotify Limits Breaking Production](https://www.techtransit.org/inotify-limits-linux/) -- Real-world production failure from inotify exhaustion
- [Correct or inotify: pick one](https://wingolog.org/archives/2018/05/21/correct-or-inotify-pick-one) -- Analysis of why correct recursive directory watching with inotify is impossible
- [Deno File Watcher Race Condition #13035](https://github.com/denoland/deno/issues/13035) -- MODIFY event fires when file is empty mid-write
- [SQLite WAL Mode Documentation](https://sqlite.org/wal.html) -- Official docs on write serialization, checkpoint behavior, WAL growth
- [SQLite Concurrent Writes](https://oldmoe.blog/2024/07/08/the-write-stuff-concurrent-write-transactions-in-sqlite/) -- Deep dive on write contention in WAL mode
- [WebSocket Scaling Patterns for Dashboards](https://medium.com/@sparknp1/10-websocket-scaling-patterns-for-real-time-dashboards-1e9dc4681741) -- Common mistakes in real-time dashboard WebSocket architecture
- [WebSocket Architecture Best Practices](https://ably.com/topic/websocket-architecture-best-practices) -- Backpressure, fan-out, state management
- [WebSocket Heartbeat Implementation](https://oneuptime.com/blog/post/2026-01-27-websocket-heartbeat/view) -- Ping/pong patterns for zombie connection detection
- [Weaponizing ANSI Escape Sequences](https://www.packetlabs.net/posts/weaponizing-ansi-escape-sequences) -- Attack techniques via terminal escape injection
- [Terminal Escape Sequence Injection Repository](https://github.com/InfosecMatter/terminal-escape-injections) -- Catalog of injection techniques
- [VS Code Remote Zombie Processes #10730](https://github.com/microsoft/vscode-remote-release/issues/10730) -- Real-world PTY zombie accumulation in production
- [Inclusive Dark Mode Design](https://www.smashingmagazine.com/2025/04/inclusive-dark-mode-designing-accessible-dark-themes/) -- Accessibility pitfalls specific to dark themes

---
*Pitfalls research for: Real-time developer dashboard with file-watching pipeline, browser terminals, and WebSocket streaming*
*Researched: 2026-03-06*
