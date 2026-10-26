/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { DEFAULT_SETTINGS, readSettings, settingsAttributes } from './settings';

describe('settings', () => {
	test('automatic update checks default on and preserve an explicit opt-out', () => {
		expect(readSettings({ version: 2 }).automaticUpdates).toBe(true);
		expect(readSettings({ automaticUpdates: false }).automaticUpdates).toBe(false);
		expect(readSettings({ automaticUpdates: 'no' }).automaticUpdates).toBe(true);
	});
	test('the tour shows once', () => {
		expect(readSettings(null).tourSeen).toBe(false);
		expect(readSettings({ tourSeen: true }).tourSeen).toBe(true);
	});

	test('fills defaults and ignores unknown values', () => {
		expect(readSettings(null)).toEqual(DEFAULT_SETTINGS);
		expect(readSettings({ accent: 'pink', radius: 'large', reduceMotion: 'yes', extra: 1 })).toEqual({
			...DEFAULT_SETTINGS,
			radius: 'large'
		});
	});

	test('square corners from before Focus mode become auto, later choices are kept', () => {
		expect(readSettings({ radius: 'none' }).radius).toBe('auto');
		expect(readSettings({ radius: 'none', version: 2 }).radius).toBe('none');
		expect(readSettings({ radius: 'small' }).radius).toBe('small');
	});

	test('focus mode marks <html>', () => {
		expect(settingsAttributes({ ...DEFAULT_SETTINGS, focusMode: true })['data-focus']).toBe('');
		expect(settingsAttributes({ ...DEFAULT_SETTINGS, radius: 'none' })['data-radius']).toBe('none');
	});

	test('defaults leave <html> untouched', () => {
		expect(Object.values(settingsAttributes(DEFAULT_SETTINGS)).every((v) => v === null)).toBe(true);
		expect(settingsAttributes({ ...DEFAULT_SETTINGS, accent: 'blue', reduceMotion: true })).toMatchObject({
			'data-ui-accent': 'blue',
			'data-reduce-motion': ''
		});
	});
});
