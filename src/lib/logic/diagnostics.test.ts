/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { formatDetails, screenOf } from './diagnostics';

describe('error details', () => {
	test('lists what is known, leaving out the rest', () => {
		const text = formatDetails({
			title: 'Check the values',
			message: 'Price is not a number',
			kind: 'validation',
			screen: '/db',
			version: '0.1.0',
			at: new Date('2026-10-26T10:00:00Z')
		});
		expect(text).toBe(
			'Check the values: Price is not a number\n\nKind: validation\nScreen: /db\nSlateDB: 0.1.0\nTime: 2026-10-26T10:00:00.000Z'
		);
	});

	test('screens drop trailing slashes', () => {
		expect(screenOf('/app/')).toBe('/app');
		expect(screenOf('/')).toBe('/');
	});
});
