---
phase: 1
slug: backend-foundation-and-state-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test framework + cargo test |
| **Config file** | none — Cargo.toml `[dev-dependencies]` section |
| **Quick run command** | `cargo test --lib` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | INFRA-01 | integration | `cargo test --test server_integration` | Wave 0 | pending |
| 01-02-01 | 02 | 1 | STATE-01 | integration | `cargo test --test watcher_integration` | Wave 0 | pending |
| 01-02-02 | 02 | 1 | STATE-02 | unit | `cargo test parser::state_md::tests` | Wave 0 | pending |
| 01-02-03 | 02 | 1 | STATE-03 | unit | `cargo test parser::roadmap::tests` | Wave 0 | pending |
| 01-02-04 | 02 | 1 | STATE-04 | unit | `cargo test parser::plan::tests` | Wave 0 | pending |
| 01-02-05 | 02 | 1 | STATE-05 | unit | `cargo test parser::summary::tests` | Wave 0 | pending |
| 01-02-06 | 02 | 1 | STATE-06 | unit | `cargo test parser::summary::tests::commits` | Wave 0 | pending |
| 01-02-07 | 02 | 1 | STATE-07 | unit | `cargo test parser::stage::tests` | Wave 0 | pending |
| 01-02-08 | 02 | 1 | STATE-08 | unit | `cargo test parser::verification::tests` | Wave 0 | pending |
| 01-02-09 | 02 | 1 | STATE-09 | unit | `cargo test parser::config_json::tests` | Wave 0 | pending |
| 01-02-10 | 02 | 1 | STATE-10 | unit | `cargo test parser::agent_history::tests` | Wave 0 | pending |
| 01-02-11 | 02 | 1 | STATE-11 | integration | `cargo test --test db_integration` | Wave 0 | pending |
| 01-02-12 | 02 | 1 | STATE-12 | integration | `cargo test --test db_integration::history` | Wave 0 | pending |
| 01-03-01 | 03 | 2 | STATE-13 | integration | `cargo test --test ws_integration` | Wave 0 | pending |
| 01-03-02 | 03 | 2 | STATE-14 | integration | `cargo test --test ws_integration::reconnect` | Wave 0 | pending |
| 01-03-03 | 03 | 2 | STATE-15 | integration | `cargo test --test api_integration` | Wave 0 | pending |

*Status: pending / green / red / flaky*

---

## Wave 0 Requirements

- [ ] `tests/watcher_integration.rs` — stubs for STATE-01 (file watcher timing)
- [ ] `tests/db_integration.rs` — stubs for STATE-11, STATE-12 (database persistence)
- [ ] `tests/ws_integration.rs` — stubs for STATE-13, STATE-14 (WebSocket protocol)
- [ ] `tests/api_integration.rs` — stubs for STATE-15, INFRA-01 (REST API + server)
- [ ] Unit test modules in each `src/parser/*.rs` file — stubs for STATE-02 through STATE-10
- [ ] Test fixtures: sample `.planning/` directory with all GSD file types
- [ ] `[dev-dependencies]` in Cargo.toml: `tokio-test`, `tempfile`, `axum-test`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| File change detection within 100ms | STATE-01 | Timing-sensitive, CI variability | 1. Start daemon watching test project 2. Write file 3. Verify WebSocket update arrives within 100ms using timestamp comparison |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
