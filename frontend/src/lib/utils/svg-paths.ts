/**
 * SVG path generation utilities for dependency arrows.
 *
 * Generates cubic Bezier connector paths between plan cards
 * with gentle curves based on vertical distance.
 */

export interface Point {
	x: number;
	y: number;
}

/**
 * Generate a cubic Bezier SVG path from one point to another.
 * Used to draw dependency arrows from the bottom-center of a source card
 * to the top-center of a target card.
 *
 * Control point offset is proportional to vertical distance for gentle curves.
 */
export function cubicConnectorPath(from: Point, to: Point): string {
	const dy = Math.abs(to.y - from.y);
	const offset = Math.max(dy * 0.4, 30);

	const cx1 = from.x;
	const cy1 = from.y + offset;
	const cx2 = to.x;
	const cy2 = to.y - offset;

	return `M ${from.x} ${from.y} C ${cx1} ${cy1}, ${cx2} ${cy2}, ${to.x} ${to.y}`;
}

/**
 * Get the bottom-center position of an element relative to a container.
 */
export function getBottomCenter(el: HTMLElement, container: HTMLElement): Point {
	const elRect = el.getBoundingClientRect();
	const containerRect = container.getBoundingClientRect();

	return {
		x: elRect.left - containerRect.left + elRect.width / 2,
		y: elRect.bottom - containerRect.top
	};
}

/**
 * Get the top-center position of an element relative to a container.
 */
export function getTopCenter(el: HTMLElement, container: HTMLElement): Point {
	const elRect = el.getBoundingClientRect();
	const containerRect = container.getBoundingClientRect();

	return {
		x: elRect.left - containerRect.left + elRect.width / 2,
		y: elRect.top - containerRect.top
	};
}
