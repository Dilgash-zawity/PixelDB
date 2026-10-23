import type { FieldType, JsonValue } from '../api/types';
import type { LookupOption } from './lookups';
import { displayValue } from './values';

/**
 * Board widgets: records as cards in a column ("lane") per value of one
 * field. Dropping a card on a lane stores that lane's value in the field.
 */

export interface Lane {
	/** Stable identity of the lane's value. */
	key: string;
	/** What dropping a card here stores; null for the "None" lane. */
	value: JsonValue;
	label: string;
}

/** Category colors (see `--tone-*` in app.css), handed out by value so a value keeps its color. */
export const TONES = ['blue', 'violet', 'cyan', 'green', 'amber', 'pink'] as const;
export type Tone = (typeof TONES)[number];

/** The tone a label always gets, from a hash of its text. */
export function toneFor(text: string): Tone {
	let h = 0;
	for (let i = 0; i < text.length; i++) h = (h * 31 + text.charCodeAt(i)) | 0;
	return TONES[Math.abs(h) % TONES.length];
}

const isNumeric = (t: FieldType | null) => !!t && ['number', 'large_number', 'auto_number', 'currency'].includes(t.type);

/** A field value as the board compares it: Yes/No as booleans, empty text as no value. */
export function normalize(value: JsonValue | undefined, t: FieldType | null): JsonValue {
	if (value === undefined || value === null || value === '') return null;
	if (t?.type === 'yes_no') return value === true || value === 1 || value === '1';
	return value;
}

export function laneKey(value: JsonValue | undefined, t: FieldType | null): string {
	return JSON.stringify(normalize(value, t));
}

/** The stored value for a lane typed into the board's settings, or null when it can't be one. */
export function laneValue(text: string, t: FieldType | null): JsonValue {
	const s = text.trim();
	if (!s) return null;
	if (!isNumeric(t)) return s;
	const n = Number(s);
	if (!Number.isFinite(n)) return null;
	// Currency is stored in minor units.
	return t?.type === 'currency' ? Math.round(n * 10 ** (t.scale ?? 2)) : n;
}

function compare(a: JsonValue, b: JsonValue): number {
	if (typeof a === 'number' && typeof b === 'number') return a - b;
	return String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: 'base' });
}

/**
 * The lanes of a board, left to right: a relation's records or No/Yes when
 * the field has fixed choices, else the configured lanes; then any other
 * value found in `found`, sorted; then "None" when some card has no value.
 */
export function boardLanes(t: FieldType | null, configured: string[], found: JsonValue[], lookup?: LookupOption[]): Lane[] {
	const out: Lane[] = [];
	const seen = new Set<string>();
	const label = (v: JsonValue) => (t?.type === 'lookup' ? (lookup?.find((o) => laneKey(o.value, t) === laneKey(v, t))?.label ?? displayValue(v)) : displayValue(v, t ?? undefined));
	const add = (v: JsonValue) => {
		const value = normalize(v, t);
		const key = JSON.stringify(value);
		if (value === null || seen.has(key)) return;
		seen.add(key);
		out.push({ key, value, label: label(value) });
	};
	if (t?.type === 'yes_no') {
		add(false);
		add(true);
	} else if (t?.type === 'lookup' && lookup) {
		for (const o of lookup) add(o.value);
	} else {
		for (const c of configured) add(laneValue(c, t));
	}
	const extra = found.map((v) => normalize(v, t)).filter((v) => v !== null && !seen.has(JSON.stringify(v)));
	for (const v of extra.sort(compare)) add(v);
	if (found.some((v) => normalize(v, t) === null)) out.push({ key: 'null', value: null, label: 'None' });
	return out;
}
