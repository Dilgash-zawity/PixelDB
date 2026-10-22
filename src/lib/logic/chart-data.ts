import type { Aggregate, AggregateResult, FieldType, JsonValue } from '../api/types';
import type { ChartSeries } from './app-def';
import { displayValue } from './values';

/** Rows for a chart: `label` for the group, `s0`, `s1`… for each series. */
export interface ChartData {
	rows: Record<string, string | number>[];
	series: { key: string; label: string }[];
	/** Formats a plotted value of series `key` for tooltips and axes. */
	format: (key: string, value: number) => string;
	truncated: boolean;
}

export const AGGREGATE_LABELS: Record<Aggregate, string> = {
	count: 'Count',
	count_distinct: 'Count distinct',
	sum: 'Sum',
	avg: 'Average',
	min: 'Minimum',
	max: 'Maximum'
};

export function seriesLabel(s: ChartSeries): string {
	if (!s.column) return 'Number of records';
	return `${AGGREGATE_LABELS[s.agg]} of ${s.column}`;
}

/** A total as a plain number to plot: currency in major units. */
export function plotValue(value: JsonValue, t: FieldType | null): number {
	if (typeof value !== 'number') return typeof value === 'string' && value.trim() && !isNaN(Number(value)) ? Number(value) : 0;
	if (t?.type === 'currency') return value / 10 ** (t.scale ?? 2);
	return value;
}

const plain = new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 });

/** A plotted number as the user expects to read it. */
export function formatPlotted(value: number, t: FieldType | null): string {
	if (t?.type === 'currency') {
		const scale = t.scale ?? 2;
		return new Intl.NumberFormat(undefined, { minimumFractionDigits: scale, maximumFractionDigits: scale }).format(value);
	}
	return plain.format(value);
}

/** A total read straight from the backend (e.g. for a number card). */
export function formatTotal(value: JsonValue, t: FieldType | null): string {
	if (value === null || value === undefined) return '—';
	if (typeof value === 'number') return formatPlotted(plotValue(value, t), t);
	return displayValue(value, t ?? undefined);
}

/** Text for a group value on the axis or in the legend. */
export function groupLabel(value: JsonValue, t: FieldType | null): string {
	if (value === null || value === undefined || value === '') return '(empty)';
	return displayValue(value, t ?? undefined);
}

/**
 * Turns an aggregate result into chart rows. `grouped` says whether the
 * first column is the group.
 */
export function toChartData(result: AggregateResult, series: ChartSeries[], grouped: boolean): ChartData {
	const offset = grouped ? 1 : 0;
	const types = result.types.slice(offset);
	const keys = series.map((_, i) => `s${i}`);
	const rows = result.rows.map((row) => {
		const out: Record<string, string | number> = {
			label: grouped ? groupLabel(row[0], result.types[0]) : 'Total'
		};
		keys.forEach((k, i) => (out[k] = plotValue(row[i + offset], types[i] ?? null)));
		return out;
	});
	return {
		rows,
		series: keys.map((key, i) => ({ key, label: seriesLabel(series[i]) })),
		format: (key, value) => formatPlotted(value, types[keys.indexOf(key)] ?? null),
		truncated: result.truncated
	};
}
