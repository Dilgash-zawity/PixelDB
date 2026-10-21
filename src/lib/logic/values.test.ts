/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { displayValue, formatMinorUnits, inputKind, toEditorValue } from './values';

describe('formatMinorUnits', () => {
	test('formats exactly', () => {
		expect(formatMinorUnits(1234, 2)).toBe('12.34');
		expect(formatMinorUnits(5, 2)).toBe('0.05');
		expect(formatMinorUnits(-50, 2)).toBe('-0.50');
		expect(formatMinorUnits(0, 2)).toBe('0.00');
		expect(formatMinorUnits(7, 0)).toBe('7');
		expect(formatMinorUnits(900719925474099, 4)).toBe('90071992547.4099');
	});
});

describe('display and editor values', () => {
	test('display per type', () => {
		expect(displayValue(1234, { type: 'currency', scale: 2 })).toBe('12.34');
		expect(displayValue(1, { type: 'yes_no' })).toBe('Yes');
		expect(displayValue({ blob: { size: 3 } })).toBe('[3 bytes]');
		expect(displayValue(null)).toBe('');
	});
	test('editor values', () => {
		expect(toEditorValue(0, { type: 'yes_no' })).toBe(false);
		expect(toEditorValue(250, { type: 'currency' })).toBe('2.50');
		expect(inputKind({ type: 'calculated', expression: 'a' })).toBe('readonly');
	});
});
