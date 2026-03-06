<script lang="ts">
	import type { PhaseState, ExecutionRun } from '$lib/types/api.js';
	import PhaseChip from './PhaseChip.svelte';

	interface Props {
		milestone: string;
		phases: PhaseState[];
		runs: ExecutionRun[];
		collapsed: boolean;
		selectedPhaseNumber: string | null;
		density: 'rich' | 'medium' | 'minimal';
		onToggle: () => void;
		onSelectPhase: (phase: PhaseState) => void;
	}

	let {
		milestone,
		phases,
		runs,
		collapsed,
		selectedPhaseNumber,
		density,
		onToggle,
		onSelectPhase
	}: Props = $props();

	/** Check if all phases in this milestone are complete (executed or verified) */
	const allComplete = $derived(
		phases.length > 0 &&
			phases.every((p) => p.stage === 'executed' || p.stage === 'verified')
	);
</script>

{#if collapsed}
	<button
		class="milestone-chip"
		class:complete={allComplete}
		onclick={onToggle}
		type="button"
		title="Click to expand {milestone} ({phases.length} phases)"
	>
		{#if allComplete}
			<span class="check-icon">&#10003;</span>
		{/if}
		<span class="milestone-label">{milestone}</span>
		<span class="phase-count">{phases.length} phases</span>
	</button>
{:else}
	<div class="milestone-expanded">
		{#if phases.length > 1}
			<button
				class="milestone-header"
				onclick={onToggle}
				type="button"
				title="Click to collapse {milestone}"
			>
				<span class="milestone-label">{milestone}</span>
			</button>
		{/if}
		{#each phases as phase (phase.id)}
			<PhaseChip
				{phase}
				{runs}
				selected={selectedPhaseNumber === phase.phase_number}
				{density}
				onclick={() => onSelectPhase(phase)}
			/>
		{/each}
	</div>
{/if}

<style>
	.milestone-chip {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--bg-elevated);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		cursor: pointer;
		white-space: nowrap;
		transition: background var(--transition-fast);
		font-family: var(--font-sans);
		font-size: var(--text-sm);
		color: var(--fg-secondary);
		flex-shrink: 0;
	}

	.milestone-chip:hover {
		filter: brightness(1.15);
	}

	.milestone-chip.complete {
		border-color: var(--status-done);
	}

	.check-icon {
		color: var(--status-done);
		font-size: var(--text-sm);
	}

	.milestone-label {
		font-weight: 600;
		color: var(--fg-primary);
	}

	.phase-count {
		font-size: var(--text-xs);
		color: var(--fg-muted);
	}

	.milestone-expanded {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.milestone-header {
		display: flex;
		align-items: center;
		padding: var(--space-1) var(--space-2);
		background: none;
		border: 1px solid transparent;
		border-radius: 4px;
		cursor: pointer;
		font-family: var(--font-sans);
		font-size: var(--text-xs);
		color: var(--fg-muted);
		flex-shrink: 0;
	}

	.milestone-header:hover {
		border-color: var(--border-subtle);
		color: var(--fg-secondary);
	}

	.milestone-header .milestone-label {
		font-weight: 500;
		font-size: var(--text-xs);
		color: inherit;
	}
</style>
