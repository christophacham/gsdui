<script lang="ts">
	import type { PhaseState, ExecutionRun } from '$lib/types/api.js';
	import { calculatePhaseDuration, formatDuration } from '$lib/utils/duration.js';
	import ProgressBar from '$lib/components/shared/ProgressBar.svelte';

	interface Props {
		phase: PhaseState;
		runs: ExecutionRun[];
		selected: boolean;
		density: 'rich' | 'medium' | 'minimal';
		onclick: () => void;
	}

	let { phase, runs, selected, density, onclick }: Props = $props();

	/** Map backend stage values to display labels and status categories */
	const stageInfo = $derived.by(() => {
		const map: Record<string, { label: string; color: string }> = {
			planned: { label: 'Planned', color: '--status-pending' },
			discussed: { label: 'Discussed', color: '--status-pending' },
			researched: { label: 'Researched', color: '--status-pending' },
			planned_ready: { label: 'Ready', color: '--status-working' },
			executing: { label: 'Executing', color: '--status-working' },
			executed: { label: 'Executed', color: '--status-done' },
			verified: { label: 'Verified', color: '--status-done' }
		};
		return map[phase.stage] ?? { label: phase.stage, color: '--status-pending' };
	});

	/** Progress percentage from completed/total plan counts */
	const progress = $derived(
		phase.plan_count && phase.plan_count > 0
			? ((phase.completed_plan_count ?? 0) / phase.plan_count) * 100
			: 0
	);

	/** Duration calculated from execution runs for this phase */
	const phaseRuns = $derived(runs.filter((r) => r.phase_number === phase.phase_number));
	const duration = $derived(calculatePhaseDuration(phaseRuns));
</script>

<button
	class="phase-chip"
	class:selected
	class:minimal={density === 'minimal'}
	onclick={onclick}
	type="button"
	aria-pressed={selected}
	title="{phase.phase_name} ({stageInfo.label})"
>
	<span class="phase-number">{phase.phase_number}</span>

	{#if density === 'minimal'}
		<span class="stage-dot" style:background="var({stageInfo.color})"></span>
	{:else}
		<span class="phase-name">{phase.phase_name}</span>
		<span class="stage-badge" style:background="var({stageInfo.color})">
			{stageInfo.label}
		</span>

		{#if density === 'rich'}
			{#if phase.plan_count && phase.plan_count > 0}
				<div class="progress-area">
					<ProgressBar value={progress} height="3px" />
					<span class="progress-label">{phase.completed_plan_count ?? 0}/{phase.plan_count}</span>
				</div>
			{/if}
			<span class="duration">{formatDuration(duration)}</span>
		{/if}
	{/if}
</button>

<style>
	.phase-chip {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--bg-surface);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		cursor: pointer;
		white-space: nowrap;
		transition: background var(--transition-fast), border-color var(--transition-fast);
		font-family: var(--font-sans);
		font-size: var(--text-sm);
		color: var(--fg-primary);
		flex-shrink: 0;
	}

	.phase-chip:hover {
		filter: brightness(1.15);
	}

	.phase-chip.selected {
		background: var(--bg-elevated);
		border-color: var(--border-focus);
	}

	.phase-chip.minimal {
		padding: var(--space-1) var(--space-2);
		gap: var(--space-1);
	}

	.phase-number {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: var(--text-xs);
		color: var(--fg-accent);
	}

	.phase-name {
		color: var(--fg-primary);
		max-width: 140px;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.stage-badge {
		font-size: var(--text-xs);
		padding: 1px 6px;
		border-radius: 9999px;
		color: #fff;
		font-weight: 500;
		line-height: 1.4;
	}

	.stage-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.progress-area {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-width: 60px;
	}

	.progress-label {
		font-size: var(--text-xs);
		color: var(--fg-muted);
		font-family: var(--font-mono);
	}

	.duration {
		font-size: var(--text-xs);
		color: var(--fg-muted);
		font-family: var(--font-mono);
	}
</style>
