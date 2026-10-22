/** The step-by-step "New table" and "Add field" flows: their steps, quick picks and checks. */
import type { FieldDef, FieldTypeName, TableDef } from '../api/types';
import { makeFieldType } from './field-types';
import { identifierProblem } from './values';

export interface WizardField {
	name: string;
	type: FieldTypeName;
}

/** Types the wizards offer: Auto ID is added for you, Relation is made on the Diagram, and a Formula needs one written. */
export const WIZARD_EXCLUDED_TYPES: FieldTypeName[] = ['auto_number', 'lookup', 'calculated'];

/** One-click fields for the New table flow. */
export const QUICK_FIELDS: WizardField[] = [
	{ name: 'Name', type: 'short_text' },
	{ name: 'Email', type: 'short_text' },
	{ name: 'Phone', type: 'short_text' },
	{ name: 'Date', type: 'date_time' },
	{ name: 'Price', type: 'currency' },
	{ name: 'Quantity', type: 'number' },
	{ name: 'Done', type: 'yes_no' },
	{ name: 'Notes', type: 'long_text' }
];

const lower = (s: string) => s.trim().toLowerCase();

/**
 * A problem with each field's name (or null), in order. `taken` are names already in
 * the table — the automatic ID for a new table, the existing fields when adding one.
 */
export function fieldNameProblems(names: string[], taken: string[] = ['ID']): (string | null)[] {
	const used = new Set(taken.map(lower));
	return names.map((name, i) => {
		const p = identifierProblem(name.trim(), 'Name');
		if (p) return p;
		if (used.has(lower(name))) return 'The table already has a field with this name';
		if (names.slice(0, i).some((n) => lower(n) === lower(name))) return 'Two fields have this name';
		return null;
	});
}

/** The new table: an automatic ID followed by the chosen fields. */
export function wizardTable(name: string, fields: WizardField[]): TableDef {
	return {
		name: name.trim(),
		fields: [
			{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
			...fields.map((f): FieldDef => ({ name: f.name.trim(), field_type: makeFieldType(f.type) }))
		]
	};
}
