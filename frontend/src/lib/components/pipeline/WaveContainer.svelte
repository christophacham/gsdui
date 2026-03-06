<script lang="ts">
	/**
	 * Wave swim lanes container - groups plans by wave number.
	 *
	 * Reads plans from projectStore, groups by wave, tracks expanded/highlighted state,
	 * renders WaveLane components stacked vertically with DependencyArrows SVG overlay.
	 */

	import type { PlanState, ExecutionRun, AgentSession } from '$lib/types/api.js';
	import { projectStore } from '$lib/stores/project.svelte.js';
	import { settingsStore } from '$lib/stores/settings.svelte.js';
	import { getFullChain } from '$lib/utils/dependency-graph.js';
	import WaveLane from './WaveLane.svelte';
	import DependencyArrows from './DependencyArrows.svelte';

	interface Props {
		phaseNumber: string;
		projectId: string;
	}

	let { phaseNumber, projectId }: Props = $props();

	/** Container element reference for DependencyArrows positioning */
	let containerEl: HTMLDivElement | undefined = $state();

	/** Which plan card is expanded (null = none) */
	let expandedPlan = $state<string | null>(null);

	/** Which plan is highlighted for dependency chain */
	let highlightedPlan = $state<string | null>(null);

	/** All plans for this phase */
	const phasePlans = $derived<PlanState[]>(
		projectStore.state?.plans[phaseNumber] ?? []
	);

	/** All runs for this phase */
	const phaseRuns = $derived<ExecutionRun[]>(
		(projectStore.state?.recent_runs ?? []).filter(
			(r) => r.phase_number === phaseNumber
		)
	);

	/** All agent sessions for this phase */
	const phaseAgentSessions = $derived<AgentSession[]>(
		(projectStore.state?.agent_sessions ?? []).filter(
			(s) => s.phase_number === phaseNumber
		)
	);

	/** Group plans by wave number, sorted ascending */
	const waveGroups = $derived(() => {
		const groups = new Map<number, PlanState[]>();
		for (const plan of phasePlans) {
			const wave = plan.wave ?? 0;
			if (!groups.has(wave)) {
				groups.set(wave, []);
			}
			groups.get(wave)!.push(plan);
		}

		// Sort by wave number
		return [...groups.entries()]
			.sort(([a], [b]) => a - b)
			.map(([wave, plans]) => ({
				wave,
				plans: plans.sort((a, b) => a.plan_number.localeCompare(b.plan_number))
			}));
	});

	/** Compute highlighted chain from the currently highlighted plan */
	const highlightedChain = $derived<Set<string> | null>(
		highlightedPlan ? getFullChain(highlightedPlan, phasePlans) : null
	);

	/** Auto-expand the active plan */
	$effect(() => {
		if (settingsStore.autoExpandActive && !expandedPlan) {
			const activePlan = phasePlans.find((p) => p.status === 'working');
			if (activePlan) {
				expandedPlan = activePlan.plan_number;
			}
		}
	});

	function handleTogglePlan(planNumber: string) {
		expandedPlan = expandedPlan === planNumber ? null : planNumber;
	}

	function handleHighlight(planNumber: string | null) {
		highlightedPlan = planNumber;
	}
</script>

<div class="wave-container" bind:this={containerEl}>
	{#if phasePlans.length === 0}
		<div class="empty-state">
			<p>No plans found for this phase.</p>
		</div>
	{:else}
		<div class="wave-lanes">
			{#each waveGroups() as group (group.wave)}
				<WaveLane
					wave={group.wave}
					plans={group.plans}
					runs={phaseRuns}
					agentSessions={phaseAgentSessions}
					{highlightedChain}
					{expandedPlan}
					onTogglePlan={handleTogglePlan}
					onHighlight={handleHighlight}
					{projectId}
				/>
			{/each}
		</div>

		<DependencyArrows
			plans={phasePlans}
			containerEl={containerEl ?? null}
			{highlightedChain}
		/>
	{/if}
</div>

<style>
	.wave-container {
		position: relative;
		min-height: 100px;
	}

	.wave-lanes {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		position: relative;
		z-index: 2;
	}

	.empty-state {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 120px;
		color: var(--fg-muted);
		font-size: var(--text-sm);
	}
</style>
