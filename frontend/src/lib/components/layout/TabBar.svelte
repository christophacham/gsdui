<script lang="ts">
	import { page } from '$app/stores';

	interface Tab {
		label: string;
		href: string;
	}

	const tabs: Tab[] = [
		{ label: 'Pipeline', href: '/' },
		{ label: 'Console', href: '/console' },
		{ label: 'System', href: '/system' }
	];

	function isActive(tabHref: string, currentPath: string): boolean {
		if (tabHref === '/') {
			return currentPath === '/';
		}
		return currentPath.startsWith(tabHref);
	}
</script>

<div class="tab-bar" role="tablist" aria-label="Main navigation">
	{#each tabs as tab (tab.href)}
		<a
			class="tab"
			class:active={isActive(tab.href, $page.url.pathname)}
			href={tab.href}
			role="tab"
			aria-selected={isActive(tab.href, $page.url.pathname)}
		>
			{tab.label}
		</a>
	{/each}
</div>

<style>
	.tab-bar {
		display: flex;
		flex: 1;
		gap: 0;
		background: var(--bg-surface);
		padding: 0 var(--space-4);
	}

	.tab {
		padding: var(--space-3) var(--space-4);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--fg-secondary);
		text-decoration: none;
		border-bottom: 2px solid transparent;
		transition: color var(--transition-fast), border-color var(--transition-fast);
	}

	.tab:hover {
		color: var(--fg-primary);
		text-decoration: none;
	}

	.tab.active {
		color: var(--fg-accent);
		border-bottom-color: var(--fg-accent);
	}
</style>
