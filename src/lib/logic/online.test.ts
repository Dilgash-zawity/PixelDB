/// <reference types="bun" />
import { describe, expect, test } from 'bun:test';
import { connectionProblems, connectionSummary, newSettings, parseConnectionUrl, suggestedName } from './online';

describe('parseConnectionUrl', () => {
	test('reads a Neon connection string', () => {
		const p = parseConnectionUrl('postgresql://neondb_owner:p%40ss@ep-cool-1234.eu-central-1.aws.neon.tech/neondb?sslmode=require');
		expect(p).toEqual({
			settings: {
				provider: 'neon',
				engine: 'postgres',
				host: 'ep-cool-1234.eu-central-1.aws.neon.tech',
				port: null,
				database: 'neondb',
				user: 'neondb_owner',
				ssl: true
			},
			secret: 'p@ss'
		});
	});

	test('keeps a non-default port and the picked provider of the same engine', () => {
		const p = parseConnectionUrl('postgres://me:pw@10.0.0.5:6543/shop', 'supabase');
		expect(p?.settings).toMatchObject({ provider: 'supabase', host: '10.0.0.5', port: 6543, database: 'shop', ssl: true });
		expect(parseConnectionUrl('postgres://me@localhost/shop', 'turso')?.settings.provider).toBe('postgres');
		expect(parseConnectionUrl('postgres://me@localhost:5432/shop?sslmode=disable')?.settings).toMatchObject({ port: null, ssl: false });
	});

	test('reads MySQL, MariaDB and PlanetScale', () => {
		expect(parseConnectionUrl('mysql://u:pw@aws.connect.psdb.cloud/app?ssl={"rejectUnauthorized":true}')?.settings).toMatchObject({
			provider: 'planetscale',
			engine: 'mysql',
			ssl: true
		});
		expect(parseConnectionUrl('mariadb://root@db.local:3307/x')?.settings).toMatchObject({ provider: 'mariadb', port: 3307, ssl: false });
	});

	test('reads a Turso URL and its token', () => {
		const p = parseConnectionUrl('libsql://notes-acme.turso.io?authToken=abc.def');
		expect(p).toEqual({
			settings: { provider: 'turso', engine: 'libsql', host: 'notes-acme.turso.io', port: null, database: '', user: '', ssl: true },
			secret: 'abc.def'
		});
		expect(suggestedName(p!.settings)).toBe('notes');
	});

	test('rejects text that is not a connection string', () => {
		for (const t of ['', 'db.example.com', 'ftp://x.com', 'postgres://']) expect(parseConnectionUrl(t)).toBeNull();
	});
});

describe('connectionProblems', () => {
	test('flags missing and malformed fields', () => {
		expect(connectionProblems(newSettings('neon'))).toEqual({ host: 'Enter the server address', user: 'Enter a user name' });
		const s = { ...newSettings('postgres'), host: 'x.com/db', port: 70000, user: 'u' };
		expect(Object.keys(connectionProblems(s))).toEqual(['host', 'port']);
		expect(connectionProblems({ ...newSettings('turso'), host: 'a.turso.io' })).toEqual({});
	});

	test('summarises a connection', () => {
		expect(connectionSummary({ ...newSettings('postgres'), host: 'h', port: 5433, user: 'u', database: 'd' })).toBe('u@h:5433/d');
	});
});
