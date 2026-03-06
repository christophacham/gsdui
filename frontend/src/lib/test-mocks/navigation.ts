/**
 * Mock for $app/navigation in test environment.
 */
export function goto(url: string, opts?: Record<string, unknown>) {
	return Promise.resolve();
}

export function invalidate(url: string) {
	return Promise.resolve();
}

export function invalidateAll() {
	return Promise.resolve();
}

export function preloadData(url: string) {
	return Promise.resolve();
}

export function preloadCode(...urls: string[]) {
	return Promise.resolve();
}

export function beforeNavigate(fn: (navigation: unknown) => void) {}
export function afterNavigate(fn: (navigation: unknown) => void) {}
export function onNavigate(fn: (navigation: unknown) => void) {}
