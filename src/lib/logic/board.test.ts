/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { boardLanes, laneKey, laneValue, toneFor, TONES } from './board';

describe('boardLanes', () => {
	const text = { type: 'short_text' } as const;

	test('configured lanes come first in order, then other values sorted, then None', () => {
		const lanes = boardLanes(text, ['To do', 'Doing', 'Done'], ['Done', 'Blocked', null, 'Archived', 'To do', '']);
		expect(lanes.map((l) => l.label)).toEqual(['To do', 'Doing', 'Done', 'Archived', 'Blocked', 'None']);
		expect(lanes.at(-1)?.value).toBeNull();
	});

	test('no None lane when every card has a value', () => {
		expect(boardLanes(text, [], ['b', 'a']).map((l) => l.label)).toEqual(['a', 'b']);
	});

	test('Yes/No fields always have both lanes, whatever is stored', () => {
		const lanes = boardLanes({ type: 'yes_no' }, ['ignored'], [1, 0, true]);
		expect(lanes.map((l) => [l.label, l.value])).toEqual([
			['No', false],
			['Yes', true]
		]);
		expect(laneKey(1, { type: 'yes_no' })).toBe(lanes[1].key);
	});

	test('relations use the related records as lanes, labelled', () => {
		const lookup = [
			{ value: 1, label: 'Alice' },
			{ value: 2, label: 'Bob' }
		];
		const lanes = boardLanes({ type: 'lookup' }, [], [2, 3], lookup);
		expect(lanes.map((l) => l.label)).toEqual(['Alice', 'Bob', '3']);
		expect(lanes[1].value).toBe(2);
	});

	test('numbers sort numerically and typed lanes become numbers', () => {
		const t = { type: 'number' } as const;
		expect(boardLanes(t, ['5'], [10, 2]).map((l) => l.value)).toEqual([5, 2, 10]);
		expect(laneValue('x', t)).toBeNull();
		expect(laneValue('12.5', { type: 'currency', scale: 2 })).toBe(1250);
	});
});

test('toneFor is stable and always a known tone', () => {
	expect(toneFor('Done')).toBe(toneFor('Done'));
	for (const s of ['', 'a', 'Open', 'Closed', 'In progress']) expect(TONES).toContain(toneFor(s));
});
