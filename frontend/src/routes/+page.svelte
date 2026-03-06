<script lang="ts">
	import { projectStore } from '$lib/stores/project.svelte.js';
	import PhaseTimeline from '$lib/components/pipeline/PhaseTimeline.svelte';
	import PhaseDetail from '$lib/components/pipeline/PhaseDetail.svelte';
	import Skeleton from '$lib/components/shared/Skeleton.svelte';

	/** Track which phase is selected in the timeline */
	let selectedPhaseNumber = $state<string | null>(null);

	function handlePhaseSelect(phaseNumber: string) {
		selectedPhaseNumber = phaseNumber;
	}
</script>

<div class="pipeline-page">
	{#if !projectStore.selectedProjectId}
		<div class="empty-state">
			<h2>Pipeline Dashboard</h2>
			<p>Select a project from the sidebar to view its pipeline status.</p>
		</div>
	{:else if projectStore.loading}
		<div class="loading-state">
			<div class="timeline-skeleton">
				<Skeleton width="120px" height="36px" />
				<Skeleton width="160px" height="36px" />
				<Skeleton width="140px" height="36px" />
				<Skeleton width="130px" height="36px" />
			</div>
			<div class="detail-skeleton">
				<Skeleton width="40%" height="1.5rem" />
				<Skeleton width="70%" height="1rem" />
				<Skeleton width="100%" height="60px" />
				<Skeleton width="100%" height="200px" />
			</div>
		</div>
	{:else if projectStore.state}
		<div class="pipeline-content">
			<PhaseTimeline
				onPhaseSelect={handlePhaseSelect}
				{selectedPhaseNumber}
			/>
			{#if selectedPhaseNumber && projectStore.selectedProjectId}
				<div class="detail-area">
					<PhaseDetail phaseNumber={selectedPhaseNumber} projectId={projectStore.selectedProjectId} />
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.pipeline-page {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.empty-state {
		padding: var(--space-4);
	}

	.empty-state h2 {
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--fg-primary);
		margin-bottom: var(--space-4);
	}

	.empty-state p {
		color: var(--fg-muted);
		font-size: var(--text-sm);
	}

	.loading-state {
		display: flex;
		flex-direction: column;
	}

	.timeline-skeleton {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--bg-surface);
		border-bottom: 1px solid var(--border-subtle);
	}

	.detail-skeleton {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-4);
	}

	.pipeline-content {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.detail-area {
		flex: 1;
		overflow-y: auto;
	}
</style>
