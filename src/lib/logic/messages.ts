/** Rewrites raw SQLite constraint errors as plain-language explanations. */
export function humanizeError(message: string): string {
	const prefix = message.match(/^((?:record|operation) #\d+: )/)?.[1] ?? '';
	const where = prefix.startsWith('operation') ? prefix : '';
	const text = message.slice(prefix.length);
	const column = (spec: string) => spec.split(',')[0].trim().split('.').pop() ?? spec;

	if (/FOREIGN KEY constraint failed/i.test(text)) {
		return `${where}This record is linked to other records. Remove or change those links first — or it points to a record that doesn't exist.`;
	}
	let m = text.match(/NOT NULL constraint failed: (.+)$/i);
	if (m) return `${where}“${column(m[1])}” must have a value.`;
	m = text.match(/UNIQUE constraint failed: (.+)$/i);
	if (m) {
		const cols = m[1].split(',').map(column);
		return `${where}Another record already has the same ${cols.map((c) => `“${c}”`).join(' and ')}.`;
	}
	if (/CHECK constraint failed/i.test(text)) {
		return `${where}A value doesn't meet one of this table's validation rules.`;
	}
	return message;
}
