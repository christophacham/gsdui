<script lang="ts">
	/**
	 * Wave swim lane - horizontal row of plan cards for a single wave.
	 *
	 * Plans within the same wave sit side by side (parallel execution).
	 * Horizontal scroll if plans overflow, with gradient fade indicators.
	 */

	import type { PlanState, ExecutionRun, AgentSession } from '$lib/types/api.js';
	import PlanCard from './PlanCard.svelte';
	import PlanCardExpanded from './PlanCardExpanded.svelte';

	interface Props {
		wave: number;
		plans: PlanState[];
		runs: ExecutionRun[];
		agentSessions: AgentSession[];
		highlightedChain: Set<string> | null;
		expandedPlan: string | null;
		onTogglePlan: (planNumber: string) => void;
		onHighlight: (planNumber: string | null) => void;
		projectId: string;
	}

	let {
		wave,
		plans,
		runs,
		agentSessions,
		highlightedChain,
		expandedPlan,
		onTogglePlan,
		onHighlight,
		projectId
	}: Props = $props();

	/** Track scroll state for edge fade indicators */
	let laneEl: HTMLDivElement | undefined = $state();
	let canScrollLeft = $state(false);
	let canScrollRight = $state(false);

	function updateScrollIndicators() {
		if (!laneEl) return;
		canScrollLeft = laneEl.scrollLeft > 0;
		canScrollRight = laneEl.scrollLeft + laneEl.clientWidth < laneEl.scrollWidth - 1;
	}

	$effect(() => {
		if (!laneEl) return;
		updateScrollIndicators();
		const observer = new ResizeObserver(() => updateScrollIndicators());
		observer.observe(laneEl);
		return () => observer.disconnect();
	});

	/** Find the agent session for a plan */
	function getAgentSession(plan: PlanState): AgentSession | null {
		return (
			agentSessions.find(
				(s) =>
					s.phase_number === plan.phase_number && s.plan_number === plan.plan_number
			) ?? null
		);
	}

	/** Get runs for a specific plan */
	function getPlanRuns(plan: PlanState): ExecutionRun[] {
		return runs.filter((r) => r.plan_number === plan.plan_number);
	}

	/** Determine if a card should be highlighted or dimmed */
	function isHighlighted(planNumber: string): boolean {
		if (!highlightedChain) return false;
		return highlightedChain.has(planNumber);
	}

	function isDimmed(planNumber: string): boolean {
		if (!highlightedChain) return false;
		return !highlightedChain.has(planNumber);
	}
</script>

<div class="wave-lane">
	<div class="wave-label">Wave {wave}</div>
	<div class="lane-scroll-container">
		{#if canScrollLeft}
			<div class="scroll-fade scroll-fade-left"></div>
		{/if}
		<div
			class="lane-cards"
			bind:this={laneEl}
			onscroll={updateScrollIndicators}
		>
			{#each plans as plan (plan.plan_number)}
				<div class="card-wrapper">
					<PlanCard
						{plan}
						runs={getPlanRuns(plan)}
						agentSession={getAgentSession(plan)}
						expanded={expandedPlan === plan.plan_number}
						highlighted={isHighlighted(plan.plan_number)}
						dimmed={isDimmed(plan.plan_number)}
						onToggle={() => onTogglePlan(plan.plan_number)}
						{onHighlight}
					/>
					{#if expandedPlan === plan.plan_number}
						<PlanCardExpanded
							{plan}
							runs={getPlanRuns(plan)}
							{projectId}
						/>
					{/if}
				</div>
			{/each}
		</div>
		{#if canScrollRight}
			<div class="scroll-fade scroll-fade-right"></div>
		{/if}
	</div>
</div>

<style>
	.wave-lane {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
	}

	.wave-label {
		flex-shrink: 0;
		width: 60px;
		padding-top: var(--space-2);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--fg-muted);
		text-align: right;
	}

	.lane-scroll-container {
		position: relative;
		flex: 1;
		min-width: 0;
	}

	.lane-cards {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		overflow-x: auto;
		padding: var(--space-1) 0;
		scrollbar-width: thin;
	}

	.card-wrapper {
		flex-shrink: 0;
	}

	.scroll-fade {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 20px;
		z-index: 2;
		pointer-events: none;
	}

	.scroll-fade-left {
		left: 0;
		background: linear-gradient(to right, var(--bg-surface), transparent);
	}

	.scroll-fade-right {
		right: 0;
		background: linear-gradient(to left, var(--bg-surface), transparent);
	}
</style>
