/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { PinchTracker, WHEEL_ZOOM_RATE, pinchDelta } from './pinch';

describe('pinch', () => {
	test('the wheel delta zooms the canvases by the pinched ratio', () => {
		for (const ratio of [0.5, 1.1, 2]) expect(Math.exp(-pinchDelta(ratio) * WHEEL_ZOOM_RATE)).toBeCloseTo(ratio, 10);
		expect(pinchDelta(1)).toBeCloseTo(0, 10);
		expect(pinchDelta(0)).toBe(0);
		expect(pinchDelta(Number.NaN)).toBe(0);
	});

	test('steps are relative to the previous one, and a new pinch starts over', () => {
		const t = new PinchTracker();
		t.begin();
		expect(t.step(1.5)).toBeCloseTo(1.5);
		expect(t.step(3)).toBeCloseTo(2);
		expect(t.step(3)).toBeNull();
		t.begin();
		expect(t.step(0.5)).toBeCloseTo(0.5);
	});
});
