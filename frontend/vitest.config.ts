import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [svelte({ hot: !process.env.VITEST })],
	test: {
		include: ['src/**/*.{test,spec}.{js,ts}'],
		environment: 'jsdom',
		globals: true,
		alias: {
			$lib: new URL('./src/lib', import.meta.url).pathname,
			'$app/navigation': new URL('./src/lib/test-mocks/navigation.ts', import.meta.url).pathname,
			'$app/stores': new URL('./src/lib/test-mocks/stores.ts', import.meta.url).pathname
		}
	}
});
