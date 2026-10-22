import type { DatabaseFile, WorkspaceInfo } from '../api/types';
import { pickFolder } from '../services/dialogs';
import { workspaceService } from '../services/workspace';
import { attempt, confirm } from './feedback.svelte';

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

	/** Renames a database file; returns true on success. */
	async rename(name: string, newName: string): Promise<boolean> {
		if (name === newName) return true;
		const ok = (await attempt(() => workspaceService.renameDatabase(name, newName))) !== undefined;
		await this.refresh();
		return ok;
	}

	/** Copies a database under a new name; returns true on success. */
	async duplicate(name: string, newName: string): Promise<boolean> {
		const ok = (await attempt(() => workspaceService.duplicateDatabase(name, newName))) !== undefined;
		await this.refresh();
		return ok;
	}

	async remove(name: string) {
		const ok = await confirm({
			title: `Delete “${name}”?`,
			description: 'The database file and every table and record in it will be permanently deleted. This cannot be undone.',
			action: 'Delete database',
			destructive: true
		});
		if (!ok) return;
		await attempt(() => workspaceService.deleteDatabase(name, true));
		await this.refresh();
	}

	/** True when another database already uses `name` (case-insensitively). */
	nameTaken(name: string, except?: string): boolean {
		return this.databases.some((d) => d.name !== except && d.name.toLowerCase() === name.toLowerCase());
	}

	/** First free "<name> copy", "<name> copy 2", … */
	copyName(name: string): string {
		let candidate = `${name} copy`;
		for (let n = 2; this.nameTaken(candidate); n++) candidate = `${name} copy ${n}`;
		return candidate;
	}
}

export const workspace = new WorkspaceState();
