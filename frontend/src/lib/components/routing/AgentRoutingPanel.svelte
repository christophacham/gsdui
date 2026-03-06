<script lang="ts">
	/**
	 * Agent routing configuration panel.
	 *
	 * Shows the agent routing cascade hierarchy:
	 * 1. Project default agent
	 * 2. Stage overrides (Discuss, Plan, Execute, Verify)
	 * 3. Per-plan overrides for the current phase
	 *
	 * Changes auto-save immediately via PUT /api/v1/projects/:id/config.
	 * The file watcher detects the change and broadcasts a ConfigUpdated delta.
	 */

	import type { PlanState } from '$lib/types/api.js';
	import { projectStore } from '$lib/stores/project.svelte.js';
	import AgentDropdown from './AgentDropdown.svelte';
	import AgentBadge from '$lib/components/pipeline/AgentBadge.svelte';

	interface Props {
		/** Phase number for displaying per-plan overrides */
		phaseNumber: string;
		/** Plans in this phase */
		plans: PlanState[];
		/** Project ID for API calls */
		projectId: string;
	}

	let { phaseNumber, plans, projectId }: Props = $props();

	let collapsed = $state(true);
	let saving = $state(false);
	let toastMessage = $state<string | null>(null);
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;

	/** The 4 GSD stages */
	const stages = ['discuss', 'plan', 'execute', 'verify'] as const;

	/** Parse the config from the project store */
	const configObj = $derived(() => {
		const raw = projectStore.state?.config?.config_json;
		if (!raw) return null;
		try {
			return JSON.parse(raw);
		} catch {
			return null;
		}
	});

	/** Get the agent_routing section from config */
	const routing = $derived(() => {
		const cfg = configObj();
		return cfg?.agent_routing ?? null;
	});

	/** Project default agent */
	const projectDefault = $derived(() => {
		return routing()?.default ?? 'claude';
	});

	/** Stage overrides map */
	const stageOverrides = $derived((): Record<string, string> => {
		return routing()?.stage_overrides ?? {};
	});

	/** Plan overrides map */
	const planOverrides = $derived((): Record<string, string> => {
		return routing()?.plan_overrides ?? {};
	});

	/** Resolve effective agent for a plan through the cascade */
	function resolveEffectiveAgent(planNumber: string): string {
		// Plan override takes highest priority
		const planOverride = planOverrides()[planNumber];
		if (planOverride) return planOverride;

		// Then check stage overrides -- use the plan's type as the stage
		// Plans in the execute stage typically: look at the plan_type field
		const plan = plans.find((p) => p.plan_number === planNumber);
		const planType = plan?.plan_type?.toLowerCase();
		if (planType && stageOverrides()[planType]) {
			return stageOverrides()[planType];
		}

		// Fall back to project default
		return projectDefault();
	}

	/** Resolve effective agent for a stage */
	function resolveStageEffectiveAgent(stage: string): string {
		return stageOverrides()[stage] ?? projectDefault();
	}

	/** Build the full config object and save it */
	async function saveConfig(updatedRouting: {
		default: string;
		stage_overrides: Record<string, string>;
		plan_overrides: Record<string, string>;
	}) {
		saving = true;

		// Merge with existing config to preserve non-routing fields
		let fullConfig: Record<string, unknown> = {};
		const cfg = configObj();
		if (cfg && typeof cfg === 'object') {
			fullConfig = { ...cfg };
		}
		fullConfig.agent_routing = updatedRouting;

		try {
			const response = await fetch(`/api/v1/projects/${projectId}/config`, {
				method: 'PUT',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ config_json: fullConfig })
			});

			if (response.ok) {
				showToast('Agent routing updated');
			} else {
				const err = await response.json().catch(() => ({ error: 'Unknown error' }));
				showToast(`Error: ${err.error ?? 'Failed to save'}`);
			}
		} catch (err) {
			showToast('Error: Network request failed');
		} finally {
			saving = false;
		}
	}

	function showToast(message: string) {
		toastMessage = message;
		if (toastTimeout) clearTimeout(toastTimeout);
		toastTimeout = setTimeout(() => {
			toastMessage = null;
		}, 2500);
	}

	/** Handle project default change */
	function onProjectDefaultChange(agent: string | null) {
		const newDefault = agent ?? 'claude';
		saveConfig({
			default: newDefault,
			stage_overrides: { ...stageOverrides() },
			plan_overrides: { ...planOverrides() }
		});
	}

	/** Handle stage override change */
	function onStageOverrideChange(stage: string, agent: string | null) {
		const newOverrides = { ...stageOverrides() };
		if (agent === null) {
			delete newOverrides[stage];
		} else {
			newOverrides[stage] = agent;
		}
		saveConfig({
			default: projectDefault(),
			stage_overrides: newOverrides,
			plan_overrides: { ...planOverrides() }
		});
	}

	/** Handle plan override change */
	function onPlanOverrideChange(planNumber: string, agent: string | null) {
		const newOverrides = { ...planOverrides() };
		if (agent === null) {
			delete newOverrides[planNumber];
		} else {
			newOverrides[planNumber] = agent;
		}
		saveConfig({
			default: projectDefault(),
			stage_overrides: { ...stageOverrides() },
			plan_overrides: newOverrides
		});
	}

	/** Sort plans by plan_number */
	const sortedPlans = $derived(
		[...plans].sort((a, b) => a.plan_number.localeCompare(b.plan_number, undefined, { numeric: true }))
	);
