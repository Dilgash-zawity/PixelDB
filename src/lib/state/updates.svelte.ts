import { isTauri } from '@tauri-apps/api/core';
import { UpdateManager, initialUpdateSnapshot } from '../services/updates';
import { settings } from './settings.svelte';

class UpdatesState {
	current = $state({ ...initialUpdateSnapshot });
	readonly manager = new UpdateManager(
		async () => {
			const { check } = await import('@tauri-apps/plugin-updater');
			return check({ timeout: 15_000 });
		},
		async () => {
			const { relaunch } = await import('@tauri-apps/plugin-process');
			await relaunch();
		},
		(snapshot) => { this.current = snapshot; }
	);

	/** Automatic checks run only in installed builds; manual checks remain available in development. */
	start() {
		if (!isTauri() || import.meta.env.DEV) return () => {};
		const check = () => {
			if (settings.current.automaticUpdates) void this.manager.check();
		};
		check();
		const timer = setInterval(check, 6 * 60 * 60 * 1000);
		return () => clearInterval(timer);
	}
}

export const updates = new UpdatesState();
