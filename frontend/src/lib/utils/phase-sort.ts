/**
 * Numeric sorting for decimal phase numbers.
 *
 * Phase numbers can be integers ("01", "02") or decimals ("2.1", "2.2").
 * String sorting would put "10" before "2"; parseFloat gives correct numeric order.
 */

/**
 * Sort an array of objects with phase_number by numeric value.
 * "2.1" sorts between "2" and "3", "10" sorts after "9".
 * Returns a new sorted array (does not mutate input).
 */
export function sortPhaseNumbers<T extends { phase_number: string }>(phases: T[]): T[] {
	return [...phases].sort((a, b) => {
		return parseFloat(a.phase_number) - parseFloat(b.phase_number);
	});
}

/**
 * Returns true if the phase number contains a decimal point.
 * E.g., "2.1" -> true, "02" -> false, "10" -> false
 */
export function isDecimalPhase(phaseNumber: string): boolean {
	return phaseNumber.includes('.');
}
