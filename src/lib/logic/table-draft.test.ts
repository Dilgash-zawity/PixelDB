/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import type { TableDef } from '../api/types';
import {
	addField,
	draftFromDefinition,
	moveField,
	newDraft,
	removeField,
	removedFields,
	renameField,
	setFieldType,
	setLookupTarget,
	toDesign,
	validateDraft
} from './table-draft';

const saved: TableDef = {
	name: 'Orders',
	fields: [
		{ name: 'id', field_type: { type: 'auto_number' }, primary_key: true },
		{ name: 'customer', field_type: { type: 'lookup' } },
		{ name: 'note', field_type: { type: 'long_text' } }
	],
	foreign_keys: [{ columns: ['customer'], ref_table: 'Customers', ref_columns: ['id'] }],
	unique_constraints: [['customer', 'note']],
	checks: []
};

describe('table draft', () => {
	test('new draft starts valid with an AutoNumber key', () => {
		const d = newDraft('People');
		addField(d);
		expect(d.fields.map((f) => f.field.name)).toEqual(['ID', 'Field2']);
		expect(validateDraft(d)).toEqual([]);
		expect(toDesign(d).renames).toEqual([]);
	});

	test('renames propagate and are reported', () => {
		const d = draftFromDefinition(saved);
		renameField(d, 2, 'client');
		const design = toDesign(d);
		expect(design.renames).toEqual([{ from: 'customer', to: 'client' }]);
		expect(design.definition.foreign_keys?.[0].columns).toEqual(['client']);
		expect(design.definition.unique_constraints).toEqual([['client', 'note']]);
		expect(removedFields(d, saved)).toEqual([]);
	});

	test('removing a field drops dependent constraints and is reported', () => {
		const d = draftFromDefinition(saved);
		removeField(d, 2);
		expect(d.foreign_keys).toEqual([]);
		expect(d.unique_constraints).toEqual([]);
		expect(removedFields(d, saved)).toEqual(['customer']);
	});

	test('validation catches common mistakes', () => {
		const d = newDraft('');
		const f = addField(d, 'lookup');
		addField(d, 'short_text', 'id');
		setFieldType(d, addField(d).id, 'auto_number');
		const problems = validateDraft(d);
		expect(problems).toContain('Table name is empty');
		expect(problems).toContain('Duplicate field name "id"');
		expect(problems).toContain(`Lookup field "${f.field.name}" needs a related table`);
		setLookupTarget(d, f.id, 'Customers', 'id');
		expect(validateDraft(d)).not.toContain(`Lookup field "${f.field.name}" needs a related table`);
	});

	test('setting AutoNumber makes it the only primary key', () => {
		const d = newDraft('T');
		const f = addField(d);
		setFieldType(d, f.id, 'auto_number');
		expect(d.fields.filter((x) => x.field.primary_key).map((x) => x.id)).toEqual([f.id]);
		moveField(d, f.id, -1);
		expect(d.fields[0].id).toBe(f.id);
	});
});
