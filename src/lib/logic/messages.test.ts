/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { humanizeError } from './messages';

describe('humanizeError', () => {
	test('constraint failures', () => {
		expect(humanizeError('record #1: FOREIGN KEY constraint failed')).toStartWith('This record is linked');
		expect(humanizeError('NOT NULL constraint failed: Customers.Name')).toBe('“Name” must have a value.');
		expect(humanizeError('UNIQUE constraint failed: People.First, People.Last')).toBe(
			'Another record already has the same “First” and “Last”.'
		);
		expect(humanizeError('operation #2: CHECK constraint failed: qty >= 0')).toBe(
			"operation #2: A value doesn't meet one of this table's validation rules."
		);
	});

	test('other messages pass through as a sentence', () => {
		expect(humanizeError('field "Balance": "oops" is not a valid amount')).toBe('Field "Balance": "oops" is not a valid amount');
		expect(humanizeError('only queries that read data (SELECT) can be run here')).toBe(
			'Only queries that read data (SELECT) can be run here'
		);
	});
});
