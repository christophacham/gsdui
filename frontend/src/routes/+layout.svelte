<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/layout/Sidebar.svelte';
	import TabBar from '$lib/components/layout/TabBar.svelte';
	import ReconnectBanner from '$lib/components/layout/ReconnectBanner.svelte';
	import { projectStore } from '$lib/stores/project.svelte.js';
	import { onMount } from 'svelte';

	let { children } = $props();

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
		<TabBar />
		<main class="content">
			{@render children()}
		</main>
	</div>
</div>

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

	.content {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-4);
	}
</style>