</script>

<div class="routing-panel">
	<button class="panel-header" onclick={() => (collapsed = !collapsed)} type="button">
		<svg class="chevron" class:open={!collapsed} width="14" height="14" viewBox="0 0 14 14">
			<path d="M4 5L7 8L10 5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
		<span class="panel-title">Agent Routing</span>
		{#if saving}
			<span class="saving-indicator">Saving...</span>
		{/if}
	</button>

	{#if !collapsed}
		<div class="panel-body">
			<!-- Project Default -->
			<div class="tier">
				<div class="tier-header">Project Default</div>
				<div class="tier-row">
					<span class="tier-label">Default agent</span>
					<div class="tier-controls">
						<AgentDropdown
							selected={projectDefault()}
							onSelect={onProjectDefaultChange}
							showInherit={false}
						/>
					</div>
				</div>
			</div>

			<!-- Stage Overrides -->
			<div class="tier">
				<div class="tier-header">Stage Overrides</div>
				{#each stages as stage}
					<div class="tier-row">
						<span class="tier-label stage-label">{stage.charAt(0).toUpperCase() + stage.slice(1)}</span>
						<div class="tier-controls">
							<AgentDropdown
								selected={stageOverrides()[stage] ?? null}
								onSelect={(agent) => onStageOverrideChange(stage, agent)}
								showInherit={true}
							/>
							<span class="effective-label">
								<AgentBadge agentType={resolveStageEffectiveAgent(stage)} />
							</span>
						</div>
					</div>
				{/each}
			</div>

			<!-- Per-Plan Overrides -->
			{#if sortedPlans.length > 0}
				<div class="tier">
					<div class="tier-header">Plan Overrides</div>
					{#each sortedPlans as plan}
						{@const hasOverride = planOverrides()[plan.plan_number] != null}
						{@const effectiveAgent = resolveEffectiveAgent(plan.plan_number)}
						<div class="tier-row">
							<span class="tier-label plan-label" class:overridden={hasOverride}>
								{plan.plan_number}
								{#if plan.plan_name}
									<span class="plan-name">{plan.plan_name}</span>
								{/if}
							</span>
							<div class="tier-controls">
								<AgentDropdown
									selected={planOverrides()[plan.plan_number] ?? null}
									onSelect={(agent) => onPlanOverrideChange(plan.plan_number, agent)}
									showInherit={true}
								/>
								<span class="effective-label">
									<AgentBadge agentType={effectiveAgent} />
								</span>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	{#if toastMessage}
		<div class="toast" class:error={toastMessage.startsWith('Error')}>
			{toastMessage}
		</div>
	{/if}
</div>

<style>
	.routing-panel {
		position: relative;
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		background: var(--bg-surface);
		overflow: hidden;
	}

	.panel-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: none;
		border: none;
		color: var(--fg-secondary);
		font-size: var(--text-sm);
		font-weight: 600;
		cursor: pointer;
		transition: color var(--transition-fast);
		text-align: left;
	}

	.panel-header:hover {
		color: var(--fg-primary);
	}

	.chevron {
		transition: transform var(--transition-fast);
		flex-shrink: 0;
	}

	.chevron.open {
		transform: rotate(180deg);
	}

	.panel-title {
		flex: 1;
	}

	.saving-indicator {
		font-size: var(--text-xs);
		color: var(--fg-accent);
		font-weight: 400;
	}

	.panel-body {
		padding: 0 var(--space-3) var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.tier {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.tier-header {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--fg-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding-bottom: var(--space-1);
		border-bottom: 1px solid var(--border-subtle);
	}

	.tier-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		padding: var(--space-1) 0;
	}

	.tier-label {
		font-size: var(--text-sm);
		color: var(--fg-secondary);
		min-width: 80px;
	}

	.stage-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	.plan-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.plan-label.overridden {
		font-weight: 700;
		color: var(--fg-primary);
	}

	.plan-name {
		font-family: var(--font-sans);
		color: var(--fg-muted);
		font-weight: 400;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 180px;
	}

	.tier-controls {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.effective-label {
		font-size: var(--text-xs);
	}

	.toast {
		position: absolute;
		bottom: var(--space-2);
		right: var(--space-2);
		padding: var(--space-1) var(--space-3);
		background: var(--status-done);
		color: var(--bg-base);
		font-size: var(--text-xs);
		font-weight: 600;
		border-radius: 6px;
		animation: fadeInOut 2.5s ease forwards;
		pointer-events: none;
	}

	.toast.error {
		background: var(--status-failed);
	}

	@keyframes fadeInOut {
		0% {
			opacity: 0;
			transform: translateY(4px);
		}
		10% {
			opacity: 1;
			transform: translateY(0);
		}
		80% {
			opacity: 1;
		}
		100% {
			opacity: 0;
		}
	}
</style>
