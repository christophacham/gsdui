/**
 * Duration and timestamp formatting helpers.
 *
 * Used by PhaseChip and PlanCard components to display timing information.
 */

import type { ExecutionRun } from '$lib/types/api.js';

/**
 * Format a duration in minutes to a human-readable string.
 *
 * - null -> "--"
 * - 0 or < 1 -> "<1m"
 * - 1-59 -> "Xm"
 * - 60+ -> "Xh Ym"
 */
export function formatDuration(minutes: number | null): string {
	if (minutes === null || minutes === undefined) return '--';
	if (minutes < 1) return '<1m';

	const hrs = Math.floor(minutes / 60);
	const mins = Math.round(minutes % 60);

	if (hrs === 0) return `${mins}m`;
	if (mins === 0) return `${hrs}h`;
	return `${hrs}h ${mins}m`;
}

/**
 * Format an ISO timestamp string to a relative or date string.
 *
 * - null -> "--"
 * - < 60 seconds ago -> "just now"
 * - < 60 minutes ago -> "Xm ago"
 * - < 24 hours ago -> "Xh ago"
 * - otherwise -> short date (e.g., "Mar 6")
 */
export function formatTimestamp(isoString: string | null): string {
	if (!isoString) return '--';

	const date = new Date(isoString);
	const now = new Date();
	const diffMs = now.getTime() - date.getTime();
	const diffSec = Math.floor(diffMs / 1000);
	const diffMin = Math.floor(diffSec / 60);
	const diffHr = Math.floor(diffMin / 60);

	if (diffSec < 60) return 'just now';
	if (diffMin < 60) return `${diffMin}m ago`;
	if (diffHr < 24) return `${diffHr}h ago`;

	return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

/**
 * Calculate total duration for a phase by summing duration_minutes
 * from all non-superseded execution runs in that phase.
 *
 * Returns null if no runs have duration data.
 */
export function calculatePhaseDuration(runs: ExecutionRun[]): number | null {
	const validRuns = runs.filter((r) => r.superseded === 0 && r.duration_minutes !== null);
	if (validRuns.length === 0) return null;

	return validRuns.reduce((sum, r) => sum + (r.duration_minutes ?? 0), 0);
}
