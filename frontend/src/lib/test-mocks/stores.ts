/**
 * Mock for $app/stores in test environment.
 */
import { readable, writable } from 'svelte/store';

export const page = readable({
	url: new URL('http://localhost:5173'),
	params: {},
	route: { id: '/' },
	status: 200,
	error: null
});

export const navigating = readable(null);
export const updated = { check: () => Promise.resolve(false), subscribe: readable(false).subscribe };
