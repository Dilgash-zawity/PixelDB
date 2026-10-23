import { describe, expect, test } from 'bun:test';
import { UpdateManager, type AvailableUpdate, type UpdateSnapshot } from './updates';

function fixture(overrides: Partial<AvailableUpdate> = {}) {
	const calls: string[] = [];
	let state: UpdateSnapshot | undefined;
	const update: AvailableUpdate = {
		version: '0.0.11',
		download: async (progress) => {
			calls.push('download');
			progress({ event: 'Started', data: { contentLength: 100 } });
			progress({ event: 'Progress', data: { chunkLength: 50 } });
		},
		install: async () => { calls.push('install'); },
		close: async () => { calls.push('close'); },
		...overrides
	};
	const manager = new UpdateManager(async () => update, async () => { calls.push('restart'); }, (next) => { state = next; });
	return { manager, calls, state: () => state! };
}

describe('signed app updates', () => {
	test('checks without downloading, and declining leaves the update available', async () => {
		const f = fixture();
		await f.manager.check();
		expect(f.state().status).toBe('available');
		await f.manager.install(async () => false);
		expect(f.calls).toEqual([]);
		expect(f.state().status).toBe('available');
	});

	test('signature or network failure prevents installation and restart', async () => {
		const f = fixture({ download: async () => { throw new Error('invalid signature'); } });
		await f.manager.check();
		await f.manager.install(async () => true);
		await f.manager.restart();
		expect(f.state().status).toBe('error');
		expect(f.calls).toEqual([]);
	});

	test('installs only once and waits for an explicit restart', async () => {
		const f = fixture();
		await f.manager.check();
		await f.manager.install(async () => true);
		await f.manager.install(async () => true);
		await f.manager.check();
		expect(f.state().status).toBe('ready');
		expect(f.calls).toEqual(['download', 'install', 'close']);
		await f.manager.restart();
		expect(f.calls.at(-1)).toBe('restart');
	});

	test('prevents a second check or install while confirmation is pending', async () => {
		const f = fixture();
		await f.manager.check();
		let approve!: (value: boolean) => void;
		const installing = f.manager.install(() => new Promise((resolve) => { approve = resolve; }));
		await f.manager.check();
		await f.manager.install(async () => true);
		expect(f.state().status).toBe('confirming');
		approve(true);
		await installing;
		expect(f.calls).toEqual(['download', 'install', 'close']);
	});

	test('offline checks report an error without claiming the app is current', async () => {
		let state: UpdateSnapshot | undefined;
		const manager = new UpdateManager(async () => { throw new Error('offline'); }, async () => {}, (next) => { state = next; });
		await manager.check();
		expect(state?.status).toBe('error');
	});
});
