/**
 * Reactive project state store using Svelte 5 runes.
 *
 * Manages the full ProjectState lifecycle:
 * - Fetches project list from REST API
 * - Handles WebSocket snapshot/delta messages
 * - Applies deltas surgically for fine-grained reactivity
 *
 * The store is designed to be the single source of truth for all project data
 * consumed by pipeline components.
 */

import type { Project, ProjectState } from '$lib/types/api.js';
import type { StateChange, WsMessage } from '$lib/types/websocket.js';
import { wsManager } from './websocket.svelte.js';

class ProjectStore {
	/** Full reactive project state (null until first snapshot) */
	state = $state<ProjectState | null>(null);

	/** True until the first snapshot is received after selecting a project */
	loading = $state(true);

	/** List of all projects (for sidebar) */
	projects = $state<Project[]>([]);

	/** Currently selected project ID */
	selectedProjectId = $state<string | null>(null);

	constructor() {
		// Wire up WebSocket message routing
		wsManager.onMessage((msg) => this.handleMessage(msg));
	}

	/**
	 * Fetch the project list from the REST API.
	 */
	async fetchProjects(): Promise<void> {
		try {
			const response = await fetch('/api/v1/projects');
			if (!response.ok) {
				console.error('[ProjectStore] Failed to fetch projects:', response.status);
				return;
			}
			this.projects = await response.json();
		} catch (err) {
			console.error('[ProjectStore] Error fetching projects:', err);
		}
	}

	/**
	 * Select a project and subscribe to its WebSocket updates.
	 */
	selectProject(id: string): void {
		if (this.selectedProjectId === id) return;

		// Unsubscribe from previous project
		if (this.selectedProjectId) {
			wsManager.unsubscribe([this.selectedProjectId]);
		}

		this.selectedProjectId = id;
		this.state = null;
		this.loading = true;

		// Connect/subscribe to new project
		if (wsManager.status === 'disconnected') {
			wsManager.connect([id]);
		} else {
			wsManager.subscribe([id]);
		}
	}

	/**
	 * Handle incoming WebSocket messages.
	 * Routes snapshot/delta/health/error to appropriate handlers.
	 */
	handleMessage(msg: WsMessage): void {
		switch (msg.type) {
			case 'snapshot':
				this.applySnapshot(msg.data);
				break;
			case 'delta':
				this.applyDelta(msg.changes);
				break;
			case 'health':
				// Health messages are informational; could be used by System tab
				break;
			case 'error':
				console.error(`[WebSocket Error] ${msg.code}: ${msg.message}`);
				break;
		}
	}

	/**
	 * Replace full project state from a snapshot.
	 * Called on initial subscribe and on reconnection.
	 */
	applySnapshot(data: ProjectState): void {
		this.state = data;
		this.loading = false;
	}

	/**
	 * Apply incremental state changes from a delta message.
	 * Each change is applied surgically to maintain fine-grained reactivity.
	 */
	applyDelta(changes: StateChange[]): void {
		if (!this.state) return;

		for (const change of changes) {
			switch (change.type) {
				case 'PhaseUpdated': {
					const phase = this.state.phases.find(
						(p) => p.phase_number === change.data.phase_number
					);
					if (phase) {
						phase.stage = change.data.stage;
					}
					break;
				}

				case 'PlanUpdated': {
					const plans = this.state.plans[change.data.phase_number];
					if (plans) {
						const plan = plans.find((p) => p.plan_number === change.data.plan_number);
						if (plan) {
							plan.status = change.data.status;
						}
					}
					break;
				}

				case 'VerificationUpdated': {
					const existing = this.state.verifications[change.data.phase_number];
					if (existing) {
						existing.status = change.data.status;
					}
					break;
				}

				case 'ConfigUpdated': {
					// Config delta doesn't include data; reload via REST
					this.reloadConfig();
					break;
				}

				case 'AgentHistoryUpdated': {
					// Agent sessions changed; reload via REST
					this.reloadAgentSessions();
					break;
				}

				case 'ProjectStateUpdated': {
					if (change.data.status !== null) {
						this.state.project.status = change.data.status;
					}
					break;
				}

				case 'ParseError': {
					this.state.parse_errors.push({
						id: 0, // server will assign real ID
						project_id: this.state.project.id,
						file_path: change.data.file_path,
						error_message: change.data.error,
						severity: 'error',
						occurred_at: new Date().toISOString(),
						resolved_at: null
					});
					break;
				}
			}
		}
	}

	/**
	 * Reload config from REST API after a ConfigUpdated delta.
	 */
	private async reloadConfig(): Promise<void> {
		if (!this.selectedProjectId || !this.state) return;

		try {
			const response = await fetch(`/api/v1/projects/${this.selectedProjectId}/state`);
			if (response.ok) {
				const fullState: ProjectState = await response.json();
				this.state.config = fullState.config;
			}
		} catch (err) {
			console.error('[ProjectStore] Error reloading config:', err);
		}
	}

	/**
	 * Reload agent sessions from REST API after an AgentHistoryUpdated delta.
	 */
	private async reloadAgentSessions(): Promise<void> {
		if (!this.selectedProjectId || !this.state) return;

		try {
			const response = await fetch(`/api/v1/projects/${this.selectedProjectId}/state`);
			if (response.ok) {
				const fullState: ProjectState = await response.json();
				this.state.agent_sessions = fullState.agent_sessions;
			}
		} catch (err) {
			console.error('[ProjectStore] Error reloading agent sessions:', err);
		}
	}
}

export const projectStore = new ProjectStore();
