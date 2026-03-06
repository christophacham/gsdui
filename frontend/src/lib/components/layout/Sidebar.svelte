<script lang="ts">
	import { projectStore } from '$lib/stores/project.svelte.js';
	import { onMount } from 'svelte';

	onMount(() => {
		if (projectStore.projects.length === 0) {
			projectStore.fetchProjects();
		}
	});

	function handleSelect(id: string) {
		projectStore.selectProject(id);
	}
</script>

<aside class="sidebar">
	<div class="sidebar-header">
		<h2>Projects</h2>
	</div>

	<nav class="project-list" aria-label="Project list">
		{#if projectStore.projects.length === 0}
			<div class="empty-state">
				<p>No projects found</p>
				<p class="hint">Start the daemon to see projects</p>
			</div>
		{:else}
			{#each projectStore.projects as project (project.id)}
				<button
					class="project-item"
					class:selected={projectStore.selectedProjectId === project.id}
					onclick={() => handleSelect(project.id)}
					aria-current={projectStore.selectedProjectId === project.id ? 'true' : undefined}
				>
					<span class="project-name">{project.name}</span>
					{#if project.status !== 'offline'}
						<span class="active-dot" title="Active execution"></span>
					{/if}
				</button>
			{/each}
		{/if}
	</nav>
</aside>

<style>
	.sidebar {
		width: 240px;
		min-width: 240px;
		height: 100%;
		background: var(--bg-surface);
		border-right: 1px solid var(--border-subtle);
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}

	.sidebar-header {
		padding: var(--space-4);
		border-bottom: 1px solid var(--border-subtle);
	}

	.sidebar-header h2 {
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--fg-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.project-list {
		flex: 1;
		padding: var(--space-2) 0;
	}

	.empty-state {
		padding: var(--space-4);
		text-align: center;
		color: var(--fg-muted);
	}

	.empty-state p {
		font-size: var(--text-sm);
	}

	.empty-state .hint {
		font-size: var(--text-xs);
		margin-top: var(--space-2);
	}

	.project-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2) var(--space-4);
		background: none;
		border: none;
		border-left: 3px solid transparent;
		color: var(--fg-secondary);
		font-size: var(--text-sm);
		font-family: inherit;
		cursor: pointer;
		text-align: left;
		transition: background var(--transition-fast), color var(--transition-fast),
			border-color var(--transition-fast);
	}

	.project-item:hover {
		background: var(--bg-elevated);
		color: var(--fg-primary);
	}

	.project-item.selected {
		background: var(--bg-elevated);
		border-left-color: var(--fg-accent);
		color: var(--fg-primary);
	}

	.project-name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.active-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--status-working);
		flex-shrink: 0;
		animation: pulse 2s infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.4;
		}
	}
</style>
