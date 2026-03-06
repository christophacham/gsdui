<script lang="ts">
	import { wsManager } from '$lib/stores/websocket.svelte.js';

	const isReconnecting = $derived(
		wsManager.status === 'reconnecting' || wsManager.status === 'connecting'
	);
</script>

{#if isReconnecting}
	<div class="reconnect-banner" role="alert" aria-live="assertive">
		<span class="pulse-dot"></span>
		<span>Reconnecting to server...</span>
	</div>
{/if}

<style>
	.reconnect-banner {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 1000;
		padding: var(--space-2) var(--space-4);
		background: var(--status-working);
		color: white;
		text-align: center;
		font-size: var(--text-sm);
		font-weight: 500;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		animation: slideDown 200ms ease-out;
	}

	.pulse-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: white;
		flex-shrink: 0;
		animation: pulse 1s infinite;
	}

	@keyframes slideDown {
		from {
			transform: translateY(-100%);
		}
		to {
			transform: translateY(0);
		}
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
		}
	}
</style>
