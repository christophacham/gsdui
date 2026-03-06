<script lang="ts">
	interface Props {
		/** Progress value from 0 to 100 */
		value: number;
		/** CSS custom property name for the fill color (e.g., '--status-done') */
		color?: string;
		/** Height of the bar */
		height?: string;
	}

	let { value = 0, color = '--status-done', height = '4px' }: Props = $props();

	const clampedValue = $derived(Math.min(100, Math.max(0, value)));
</script>

<div class="progress-track" style:height role="progressbar" aria-valuenow={clampedValue} aria-valuemin={0} aria-valuemax={100}>
	<div
		class="progress-fill"
		style:width="{clampedValue}%"
		style:background="var({color})"
	></div>
</div>

<style>
	.progress-track {
		width: 100%;
		background: var(--bg-elevated);
		border-radius: 2px;
		overflow: hidden;
	}

	.progress-fill {
		height: 100%;
		border-radius: 2px;
		transition: width var(--transition-normal);
	}
</style>
