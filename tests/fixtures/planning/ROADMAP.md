# Roadmap: GSD Pipeline UI

## Overview

GSD Pipeline UI delivers a browser-based dashboard for observing and interacting with GSD agent pipelines in real-time.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [ ] **Phase 1: Backend Foundation and State Pipeline** - Axum daemon, file watcher, GSD state parsers
- [ ] **Phase 2: Pipeline Dashboard** - Pipeline visualization frontend
- [ ] **Phase 2.1: Hotfix Sprint** - Critical fixes identified during Phase 2
- [ ] **Phase 3: Interactive Terminal System** - Browser-based PTY terminals

## Phase Details

### Phase 1: Backend Foundation and State Pipeline
**Goal**: The daemon can watch any registered project's `.planning/` directory and push live state updates
**Depends on**: Nothing (first phase)
**Requirements**: STATE-01, STATE-02, STATE-03
**Plans:** 4 plans

Plans:
- [x] 01-01-PLAN.md -- Project scaffold, REST API, database layer
- [ ] 01-02-PLAN.md -- GSD state file parsers with TDD
- [ ] 01-03-PLAN.md -- File watcher with custom debouncer
- [ ] 01-04-PLAN.md -- WebSocket state endpoint

### Phase 2: Pipeline Dashboard
**Goal**: Users can see the full GSD pipeline status in real-time
**Depends on**: Phase 1
**Requirements**: PIPE-01, PIPE-02
**Plans**: 3 plans

Plans:
- [ ] 02-01-PLAN.md -- SvelteKit scaffold, Aurora theme
- [ ] 02-02-PLAN.md -- WebSocket client, phase timeline
- [ ] 02-03-PLAN.md -- Wave visualization, plan cards

### Phase 2.1: Hotfix Sprint
**Goal**: Address critical issues discovered during Phase 2 development
**Depends on**: Phase 2
**Requirements**: PIPE-03
**Plans**: 1 plan

Plans:
- [ ] 02.1-01-PLAN.md -- Fix rendering bugs in plan cards

### Phase 3: Interactive Terminal System
**Goal**: Users can open interactive terminal sessions in the browser
**Depends on**: Phase 1
**Requirements**: TERM-01, TERM-02
**Plans**: 2 plans

Plans:
- [ ] 03-01-PLAN.md -- PTY manager and binary WebSocket endpoint
- [ ] 03-02-PLAN.md -- xterm.js Console tab with sub-tabs

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 2.1 -> 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Backend Foundation and State Pipeline | 1/4 | In Progress | - |
| 2. Pipeline Dashboard | 0/3 | Not started | - |
| 2.1. Hotfix Sprint | 0/1 | Not started | - |
| 3. Interactive Terminal System | 0/2 | Not started | - |
