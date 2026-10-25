import type { ConnectionSettings, Engine } from '../api/types';

/** A kind of online database: a plain server or a hosting service. Mirrors `PROVIDERS` in online.rs. */
export interface Provider {
	id: string;
	name: string;
	engine: Engine;
	/** Plain servers are "database"; hosting services are "service". */
	group: 'database' | 'service';
	description: string;
	port: number;
	/** An example address, shown as the host placeholder. */
	host: string;
	database: string;
	user: string;
	/** Whether the service only accepts encrypted connections. */
	sslRequired: boolean;
	/** What the secret is called. */
	secretLabel: string;
	/** Where to find the connection details at that service. */
	hint: string;
}

export const ENGINE_NAMES: Record<Engine, string> = { postgres: 'PostgreSQL', mysql: 'MySQL', libsql: 'SQLite (libSQL)' };

export const PROVIDERS: Provider[] = [
	{
		id: 'postgres',
		name: 'PostgreSQL',
		engine: 'postgres',
		group: 'database',
		description: 'Any PostgreSQL server, hosted or your own.',
		port: 5432,
		host: 'db.example.com',
		database: 'postgres',
		user: 'postgres',
		sslRequired: false,
		secretLabel: 'Password',
		hint: 'Ask whoever runs the server for its address, a user name and password.'
	},
	{
		id: 'mysql',
		name: 'MySQL',
		engine: 'mysql',
		group: 'database',
		description: 'Any MySQL server.',
		port: 3306,
		host: 'db.example.com',
		database: '',
		user: 'root',
		sslRequired: false,
		secretLabel: 'Password',
		hint: 'Ask whoever runs the server for its address, a user name and password.'
	},
	{
		id: 'mariadb',
		name: 'MariaDB',
		engine: 'mysql',
		group: 'database',
		description: 'Any MariaDB server.',
		port: 3306,
		host: 'db.example.com',
		database: '',
		user: 'root',
		sslRequired: false,
		secretLabel: 'Password',
		hint: 'Ask whoever runs the server for its address, a user name and password.'
	},
	{
		id: 'libsql',
		name: 'SQLite (libSQL)',
		engine: 'libsql',
		group: 'database',
		description: 'A libSQL server (sqld) serving SQLite over HTTPS.',
		port: 443,
		host: 'sqld.example.com',
		database: '',
		user: '',
		sslRequired: true,
		secretLabel: 'Auth token',
		hint: 'The server address and, if it asks for one, an auth token.'
	},
	{
		id: 'neon',
		name: 'Neon',
		engine: 'postgres',
		group: 'service',
		description: 'Serverless Postgres with branching.',
		port: 5432,
		host: 'ep-name-123456.eu-central-1.aws.neon.tech',
		database: 'neondb',
		user: 'neondb_owner',
		sslRequired: true,
		secretLabel: 'Password',
		hint: 'In the Neon console, open your project and press Connect. Copy the connection string and paste it below.'
	},
	{
		id: 'supabase',
		name: 'Supabase',
		engine: 'postgres',
		group: 'service',
		description: 'Postgres with auth, storage and a dashboard.',
		port: 5432,
		host: 'aws-0-eu-central-1.pooler.supabase.com',
		database: 'postgres',
		user: 'postgres.yourproject',
		sslRequired: true,
		secretLabel: 'Database password',
		hint: 'In Supabase, press Connect at the top of your project and copy the Session pooler URI.'
	},
	{
		id: 'turso',
		name: 'Turso',
		engine: 'libsql',
		group: 'service',
		description: 'SQLite in the cloud, built on libSQL.',
		port: 443,
		host: 'mydb-yourorg.turso.io',
		database: '',
		user: '',
		sslRequired: true,
		secretLabel: 'Auth token',
		hint: 'Run “turso db show <name> --url” for the URL and “turso db tokens create <name>” for a token, or copy both from the Turso dashboard.'
	},
	{
		id: 'planetscale',
		name: 'PlanetScale',
		engine: 'mysql',
		group: 'service',
		description: 'Serverless MySQL on Vitess.',
		port: 3306,
		host: 'aws.connect.psdb.cloud',
		database: '',
		user: '',
		sslRequired: true,
		secretLabel: 'Password',
		hint: 'In PlanetScale, open your database, press Connect and create a password; copy the connection details.'
	},
	{
		id: 'cockroachdb',
		name: 'CockroachDB',
		engine: 'postgres',
		group: 'service',
		description: 'Distributed SQL that speaks Postgres.',
		port: 26257,
		host: 'free-tier.gcp-us-central1.cockroachlabs.cloud',
		database: 'defaultdb',
		user: '',
		sslRequired: true,
		secretLabel: 'Password',
		hint: 'In CockroachDB Cloud, open your cluster and press Connect; copy the connection string.'
	}
];

