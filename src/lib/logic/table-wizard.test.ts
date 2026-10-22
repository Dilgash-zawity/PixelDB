/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { QUICK_FIELDS, WIZARD_EXCLUDED_TYPES, fieldNameProblems, fieldOptionBadges, wizardField, wizardTable } from './table-wizard';

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
			wizardField({ name: ' Date ', type: 'date_time' }),
			wizardField({ name: 'Total', type: 'currency' })
		]);
		expect(def.name).toBe('Orders');
		expect(def.fields.map((f) => f.name)).toEqual(['ID', 'Date', 'Total']);
		expect(def.fields[0]).toMatchObject({ primary_key: true, field_type: { type: 'auto_number' } });
		expect(def.fields[2].field_type).toEqual({ type: 'currency', scale: 2 });
	});

	test('keeps chosen options and drops unfinished ones', () => {
		const def = wizardTable('People', [
			{
				name: 'Email',
				field_type: { type: 'short_text', max_length: 80 },
				required: true,
				unique: true,
				default: { kind: 'literal', value: ' ' },
				check: '  ',
				description: ' Work address '
			}
		]);
		expect(def.fields[1]).toMatchObject({
			name: 'Email',
			field_type: { type: 'short_text', max_length: 80 },
			required: true,
			unique: true,
			default: null,
			check: null,
			description: 'Work address'
		});
	});

	test('summarises options beyond the defaults', () => {
		expect(fieldOptionBadges(wizardField({ name: 'Name', type: 'short_text' }))).toEqual([]);
		expect(
			fieldOptionBadges({ name: 'Code', field_type: { type: 'short_text', max_length: 10 }, required: true, unique: true })
		).toEqual(['Up to 10 characters', 'Required', 'No duplicates']);
	});

	test('quick picks only use offered types', () => {
		expect(QUICK_FIELDS.every((f) => !WIZARD_EXCLUDED_TYPES.includes(f.type))).toBe(true);
	});
});
