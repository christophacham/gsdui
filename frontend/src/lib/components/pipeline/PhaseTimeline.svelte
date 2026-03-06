<script lang="ts">
	import type { PhaseState } from '$lib/types/api.js';
	import { projectStore } from '$lib/stores/project.svelte.js';
	import { settingsStore } from '$lib/stores/settings.svelte.js';
	import { sortPhaseNumbers } from '$lib/utils/phase-sort.js';
	import MilestoneGroup from './MilestoneGroup.svelte';
	import Skeleton from '$lib/components/shared/Skeleton.svelte';

	interface Props {
		onPhaseSelect: (phaseNumber: string) => void;
		selectedPhaseNumber: string | null;
	}

	let { onPhaseSelect, selectedPhaseNumber }: Props = $props();

	/** Track which milestone groups are collapsed */
	let collapsedMilestones = $state<Set<string>>(new Set());

	/** Sorted phases from the project store */
	const sortedPhases = $derived(
		projectStore.state ? sortPhaseNumbers(projectStore.state.phases) : []
	);

	/** All execution runs from the project store */
	const allRuns = $derived(projectStore.state?.recent_runs ?? []);

	/**
	 * Group phases by milestone.
	 * Since PhaseState doesn't have per-phase milestone info, we treat all phases
	 * as belonging to the project-level milestone (from STATE.md).
	 * Future: could derive from ROADMAP structure.
	 */
	const milestoneGroups = $derived.by(() => {
		const milestone = projectStore.state?.project.name ?? 'Project';
		return [{ milestone, phases: sortedPhases }];
	});

	/**
	 * Auto-select the currently executing phase on initial data load.
	 * Priority: executing > next pending > last phase
	 */
	let hasAutoSelected = $state(false);

	$effect(() => {
		if (sortedPhases.length > 0 && !hasAutoSelected && !selectedPhaseNumber) {
			const executing = sortedPhases.find((p) => p.stage === 'executing');
			if (executing) {
				onPhaseSelect(executing.phase_number);
			} else {
				const pending = sortedPhases.find(
					(p) => p.stage === 'planned' || p.stage === 'discussed' || p.stage === 'researched' || p.stage === 'planned_ready'
				);
				if (pending) {
					onPhaseSelect(pending.phase_number);
				} else {
					// All done; select last phase
					onPhaseSelect(sortedPhases[sortedPhases.length - 1].phase_number);
				}
			}
			hasAutoSelected = true;

			// Collapse completed milestones by default
			for (const group of milestoneGroups) {
				const allComplete = group.phases.every(
					(p) => p.stage === 'executed' || p.stage === 'verified'
				);
				if (allComplete) {
					collapsedMilestones.add(group.milestone);
					// Trigger reactivity on the Set
					collapsedMilestones = new Set(collapsedMilestones);
				}
			}
		}
	});

	function toggleMilestone(milestone: string) {
		const next = new Set(collapsedMilestones);
		if (next.has(milestone)) {
			next.delete(milestone);
		} else {
			next.add(milestone);
		}
		collapsedMilestones = next;
	}

	function handlePhaseSelect(phase: PhaseState) {
		onPhaseSelect(phase.phase_number);
	}

	/** Track if timeline overflows for scroll indicators */
	let timelineEl: HTMLDivElement | undefined = $state();
	let showLeftFade = $state(false);
	let showRightFade = $state(false);

	function updateScrollIndicators() {
		if (!timelineEl) return;
		showLeftFade = timelineEl.scrollLeft > 4;
		showRightFade =
			timelineEl.scrollLeft < timelineEl.scrollWidth - timelineEl.clientWidth - 4;
	}

	$effect(() => {
		if (timelineEl) {
			updateScrollIndicators();
		}
	});
</script>

<div class="timeline-strip" class:left-fade={showLeftFade} class:right-fade={showRightFade}>
	{#if projectStore.loading}
		<div class="skeleton-row">
			<Skeleton width="120px" height="36px" />
			<Skeleton width="160px" height="36px" />
			<Skeleton width="140px" height="36px" />
			<Skeleton width="130px" height="36px" />
		</div>
	{:else}
		<div
			class="timeline-scroll"
			bind:this={timelineEl}
			onscroll={updateScrollIndicators}
			role="tablist"
			aria-label="Phase timeline"
		>
			{#each milestoneGroups as group (group.milestone)}
				<MilestoneGroup
					milestone={group.milestone}
					phases={group.phases}
					runs={allRuns}
					collapsed={collapsedMilestones.has(group.milestone)}
					{selectedPhaseNumber}
					density={settingsStore.timelineDensity}
					onToggle={() => toggleMilestone(group.milestone)}
					onSelectPhase={handlePhaseSelect}
				/>
			{/each}
		</div>
	{/if}
</div>

<style>
	.timeline-strip {
		position: sticky;
		top: 0;
		z-index: 10;
		background: var(--bg-surface);
		border-bottom: 1px solid var(--border-subtle);
		padding: var(--space-2) var(--space-3);
		min-height: 52px;
		display: flex;
		align-items: center;
	}

	.timeline-scroll {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		overflow-x: auto;
		overflow-y: hidden;
		scrollbar-width: thin;
		scroll-behavior: smooth;
		width: 100%;
	}

	/* Scroll fade indicators */
	.timeline-strip.left-fade::before,
	.timeline-strip.right-fade::after {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		width: 24px;
		z-index: 1;
		pointer-events: none;
	}

	.timeline-strip.left-fade::before {
		left: 0;
		background: linear-gradient(to right, var(--bg-surface), transparent);
	}

	.timeline-strip.right-fade::after {
		right: 0;
		background: linear-gradient(to left, var(--bg-surface), transparent);
	}

	.skeleton-row {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-1) 0;
	}
</style>
