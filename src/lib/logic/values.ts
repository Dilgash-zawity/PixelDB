import type { FieldType, JsonValue } from '../api/types';

export interface BlobSummary {
	blob: { size: number };
}

export function isBlobSummary(v: JsonValue | undefined): v is { blob: { size: number } } {
	return (
		typeof v === 'object' && v !== null && !Array.isArray(v) && 'blob' in v && typeof v.blob === 'object'
	);
}

/** Formats an integer amount of minor units as a decimal string, exactly. */
export function formatMinorUnits(minor: number, scale: number): string {
	if (!Number.isInteger(minor)) return String(minor);
	const negative = minor < 0;
	let digits = BigInt(Math.abs(minor)).toString();
	if (scale === 0) return (negative ? '-' : '') + digits;
	digits = digits.padStart(scale + 1, '0');
	const out = `${digits.slice(0, -scale)}.${digits.slice(-scale)}`;
	return negative ? `-${out}` : out;
}

/** Human-readable value for listings. */
export function displayValue(value: JsonValue | undefined, t?: FieldType): string {
	if (value === null || value === undefined) return '';
	if (isBlobSummary(value)) return `[${value.blob.size} bytes]`;
	if (t?.type === 'currency' && typeof value === 'number') return formatMinorUnits(value, t.scale ?? 2);
	if (t?.type === 'yes_no') return value === 1 || value === true ? 'Yes' : value === 0 || value === false ? 'No' : String(value);
	if (typeof value === 'object') return JSON.stringify(value);
	return String(value);
}

export type InputKind = 'text' | 'textarea' | 'checkbox' | 'file' | 'readonly';

export function inputKind(t: FieldType): InputKind {
	switch (t.type) {
		case 'yes_no':
			return 'checkbox';
		case 'long_text':
			return 'textarea';
		case 'attachment':
			return 'file';
		case 'calculated':
			return 'readonly';
		default:
			return 'text';
	}
}

/**
 * Converts a stored value to what an editor input holds. Currency becomes a
 * decimal string, which the backend parses back into minor units.
 */
export function toEditorValue(value: JsonValue | undefined, t: FieldType): JsonValue {
	if (value === undefined) return null;
	if (t.type === 'yes_no') return value === 1 || value === true;
	if (t.type === 'currency' && typeof value === 'number') return formatMinorUnits(value, t.scale ?? 2);
	return value;
}

export function placeholderFor(t: FieldType): string {
	switch (t.type) {
		case 'date_time':
			return 'YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS';
		case 'currency':
			return '0.00';
		case 'auto_number':
			return '(automatic)';
		case 'hyperlink':
			return 'https://';
		default:
			return '';
	}
}

/** Reads a File into the `{ base64 }` shape the backend accepts. */
export async function fileToAttachment(file: File): Promise<{ base64: string }> {
	const bytes = new Uint8Array(await file.arrayBuffer());
	let binary = '';
	for (let i = 0; i < bytes.length; i += 0x8000) {
		binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
	}
	return { base64: btoa(binary) };
}

/**
 * Deep copy of JSON data. Unlike `structuredClone`, this also works on
 * Svelte `$state` proxies.
 */
export function jsonClone<T>(value: T): T {
	return value === undefined ? value : JSON.parse(JSON.stringify(value));
}

export function formatBytes(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	const units = ['KB', 'MB', 'GB'];
	let n = bytes / 1024;
	let i = 0;
	while (n >= 1024 && i < units.length - 1) {
		n /= 1024;
		i++;
	}
	return `${n.toFixed(n < 10 ? 1 : 0)} ${units[i]}`;
}

/** Client-side mirror of the backend's database name rules. */
export function databaseNameProblem(name: string): string | null {
	if (!name.trim()) return 'Enter a name';
	if (name.trim() !== name) return 'Remove leading or trailing spaces';
	if (name.length > 100) return 'Use at most 100 characters';
	if (name.startsWith('.') || name.endsWith('.')) return 'Do not start or end with a dot';
	const bad = [...name].find((c) => c.charCodeAt(0) < 32 || '<>:"/\\|?*'.includes(c));
	if (bad) return `Do not use ${JSON.stringify(bad)}`;
	if (/^(con|prn|aux|nul|com\d|lpt\d)(\.|$)/i.test(name)) return 'This name is reserved by the operating system';
	return null;
}

/** Client-side mirror of the backend's table/field name rules. */
export function identifierProblem(name: string, kind = 'Name'): string | null {
	if (!name.trim()) return `${kind} is required`;
	if (name.trim() !== name) return 'Remove leading or trailing spaces';
	if (name.length > 128) return 'Use at most 128 characters';
	if (/^(sqlite_|__app_)/i.test(name)) return 'Names starting with sqlite_ or __app_ are reserved';
	if ([...name].some((c) => c.charCodeAt(0) < 32)) return 'Control characters are not allowed';
	return null;
}
