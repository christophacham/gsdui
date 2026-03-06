/**
 * Shared test utilities for the GSD Pipeline UI frontend.
 *
 * Provides mock factories for ProjectState, WebSocket, and related types.
 */

import type {
	AgentSession,
	ExecutionRun,
	ParseError,
	PhaseState,
	PlanState,
	Project,
	ProjectConfig,
	ProjectState,
	VerificationResult
} from './types/api.js';

// ---------------------------------------------------------------------------
// Factory helpers
// ---------------------------------------------------------------------------

export function createMockProject(overrides: Partial<Project> = {}): Project {
	return {
		id: 'proj-001',
		name: 'test-project',
		path: '/home/user/projects/test-project',
		status: 'active',
		retention_days: 30,
		created_at: '2026-03-01T00:00:00Z',
		last_seen_at: '2026-03-06T12:00:00Z',
		...overrides
	};
}

export function createMockPhase(overrides: Partial<PhaseState> = {}): PhaseState {
	return {
		id: 1,
		project_id: 'proj-001',
		phase_number: '01',
		phase_name: 'Backend Foundation',
		goal: 'Build the backend state engine',
		depends_on: null,
		stage: 'executed',
		status: null,
		requirements: 'STATE-01,STATE-02',
		plan_count: 4,
		completed_plan_count: 4,
		updated_at: '2026-03-06T12:00:00Z',
		...overrides
	};
}

export function createMockPlan(overrides: Partial<PlanState> = {}): PlanState {
	return {
		id: 1,
		project_id: 'proj-001',
		phase_number: '01',
		plan_number: '01',
		plan_name: 'Project scaffold and database',
		wave: 1,
		depends_on: null,
		plan_type: 'execute',
		status: 'done',
		requirements: 'STATE-01',
		files_modified: 'src/main.rs,src/db/mod.rs',
		updated_at: '2026-03-06T12:00:00Z',
		...overrides
	};
}

export function createMockExecutionRun(overrides: Partial<ExecutionRun> = {}): ExecutionRun {
	return {
		id: 1,
		project_id: 'proj-001',
		phase_number: '01',
		plan_number: '01',
		run_number: 1,
		superseded: 0,
		started_at: '2026-03-06T10:00:00Z',
		completed_at: '2026-03-06T10:09:00Z',
		duration_minutes: 9,
		status: 'done',
		key_files_created: 'src/main.rs',
		key_files_modified: null,
		requirements_completed: 'STATE-01',
		created_at: '2026-03-06T10:00:00Z',
		...overrides
	};
}

export function createMockAgentSession(overrides: Partial<AgentSession> = {}): AgentSession {
	return {
		id: 1,
		project_id: 'proj-001',
		agent_id: 'agent-abc123',
		agent_type: 'claude',
		phase_number: '01',
		plan_number: '01',
		started_at: '2026-03-06T10:00:00Z',
		ended_at: '2026-03-06T10:09:00Z',
		created_at: '2026-03-06T10:00:00Z',
		...overrides
	};
}

export function createMockVerification(
	overrides: Partial<VerificationResult> = {}
): VerificationResult {
	return {
		id: 1,
		project_id: 'proj-001',
		phase_number: '01',
		status: 'passed',
		score: '100',
		verified_at: '2026-03-06T12:00:00Z',
		created_at: '2026-03-06T12:00:00Z',
		...overrides
	};
}

export function createMockProjectState(overrides: Partial<ProjectState> = {}): ProjectState {
	const phase1 = createMockPhase();
	const phase2 = createMockPhase({
		id: 2,
		phase_number: '02',
		phase_name: 'Pipeline Dashboard',
		goal: 'Build the pipeline visualization frontend',
		depends_on: '01',
		stage: 'executing',
		plan_count: 4,
		completed_plan_count: 0
	});

	return {
		project: createMockProject(),
		phases: [phase1, phase2],
		plans: {
			'01': [
				createMockPlan(),
				createMockPlan({ id: 2, plan_number: '02', plan_name: 'GSD parsers', wave: 1 }),
				createMockPlan({
					id: 3,
					plan_number: '03',
					plan_name: 'File watcher',
					wave: 2,
					depends_on: '01,02'
				}),
				createMockPlan({
					id: 4,
					plan_number: '04',
					plan_name: 'WebSocket API',
					wave: 3,
					depends_on: '03'
				})
			],
			'02': [
				createMockPlan({
					id: 5,
					phase_number: '02',
					plan_number: '01',
					plan_name: 'SvelteKit scaffold',
					wave: 1,
					status: 'working'
				})
			]
		},
		recent_runs: [createMockExecutionRun()],
		agent_sessions: [createMockAgentSession()],
		verifications: {
			'01': createMockVerification()
		},
		config: null,
		parse_errors: [],
		...overrides
	};
}

// ---------------------------------------------------------------------------
// Mock WebSocket
// ---------------------------------------------------------------------------

export class MockWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	readonly CONNECTING = 0;
	readonly OPEN = 1;
	readonly CLOSING = 2;
	readonly CLOSED = 3;

	url: string;
	readyState: number = MockWebSocket.CONNECTING;
	protocol = '';
	extensions = '';
	bufferedAmount = 0;
	binaryType: BinaryType = 'blob';

	onopen: ((event: Event) => void) | null = null;
	onclose: ((event: CloseEvent) => void) | null = null;
	onmessage: ((event: MessageEvent) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;

	sentMessages: string[] = [];

	private listeners: Record<string, Array<EventListenerOrEventListenerObject>> = {};

	constructor(url: string, _protocols?: string | string[]) {
		this.url = url;
	}

	send(data: string): void {
		this.sentMessages.push(data);
	}

	close(_code?: number, _reason?: string): void {
		this.readyState = MockWebSocket.CLOSED;
	}

	addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
		if (!this.listeners[type]) this.listeners[type] = [];
		this.listeners[type].push(listener);
	}

	removeEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
		if (!this.listeners[type]) return;
		this.listeners[type] = this.listeners[type].filter((l) => l !== listener);
	}

	dispatchEvent(event: Event): boolean {
		const listeners = this.listeners[event.type] || [];
		for (const listener of listeners) {
			if (typeof listener === 'function') {
				listener(event);
			} else {
				listener.handleEvent(event);
			}
		}
		return true;
	}

	/** Simulate server opening the connection */
	simulateOpen(): void {
		this.readyState = MockWebSocket.OPEN;
		const event = new Event('open');
		this.onopen?.(event);
		this.dispatchEvent(event);
	}

	/** Simulate receiving a message from server */
	simulateMessage(data: unknown): void {
		const event = new MessageEvent('message', {
			data: typeof data === 'string' ? data : JSON.stringify(data)
		});
		this.onmessage?.(event);
		this.dispatchEvent(event);
	}

	/** Simulate connection close */
	simulateClose(code = 1000, reason = ''): void {
		this.readyState = MockWebSocket.CLOSED;
		const event = new CloseEvent('close', { code, reason });
		this.onclose?.(event);
		this.dispatchEvent(event);
	}

	/** Simulate error */
	simulateError(): void {
		const event = new Event('error');
		this.onerror?.(event);
		this.dispatchEvent(event);
	}
}
