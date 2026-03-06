---
phase: 02-pipeline-dashboard
plan: 03
subsystem: ui
tags: [svelte5, pipeline, wave-lanes, plan-cards, dependency-arrows, svg, markdown-viewer, runes]

# Dependency graph
requires:
  - phase: 02-pipeline-dashboard
    plan: 01
    provides: SvelteKit scaffold, Aurora theme tokens, TypeScript types, project/settings stores
provides:
  - Wave swim lanes grouping plans by wave number with horizontal layout
  - Plan cards (collapsed and expanded) with agent badges, status icons, stats
  - SVG dependency arrow overlay with cubic Bezier paths and chain highlighting
  - Markdown and diff viewer panels for PLAN.md/SUMMARY.md content
  - Dependency graph traversal utilities (transitive deps, dependents, full chain)
  - SVG path generation utilities for card-to-card connectors
affects: [02-04]

# Tech tracking
tech-stack:
  added: []
  patterns: [svg-dependency-overlay, cubic-bezier-connectors, debounced-resize-recalculation, dependency-chain-highlighting]

key-files:
  created:
    - frontend/src/lib/utils/dependency-graph.ts
    - frontend/src/lib/utils/svg-paths.ts
    - frontend/src/lib/components/pipeline/AgentBadge.svelte
    - frontend/src/lib/components/pipeline/StatusIcon.svelte
    - frontend/src/lib/components/pipeline/PlanCard.svelte
    - frontend/src/lib/components/pipeline/PlanCardExpanded.svelte
    - frontend/src/lib/components/pipeline/WaveLane.svelte
    - frontend/src/lib/components/pipeline/WaveContainer.svelte
    - frontend/src/lib/components/pipeline/DependencyArrows.svelte
    - frontend/src/lib/components/viewers/MarkdownPanel.svelte
    - frontend/src/lib/components/viewers/DiffPanel.svelte
  modified:
    - frontend/src/lib/components/pipeline/PhaseDetail.svelte
    - frontend/src/routes/+page.svelte

key-decisions:
  - "Manual +/- line coloring for DiffPanel instead of @git-diff-view/svelte (simpler, no compatibility risk with Svelte 5)"
  - "color-mix() CSS for AgentBadge backgrounds (15% agent color mixed with transparent for subtle tinted pills)"
  - "MutationObserver + ResizeObserver on DependencyArrows for auto-recalculation when cards expand/collapse"
  - "WaveContainer uses $derived with closure pattern for wave grouping computation"
  - "Phase-aware wave display: WaveContainer only renders when phase stage is planned_ready or later"

patterns-established:
  - "Dependency graph utilities: parseDependsOn handles null, CSV, JSON array formats uniformly"
  - "SVG overlay pattern: absolutely-positioned SVG with pointer-events:none, stroke-only pointer-events on paths"
  - "Debounced recalculation: 50ms debounce + requestAnimationFrame for layout-dependent calculations"
  - "Panel overlay pattern: fixed overlay with side-panel slide-in, Escape to close"

requirements-completed: [PIPE-03, PIPE-04, PIPE-05, PIPE-06, PIPE-09, UI-03]

# Metrics
duration: 7min
completed: 2026-03-06
---

# Phase 2 Plan 03: Wave Swim Lanes and Plan Cards Summary

**Wave swim lanes with plan cards showing agent/status/stats, SVG dependency arrows with chain highlighting, and markdown/diff viewer panels for drilling into plan files**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-06T22:23:25Z
- **Completed:** 2026-03-06T22:30:31Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments
- Wave swim lanes render plans grouped by wave number, parallel plans side by side, with scroll fade indicators
- Plan cards show status icon (pending/working/done/failed with pulse), plan name, agent badge (claude=amber, codex=green, gemini=purple), run count, file count, duration, wave badge
- Expanded plan cards show output area with run summary, PLAN.md/Diff/SUMMARY.md file links, and disabled "Jump to Console" button
- SVG dependency arrows connect plan cards with cubic Bezier paths, colored by dependency status (green=satisfied, blue-animated=in-progress, gray=pending)
- Clicking/hovering a plan card highlights the full transitive dependency chain; non-chain cards dim to 0.3 opacity
- MarkdownPanel renders fetched markdown with highlight.js syntax highlighting
- DiffPanel renders unified diff with manual +/- line coloring (additions=green, deletions=red)
- Dependency graph utilities handle all depends_on formats (null, CSV, JSON array) and compute transitive chains
- PhaseDetail wired with WaveContainer, replacing placeholder from Plan 02

## Task Commits

Each task was committed atomically:

1. **Task 1: Utility functions, agent badge, status icon, plan card components** - `3d9cff6` (feat)
2. **Task 2: Wave swim lanes, dependency arrows, and file viewer panels** - `9783113` (feat)
3. **Task 3: Wire wave visualization into PhaseDetail and Pipeline page** - `f76a18d` (feat)

## Files Created/Modified
- `frontend/src/lib/utils/dependency-graph.ts` - parseDependsOn, getTransitiveDeps, getTransitiveDependents, getFullChain
- `frontend/src/lib/utils/svg-paths.ts` - cubicConnectorPath, getBottomCenter, getTopCenter
- `frontend/src/lib/components/pipeline/AgentBadge.svelte` - Colored pill badge per agent type
- `frontend/src/lib/components/pipeline/StatusIcon.svelte` - SVG icons for 4 plan statuses
- `frontend/src/lib/components/pipeline/PlanCard.svelte` - Collapsed card row with all status info
- `frontend/src/lib/components/pipeline/PlanCardExpanded.svelte` - Expanded area with output, file links
- `frontend/src/lib/components/pipeline/WaveLane.svelte` - Horizontal row of cards per wave
- `frontend/src/lib/components/pipeline/WaveContainer.svelte` - Groups plans by wave, manages expand/highlight state
- `frontend/src/lib/components/pipeline/DependencyArrows.svelte` - SVG overlay with Bezier arrows
- `frontend/src/lib/components/viewers/MarkdownPanel.svelte` - Side panel with marked + highlight.js
- `frontend/src/lib/components/viewers/DiffPanel.svelte` - Side panel with +/- line coloring
- `frontend/src/lib/components/pipeline/PhaseDetail.svelte` - Updated: replaced placeholder with WaveContainer
- `frontend/src/routes/+page.svelte` - Updated: passes projectId, scrollable detail area

## Decisions Made
- Used manual +/- line coloring for DiffPanel rather than @git-diff-view/svelte to avoid Svelte 5 compatibility risk
- Used color-mix() CSS function for AgentBadge pill backgrounds (15% tint of agent color)
- DependencyArrows uses both ResizeObserver and MutationObserver for comprehensive recalculation triggers
- WaveContainer shows wave lanes only when phase has reached planned_ready stage or later

## Deviations from Plan

None - plan executed exactly as written. Plan 02-02 had already been executed, so StageRail and PhaseDetail existed and were updated rather than created from scratch.

## Issues Encountered
- lefthook has a `no-ai-in-commit-msg` hook that blocks Co-Authored-By lines with AI attribution; commits made without that trailer

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full pipeline visualization chain operational: timeline -> phase chip -> stage rail -> wave lanes -> plan cards -> dependency arrows
- File viewer panels ready for PLAN.md and SUMMARY.md content
- Ready for Plan 04 (settings panel and agent routing configuration)

## Self-Check: PASSED

All 14 key files verified present. All 3 task commits verified in git log.

---
*Phase: 02-pipeline-dashboard*
*Completed: 2026-03-06*
