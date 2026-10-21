/// <reference types="bun" />
import { afterEach, describe, expect, test } from 'bun:test';
import { ApiError, setInvoke } from './client';
import { tableService } from '../services/tables';
import { recordService } from '../services/records';

afterEach(() => setInvoke(null));

describe('command wrappers', () => {
	test('pass camelCase arguments to the matching command', async () => {
		const calls: [string, unknown][] = [];
		setInvoke(async <T>(cmd: string, args?: Record<string, unknown>) => {
			calls.push([cmd, args]);
			return undefined as T;
		});
		await tableService.rename('a', 'b');
		await recordService.batch('t', [], true);
		expect(calls).toEqual([
			['rename_table', { name: 'a', newName: 'b' }],
			['batch_records', { table: 't', ops: [], confirmDeletes: true }]
		]);
	});

	test('structured backend errors become ApiError', async () => {
		setInvoke(async () => {
			throw { kind: 'confirmation_required', message: 'confirmation required: x' };
		});
		const err = await tableService.drop('t', false).catch((e) => e);
		expect(err).toBeInstanceOf(ApiError);
		expect(err.kind).toBe('confirmation_required');
	});

	test('unexpected failures are reported as ipc errors', async () => {
		setInvoke(async () => {
			throw 'boom';
		});
		const err = await tableService.get('t').catch((e) => e);
		expect(err.kind).toBe('ipc');
		expect(err.message).toBe('boom');
	});
});
