import type { ConnectionInput, Provider, TlsMode } from '../api/types';

/** The wire protocol a provider speaks; providers sharing one share a form. */
export type Engine = 'postgres' | 'mysql' | 'mssql' | 'libsql';

export interface ProviderInfo {
	id: Provider;
	name: string;
	description: string;
	engine: Engine;
	/** Port filled in for new connections (libSQL uses a URL instead). */
	port: number | null;
	tls: TlsMode;
	/** True for hosted services, shown under "Cloud services". */
	cloud: boolean;
	/** Where to find the connection details, shown on the Connection step. */
	hint: string;
	hostPlaceholder: string;
}

export const PROVIDERS: ProviderInfo[] = [
	{
		id: 'postgres',
		name: 'PostgreSQL',
		description: 'Any PostgreSQL server you can reach.',
		engine: 'postgres',
		port: 5432,
		tls: 'prefer',
		cloud: false,
		hint: 'Ask whoever runs the server for its address, a user and a password.',
		hostPlaceholder: 'db.example.com'
	},
	{
		id: 'mysql',
		name: 'MySQL',
		description: 'MySQL or MariaDB.',
		engine: 'mysql',
		port: 3306,
		tls: 'prefer',
		cloud: false,
		hint: 'Ask whoever runs the server for its address, a user and a password.',
		hostPlaceholder: 'db.example.com'
	},
	{
		id: 'sql_server',
		name: 'SQL Server',
		description: 'Microsoft SQL Server or Azure SQL.',
		engine: 'mssql',
		port: 1433,
		tls: 'require',
		cloud: false,
		hint: 'Uses SQL Server sign-in (a user and password), not Windows sign-in.',
		hostPlaceholder: 'sql.example.com'
	},
	{
		id: 'sqlite',
		name: 'SQLite server',
		description: 'A libSQL (sqld) server over HTTP.',
		engine: 'libsql',
		port: null,
		tls: 'require',
		cloud: false,
		hint: 'The server’s address, like http://localhost:8080, and its token if it needs one.',
		hostPlaceholder: ''
	},
	{
		id: 'supabase',
		name: 'Supabase',
		description: 'The Postgres database of a Supabase project.',
		engine: 'postgres',
		port: 5432,
		tls: 'require',
		cloud: true,
		hint: 'In your project, open Connect and copy the connection string (Session pooler works everywhere).',
		hostPlaceholder: 'aws-0-eu-central-1.pooler.supabase.com'
	},
	{
		id: 'neon',
		name: 'Neon',
		description: 'Serverless Postgres from Neon.',
		engine: 'postgres',
		port: 5432,
		tls: 'require',
		cloud: true,
		hint: 'On your project dashboard, click Connect and copy the connection string.',
		hostPlaceholder: 'ep-example-123456.eu-central-1.aws.neon.tech'
	},
	{
		id: 'turso',
		name: 'Turso',
		description: 'SQLite in the cloud, built on libSQL.',
		engine: 'libsql',
		port: null,
		tls: 'require',
		cloud: true,
		hint: 'Run “turso db show <name> --url” for the URL and “turso db tokens create <name>” for a token.',
		hostPlaceholder: ''
	}
];

export function providerInfo(id: Provider): ProviderInfo {
	return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[0];
}

/** A new, empty connection for `provider`. */
export function blankConnection(provider: Provider, name = ''): ConnectionInput {
	const p = providerInfo(provider);
	return {
		name,
		provider,
		host: '',
		port: p.port,
		database: p.engine === 'postgres' ? 'postgres' : '',
		user: provider === 'supabase' || provider === 'postgres' ? 'postgres' : '',
		password: '',
		url: '',
		token: '',
		tls: p.tls,
		trust_certificate: false
	};
}

const SCHEMES: Record<string, Engine> = {
	postgres: 'postgres',
	postgresql: 'postgres',
	mysql: 'mysql',
	mariadb: 'mysql',
	sqlserver: 'mssql',
	mssql: 'mssql',
	libsql: 'libsql',
	http: 'libsql',
	https: 'libsql'
};

