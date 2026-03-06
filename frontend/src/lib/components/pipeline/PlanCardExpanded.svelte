<script lang="ts">
	/**
	 * Expanded plan card content - shows output area, file links, and jump-to-console.
	 *
	 * Renders below PlanCard when expanded. Fetches markdown content via REST API
	 * and opens MarkdownPanel overlay for viewing PLAN.md / SUMMARY.md.
	 */

	import type { PlanState, ExecutionRun } from '$lib/types/api.js';
	import { settingsStore } from '$lib/stores/settings.svelte.js';
	import MarkdownPanel from '$lib/components/viewers/MarkdownPanel.svelte';
	import DiffPanel from '$lib/components/viewers/DiffPanel.svelte';

	interface Props {
		plan: PlanState;
		runs: ExecutionRun[];
		projectId: string;
	}

	let { plan, runs, projectId }: Props = $props();

	/** Which panel is currently open */
	let openPanel = $state<'plan' | 'summary' | 'diff' | null>(null);
	let panelContent = $state('');
	let panelTitle = $state('');
	let panelLoading = $state(false);

	/** Output area scroll state */
	let outputEl: HTMLDivElement | undefined = $state();
	let userScrolled = $state(false);

	/** Latest non-superseded run */
	const latestRun = $derived(
		runs
			.filter((r) => r.plan_number === plan.plan_number && !r.superseded)
			.sort((a, b) => b.run_number - a.run_number)[0] ?? null
	);

	/** Build a summary text from the latest run */
	const outputText = $derived(() => {
		if (!latestRun) return 'No execution data yet';

		const lines: string[] = [];
		lines.push(`Run #${latestRun.run_number} - Status: ${latestRun.status ?? 'unknown'}`);
		if (latestRun.started_at) lines.push(`Started: ${latestRun.started_at}`);
		if (latestRun.completed_at) lines.push(`Completed: ${latestRun.completed_at}`);
		if (latestRun.duration_minutes !== null)
			lines.push(`Duration: ${latestRun.duration_minutes} min`);
		if (latestRun.key_files_created) lines.push(`Files created: ${latestRun.key_files_created}`);
		if (latestRun.key_files_modified)
			lines.push(`Files modified: ${latestRun.key_files_modified}`);
		if (latestRun.requirements_completed)
			lines.push(`Requirements: ${latestRun.requirements_completed}`);
		return lines.join('\n');
	});

	/** Compute output area height based on settings */
	const outputHeight = $derived(`${settingsStore.outputLineCount * 20}px`);

	function handleScroll() {
		if (!outputEl) return;
		const atBottom = outputEl.scrollHeight - outputEl.scrollTop - outputEl.clientHeight < 20;
		userScrolled = !atBottom;
	}

	/** Build the file path for the phase directory */
	function buildPhasePath(): string {
		// Phase number like "01" maps to directories like "01-name"
		// We try the phase number directly; the backend resolves the path
		return plan.phase_number;
	}

	async function fetchFile(filename: string): Promise<string> {
		const phasePath = buildPhasePath();
		const url = `/api/v1/projects/${projectId}/files/.planning/phases/${phasePath}/${plan.phase_number}-${plan.plan_number}-${filename}`;
		try {
			const response = await fetch(url);
			if (!response.ok) {
				return `Failed to load ${filename}: ${response.status} ${response.statusText}`;
			}
			return await response.text();
		} catch (err) {
			return `Error loading ${filename}: ${err}`;
		}
	}

	async function openPlan() {
		panelLoading = true;
		openPanel = 'plan';
		panelTitle = `${plan.plan_number}-PLAN.md`;
		panelContent = await fetchFile('PLAN.md');
		panelLoading = false;
	}

	async function openSummary() {
		panelLoading = true;
		openPanel = 'summary';
		panelTitle = `${plan.plan_number}-SUMMARY.md`;
		panelContent = await fetchFile('SUMMARY.md');
		panelLoading = false;
	}

	function openDiff() {
		openPanel = 'diff';
		panelTitle = 'Diff';
		panelContent = 'Diff data not yet available. Commit diff will be available after execution.';
	}

	function closePanel() {
		openPanel = null;
		panelContent = '';
		panelTitle = '';
	}
</script>

<div class="expanded-content">
	<div class="output-area" style:height={outputHeight} bind:this={outputEl} onscroll={handleScroll}>
		<pre class="output-text">{outputText()}</pre>
	</div>

	<div class="file-links">
		<button class="file-link" onclick={openPlan}>PLAN.md</button>
		<button class="file-link" onclick={openDiff}>Diff</button>
		<button class="file-link" onclick={openSummary}>SUMMARY.md</button>

		<button class="console-link" disabled title="Available in Phase 3">
			Jump to Console
		</button>
	</div>
</div>

{#if openPanel === 'plan' || openPanel === 'summary'}
	<MarkdownPanel
		content={panelLoading ? 'Loading...' : panelContent}
		title={panelTitle}
		onClose={closePanel}
	/>
{:else if openPanel === 'diff'}
	<DiffPanel diffContent={panelContent} title={panelTitle} onClose={closePanel} />
{/if}

<style>
	.expanded-content {
		border-top: 1px solid var(--border-subtle);
		background: var(--bg-surface);
		padding: var(--space-3);
	}

	.output-area {
		overflow-y: auto;
		background: var(--bg-base);
		border: 1px solid var(--border-subtle);
		border-radius: 4px;
		padding: var(--space-2);
		margin-bottom: var(--space-2);
	}

	.output-text {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--fg-secondary);
		white-space: pre-wrap;
		word-break: break-word;
		margin: 0;
		line-height: 20px;
	}

	.file-links {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.file-link {
		background: none;
		border: none;
		color: var(--fg-accent);
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		cursor: pointer;
		padding: 2px 6px;
		border-radius: 4px;
		transition: background var(--transition-fast);
	}

	.file-link:hover {
		background: var(--bg-overlay);
	}

	.console-link {
		margin-left: auto;
		background: none;
		border: 1px solid var(--border-subtle);
		color: var(--fg-muted);
		font-size: var(--text-xs);
		padding: 2px 8px;
		border-radius: 4px;
		cursor: not-allowed;
		opacity: 0.5;
	}
</style>
