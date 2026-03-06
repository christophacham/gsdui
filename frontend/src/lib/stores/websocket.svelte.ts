/**
 * WebSocket connection manager with reconnection and exponential backoff.
 *
 * Uses Svelte 5 runes ($state) for reactive connection status.
 * Connects to /api/v1/ws/state and routes messages to the project store.
 *
 * CRITICAL: Subscribe message must be sent within 10 seconds of connection
 * (backend timeout enforced in src/ws/mod.rs).
 */

import type { ClientMessage, WsMessage } from '$lib/types/websocket.js';

export type ConnectionStatus = 'connecting' | 'connected' | 'reconnecting' | 'disconnected';

class WebSocketManager {
	status = $state<ConnectionStatus>('disconnected');

	private ws: WebSocket | null = null;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectDelay = 1000;
	private maxReconnectDelay = 30000;
	private subscribedProjects: string[] = [];
	private messageHandler: ((msg: WsMessage) => void) | null = null;

	/**
	 * Register a handler for incoming WebSocket messages.
	 * Must be called before connect() to receive messages.
	 */
	onMessage(handler: (msg: WsMessage) => void): void {
		this.messageHandler = handler;
	}

	/**
	 * Open WebSocket connection and subscribe to the given projects.
	 * Reconnects automatically with exponential backoff on disconnection.
	 */
	connect(projectIds: string[]): void {
		this.subscribedProjects = projectIds;
		this.status = 'connecting';

		// Close any existing connection cleanly
		if (this.ws) {
			this.ws.onclose = null; // prevent reconnect loop
			this.ws.close();
			this.ws = null;
		}

		const protocol = typeof location !== 'undefined' && location.protocol === 'https:' ? 'wss:' : 'ws:';
		const host = typeof location !== 'undefined' ? location.host : 'localhost:5173';
		this.ws = new WebSocket(`${protocol}//${host}/api/v1/ws/state`);

		this.ws.onopen = () => {
			this.status = 'connected';
			this.reconnectDelay = 1000; // reset backoff

			// CRITICAL: send Subscribe immediately (10s backend timeout)
			this.send({ type: 'subscribe', projects: this.subscribedProjects });
		};

		this.ws.onmessage = (event: MessageEvent) => {
			try {
				const msg: WsMessage = JSON.parse(event.data);
				this.messageHandler?.(msg);
			} catch (err) {
				console.error('[WebSocket] Failed to parse message:', err);
			}
		};

		this.ws.onclose = () => {
			this.status = 'reconnecting';
			this.scheduleReconnect();
		};

		this.ws.onerror = (event) => {
			console.error('[WebSocket] Connection error:', event);
			// Let onclose handle reconnection
		};
	}

	/**
	 * Disconnect and stop all reconnection attempts.
	 */
	disconnect(): void {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}

		if (this.ws) {
			this.ws.onclose = null; // prevent reconnect
			this.ws.close();
			this.ws = null;
		}

		this.status = 'disconnected';
	}

	/**
	 * Send a message to the server.
	 */
	send(msg: ClientMessage): void {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(msg));
		}
	}

	/**
	 * Subscribe to additional projects without reconnecting.
	 */
	subscribe(projectIds: string[]): void {
		this.subscribedProjects = [...new Set([...this.subscribedProjects, ...projectIds])];
		this.send({ type: 'subscribe', projects: projectIds });
	}

	/**
	 * Unsubscribe from projects.
	 */
	unsubscribe(projectIds: string[]): void {
		this.subscribedProjects = this.subscribedProjects.filter((id) => !projectIds.includes(id));
		this.send({ type: 'unsubscribe', projects: projectIds });
	}

	private scheduleReconnect(): void {
		this.reconnectTimer = setTimeout(() => {
			this.connect(this.subscribedProjects);
		}, this.reconnectDelay);

		// Exponential backoff: 1s -> 2s -> 4s -> 8s -> ... -> 30s max
		this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
	}
}

export const wsManager = new WebSocketManager();
