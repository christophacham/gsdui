<script lang="ts">
	/**
	 * Settings panel - slide-out from right edge.
	 *
	 * Provides comprehensive display configuration controls binding
	 * directly to settingsStore $state fields. All changes apply
	 * immediately and persist to localStorage via $effect.
	 */

	import { settingsStore } from '$lib/stores/settings.svelte.js';

	interface Props {
		/** Whether the panel is open */
		open: boolean;
		/** Callback to close the panel */
		onClose: () => void;
	}

	let { open, onClose }: Props = $props();

	let panelRef = $state<HTMLDivElement | null>(null);

	function handleClickOutside(event: MouseEvent) {
		if (panelRef && !panelRef.contains(event.target as Node)) {
			onClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}

	$effect(() => {
		if (open) {
			document.addEventListener('click', handleClickOutside, true);
			document.addEventListener('keydown', handleKeydown);
			return () => {
				document.removeEventListener('click', handleClickOutside, true);
				document.removeEventListener('keydown', handleKeydown);
			};
		}
	});
</script>

{#if open}
	<div class="settings-backdrop">
		<div class="settings-panel" bind:this={panelRef}>
			<div class="panel-header">
				<h3 class="panel-title">Settings</h3>
				<button class="close-btn" onclick={onClose} type="button" aria-label="Close settings">
					<svg width="16" height="16" viewBox="0 0 16 16">
						<path d="M4 4L12 12M12 4L4 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
					</svg>
				</button>
			</div>

			<div class="panel-body">
				<!-- Output Display -->
				<section class="settings-section">
					<h4 class="section-title">Output Display</h4>

					<label class="setting-row">
						<span class="setting-label">Output lines</span>
						<input
							type="number"
							class="number-input"
							min="5"
							max="50"
							bind:value={settingsStore.outputLineCount}
						/>
					</label>

					<fieldset class="setting-row radio-group">
						<legend class="setting-label">Default card state</legend>
						<label class="radio-option">
							<input
								type="radio"
								name="defaultCardState"
								value="collapsed"
								bind:group={settingsStore.defaultCardState}
							/>
							Collapsed
						</label>
						<label class="radio-option">
							<input
								type="radio"
								name="defaultCardState"
								value="expanded"
								bind:group={settingsStore.defaultCardState}
							/>
							Expanded
						</label>
					</fieldset>

					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.autoExpandActive} />
						<span>Auto-expand active card</span>
					</label>
				</section>

				<!-- Visible Stats -->
				<section class="settings-section">
					<h4 class="section-title">Visible Stats</h4>

					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.visibleStats.steps} />
						<span>Steps</span>
					</label>
					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.visibleStats.commits} />
						<span>Commits</span>
					</label>
					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.visibleStats.duration} />
						<span>Duration</span>
					</label>
					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.visibleStats.wave} />
						<span>Wave</span>
					</label>
				</section>

				<!-- Auto-scroll -->
				<section class="settings-section">
					<h4 class="section-title">Auto-scroll</h4>

					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.autoScroll} />
						<span>Auto-scroll output to bottom</span>
					</label>
				</section>

				<!-- Timeline -->
				<section class="settings-section">
					<h4 class="section-title">Timeline</h4>

					<fieldset class="setting-row radio-group">
						<legend class="setting-label">Chip density</legend>
						<label class="radio-option">
							<input
								type="radio"
								name="timelineDensity"
								value="rich"
								bind:group={settingsStore.timelineDensity}
							/>
							Rich
						</label>
						<label class="radio-option">
							<input
								type="radio"
								name="timelineDensity"
								value="medium"
								bind:group={settingsStore.timelineDensity}
							/>
							Medium
						</label>
						<label class="radio-option">
							<input
								type="radio"
								name="timelineDensity"
								value="minimal"
								bind:group={settingsStore.timelineDensity}
							/>
							Minimal
						</label>
					</fieldset>
				</section>

				<!-- Theme -->
				<section class="settings-section">
					<h4 class="section-title">Theme</h4>

					<label class="setting-row slider-row">
						<span class="setting-label">Font size: {settingsStore.fontSize}px</span>
						<input
							type="range"
							class="slider"
							min="12"
							max="20"
							step="1"
							bind:value={settingsStore.fontSize}
						/>
					</label>
				</section>

				<!-- Notifications -->
				<section class="settings-section">
					<h4 class="section-title">Notifications</h4>

					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.notifications.planCompletion} />
						<span>Plan completion</span>
					</label>
					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.notifications.errors} />
						<span>Errors</span>
					</label>
					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.notifications.phaseTransitions} />
						<span>Phase transitions</span>
					</label>
					<label class="setting-row checkbox">
						<input type="checkbox" bind:checked={settingsStore.notifications.agentSwitches} />
						<span>Agent switches</span>
					</label>
				</section>
			</div>
		</div>
	</div>
{/if}

<style>
	.settings-backdrop {
		position: fixed;
		top: 0;
		right: 0;
		bottom: 0;
		left: 0;
		z-index: 100;
		pointer-events: none;
	}

	.settings-panel {
		position: absolute;
		top: 0;
		right: 0;
		bottom: 0;
		width: 320px;
		max-width: 100vw;
		background: var(--bg-surface);
		border-left: 1px solid var(--border-subtle);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		pointer-events: all;
		animation: slideIn 200ms ease;
		box-shadow: -4px 0 24px rgba(0, 0, 0, 0.3);
	}

	@keyframes slideIn {
		from {
			transform: translateX(100%);
		}
		to {
			transform: translateX(0);
		}
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
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--fg-primary);
		margin: 0;
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		background: none;
		border: none;
		border-radius: 4px;
		color: var(--fg-muted);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.close-btn:hover {
		background: var(--bg-elevated);
		color: var(--fg-primary);
	}

	.panel-body {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-3) var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}

	.settings-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.section-title {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--fg-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0;
		padding-bottom: var(--space-1);
		border-bottom: 1px solid var(--border-subtle);
	}

	.setting-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
		color: var(--fg-secondary);
	}

	.setting-label {
		color: var(--fg-secondary);
		font-size: var(--text-sm);
	}

	.number-input {
		width: 60px;
		padding: 2px 6px;
		background: var(--bg-elevated);
		border: 1px solid var(--border-subtle);
		border-radius: 4px;
		color: var(--fg-primary);
		font-size: var(--text-sm);
		font-family: var(--font-mono);
		text-align: center;
	}

	.number-input:focus {
		outline: none;
		border-color: var(--border-focus);
	}

	.radio-group {
		border: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.radio-group legend {
		margin-bottom: var(--space-1);
	}

	.radio-option {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding-left: var(--space-2);
		font-size: var(--text-sm);
		color: var(--fg-secondary);
		cursor: pointer;
	}

	.checkbox {
		cursor: pointer;
	}

	.checkbox input[type='checkbox'] {
		accent-color: var(--fg-accent);
	}

	.radio-option input[type='radio'] {
		accent-color: var(--fg-accent);
	}

	.slider-row {
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-1);
	}

	.slider {
		width: 100%;
		accent-color: var(--fg-accent);
	}
</style>
