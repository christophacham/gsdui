<script lang="ts">
	import { projectStore } from '$lib/stores/project.svelte.js';
	import Skeleton from '$lib/components/shared/Skeleton.svelte';
</script>

<div class="pipeline-page">
	{#if !projectStore.selectedProjectId}
		<div class="empty-state">
			<h2>Pipeline Dashboard</h2>
			<p>Select a project from the sidebar to view its pipeline status.</p>
		</div>
	{:else if projectStore.loading}
		<div class="loading-state">
			<h2>Pipeline</h2>
			<div class="skeleton-group">
				<Skeleton width="60%" height="2rem" />
				<Skeleton width="100%" height="4rem" />
				<Skeleton width="100%" height="4rem" />
				<Skeleton width="80%" height="4rem" />
			</div>
		</div>
	{:else if projectStore.state}
		<div class="pipeline-content">
			<h2>{projectStore.state.project.name}</h2>
			<p class="placeholder">
				Phase timeline and plan cards will render here.
			</p>
			<div class="stats">
				<span class="stat">
					{projectStore.state.phases.length} phases
				</span>
				<span class="stat-sep">/</span>
				<span class="stat">
					{Object.values(projectStore.state.plans).flat().length} plans
				</span>
				{#if projectStore.state.parse_errors.length > 0}
					<span class="stat-sep">/</span>
					<span class="stat error">
						{projectStore.state.parse_errors.length} parse errors
					</span>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.pipeline-page {
		height: 100%;
	}

	.empty-state,
	.loading-state {
		padding: var(--space-4);
	}

	.empty-state h2,
	.loading-state h2,
	.pipeline-content h2 {
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--fg-primary);
		margin-bottom: var(--space-4);
	}

	.empty-state p {
		color: var(--fg-muted);
		font-size: var(--text-sm);
	}

	.skeleton-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.pipeline-content {
		padding: 0;
	}

	.placeholder {
		color: var(--fg-muted);
		font-size: var(--text-sm);
		padding: var(--space-6);
		background: var(--bg-surface);
		border: 1px solid var(--border-subtle);
		border-radius: 8px;
		text-align: center;
	}

	.stats {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--space-4);
		font-size: var(--text-sm);
		color: var(--fg-secondary);
	}

	.stat-sep {
		color: var(--fg-muted);
	}

	.stat.error {
		color: var(--status-failed);
	}
</style>