export function provider(id: string): Provider | undefined {
	return PROVIDERS.find((p) => p.id === id);
}

/** A blank connection to fill in for `id`. */
export function newSettings(id: string): ConnectionSettings {
	const p = provider(id) ?? PROVIDERS[0];
	return { provider: p.id, engine: p.engine, host: '', port: null, database: p.database, user: '', ssl: p.sslRequired };
}

/** The plain-server provider of each engine. */
const GENERIC: Record<Engine, string> = { postgres: 'postgres', mysql: 'mysql', libsql: 'libsql' };

const SCHEMES: Record<string, Engine> = {
	postgres: 'postgres',
	postgresql: 'postgres',
	mysql: 'mysql',
	mariadb: 'mysql',
	libsql: 'libsql',
	https: 'libsql',
	wss: 'libsql'
};

/** Guesses a hosting service from its address. */
export function providerForHost(host: string, engine: Engine): string {
	const h = host.toLowerCase();
	const suffixes: [string, string][] = [
		['.neon.tech', 'neon'],
		['.supabase.co', 'supabase'],
		['.supabase.com', 'supabase'],
		['.turso.io', 'turso'],
		['.psdb.cloud', 'planetscale'],
		['.cockroachlabs.cloud', 'cockroachdb']
	];
	const found = suffixes.find(([s]) => h.endsWith(s))?.[1];
	return found && provider(found)?.engine === engine ? found : GENERIC[engine];
}

export interface ParsedUrl {
	settings: ConnectionSettings;
	/** Password or auth token found in the URL. */
	secret: string;
}

/**
 * Reads a connection string such as `postgresql://user:pw@host/db?sslmode=require`,
 * `mysql://…` or `libsql://db-org.turso.io?authToken=…`. Null when it isn't one.
 */
