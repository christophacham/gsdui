<script lang="ts">
	/**
	 * Agent selection dropdown with colored badges.
	 *
	 * Displays the currently selected agent with a colored dot and label.
	 * Opens a dropdown menu with agent options on click.
	 * Closes on outside click or selection.
	 */

	interface Props {
		/** Currently selected agent (null = inherit/no override) */
		selected: string | null;
		/** Callback when an agent is selected */
		onSelect: (agent: string | null) => void;
		/** Whether to show the "Inherit default" option */
		showInherit: boolean;
	}

	let { selected, onSelect, showInherit }: Props = $props();

	let open = $state(false);
	let dropdownRef = $state<HTMLDivElement | null>(null);

	const agents = [
		{ id: 'claude', label: 'Claude', color: 'var(--agent-claude)' },
		{ id: 'codex', label: 'Codex', color: 'var(--agent-codex)' },
		{ id: 'gemini', label: 'Gemini', color: 'var(--agent-gemini)' }
	];

	const selectedAgent = $derived(agents.find((a) => a.id === selected) ?? null);
	const displayLabel = $derived(selectedAgent ? selectedAgent.label : 'Inherit default');
	const displayColor = $derived(selectedAgent ? selectedAgent.color : 'var(--fg-muted)');

	function toggle() {
		open = !open;
	}

	function select(agentId: string | null) {
		onSelect(agentId);
		open = false;
	}

	function handleClickOutside(event: MouseEvent) {
		if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
			open = false;
		}
	}

	$effect(() => {
		if (open) {
			document.addEventListener('click', handleClickOutside, true);
			return () => document.removeEventListener('click', handleClickOutside, true);
		}
	});
</script>

<div class="agent-dropdown" bind:this={dropdownRef}>
	<button class="dropdown-trigger" onclick={toggle} type="button">
		<span class="dot" style:background={displayColor}></span>
		<span class="label">{displayLabel}</span>
		<svg class="chevron" class:open width="12" height="12" viewBox="0 0 12 12">
			<path d="M3 4.5L6 7.5L9 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
		</svg>
	</button>

	{#if open}
		<div class="dropdown-menu">
			{#if showInherit}
				<button
					class="dropdown-item"
					class:active={selected === null}
					onclick={() => select(null)}
					type="button"
				>
					<span class="dot" style:background="var(--fg-muted)"></span>
					<span>Inherit default</span>
				</button>
			{/if}
			{#each agents as agent}
				<button
					class="dropdown-item"
					class:active={selected === agent.id}
					onclick={() => select(agent.id)}
					type="button"
				>
					<span class="dot" style:background={agent.color}></span>
					<span>{agent.label}</span>
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.agent-dropdown {
		position: relative;
		display: inline-block;
	}

	.dropdown-trigger {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px;
		background: var(--bg-elevated);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
		color: var(--fg-primary);
		font-size: var(--text-xs);
		cursor: pointer;
		transition: border-color var(--transition-fast);
		white-space: nowrap;
	}

	.dropdown-trigger:hover {
		border-color: var(--fg-muted);
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.label {
		font-weight: 500;
	}

	.chevron {
		color: var(--fg-muted);
		transition: transform var(--transition-fast);
	}

	.chevron.open {
		transform: rotate(180deg);
	}

	.dropdown-menu {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		min-width: 160px;
		background: var(--bg-elevated);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		padding: 4px;
		z-index: 50;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
	}

	.dropdown-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 6px 10px;
		background: none;
		border: none;
		border-radius: 4px;
		color: var(--fg-primary);
		font-size: var(--text-xs);
		cursor: pointer;
		transition: background var(--transition-fast);
		text-align: left;
	}

	.dropdown-item:hover {
		background: var(--bg-overlay);
	}

	.dropdown-item.active {
		background: var(--bg-overlay);
		font-weight: 600;
	}
</style>
