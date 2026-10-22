import type { FieldType, FieldTypeName } from '../api/types';

export interface FieldTypeInfo {
	name: FieldTypeName;
	/** Plain-language name shown to people. */
	label: string;
	/** One sentence explaining when to use it. */
	hint: string;
	group: 'basic' | 'advanced';
}

/** Field types offered when adding or editing a field. */
export const FIELD_TYPES: FieldTypeInfo[] = [
	{ name: 'short_text', label: 'Text', hint: 'Names, titles, codes — a single line.', group: 'basic' },
	{ name: 'long_text', label: 'Long text', hint: 'Notes and descriptions of any length.', group: 'basic' },
	{ name: 'number', label: 'Number', hint: 'Quantities and measurements.', group: 'basic' },
	{ name: 'currency', label: 'Currency', hint: 'Money, stored exactly to the cent.', group: 'basic' },
	{ name: 'date_time', label: 'Date', hint: 'A calendar date, optionally with a time.', group: 'basic' },
	{ name: 'yes_no', label: 'Checkbox', hint: 'Yes or no.', group: 'basic' },
	{ name: 'hyperlink', label: 'Link', hint: 'A web address.', group: 'basic' },
	{ name: 'attachment', label: 'File', hint: 'A document or image stored in the database.', group: 'basic' },
	{ name: 'lookup', label: 'Relation', hint: 'Points to a record in another table.', group: 'advanced' },
	{ name: 'calculated', label: 'Formula', hint: 'Computed from other fields.', group: 'advanced' },
	{ name: 'large_number', label: 'Large number', hint: 'Whole numbers beyond ±2 billion.', group: 'advanced' },
	{ name: 'auto_number', label: 'Auto ID', hint: 'A unique number assigned automatically.', group: 'advanced' }
];

export function fieldTypeInfo(t: FieldType): FieldTypeInfo | undefined {
	return FIELD_TYPES.find((i) => i.name === t.type);
}

export function fieldTypeLabel(t: FieldType): string {
	if (t.type === 'unknown') return t.declared ? `Other (${t.declared})` : 'Any';
	if (t.type === 'number' && t.real) return 'Decimal number';
	return fieldTypeInfo(t)?.label ?? t.type;
}

/** Color families used to tell field types apart at a glance in the colorful look. */
export type Tone = 'blue' | 'green' | 'amber' | 'pink' | 'violet' | 'cyan' | 'none';

export function fieldTypeTone(t: FieldType): Tone {
	switch (t.type) {
		case 'short_text':
		case 'long_text':
			return 'blue';
		case 'number':
		case 'large_number':
			return 'green';
		case 'currency':
			return 'amber';
		case 'date_time':
			return 'pink';
		case 'yes_no':
		case 'calculated':
			return 'violet';
		case 'hyperlink':
		case 'attachment':
		case 'lookup':
			return 'cyan';
		default:
			return 'none';
	}
}

/** A field type with default parameters. */
export function makeFieldType(name: FieldTypeName): FieldType {
	switch (name) {
		case 'short_text':
			return { type: 'short_text', max_length: 255 };
		case 'number':
			return { type: 'number', real: false };
		case 'currency':
			return { type: 'currency', scale: 2 };
		case 'calculated':
			return { type: 'calculated', expression: '', stored: false, result: 'any' };
		case 'lookup':
			return { type: 'lookup', value_type: 'integer', display_column: null };
		case 'unknown':
			return { type: 'unknown', declared: '' };
		default:
			return { type: name } as FieldType;
	}
}

export function isReadOnlyType(t: FieldType): boolean {
	return t.type === 'calculated' || t.type === 'auto_number';
}

export const NUMERIC_TYPES: FieldTypeName[] = ['number', 'large_number', 'currency', 'auto_number'];
