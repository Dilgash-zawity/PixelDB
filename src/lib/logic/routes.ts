/** URL of a database's page (query parameter keeps the route static). */
export function databaseHref(base: string, name: string): string {
	return `${base}?name=${encodeURIComponent(name)}`;
}

/** URL of an app's page. */
export function appHref(base: string, name: string): string {
	return `${base}?name=${encodeURIComponent(name)}`;
}
