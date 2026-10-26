/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { pruneViews, putView, readViews, removeView, renameFieldInViews, renameTableInViews, showsView, viewNameProblem, type SavedView } from './saved-views';

const cheap: SavedView = { name: 'Cheap', sort: { column: 'Price', descending: false }, filters: [{ column: 'Price', op: 'lt', value: '5' }], search: '' };

describe('saved views', () => {
	test('reads stored JSON, dropping what is malformed', () => {
		expect(readViews(null)).toEqual({});
		expect(readViews({ T: [cheap, { name: '' }, 3, { name: 'x', filters: [{ column: 1 }], sort: 'a' }], U: 'no' })).toEqual({
			T: [cheap, { name: 'x', sort: null, filters: [], search: '' }]
		});
	});

	test('adds, replaces, sorts and removes by name', () => {
		let v = putView({}, 'T', cheap);
		v = putView(v, 'T', { ...cheap, name: 'All', filters: [] });
		v = putView(v, 'T', { ...cheap, name: 'cheap ', search: 'pen' });
		expect(v.T.map((x) => [x.name, x.search])).toEqual([
			['All', ''],
			['cheap', 'pen']
		]);
		expect(viewNameProblem(v, 'T', 'ALL')).toBe('A view with this name already exists');
		expect(viewNameProblem(v, 'T', 'All', 'all')).toBeNull();
		expect(viewNameProblem(v, 'T', ' ')).toBe('Enter a name');
		v = removeView(removeView(v, 'T', 'all'), 'T', 'CHEAP');
		expect(v).toEqual({});
	});

	test('follow renames and drop what no longer exists', () => {
		const v = { T: [cheap] };
		expect(renameTableInViews(v, 'T', 'U')).toEqual({ U: [cheap] });
		const r = renameFieldInViews(v, 'T', 'Price', 'Cost');
		expect(r.T[0].sort?.column).toBe('Cost');
		expect(r.T[0].filters[0].column).toBe('Cost');
		expect(pruneViews(v, { T: ['Name'] })).toEqual({ T: [{ ...cheap, sort: null, filters: [] }] });
		expect(pruneViews(v, {})).toEqual({});
	});

	test('knows which view is showing', () => {
		expect(showsView(cheap, { sort: cheap.sort, filters: cheap.filters, search: ' ' })).toBe(true);
		expect(showsView(cheap, { sort: null, filters: cheap.filters, search: '' })).toBe(false);
	});
});