/**
 * Fills a connection from a pasted connection string (`postgresql://user:pass@host:5432/db?sslmode=require`,
 * `mysql://…`, `sqlserver://…`, `libsql://….turso.io?authToken=…`). Returns null when it can't be read or
 * is for another kind of server.
 */
export function parseConnectionString(text: string, into: ConnectionInput): ConnectionInput | null {
	const raw = text.trim();
	const engine = providerInfo(into.provider).engine;
	const scheme = /^([a-z][a-z0-9+.-]*):\/\//i.exec(raw)?.[1].toLowerCase();
	if (!scheme || SCHEMES[scheme] !== engine) return null;
	let url: URL;
	try {
		// URL only parses the parts of special schemes, so read it as http.
		url = new URL(raw.replace(/^[^:]+:/, 'http:'));
	} catch {
		return null;
	}
	if (engine === 'libsql') {
		const token = url.searchParams.get('authToken') ?? url.searchParams.get('auth_token');
		url.search = '';
		const base = (scheme === 'libsql' ? 'libsql:' : `${scheme}:`) + url.href.slice('http:'.length).replace(/\/$/, '');
		return { ...into, url: base, token: token ?? into.token };
	}
	const decode = (s: string) => {
		try {
			return decodeURIComponent(s);
		} catch {
			return s;
		}
	};
	const next: ConnectionInput = {
		...into,
		host: decode(url.hostname),
		port: url.port ? Number(url.port) : providerInfo(into.provider).port,
		user: url.username ? decode(url.username) : into.user,
		password: url.password ? decode(url.password) : into.password
	};
	const path = decode(url.pathname.replace(/^\//, ''));
	if (path) next.database = path;
	const params = new Map([...url.searchParams].map(([k, v]) => [k.toLowerCase(), v.toLowerCase()]));
	const mode = params.get('sslmode') ?? params.get('ssl-mode') ?? params.get('ssl_mode');
	if (mode) next.tls = mode === 'disable' || mode === 'disabled' ? 'disable' : mode === 'prefer' || mode === 'preferred' || mode === 'allow' ? 'prefer' : 'require';
	if (engine === 'mssql') {
		const db = params.get('database');
		if (db) next.database = url.searchParams.get('database') ?? db;
		if (params.get('trustservercertificate') === 'true') next.trust_certificate = true;
		if (params.get('encrypt') === 'false') next.tls = 'disable';
	}
	return next;
}

/** What is missing from a connection's details, or null when it can be tried. */
export function connectionProblem(c: ConnectionInput, hasSavedSecret = false): string | null {
	const p = providerInfo(c.provider);
	if (p.engine === 'libsql') {
		if (!c.url.trim()) return 'Enter the database URL';
		if (!/^(libsql|https?):\/\/[^/\s]+/i.test(c.url.trim())) return 'The URL starts with libsql://, https:// or http://';
		if (p.cloud && !c.token.trim() && !hasSavedSecret) return 'Enter an access token';
		return null;
	}
	if (!c.host.trim()) return 'Enter the server address';
	if (/[\s/]/.test(c.host.trim())) return 'The server address is a host name like db.example.com — paste a whole connection string above instead';
	if (c.port !== null && (!Number.isInteger(c.port) || c.port < 1 || c.port > 65535)) return 'The port is a number from 1 to 65535';
	if (!c.user.trim()) return 'Enter a user name';
	if (p.engine === 'postgres' && !c.database.trim()) return 'Enter the database name';
	return null;
}

/** A short line telling which server a connection points at, e.g. `db.example.com · shop`. */
export function connectionSummary(c: { provider: Provider; host: string; database: string; url: string }): string {
	if (providerInfo(c.provider).engine === 'libsql') return c.url.replace(/^[a-z]+:\/\//i, '');
	return [c.host, c.database].filter(Boolean).join(' · ');
}
