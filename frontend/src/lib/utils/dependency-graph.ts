/**
 * Dependency graph traversal utilities for plan cards.
 *
 * Handles parsing depends_on fields (null, comma-separated, JSON array)
 * and computing transitive dependency chains for highlighting.
 */

import type { PlanState } from '$lib/types/api.js';

/**
 * Parse the depends_on field into an array of plan numbers.
 * Handles: null, comma-separated "01,02", JSON array '["01","02"]', single value "01".
 */
export function parseDependsOn(dependsOn: string | null): string[] {
	if (!dependsOn) return [];

	const trimmed = dependsOn.trim();
	if (!trimmed) return [];

	// Try JSON array first
	if (trimmed.startsWith('[')) {
		try {
			const parsed = JSON.parse(trimmed);
			if (Array.isArray(parsed)) {
				return parsed.map((v: unknown) => String(v).trim()).filter(Boolean);
			}
		} catch {
			// Fall through to comma-separated
		}
	}

	// Comma-separated or single value
	return trimmed
		.split(',')
		.map((s) => s.trim().replace(/['"]/g, ''))
		.filter(Boolean);
}

/**
 * Get all upstream plan numbers that this plan transitively depends on.
 * Returns the set of plan_numbers (NOT including the plan itself).
 */
export function getTransitiveDeps(planNumber: string, allPlans: PlanState[]): Set<string> {
	const result = new Set<string>();
	const visited = new Set<string>();
	const queue = [planNumber];

	while (queue.length > 0) {
		const current = queue.pop()!;
		if (visited.has(current)) continue;
		visited.add(current);

		const plan = allPlans.find((p) => p.plan_number === current);
		if (!plan) continue;

		const deps = parseDependsOn(plan.depends_on);
		for (const dep of deps) {
			if (!result.has(dep)) {
				result.add(dep);
				queue.push(dep);
			}
		}
	}

	return result;
}

/**
 * Get all downstream plan numbers that transitively depend on this plan.
 * Returns the set of plan_numbers (NOT including the plan itself).
 */
export function getTransitiveDependents(planNumber: string, allPlans: PlanState[]): Set<string> {
	const result = new Set<string>();
	const visited = new Set<string>();
	const queue = [planNumber];

	while (queue.length > 0) {
		const current = queue.pop()!;
		if (visited.has(current)) continue;
		visited.add(current);

		// Find all plans that directly depend on current
		for (const plan of allPlans) {
			const deps = parseDependsOn(plan.depends_on);
			if (deps.includes(current) && !result.has(plan.plan_number)) {
				result.add(plan.plan_number);
				queue.push(plan.plan_number);
			}
		}
	}

	return result;
}

/**
 * Get the full dependency chain for a plan: transitive deps + transitive dependents + itself.
 * Used for highlighting the complete chain when a plan card is clicked.
 */
export function getFullChain(planNumber: string, allPlans: PlanState[]): Set<string> {
	const deps = getTransitiveDeps(planNumber, allPlans);
	const dependents = getTransitiveDependents(planNumber, allPlans);
	const chain = new Set<string>([planNumber, ...deps, ...dependents]);
	return chain;
}
