import type { JsonValue, TableDef } from '../api/types';
import { recordService } from '../services/records';
import { displayValue } from './values';

export interface LookupOption {
	value: JsonValue;
	label: string;
}

/** At most this many related records are offered in a relation picker. */
export const MAX_LOOKUP_OPTIONS = 1000;

/** Choices for each relation field of `table`, keyed by field name. */
export async function loadLookups(table: TableDef): Promise<Record<string, LookupOption[]>> {
	const result: Record<string, LookupOption[]> = {};
	for (const f of table.fields) {
		if (f.field_type.type !== 'lookup') continue;
		const fk = table.foreign_keys?.find((k) => k.columns.length === 1 && k.columns[0] === f.name);
		const refCol = fk?.ref_columns?.[0];
		if (!fk || !refCol) continue;
		const shown = f.field_type.display_column || refCol;
		try {
			const page = await recordService.list(fk.ref_table, { limit: MAX_LOOKUP_OPTIONS });
			const vi = page.columns.indexOf(refCol);
			const li = page.columns.indexOf(shown);
			if (vi < 0) continue;
			result[f.name] = page.rows.map((r) => ({
				value: r.values[vi],
				label: displayValue(r.values[li >= 0 ? li : vi])
			}));
		} catch {
			// An unreadable related table only loses the friendly labels.
		}
	}
	return result;
}

/** Label for a stored relation value, falling back to the raw value. */
export function lookupLabel(options: LookupOption[] | undefined, value: JsonValue): string | null {
	const key = JSON.stringify(value);
	return options?.find((o) => JSON.stringify(o.value) === key)?.label ?? null;
}
