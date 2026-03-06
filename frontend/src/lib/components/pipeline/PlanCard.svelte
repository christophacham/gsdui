<script lang="ts">
	/**
	 * Collapsed plan card - single row showing plan status, name, agent, stats.
	 *
	 * Click toggles expansion. Hover/click highlights dependency chain.
	 * Stat visibility controlled by settingsStore.
	 */

	import type { PlanState, ExecutionRun, AgentSession } from '$lib/types/api.js';
	import { settingsStore } from '$lib/stores/settings.svelte.js';
	import StatusIcon from './StatusIcon.svelte';
	import AgentBadge from './AgentBadge.svelte';

	interface Props {
		plan: PlanState;
		runs: ExecutionRun[];
		agentSession: AgentSession | null;
		expanded: boolean;
		highlighted: boolean;
		dimmed: boolean;
		onToggle: () => void;
		onHighlight: (planNumber: string | null) => void;
	}

	let { plan, runs, agentSession, expanded, highlighted, dimmed, onToggle, onHighlight }: Props =
		$props();

	/** Latest non-superseded run */
	const latestRun = $derived(
		runs
			.filter((r) => r.plan_number === plan.plan_number && !r.superseded)
			.sort((a, b) => b.run_number - a.run_number)[0] ?? null
	);

	/** Count of files created/modified across non-superseded runs */
	const commitCount = $derived(() => {
		let count = 0;
		for (const r of runs) {
			if (r.plan_number !== plan.plan_number || r.superseded) continue;
			if (r.key_files_created) {
				count += r.key_files_created.split(',').filter(Boolean).length;
			}
			if (r.key_files_modified) {
				count += r.key_files_modified.split(',').filter(Boolean).length;
			}
		}
		return count;
	});

	/** Format duration in minutes to human-readable */
	function formatDuration(minutes: number | null): string {
		if (minutes === null || minutes === undefined) return '--';
		if (minutes < 1) return '<1m';
		if (minutes < 60) return `${Math.round(minutes)}m`;
		const h = Math.floor(minutes / 60);
		const m = Math.round(minutes % 60);
		return m > 0 ? `${h}h ${m}m` : `${h}h`;
	}

	const displayName = $derived(plan.plan_name || `Plan ${plan.plan_number}`);

	function handleClick() {
		onToggle();
	}

	function handleMouseEnter() {
		onHighlight(plan.plan_number);
	}

	function handleMouseLeave() {
		onHighlight(null);
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="plan-card"
	class:expanded
	class:highlighted
	class:dimmed
	data-plan-number={plan.plan_number}
	onclick={handleClick}
	onmouseenter={handleMouseEnter}
	onmouseleave={handleMouseLeave}
>
	<div class="card-row">
		<StatusIcon status={plan.status} size={16} />

		<span class="plan-number">{plan.plan_number}</span>

		<span class="plan-name" title={displayName}>{displayName}</span>

		<div class="card-meta">
			<AgentBadge agentType={agentSession?.agent_type ?? null} />

			{#if settingsStore.visibleStats.steps}
				<span class="stat" title="Runs">
					{runs.filter((r) => r.plan_number === plan.plan_number && !r.superseded).length} runs
				</span>
			{/if}

			{#if settingsStore.visibleStats.commits}
				<span class="stat" title="Files touched">
					{commitCount()} files
				</span>
			{/if}

			{#if settingsStore.visibleStats.duration}
				<span class="stat" title="Duration">
					{formatDuration(latestRun?.duration_minutes ?? null)}
				</span>
			{/if}

			{#if settingsStore.visibleStats.wave && plan.wave !== null}
				<span class="wave-badge">W{plan.wave}</span>
			{/if}
		</div>
	</div>
</div>

<style>
	.plan-card {
		background: var(--bg-elevated);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
		cursor: pointer;
		transition:
			border-color var(--transition-fast),
			opacity var(--transition-fast);
		min-width: 240px;
		max-width: 400px;
	}

	.plan-card:hover {
		border-color: var(--fg-muted);
	}

	.plan-card.expanded {
		border-color: var(--fg-accent);
	}

	.plan-card.dimmed {
		opacity: 0.3;
	}

	.plan-card.highlighted {
		border-color: var(--fg-accent);
	}

	.card-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
	}

	.plan-number {
		color: var(--fg-muted);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		flex-shrink: 0;
	}

	.plan-name {
		color: var(--fg-primary);
		font-size: var(--text-sm);
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		flex: 1;
		min-width: 0;
	}

	.card-meta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	.stat {
		color: var(--fg-muted);
		font-size: var(--text-xs);
		white-space: nowrap;
	}

	.wave-badge {
		display: inline-flex;
		align-items: center;
		padding: 0 4px;
		border-radius: 4px;
		font-size: 10px;
		font-weight: 600;
		color: var(--fg-muted);
		background: var(--bg-overlay);
	}
</style>
