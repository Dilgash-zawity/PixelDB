import type { FieldDef, JsonValue, RecordKey, RecordRow, RecordValues, TableDef } from '../api/types';
import { jsonClone, toEditorValue } from './values';

/**
 * Editing state for one record. `original` holds the values as loaded (in
 * editor form); `current` holds unsaved edits. Plain data so it can be wrapped
 * in Svelte `$state`.
 */
export interface RecordBuffer {
	key: RecordKey | null;
	original: Record<string, JsonValue>;
	current: Record<string, JsonValue>;
}

function same(a: JsonValue, b: JsonValue): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

export function newBuffer(def: TableDef, row: RecordRow | null, columns?: string[]): RecordBuffer {
	const original: Record<string, JsonValue> = {};
	const cols = columns ?? def.fields.map((f) => f.name);
	for (const field of def.fields) {
		const idx = cols.indexOf(field.name);
		const raw = row && idx >= 0 ? row.values[idx] : null;
		original[field.name] = row ? toEditorValue(raw, field.field_type) : initialValue(field);
	}
	return { key: row?.key ?? null, original, current: jsonClone(original) };
}

function initialValue(field: FieldDef): JsonValue {
	if (field.field_type.type === 'yes_no') {
		const d = field.default;
		return d?.kind === 'literal' ? d.value === true || d.value === 1 || d.value === 'yes' : false;
	}
	return null;
}

export function setValue(buf: RecordBuffer, column: string, value: JsonValue): void {
	buf.current[column] = value;
}

/** Only the values that differ from what was loaded. */
export function changes(buf: RecordBuffer): RecordValues {
	const out: RecordValues = {};
	for (const [col, value] of Object.entries(buf.current)) {
		if (!same(value, buf.original[col] ?? null)) out[col] = value;
	}
	return out;
}

export function isDirty(buf: RecordBuffer): boolean {
	return Object.keys(changes(buf)).length > 0;
}

export function undoField(buf: RecordBuffer, column: string): void {
	buf.current[column] = jsonClone(buf.original[column] ?? null);
}

export function undoAll(buf: RecordBuffer): void {
	buf.current = jsonClone(buf.original);
}

/**
 * Values to submit: edits for existing records; for new records every
 * non-empty value (so an unchecked Yes/No is stored as No, not NULL).
 */
export function submission(buf: RecordBuffer, def: TableDef): RecordValues {
	const out = buf.key === null ? jsonClone(buf.current) : changes(buf);
	for (const f of def.fields) {
		const t = f.field_type.type;
		if (t === 'calculated' || (t === 'auto_number' && (out[f.name] === null || out[f.name] === ''))) {
			delete out[f.name];
		}
	}
	if (buf.key === null) {
		for (const [k, v] of Object.entries(out)) if (v === null || v === '') delete out[k];
	}
	return out;
}

/** Client-side check of required fields; the backend validates again. */
export function missingRequired(buf: RecordBuffer, def: TableDef): string[] {
	return def.fields
		.filter((f) => (f.required || f.primary_key) && !f.default)
		.filter((f) => f.field_type.type !== 'auto_number' && f.field_type.type !== 'calculated')
		.filter((f) => {
			const v = buf.current[f.name];
			return v === null || v === undefined || v === '';
		})
		.map((f) => f.name);
}
