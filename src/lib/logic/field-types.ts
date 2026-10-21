import type { FieldType, FieldTypeName } from '../api/types';

export interface FieldTypeInfo {
	name: FieldTypeName;
	label: string;
	storage: string;
}

/** Access-style field types offered by the table designer. */
export const FIELD_TYPES: FieldTypeInfo[] = [
	{ name: 'short_text', label: 'Short Text', storage: 'TEXT' },
	{ name: 'long_text', label: 'Long Text', storage: 'TEXT' },
	{ name: 'number', label: 'Number', storage: 'INTEGER / REAL' },
	{ name: 'large_number', label: 'Large Number', storage: 'INTEGER' },
	{ name: 'currency', label: 'Currency', storage: 'INTEGER (minor units)' },
	{ name: 'auto_number', label: 'AutoNumber', storage: 'INTEGER PRIMARY KEY' },
	{ name: 'date_time', label: 'Date/Time', storage: 'TEXT (ISO 8601)' },
	{ name: 'yes_no', label: 'Yes/No', storage: 'INTEGER (0/1)' },
	{ name: 'attachment', label: 'Attachment', storage: 'BLOB' },
	{ name: 'hyperlink', label: 'Hyperlink', storage: 'TEXT' },
	{ name: 'calculated', label: 'Calculated', storage: 'generated column' },
	{ name: 'lookup', label: 'Lookup', storage: 'foreign key' }
];

export function fieldTypeLabel(t: FieldType): string {
	if (t.type === 'unknown') return `Other (${t.declared || 'untyped'})`;
	return FIELD_TYPES.find((i) => i.name === t.type)?.label ?? t.type;
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
