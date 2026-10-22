/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { QUICK_FIELDS, WIZARD_EXCLUDED_TYPES, fieldNameProblems, wizardTable } from './table-wizard';

describe('table wizard', () => {
	test('checks names against the table and each other', () => {
		expect(fieldNameProblems(['Name', ' ', 'id', 'name', 'Price'])).toEqual([
			null,
			'Name is required',
			'The table already has a field with this name',
			'Two fields have this name',
			null
		]);
		expect(fieldNameProblems(['Total'], ['ID', 'total'])).toEqual(['The table already has a field with this name']);
	});

	test('builds the table with an automatic ID first', () => {
		const def = wizardTable(' Orders ', [
			{ name: ' Date ', type: 'date_time' },
			{ name: 'Total', type: 'currency' }
		]);
		expect(def.name).toBe('Orders');
		expect(def.fields.map((f) => f.name)).toEqual(['ID', 'Date', 'Total']);
		expect(def.fields[0]).toMatchObject({ primary_key: true, field_type: { type: 'auto_number' } });
		expect(def.fields[2].field_type).toEqual({ type: 'currency', scale: 2 });
	});

	test('quick picks only use offered types', () => {
		expect(QUICK_FIELDS.every((f) => !WIZARD_EXCLUDED_TYPES.includes(f.type))).toBe(true);
	});
});
