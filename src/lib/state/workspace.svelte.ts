import type { DatabaseFile, WorkspaceInfo } from '../api/types';
import { pickFolder } from '../services/dialogs';
import { workspaceService } from '../services/workspace';
import { ui } from './ui.svelte';

/** The folder of databases shown on the home screen. */
class WorkspaceState {
	info = $state<WorkspaceInfo | null>(null);
	databases = $state<DatabaseFile[]>([]);
	loaded = $state(false);

	async load() {
		await ui.attempt(async () => {
			this.info = await workspaceService.get();
			this.databases = this.info ? await workspaceService.listDatabases() : [];
		});
		this.loaded = true;
	}

	async chooseFolder() {
		await ui.attempt(async () => {
			const folder = await pickFolder();
			if (!folder) return;
			this.info = await workspaceService.set(folder);
			this.databases = await workspaceService.listDatabases();
		});
	}

	async refresh() {
		if (this.info) await ui.attempt(async () => (this.databases = await workspaceService.listDatabases()));
	}

	/** Creates a database; returns its name, or null on failure. */
	async create(name: string): Promise<string | null> {
		const created = await ui.attempt(() => workspaceService.createDatabase(name));
		return created ? name : null;
	}
}

export const workspace = new WorkspaceState();
