import type { DatabaseFile, WorkspaceInfo } from '../api/types';
import { pickFolder } from '../services/dialogs';
import { workspaceService } from '../services/workspace';
import { attempt } from './feedback.svelte';

/** The folder of databases shown on the home screen. */
class WorkspaceState {
	info = $state<WorkspaceInfo | null>(null);
	databases = $state<DatabaseFile[]>([]);
	loaded = $state(false);

	async load() {
		await attempt(async () => {
			this.info = await workspaceService.get();
			this.databases = this.info ? await workspaceService.listDatabases() : [];
		});
		this.loaded = true;
	}

	async chooseFolder() {
		await attempt(async () => {
			const folder = await pickFolder();
			if (!folder) return;
			this.info = await workspaceService.set(folder);
			this.databases = await workspaceService.listDatabases();
		});
	}

	async refresh() {
		if (this.info) await attempt(async () => (this.databases = await workspaceService.listDatabases()));
	}

	/** Creates a database; returns true on success. */
	async create(name: string): Promise<boolean> {
		return !!(await attempt(() => workspaceService.createDatabase(name)));
	}

	nameTaken(name: string): boolean {
		return this.databases.some((d) => d.name.toLowerCase() === name.toLowerCase());
	}
}

export const workspace = new WorkspaceState();
