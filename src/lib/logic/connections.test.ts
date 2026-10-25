import { describe, expect, test } from 'bun:test';
import { blankConnection, connectionProblem, connectionSummary, parseConnectionString } from './connections';

describe('parseConnectionString', () => {
	test('reads a Postgres URL with an encoded password and sslmode', () => {
		const c = parseConnectionString('postgresql://alex:p%40ss%2Fword@ep-cool-1.neon.tech:5433/shop?sslmode=require', blankConnection('neon'));
		expect(c).toMatchObject({ host: 'ep-cool-1.neon.tech', port: 5433, user: 'alex', password: 'p@ss/word', database: 'shop', tls: 'require' });
	});

	test('keeps the default port and database when the URL has none', () => {
		const c = parseConnectionString('postgres://postgres.abc:secret@aws-0-eu.pooler.supabase.com', blankConnection('supabase'));
		expect(c).toMatchObject({ host: 'aws-0-eu.pooler.supabase.com', port: 5432, database: 'postgres', user: 'postgres.abc' });
	});

	test('refuses a URL for another kind of server', () => {
		expect(parseConnectionString('mysql://u@h/db', blankConnection('postgres'))).toBeNull();
		expect(parseConnectionString('not a url', blankConnection('mysql'))).toBeNull();
	});

	test('reads MySQL ssl-mode and SQL Server options', () => {
		expect(parseConnectionString('mysql://root@localhost:3307/app?ssl-mode=DISABLED', blankConnection('mysql'))).toMatchObject({ port: 3307, tls: 'disable', database: 'app' });
		const s = parseConnectionString('sqlserver://sa:pw@sql.local:1444?database=Sales&trustServerCertificate=true', blankConnection('sql_server'));
		expect(s).toMatchObject({ host: 'sql.local', port: 1444, database: 'Sales', trust_certificate: true, password: 'pw' });
	});

	test('splits a Turso token out of the URL', () => {
		const c = parseConnectionString('libsql://shop-alex.turso.io?authToken=abc.def', blankConnection('turso'));
		expect(c).toMatchObject({ url: 'libsql://shop-alex.turso.io', token: 'abc.def' });
	});
});

describe('connectionProblem', () => {
	test('asks for what is missing', () => {
		expect(connectionProblem(blankConnection('postgres'))).toBe('Enter the server address');
		expect(connectionProblem({ ...blankConnection('mysql'), host: 'h' })).toBe('Enter a user name');
		expect(connectionProblem({ ...blankConnection('mysql'), host: 'h', user: 'u', port: 70000 })).toContain('port');
		expect(connectionProblem({ ...blankConnection('postgres'), host: 'h' })).toBeNull();
		expect(connectionProblem({ ...blankConnection('turso'), url: 'libsql://x.turso.io' })).toBe('Enter an access token');
		expect(connectionProblem({ ...blankConnection('turso'), url: 'libsql://x.turso.io' }, true)).toBeNull();
		expect(connectionProblem({ ...blankConnection('sqlite'), url: 'ftp://x' })).toContain('libsql://');
	});
});

test('connectionSummary', () => {
	expect(connectionSummary({ provider: 'mysql', host: 'h', database: 'shop', url: '' })).toBe('h · shop');
	expect(connectionSummary({ provider: 'turso', host: '', database: '', url: 'libsql://a.turso.io' })).toBe('a.turso.io');
});