export function parseConnectionUrl(text: string, current?: string): ParsedUrl | null {
	const raw = text.trim();
	const scheme = /^([a-z][a-z0-9+.-]*):\/\//i.exec(raw)?.[1].toLowerCase();
	const engine = scheme && SCHEMES[scheme];
	if (!engine) return null;
	let url: URL;
	try {
		// Parse as a non-special scheme so every kind splits the same way.
		url = new URL(raw.replace(/^[^:]+:/, 'x:'));
	} catch {
		return null;
	}
	if (!url.hostname) return null;
	const decode = (s: string) => {
		try {
			return decodeURIComponent(s);
		} catch {
			return s;
		}
	};
	const params = url.searchParams;
	const guessed = providerForHost(url.hostname, engine);
	// A hosting service recognised from the address wins, then what the user picked.
	const picked = current && provider(current)?.engine === engine ? current : undefined;
	const id = guessed !== GENERIC[engine] ? guessed : (picked ?? (scheme === 'mariadb' ? 'mariadb' : guessed));
	const sslmode = (params.get('sslmode') ?? params.get('ssl-mode') ?? '').toLowerCase();
	const insecure = ['disable', 'disabled', 'false', 'allow', 'prefer', 'preferred'];
	const ssl = engine === 'libsql' || (sslmode ? !insecure.includes(sslmode) : !!provider(id)?.sslRequired);
	const port = url.port ? Number(url.port) : null;
	return {
		settings: {
			provider: id,
			engine,
			host: url.hostname,
			port: port && port !== provider(id)?.port ? port : null,
			database: engine === 'libsql' ? '' : decode(url.pathname.replace(/^\//, '')),
			user: engine === 'libsql' ? '' : decode(url.username),
			ssl
		},
		secret: engine === 'libsql' ? (params.get('authToken') ?? params.get('auth_token') ?? '') : decode(url.password)
	};
}

/** Problems with a connection, by field (`host`, `port`, `user`). Mirrors `ConnectionSettings::validate`. */
export function connectionProblems(s: ConnectionSettings): Partial<Record<'host' | 'port' | 'user', string>> {
	const out: Partial<Record<'host' | 'port' | 'user', string>> = {};
	const host = s.host.trim();
	if (!host) out.host = 'Enter the server address';
	else if (host.length > 253 || /[\s/\\?#@]/.test(host)) out.host = 'Enter just the address, like db.example.com';
	if (s.port !== null && (!Number.isInteger(s.port) || s.port < 1 || s.port > 65535)) out.port = 'Use a port between 1 and 65535';
	if (s.engine !== 'libsql' && !s.user.trim()) out.user = 'Enter a user name';
	return out;
}

/** A one-line summary for cards: `user@host/database`. */
export function connectionSummary(s: ConnectionSettings): string {
	const user = s.user ? `${s.user}@` : '';
	const db = s.database ? `/${s.database}` : '';
	return `${user}${s.host}${s.port ? `:${s.port}` : ''}${db}`;
}

/** A name for a new connection taken from its address, e.g. "mydb" from mydb-org.turso.io. */
export function suggestedName(s: ConnectionSettings): string {
	if (s.database && !['postgres', 'neondb', 'defaultdb'].includes(s.database)) return s.database;
	const first = s.host.split('.')[0] ?? '';
	return s.provider === 'turso' ? first.split('-')[0] || first : first;
}

/** Something a kind of online database does differently from a local one. */
export interface EngineFeature {
	id: 'shared' | 'edit' | 'schemas' | 'server-types' | 'plans' | 'timed-plans' | 'edge';
	label: string;
	description: string;
}

/** What a database on `engine` offers that a local one doesn't. */
export function engineFeatures(engine: Engine): EngineFeature[] {
	const out: EngineFeature[] = [
		{ id: 'shared', label: 'Shared', description: 'Everyone and every app connected to the server sees the same data, straight away.' },
		{ id: 'edit', label: 'Edit rows', description: 'Rows of tables with a primary key can be added, changed and deleted here and from apps.' }
	];
	if (engine === 'postgres')
		out.push({ id: 'schemas', label: 'Schemas', description: 'Tables are grouped by schema; tables outside public are named schema.table.' });
	if (engine !== 'libsql')
		out.push({ id: 'server-types', label: 'Server types', description: engine === 'postgres' ? 'JSON, UUIDs, arrays, enums and exact decimals are kept as the server stores them.' : 'Exact decimals, enums, sets and JSON are kept as the server stores them.' });
	out.push({ id: 'plans', label: 'Query plans', description: 'See how the server would run a query before you rely on it.' });
	if (engine === 'postgres')
		out.push({ id: 'timed-plans', label: 'Timed plans', description: 'Run a query read-only and see where the time goes (EXPLAIN ANALYZE).' });
	if (engine === 'libsql')
		out.push({ id: 'edge', label: 'SQLite over HTTPS', description: 'The same SQLite as a local database, served from the edge.' });
	return out;
}

/** What only local databases have; online structure is changed with the server's own tools. */
export const LOCAL_ONLY = ['Designing tables, fields and links', 'Saving queries', 'Files in fields'];

/** Whether the server can time a query plan by running it. */
export const canTimePlans = (engine: Engine): boolean => engine === 'postgres';

export interface SchemaGroup<T> {
	/** Empty for the default schema. */
	schema: string;
	items: { label: string; item: T }[];
}

/**
 * Groups listed objects by schema: `sales.orders` is "orders" under "sales"
 * when `sales` is one of `schemas`; anything else is in the default group,
 * which comes first.
 */
export function groupBySchema<T extends { name: string }>(objects: T[], schemas: string[]): SchemaGroup<T>[] {
	const groups = new Map<string, SchemaGroup<T>>([['', { schema: '', items: [] }]]);
	for (const o of objects) {
		const dot = o.name.indexOf('.');
		const schema = dot > 0 && schemas.includes(o.name.slice(0, dot)) ? o.name.slice(0, dot) : '';
		if (!groups.has(schema)) groups.set(schema, { schema, items: [] });
		groups.get(schema)!.items.push({ label: schema ? o.name.slice(dot + 1) : o.name, item: o });
	}
	return [...groups.values()].filter((g) => g.items.length);
}

/** A listed name written for SQL on `engine`: `sales.orders` becomes `"sales"."orders"` when `sales` is a schema. */
export function sqlName(engine: Engine, name: string, schemas: string[]): string {
	const q = (s: string) => (engine === 'mysql' ? `\`${s.replaceAll('`', '``')}\`` : `"${s.replaceAll('"', '""')}"`);
	const dot = name.indexOf('.');
	if (dot > 0 && schemas.includes(name.slice(0, dot))) return `${q(name.slice(0, dot))}.${q(name.slice(dot + 1))}`;
	return q(name);
}
