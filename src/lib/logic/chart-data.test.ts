/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { formatTotal, toChartData } from './chart-data';
import { parseInline, parseMarkdown } from './markdown';

describe('chart data', () => {
	test('groups become labels and money is plotted in major units', () => {
		const data = toChartData(
			{
				columns: ['month', 'Sum of total', 'Count'],
				rows: [
					['2024-01', 1250, 2],
					[null, 99, 1]
				],
				types: [null, { type: 'currency', scale: 2 }, null],
				truncated: false
			},
			[
				{ column: 'total', agg: 'sum' },
				{ column: null, agg: 'count' }
			],
			true
		);
		expect(data.rows).toEqual([
			{ label: '2024-01', s0: 12.5, s1: 2 },
			{ label: '(empty)', s0: 0.99, s1: 1 }
		]);
		expect(data.series.map((s) => s.label)).toEqual(['Sum of total', 'Number of records']);
		expect(data.format('s0', 12.5)).toMatch(/12.50/);
	});

	test('totals format by type', () => {
		expect(formatTotal(123456, { type: 'currency', scale: 2 })).toMatch(/1.?234.56/);
		expect(formatTotal(null, null)).toBe('—');
		expect(formatTotal(2.3333, null)).toMatch(/2.33/);
		expect(formatTotal('2024-01-05', { type: 'date_time' })).toBe('2024-01-05');
	});
});

describe('markdown', () => {
	test('inline styles', () => {
		expect(parseInline('a **b** *c* `d` _e_ *open')).toEqual([
			{ text: 'a ' },
			{ text: 'b', bold: true },
			{ text: ' ' },
			{ text: 'c', italic: true },
			{ text: ' ' },
			{ text: 'd', code: true },
			{ text: ' ' },
			{ text: 'e', italic: true },
			{ text: ' *open' }
		]);
	});

	test('blocks', () => {
		const blocks = parseMarkdown('# Title\nline one\nline two\n\n- a\n- b\n1. c\n> note\n---\n<b>x</b>');
		expect(blocks.map((b) => b.kind)).toEqual(['heading', 'paragraph', 'list', 'list', 'quote', 'rule', 'paragraph']);
		expect(blocks[1]).toEqual({ kind: 'paragraph', spans: [{ text: 'line one line two' }] });
		expect(blocks[2]).toMatchObject({ ordered: false, items: [[{ text: 'a' }], [{ text: 'b' }]] });
		// HTML is just text.
		expect(blocks[6]).toEqual({ kind: 'paragraph', spans: [{ text: '<b>x</b>' }] });
	});
});
