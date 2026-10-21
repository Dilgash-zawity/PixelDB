/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { toFilterSpecs } from './filters';

describe('filters', () => {
	test('converts rows and skips incomplete ones', () => {
		expect(
			toFilterSpecs([
				{ column: 'Name', op: 'contains', value: 'ada' },
				{ column: 'Name', op: 'eq', value: '  ' },
				{ column: 'City', op: 'in', value: 'Paris, Rome,' },
				{ column: 'Email', op: 'is_null', value: 'ignored' }
			])
		).toEqual([
			{ column: 'Name', op: 'contains', value: 'ada' },
			{ column: 'City', op: 'in', value: ['Paris', 'Rome'] },
			{ column: 'Email', op: 'is_null', value: null }
		]);
	});
});
