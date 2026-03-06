import { describe, it, expect } from 'vitest';
import { createMockProjectState, createMockPhase, createMockPlan, MockWebSocket } from '../test-utils.js';

describe('test-utils', () => {
	it('creates a valid mock ProjectState', () => {
		const state = createMockProjectState();

		expect(state.project.id).toBe('proj-001');
		expect(state.project.name).toBe('test-project');
		expect(state.phases).toHaveLength(2);
		expect(state.phases[0].phase_number).toBe('01');
		expect(state.phases[1].phase_number).toBe('02');
		expect(Object.keys(state.plans)).toHaveLength(2);
		expect(state.plans['01']).toHaveLength(4);
		expect(state.recent_runs).toHaveLength(1);
		expect(state.agent_sessions).toHaveLength(1);
		expect(state.verifications['01'].status).toBe('passed');
		expect(state.config).toBeNull();
		expect(state.parse_errors).toHaveLength(0);
	});

	it('creates mock phase with overrides', () => {
		const phase = createMockPhase({ phase_number: '03', phase_name: 'Terminal' });

		expect(phase.phase_number).toBe('03');
		expect(phase.phase_name).toBe('Terminal');
		expect(phase.project_id).toBe('proj-001'); // default
	});

	it('creates mock plan with overrides', () => {
		const plan = createMockPlan({ status: 'working', wave: 2 });

		expect(plan.status).toBe('working');
		expect(plan.wave).toBe(2);
		expect(plan.project_id).toBe('proj-001'); // default
	});

	it('MockWebSocket tracks sent messages', () => {
		const ws = new MockWebSocket('ws://localhost/api/v1/ws/state');

		expect(ws.readyState).toBe(MockWebSocket.CONNECTING);
		expect(ws.url).toBe('ws://localhost/api/v1/ws/state');

		ws.simulateOpen();
		expect(ws.readyState).toBe(MockWebSocket.OPEN);

		ws.send(JSON.stringify({ type: 'subscribe', projects: ['proj-001'] }));
		expect(ws.sentMessages).toHaveLength(1);
		expect(JSON.parse(ws.sentMessages[0])).toEqual({
			type: 'subscribe',
			projects: ['proj-001']
		});
	});

	it('MockWebSocket simulates message reception', () => {
		const ws = new MockWebSocket('ws://localhost/api/v1/ws/state');
		const received: unknown[] = [];

		ws.onmessage = (event) => {
			received.push(JSON.parse(event.data));
		};

		ws.simulateOpen();
		ws.simulateMessage({
			type: 'snapshot',
			project: 'proj-001',
			data: createMockProjectState()
		});

		expect(received).toHaveLength(1);
		expect((received[0] as { type: string }).type).toBe('snapshot');
	});

	it('MockWebSocket simulates close', () => {
		const ws = new MockWebSocket('ws://localhost/api/v1/ws/state');
		let closed = false;

		ws.onclose = () => {
			closed = true;
		};

		ws.simulateOpen();
		ws.simulateClose();

		expect(closed).toBe(true);
		expect(ws.readyState).toBe(MockWebSocket.CLOSED);
	});
});
