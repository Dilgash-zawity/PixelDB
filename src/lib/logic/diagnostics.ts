/** The text of "Copy details": what went wrong, where, and on which version, for a bug report. */
export interface ErrorDetails {
	title: string;
	message: string;
	kind?: string;
	/** The screen it happened on (a route, without the database name). */
	screen?: string;
	version?: string;
	os?: string;
	at?: Date;
}

export function formatDetails(d: ErrorDetails): string {
	const lines = [
		`${d.title}: ${d.message}`,
		'',
		d.kind ? `Kind: ${d.kind}` : null,
		d.screen ? `Screen: ${d.screen}` : null,
		d.version ? `SlateDB: ${d.version}` : null,
		d.os ? `System: ${d.os}` : null,
		`Time: ${(d.at ?? new Date()).toISOString()}`
	];
	return lines.filter((l) => l !== null).join('\n');
}

/** A route without its query (database and app names stay private). */
export function screenOf(pathname: string): string {
	return pathname.replace(/\/+$/, '') || '/';
}
