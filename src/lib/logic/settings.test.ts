/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { DEFAULT_SETTINGS, readSettings, settingsAttributes } from './settings';

describe('settings', () => {
	test('fills defaults and ignores unknown values', () => {
		expect(readSettings(null)).toEqual(DEFAULT_SETTINGS);
		expect(readSettings({ accent: 'pink', radius: 'large', reduceMotion: 'yes', extra: 1 })).toEqual({
			...DEFAULT_SETTINGS,
			radius: 'large'
		});
	});

	test('defaults leave <html> untouched', () => {
		expect(Object.values(settingsAttributes(DEFAULT_SETTINGS)).every((v) => v === null)).toBe(true);
		expect(settingsAttributes({ ...DEFAULT_SETTINGS, accent: 'blue', reduceMotion: true })).toMatchObject({
			'data-ui-accent': 'blue',
			'data-reduce-motion': ''
		});
	});
});
