<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/layout/Sidebar.svelte';
	import TabBar from '$lib/components/layout/TabBar.svelte';
	import ReconnectBanner from '$lib/components/layout/ReconnectBanner.svelte';
	import SettingsPanel from '$lib/components/shared/SettingsPanel.svelte';
	import { projectStore } from '$lib/stores/project.svelte.js';
	import { onMount } from 'svelte';

	let { children } = $props();
	let settingsOpen = $state(false);

	onMount(async () => {
		// Fetch projects if not already loaded
		if (projectStore.projects.length === 0) {
			await projectStore.fetchProjects();
		}

		// Auto-select first project if none selected
		if (!projectStore.selectedProjectId && projectStore.projects.length > 0) {
			projectStore.selectProject(projectStore.projects[0].id);
		}
	});
</script>

<ReconnectBanner />

<div class="app-shell">
	<Sidebar />

	<div class="main-area">
		<div class="tab-row">
			<TabBar />
			<button
				class="gear-btn"
				type="button"
				aria-label="Open settings"
				onclick={() => (settingsOpen = !settingsOpen)}
			>
				<svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
					<circle cx="9" cy="9" r="2.5" />
					<path d="M14.7 11.1a1.2 1.2 0 0 0 .24 1.32l.04.04a1.45 1.45 0 1 1-2.06 2.06l-.04-.04a1.2 1.2 0 0 0-1.32-.24 1.2 1.2 0 0 0-.73 1.1v.12a1.45 1.45 0 0 1-2.9 0v-.06a1.2 1.2 0 0 0-.79-1.1 1.2 1.2 0 0 0-1.32.24l-.04.04a1.45 1.45 0 1 1-2.06-2.06l.04-.04a1.2 1.2 0 0 0 .24-1.32 1.2 1.2 0 0 0-1.1-.73h-.12a1.45 1.45 0 0 1 0-2.9h.06a1.2 1.2 0 0 0 1.1-.79 1.2 1.2 0 0 0-.24-1.32l-.04-.04a1.45 1.45 0 1 1 2.06-2.06l.04.04a1.2 1.2 0 0 0 1.32.24h.06a1.2 1.2 0 0 0 .73-1.1v-.12a1.45 1.45 0 0 1 2.9 0v.06a1.2 1.2 0 0 0 .73 1.1 1.2 1.2 0 0 0 1.32-.24l.04-.04a1.45 1.45 0 1 1 2.06 2.06l-.04.04a1.2 1.2 0 0 0-.24 1.32v.06a1.2 1.2 0 0 0 1.1.73h.12a1.45 1.45 0 0 1 0 2.9h-.06a1.2 1.2 0 0 0-1.1.73z" />
				</svg>
			</button>
		</div>
		<main class="content">
			{@render children()}
		</main>
	</div>
</div>

<SettingsPanel open={settingsOpen} onClose={() => (settingsOpen = false)} />

<style>
	.app-shell {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		background: var(--bg-base);
	}

	.main-area {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.tab-row {
		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--border-subtle);
		background: var(--bg-surface);
	}

	.gear-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		background: none;
		border: none;
		color: var(--fg-muted);
		cursor: pointer;
		transition: all var(--transition-fast);
		flex-shrink: 0;
		margin-right: var(--space-2);
		border-radius: 6px;
	}

	.gear-btn:hover {
		color: var(--fg-primary);
		background: var(--bg-elevated);
	}

	.content {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-4);
	}
</style>
