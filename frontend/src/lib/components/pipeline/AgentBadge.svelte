<script lang="ts">
	/**
	 * Agent badge component - displays a colored pill showing agent type.
	 *
	 * Color mapping: claude=amber, codex=green, gemini=purple.
	 * Unknown types show in muted text. Null shows nothing.
	 */

	interface Props {
		agentType: string | null;
	}

	let { agentType }: Props = $props();

	const agentColors: Record<string, string> = {
		claude: 'var(--agent-claude)',
		codex: 'var(--agent-codex)',
		gemini: 'var(--agent-gemini)'
	};

	const color = $derived(
		agentType ? (agentColors[agentType.toLowerCase()] ?? 'var(--fg-muted)') : null
	);

	const label = $derived(agentType ?? null);
</script>

{#if label}
	<span class="agent-badge" style:--badge-color={color}>
		{label}
	</span>
{/if}

<style>
	.agent-badge {
		display: inline-flex;
		align-items: center;
		padding: 1px 6px;
		border-radius: 9999px;
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--badge-color, var(--fg-muted));
		background: color-mix(in srgb, var(--badge-color, var(--fg-muted)) 15%, transparent);
		white-space: nowrap;
		line-height: 1.4;
	}
</style>
