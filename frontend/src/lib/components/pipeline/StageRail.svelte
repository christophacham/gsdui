<script lang="ts">
	/**
	 * 4-stage Discuss/Plan/Execute/Verify rail.
	 *
	 * Shows the progression of a phase through GSD's 4 stages.
	 * Current stage is highlighted, completed stages are filled,
	 * future stages are hollow.
	 */

	interface Props {
		/** The phase's current stage value from the backend */
		currentStage: string;
	}

	let { currentStage }: Props = $props();

	/** The 4 GSD stages in order */
	const stages = ['Discuss', 'Plan', 'Execute', 'Verify'] as const;

	/**
	 * Map backend stage values to which GSD stage is active.
	 * Returns the index (0-3) of the current stage, or -1 for none.
	 *
	 * "planned" -> none highlighted (before Discuss)
	 * "discussed" -> Discuss highlighted
	 * "researched" -> Discuss highlighted (research is part of plan stage context)
	 * "planned_ready" -> Plan highlighted
	 * "executing" -> Execute highlighted (pulsing)
	 * "executed" -> Execute highlighted as complete
	 * "verified" -> Verify highlighted
	 */
	const stageIndex = $derived.by(() => {
		const map: Record<string, number> = {
			planned: -1,
			discussed: 0,
			researched: 0,
			planned_ready: 1,
			executing: 2,
			executed: 2,
			verified: 3
		};
		return map[currentStage] ?? -1;
	});

	/** Whether the current stage is actively executing (pulsing animation) */
	const isExecuting = $derived(currentStage === 'executing');

	/** Determine state of each stage: 'completed' | 'current' | 'future' */
	function getStageState(index: number): 'completed' | 'current' | 'future' {
		if (stageIndex < 0) return 'future';
		if (index < stageIndex) return 'completed';
		if (index === stageIndex) return 'current';
		return 'future';
	}
</script>

<div class="stage-rail" role="list" aria-label="Phase stage progression">
	{#each stages as stage, i}
		{@const state = getStageState(i)}
		{#if i > 0}
			<div
				class="connector"
				class:completed={state === 'completed' || state === 'current'}
			></div>
		{/if}
		<div class="stage" class:completed={state === 'completed'} class:current={state === 'current'} class:future={state === 'future'} role="listitem">
			<div
				class="stage-dot"
				class:completed={state === 'completed'}
				class:current={state === 'current'}
				class:pulsing={state === 'current' && isExecuting}
			></div>
			<span class="stage-label">{stage}</span>
		</div>
	{/each}
</div>

<style>
	.stage-rail {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0;
		padding: var(--space-3) var(--space-4);
	}

	.stage {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-1);
		flex-shrink: 0;
	}

	.stage-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		border: 2px solid var(--fg-muted);
		background: transparent;
		transition: all var(--transition-normal);
	}

	.stage-dot.completed {
		background: var(--status-done);
		border-color: var(--status-done);
	}

	.stage-dot.current {
		width: 16px;
		height: 16px;
		background: var(--status-working);
		border-color: var(--status-working);
	}

	.stage-dot.pulsing {
		animation: pulse-glow 2s ease-in-out infinite;
	}

	@keyframes pulse-glow {
		0%,
		100% {
			box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.4);
		}
		50% {
			box-shadow: 0 0 0 6px rgba(59, 130, 246, 0);
		}
	}

	.stage-label {
		font-size: var(--text-xs);
		color: var(--fg-muted);
		white-space: nowrap;
	}

	.stage.completed .stage-label {
		color: var(--status-done);
	}

	.stage.current .stage-label {
		color: var(--status-working);
		font-weight: 600;
	}

	.connector {
		width: 40px;
		height: 2px;
		border-top: 2px dashed var(--fg-muted);
		margin-bottom: calc(var(--text-xs) + var(--space-1));
		flex-shrink: 0;
	}

	.connector.completed {
		border-top-style: solid;
		border-color: var(--status-done);
	}
</style>
