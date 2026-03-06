<script lang="ts">
	/**
	 * SVG overlay for dependency arrows between plan cards.
	 *
	 * Draws cubic Bezier paths from source card bottom-center to
	 * target card top-center. Colors by dependency status.
	 * Recalculates on resize/expand with debounce.
	 */

	import type { PlanState } from '$lib/types/api.js';
	import { parseDependsOn } from '$lib/utils/dependency-graph.js';
	import { cubicConnectorPath, getBottomCenter, getTopCenter } from '$lib/utils/svg-paths.js';

	interface Props {
		plans: PlanState[];
		containerEl: HTMLElement | null;
		highlightedChain: Set<string> | null;
	}

	let { plans, containerEl, highlightedChain }: Props = $props();

	interface Arrow {
		id: string;
		path: string;
		color: string;
		animated: boolean;
		inChain: boolean;
	}

	let arrows = $state<Arrow[]>([]);
	let svgWidth = $state(0);
	let svgHeight = $state(0);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	function getStatusColor(sourcePlan: PlanState): { color: string; animated: boolean } {
		switch (sourcePlan.status) {
			case 'done':
				return { color: 'var(--dep-satisfied)', animated: false };
			case 'working':
				return { color: 'var(--dep-in-progress)', animated: true };
			case 'failed':
				return { color: 'var(--status-failed)', animated: false };
			default:
				return { color: 'var(--dep-pending)', animated: false };
		}
	}

	function recalculate() {
		if (!containerEl) {
			arrows = [];
			return;
		}

		const containerRect = containerEl.getBoundingClientRect();
		svgWidth = containerRect.width;
		svgHeight = containerRect.height;

		const newArrows: Arrow[] = [];

		for (const plan of plans) {
			const deps = parseDependsOn(plan.depends_on);
			if (deps.length === 0) continue;

			const targetEl = containerEl.querySelector(
				`[data-plan-number="${plan.plan_number}"]`
			) as HTMLElement | null;
			if (!targetEl) continue;

			for (const dep of deps) {
				const sourceEl = containerEl.querySelector(
					`[data-plan-number="${dep}"]`
				) as HTMLElement | null;
				if (!sourceEl) continue;

				const sourcePlan = plans.find((p) => p.plan_number === dep);
				if (!sourcePlan) continue;

				const from = getBottomCenter(sourceEl, containerEl);
				const to = getTopCenter(targetEl, containerEl);
				const pathData = cubicConnectorPath(from, to);
				const { color, animated } = getStatusColor(sourcePlan);

				const inChain = highlightedChain
					? highlightedChain.has(dep) && highlightedChain.has(plan.plan_number)
					: true;

				newArrows.push({
					id: `${dep}->${plan.plan_number}`,
					path: pathData,
					color,
					animated,
					inChain
				});
			}
		}

		arrows = newArrows;
	}

	function debouncedRecalculate() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(recalculate, 50);
	}

	// Recalculate when plans or highlighted chain changes
	$effect(() => {
		// Read reactive dependencies
		void plans;
		void highlightedChain;
		void containerEl;

		// Defer to next frame so DOM has updated
		requestAnimationFrame(debouncedRecalculate);
	});

	// ResizeObserver on container
	$effect(() => {
		if (!containerEl) return;

		const observer = new ResizeObserver(() => debouncedRecalculate());
		observer.observe(containerEl);

		// Also observe mutations (card expand/collapse)
		const mutationObserver = new MutationObserver(() => debouncedRecalculate());
		mutationObserver.observe(containerEl, {
			childList: true,
			subtree: true,
			attributes: true,
			attributeFilter: ['class', 'style']
		});

		return () => {
			observer.disconnect();
			mutationObserver.disconnect();
			if (debounceTimer) clearTimeout(debounceTimer);
		};
	});
</script>

{#if arrows.length > 0}
	<svg
		class="dependency-arrows"
		width={svgWidth}
		height={svgHeight}
		xmlns="http://www.w3.org/2000/svg"
	>
		<defs>
			<!-- Arrow marker -->
			<marker
				id="arrowhead"
				markerWidth="8"
				markerHeight="6"
				refX="7"
				refY="3"
				orient="auto"
			>
				<polygon points="0 0, 8 3, 0 6" fill="currentColor" />
			</marker>
		</defs>

		{#each arrows as arrow (arrow.id)}
			<path
				d={arrow.path}
				fill="none"
				stroke={arrow.color}
				stroke-width="1.5"
				stroke-linecap="round"
				opacity={highlightedChain ? (arrow.inChain ? 1 : 0.15) : 0.7}
				marker-end="url(#arrowhead)"
				class:animated={arrow.animated}
				style:color={arrow.color}
			/>
		{/each}
	</svg>
{/if}

<style>
	.dependency-arrows {
		position: absolute;
		top: 0;
		left: 0;
		pointer-events: none;
		z-index: 1;
	}

	.dependency-arrows :global(path) {
		pointer-events: stroke;
		cursor: pointer;
	}

	.animated {
		stroke-dasharray: 8 4;
		animation: dash-flow 1s linear infinite;
	}

	@keyframes dash-flow {
		to {
			stroke-dashoffset: -12;
		}
	}
</style>
