/// <reference types="bun" />
import { afterEach, describe, expect, test } from 'bun:test';
import { addCatalog, currentLocale, format, missingKeys, setLocale, t } from './index';

afterEach(() => setLocale('en'));

describe('i18n', () => {
	test('looks up, fills placeholders and picks plural forms', () => {
		expect(t('tabs.reports')).toBe('Reports');
		expect(t('common.records', { count: 1 })).toBe('1 record');
		expect(t('common.records', { count: 1200 })).toBe('1,200 records');
		expect(t('reports.firstOf', { shown: 5, total: 9 })).toBe('First 5 of 9 records');
		expect(format('{a} and {b}', { a: 1 })).toBe('1 and {b}');
	});

	test('falls back to English, then to the key', () => {
		addCatalog('xx', { tabs: { reports: 'Rapports' } });
		expect(setLocale('xx-YY')).toBe('xx');
		expect(currentLocale()).toBe('xx');
		expect(t('tabs.reports')).toBe('Rapports');
		expect(t('tabs.query')).toBe('Query');
		expect(t('nope.key' as never)).toBe('nope.key');
		expect(setLocale('zz')).toBe('en');
	});

	test('lists the keys a translation lacks', () => {
		expect(missingKeys({ tabs: { diagram: 'D', datasheet: 'D', query: 'Q' } }).slice(0, 2)).toEqual(['common', 'tabs.reports']);
	});
});
