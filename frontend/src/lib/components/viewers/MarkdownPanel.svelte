<script lang="ts">
	/**
	 * Read-only markdown rendered panel.
	 *
	 * Displays markdown content rendered to HTML with syntax highlighting
	 * using marked + highlight.js. Opens as a modal overlay.
	 */

	import { Marked } from 'marked';
	import { markedHighlight } from 'marked-highlight';
	import hljs from 'highlight.js';

	interface Props {
		content: string;
		title: string;
		onClose: () => void;
	}

	let { content, title, onClose }: Props = $props();

	const marked = new Marked(
		markedHighlight({
			emptyLangClass: 'hljs',
			langPrefix: 'hljs language-',
			highlight(code: string, lang: string) {
				if (lang && hljs.getLanguage(lang)) {
					try {
						return hljs.highlight(code, { language: lang }).value;
					} catch {
						// Fall through
					}
				}
				return hljs.highlightAuto(code).value;
			}
		})
	);

	const renderedHtml = $derived(marked.parse(content) as string);

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
			{@html renderedHtml}
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
		overflow-y: auto;
		padding: var(--space-4);
		color: var(--fg-secondary);
		font-size: var(--text-sm);
		line-height: 1.7;
	}

	/* Markdown content styling */
	.panel-content :global(h1),
	.panel-content :global(h2),
	.panel-content :global(h3) {
		color: var(--fg-primary);
		margin-top: var(--space-4);
		margin-bottom: var(--space-2);
	}

	.panel-content :global(h1) {
		font-size: var(--text-lg);
	}

	.panel-content :global(h2) {
		font-size: var(--text-base);
	}

	.panel-content :global(h3) {
		font-size: var(--text-sm);
	}

	.panel-content :global(p) {
		margin-bottom: var(--space-2);
	}

	.panel-content :global(code) {
		font-family: var(--font-mono);
		font-size: 0.85em;
		background: var(--bg-elevated);
		padding: 1px 4px;
		border-radius: 3px;
	}

	.panel-content :global(pre) {
		background: var(--bg-base);
		border: 1px solid var(--border-subtle);
		border-radius: 6px;
		padding: var(--space-3);
		overflow-x: auto;
		margin-bottom: var(--space-3);
	}

	.panel-content :global(pre code) {
		background: none;
		padding: 0;
		font-size: var(--text-xs);
	}

	.panel-content :global(ul),
	.panel-content :global(ol) {
		margin-bottom: var(--space-2);
		padding-left: var(--space-4);
	}

	.panel-content :global(li) {
		margin-bottom: var(--space-1);
	}

	.panel-content :global(table) {
		border-collapse: collapse;
		width: 100%;
		margin-bottom: var(--space-3);
		font-size: var(--text-xs);
	}

	.panel-content :global(th),
	.panel-content :global(td) {
		border: 1px solid var(--border-subtle);
		padding: var(--space-1) var(--space-2);
		text-align: left;
	}

	.panel-content :global(th) {
		background: var(--bg-elevated);
		color: var(--fg-primary);
		font-weight: 600;
	}

	.panel-content :global(blockquote) {
		border-left: 3px solid var(--fg-accent);
		padding-left: var(--space-3);
		color: var(--fg-muted);
		margin-bottom: var(--space-2);
	}

	.panel-content :global(hr) {
		border: none;
		border-top: 1px solid var(--border-subtle);
		margin: var(--space-4) 0;
	}

	.panel-content :global(a) {
		color: var(--fg-accent);
	}
</style>
