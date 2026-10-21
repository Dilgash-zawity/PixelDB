import type { FilterOp, FilterSpec } from '../api/types';

/** An editable filter condition (values are typed as text). */
export interface FilterRow {
	column: string;
	op: FilterOp;
	value: string;
}

export const FILTER_OPS: { op: FilterOp; label: string }[] = [
	{ op: 'contains', label: 'contains' },
	{ op: 'eq', label: 'is' },
	{ op: 'ne', label: 'is not' },
	{ op: 'starts_with', label: 'starts with' },
	{ op: 'ends_with', label: 'ends with' },
	{ op: 'gt', label: 'is greater than' },
	{ op: 'lt', label: 'is less than' },
	{ op: 'ge', label: 'is at least' },
	{ op: 'le', label: 'is at most' },
	{ op: 'in', label: 'is any of' },
	{ op: 'is_null', label: 'is empty' },
	{ op: 'is_not_null', label: 'is not empty' }
];

export function needsValue(op: FilterOp): boolean {
	return op !== 'is_null' && op !== 'is_not_null';
}

/** Converts edited rows into backend filters, skipping incomplete ones. */
export function toFilterSpecs(rows: FilterRow[]): FilterSpec[] {
	return rows
		.filter((r) => r.column && (!needsValue(r.op) || r.value.trim() !== ''))
		.map((r) => ({
			column: r.column,
			op: r.op,
			value: !needsValue(r.op)
				? null
				: r.op === 'in'
					? r.value.split(',').map((v) => v.trim()).filter(Boolean)
					: r.value
		}));
}
