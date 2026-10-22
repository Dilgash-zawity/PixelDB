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

/** A wizard field with the default options for its type. */
export function wizardField(f: WizardField): FieldDef {
	return { name: f.name, field_type: makeFieldType(f.type) };
}

/** Short labels for the options set on a field beyond its type's defaults, for a collapsed row. */
export function fieldOptionBadges(f: FieldDef): string[] {
	const t = f.field_type;
	const out: string[] = [];
	if (t.type === 'short_text' && t.max_length != null && t.max_length !== 255) out.push(`Up to ${t.max_length} characters`);
	if (t.type === 'number' && t.real) out.push('Decimals');
	if (t.type === 'currency' && t.scale != null && t.scale !== 2) out.push(`${t.scale} decimal places`);
	if (f.required) out.push('Required');
	if (f.unique) out.push('No duplicates');
	if (f.default) out.push('Default value');
	if (f.check?.trim()) out.push('Validation rule');
	if (f.description?.trim()) out.push('Description');
	return out;
}

/** The new table: an automatic ID followed by the chosen fields (names trimmed, unfinished defaults dropped). */
export function wizardTable(name: string, fields: FieldDef[]): TableDef {
	return {
		name: name.trim(),
		fields: [
			{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true },
			...fields.map((f): FieldDef => {
				const blankDefault = f.default && 'value' in f.default && String(f.default.value ?? '').trim() === '';
				return {
					...f,
					name: f.name.trim(),
					default: blankDefault ? null : f.default,
					check: f.check?.trim() || null,
					description: f.description?.trim() || null
				};
			})
		]
	};
}
