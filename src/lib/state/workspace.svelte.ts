import type { AppFile, DatabaseFile, WorkspaceInfo } from '../api/types';
import { addSource, newApp, readApp, removeSource, widgetsUsingSource } from '../logic/app-def';
import type { Template } from '../logic/templates';
import { appService } from '../services/apps';
import { databaseService } from '../services/database';
import { recordService } from '../services/records';
import { tableService } from '../services/tables';
import { pickFolder } from '../services/dialogs';
import { workspaceService } from '../services/workspace';
import { attempt, confirm } from './feedback.svelte';

/** The folder of databases shown on the home screen. */
class WorkspaceState {
	info = $state<WorkspaceInfo | null>(null);
	databases = $state<DatabaseFile[]>([]);
	apps = $state<AppFile[]>([]);
	loaded = $state(false);

	async load() {
		await attempt(async () => {
			this.info = await workspaceService.get();
			this.databases = this.info ? await workspaceService.listDatabases() : [];
			this.apps = this.info ? await appService.list() : [];
		});
		this.loaded = true;
	}

	async chooseFolder() {
		await attempt(async () => {
			const folder = await pickFolder();
			if (!folder) return;
			this.info = await workspaceService.set(folder);
			this.databases = await workspaceService.listDatabases();
			this.apps = await appService.list();
		});
	}

	async refresh() {
		if (!this.info) return;
		await attempt(async () => {
			this.databases = await workspaceService.listDatabases();
			this.apps = await appService.list();
		});
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
	copyName(name: string, taken = (n: string) => this.nameTaken(n)): string {
		let candidate = `${name} copy`;
		for (let n = 2; taken(candidate); n++) candidate = `${name} copy ${n}`;
		return candidate;
	}

	// ----- Apps -----

	/** True when another app already uses `name` (case-insensitively). */
	appNameTaken(name: string, except?: string): boolean {
		return this.apps.some((a) => a.name !== except && a.name.toLowerCase() === name.toLowerCase());
	}

	/** Creates an app over `databases`; returns true on success. */
	async createApp(name: string, databases: string[]): Promise<boolean> {
		const ok = (await attempt(() => appService.create(name, newApp(databases)))) !== undefined;
		await this.refresh();
		return ok;
	}

	/**
	 * Creates a database from a template (tables and sample records) and an
	 * app over it, both called `name`. Returns true on success.
	 */
	async createFromTemplate(template: Template, name: string): Promise<boolean> {
		const ok = await attempt(async () => {
			// Creating the database makes it the open one, which creating tables needs.
			await workspaceService.createDatabase(name);
			try {
				for (const table of template.tables) await tableService.create(table);
				for (const table of template.tables) {
					for (const values of template.rows[table.name] ?? []) await recordService.insert(table.name, values, name);
				}
			} finally {
				await databaseService.close();
			}
			await appService.create(name, template.app(name));
			return true;
		});
		await this.refresh();
		return !!ok;
	}

	async renameApp(name: string, newName: string): Promise<boolean> {
		if (name === newName) return true;
		const ok = (await attempt(() => appService.rename(name, newName))) !== undefined;
		await this.refresh();
		return ok;
	}

	async duplicateApp(name: string, newName: string): Promise<boolean> {
		const ok = (await attempt(() => appService.duplicate(name, newName))) !== undefined;
		await this.refresh();
		return ok;
	}

	/** Adds `database` to an app's sources; returns true on success. */
	async connectApp(name: string, database: string): Promise<boolean> {
		const ok = await attempt(async () => {
			const app = readApp(await appService.load(name));
			if (app.sources.some((s) => s.database === database)) return true;
			addSource(app, database);
			await appService.save(name, app);
			return true;
		});
		await this.refresh();
		return !!ok;
	}

	/** Removes `database` from an app's sources after confirming; widgets reading it lose their data. */
	async disconnectApp(name: string, database: string): Promise<boolean> {
		const app = await attempt(async () => readApp(await appService.load(name)));
		if (!app) return false;
		const aliases = app.sources.filter((s) => s.database === database).map((s) => s.alias);
		const used = aliases.reduce((n, a) => n + widgetsUsingSource(app, a).length, 0);
		const ok = await confirm({
			title: `Disconnect “${database}” from “${name}”?`,
			description: used
				? `${used} widget${used === 1 ? ' reads' : 's read'} from it and will need new data. The database itself is not changed.`
				: 'The database itself is not changed.',
			action: 'Disconnect',
			destructive: true
		});
		if (!ok) return false;
		for (const a of aliases) removeSource(app, a);
		const saved = (await attempt(() => appService.save(name, app))) !== undefined;
		await this.refresh();
		return saved;
	}

	async removeApp(name: string) {
		const ok = await confirm({
			title: `Delete “${name}”?`,
			description: 'The app and its pages will be permanently deleted. Its databases and their records are kept.',
			action: 'Delete app',
			destructive: true
		});
		if (!ok) return;
		await attempt(() => appService.delete(name, true));
		await this.refresh();
	}
}

export const workspace = new WorkspaceState();
