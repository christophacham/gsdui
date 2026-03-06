<script lang="ts">
	/**
	 * Phase detail view.
	 *
	 * Shows the selected phase's header (name, goal, requirements),
	 * stage rail progression, and a placeholder area for wave swim lanes
	 * (to be filled by Plan 03).
	 */

	import { projectStore } from '$lib/stores/project.svelte.js';
	import StageRail from './StageRail.svelte';
	import Skeleton from '$lib/components/shared/Skeleton.svelte';

	interface Props {
		/** The currently selected phase number */
		phaseNumber: string;
	}

	let { phaseNumber }: Props = $props();

	/** Find the phase data from the project store */
	const phase = $derived(
		projectStore.state?.phases.find((p) => p.phase_number === phaseNumber) ?? null
	);

	/** Get the plans for this phase */
	const plans = $derived(projectStore.state?.plans[phaseNumber] ?? []);

	/** Count requirements (comma-separated string) */
	const requirementCount = $derived(
		phase?.requirements ? phase.requirements.split(',').filter((r) => r.trim()).length : 0
	);
</script>

{#if !phase}
	<div class="detail-loading">
		<Skeleton width="40%" height="1.5rem" />
		<Skeleton width="70%" height="1rem" />
		<Skeleton width="100%" height="60px" />
	</div>
{:else}
	<div class="phase-detail">
		<div class="phase-header">
			<div class="header-top">
				<h3 class="phase-name">
					<span class="phase-num">{phase.phase_number}</span>
					{phase.phase_name}
				</h3>
				{#if requirementCount > 0}
					<span class="req-badge" title={phase.requirements ?? ''}>
						{requirementCount} req{requirementCount > 1 ? 's' : ''}
					</span>
				{/if}
			</div>
			{#if phase.goal}
				<p class="phase-goal">{phase.goal}</p>
			{/if}
		</div>

		<StageRail currentStage={phase.stage} />

		<div class="wave-placeholder" id="wave-container">
			<p class="placeholder-text">
				{#if plans.length > 0}
					{plans.length} plan{plans.length > 1 ? 's' : ''} in this phase. Wave swim lanes will render here.
				{:else}
					No plans found for this phase.
				{/if}
			</p>
		</div>
	</div>
{/if}

<style>
	.phase-detail {
		background: var(--bg-base);
		padding: var(--space-4);
	}

	.detail-loading {
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.phase-header {
		margin-bottom: var(--space-2);
	}

	.header-top {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: var(--space-1);
	}

	.phase-name {
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--fg-primary);
		margin: 0;
	}

	.phase-num {
		font-family: var(--font-mono);
		color: var(--fg-accent);
		font-size: var(--text-sm);
		margin-right: var(--space-1);
	}

	.req-badge {
		font-size: var(--text-xs);
		color: var(--fg-muted);
		background: var(--bg-elevated);
		padding: 2px 8px;
		border-radius: 9999px;
		border: 1px solid var(--border-subtle);
		white-space: nowrap;
	}

	.phase-goal {
		font-size: var(--text-sm);
		color: var(--fg-secondary);
		margin: 0;
		line-height: 1.5;
	}

	.wave-placeholder {
		margin-top: var(--space-4);
		padding: var(--space-6);
		background: var(--bg-surface);
		border: 1px dashed var(--border-subtle);
		border-radius: 8px;
		text-align: center;
	}

	.placeholder-text {
		color: var(--fg-muted);
		font-size: var(--text-sm);
		margin: 0;
	}
</style>
