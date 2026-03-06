<script lang="ts">
	/**
	 * Phase detail view.
	 *
	 * Shows the selected phase's header (name, goal, requirements),
	 * stage rail progression, and wave swim lanes with plan cards.
	 */

	import { projectStore } from '$lib/stores/project.svelte.js';
	import StageRail from './StageRail.svelte';
	import WaveContainer from './WaveContainer.svelte';
	import AgentRoutingPanel from '$lib/components/routing/AgentRoutingPanel.svelte';
	import Skeleton from '$lib/components/shared/Skeleton.svelte';

	interface Props {
		/** The currently selected phase number */
		phaseNumber: string;
		/** Project ID for file API URLs */
		projectId: string;
	}

	let { phaseNumber, projectId }: Props = $props();

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

	/** Stages that indicate plans exist and wave lanes should render */
	const planReadyStages = new Set([
		'planned_ready',
		'executing',
		'executed',
		'verified'
	]);

	const showWaveLanes = $derived(phase ? planReadyStages.has(phase.stage) : false);
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

		<div class="wave-section">
			{#if showWaveLanes}
				<WaveContainer {phaseNumber} {projectId} />
			{:else if plans.length > 0}
				<div class="not-planned-message">
					<p>{plans.length} plan{plans.length > 1 ? 's' : ''} in this phase.</p>
					<p class="stage-hint">Plans will appear as wave lanes once planning is complete.</p>
				</div>
			{:else}
				<div class="not-planned-message">
					<p>Phase has not been planned yet.</p>
					<p class="stage-hint">Current stage: {phase?.stage ?? 'unknown'}</p>
				</div>
			{/if}
		</div>

		{#if plans.length > 0}
			<div class="routing-section">
				<AgentRoutingPanel {phaseNumber} {plans} {projectId} />
			</div>
		{/if}
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

	.wave-section {
		margin-top: var(--space-4);
		flex: 1;
		overflow-y: auto;
	}

	.not-planned-message {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 120px;
		gap: var(--space-2);
		padding: var(--space-6);
		background: var(--bg-surface);
		border: 1px dashed var(--border-subtle);
		border-radius: 8px;
		text-align: center;
		color: var(--fg-muted);
		font-size: var(--text-sm);
	}

	.stage-hint {
		font-size: var(--text-xs);
		font-style: italic;
	}

	.routing-section {
		margin-top: var(--space-4);
	}
</style>
