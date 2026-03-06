<script lang="ts">
	interface Props {
		width?: string;
		height?: string;
		rounded?: boolean;
	}

	let { width = '100%', height = '1rem', rounded = false }: Props = $props();
</script>

<div
	class="skeleton"
	class:rounded
	style:width
	style:height
	role="presentation"
	aria-hidden="true"
></div>

<style>
	.skeleton {
		background: linear-gradient(
			90deg,
			var(--bg-elevated) 25%,
			var(--bg-overlay) 50%,
			var(--bg-elevated) 75%
		);
		background-size: 200% 100%;
		animation: shimmer 1.5s infinite;
		border-radius: 4px;
	}

	.skeleton.rounded {
		border-radius: 50%;
	}

	@keyframes shimmer {
		0% {
			background-position: 200% 0;
		}
		100% {
			background-position: -200% 0;
		}
	}
</style>
