<script lang="ts">
	/**
	 * Status icon component - renders an SVG icon per plan status.
	 *
	 * pending: hollow circle, working: pulsing filled circle,
	 * done: checkmark circle, failed: X circle.
	 */

	interface Props {
		status: string;
		size?: number;
	}

	let { status, size = 18 }: Props = $props();

	const statusColors: Record<string, string> = {
		pending: 'var(--status-pending)',
		working: 'var(--status-working)',
		done: 'var(--status-done)',
		failed: 'var(--status-failed)'
	};

	const color = $derived(statusColors[status] ?? 'var(--fg-muted)');
</script>

<span
	class="status-icon"
	class:pulse={status === 'working'}
	style:--status-color={color}
	title={status}
>
	<svg
		width={size}
		height={size}
		viewBox="0 0 20 20"
		fill="none"
		xmlns="http://www.w3.org/2000/svg"
	>
		{#if status === 'pending'}
			<!-- Hollow circle -->
			<circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5" />
		{:else if status === 'working'}
			<!-- Filled circle (animated via CSS) -->
			<circle cx="10" cy="10" r="7" fill="currentColor" />
		{:else if status === 'done'}
			<!-- Checkmark circle -->
			<circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5" />
			<path d="M7 10l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
		{:else if status === 'failed'}
			<!-- X circle -->
			<circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5" />
			<path d="M7.5 7.5l5 5M12.5 7.5l-5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
		{:else}
			<!-- Unknown: dotted circle -->
			<circle cx="10" cy="10" r="7" stroke="currentColor" stroke-width="1.5" stroke-dasharray="3 3" />
		{/if}
	</svg>
</span>

<style>
	.status-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--status-color);
		flex-shrink: 0;
	}

	.pulse {
		animation: pulse-glow 2s ease-in-out infinite;
	}

	@keyframes pulse-glow {
		0%, 100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}
</style>
