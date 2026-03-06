<script lang="ts">
	/**
	 * Diff viewer panel - renders unified diff with line coloring.
	 *
	 * Uses manual +/- line coloring for Phase 2 (green for additions,
	 * red for deletions). Full git-diff-view integration deferred.
	 */

	interface Props {
		diffContent: string;
		title: string;
		onClose: () => void;
	}

	let { diffContent, title, onClose }: Props = $props();

	interface DiffLine {
		text: string;
		type: 'addition' | 'deletion' | 'context' | 'header';
	}

	const lines = $derived<DiffLine[]>(
		diffContent.split('\n').map((line) => {
			if (line.startsWith('+++') || line.startsWith('---') || line.startsWith('diff ') || line.startsWith('index ') || line.startsWith('@@')) {
				return { text: line, type: 'header' };
			}
			if (line.startsWith('+')) {
				return { text: line, type: 'addition' };
			}
			if (line.startsWith('-')) {
				return { text: line, type: 'deletion' };
			}
			return { text: line, type: 'context' };
		})
	);

	function handleOverlayClick(e: MouseEvent) {
		if ((e.target as HTMLElement).classList.contains('panel-overlay')) {
			onClose();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClose();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel-overlay" onclick={handleOverlayClick}>
	<div class="panel" role="dialog" aria-label={title}>
		<div class="panel-header">
			<h3 class="panel-title">{title}</h3>
			<button class="close-btn" onclick={onClose} aria-label="Close panel">
				<svg width="16" height="16" viewBox="0 0 16 16" fill="none">
					<path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
				</svg>
			</button>
		</div>
		<div class="panel-content">
			<pre class="diff-view">{#each lines as line}<span class="diff-line {line.type}">{line.text}
</span>{/each}</pre>
		</div>
	</div>
</div>

<style>
	.panel-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 100;
		display: flex;
		justify-content: flex-end;
	}

	.panel {
		width: 100%;
		max-width: 600px;
		height: 100%;
		background: var(--bg-surface);
		display: flex;
		flex-direction: column;
		box-shadow: -4px 0 20px rgba(0, 0, 0, 0.3);
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border-subtle);
		flex-shrink: 0;
	}

	.panel-title {
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--fg-primary);
		font-family: var(--font-mono);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--fg-muted);
		cursor: pointer;
		padding: var(--space-1);
		border-radius: 4px;
		display: flex;
		align-items: center;
		transition: color var(--transition-fast);
	}

	.close-btn:hover {
		color: var(--fg-primary);
	}

	.panel-content {
		flex: 1;
		overflow: auto;
		padding: var(--space-3);
	}

	.diff-view {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		line-height: 1.6;
		margin: 0;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.diff-line {
		display: block;
	}

	.diff-line.addition {
		color: var(--status-done);
		background: rgba(34, 197, 94, 0.08);
	}

	.diff-line.deletion {
		color: var(--status-failed);
		background: rgba(239, 68, 68, 0.08);
	}

	.diff-line.header {
		color: var(--fg-accent);
		font-weight: 600;
	}

	.diff-line.context {
		color: var(--fg-secondary);
	}
</style>
