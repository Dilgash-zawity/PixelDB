import type { AppFile, ConnectionSettings, ConnectionTest, DatabaseFile, OnlineDatabase, WorkspaceInfo } from '../api/types';
import { addSource, newApp, readApp, removeSource, widgetsUsingSource } from '../logic/app-def';
import { generatePages } from '../logic/app-generate';
import type { Template } from '../logic/templates';
import { appService } from '../services/apps';
import { databaseService } from '../services/database';
import { recordService } from '../services/records';
import { tableService } from '../services/tables';
import { pickFolder } from '../services/dialogs';
import { onlineService } from '../services/online';
import { workspaceService } from '../services/workspace';
import { toast } from 'svelte-sonner';
import { attempt, confirm } from './feedback.svelte';

/** The folder of databases shown on the home screen. */
class WorkspaceState {
	info = $state<WorkspaceInfo | null>(null);
	databases = $state<DatabaseFile[]>([]);
	/** Saved connections to online databases. */
	online = $state<OnlineDatabase[]>([]);
	apps = $state<AppFile[]>([]);
	loaded = $state(false);

	async load() {
		await attempt(async () => {
			this.info = await workspaceService.get();
			this.databases = this.info ? await workspaceService.listDatabases() : [];
			this.online = this.info ? await onlineService.list() : [];
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
			this.online = await onlineService.list();
			this.apps = await appService.list();
		});
	}

	async refresh() {
		if (!this.info) return;
		await attempt(async () => {
			this.databases = await workspaceService.listDatabases();
			this.online = await onlineService.list();
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
		const users = this.apps.filter((a) => a.databases.includes(name)).map((a) => `“${a.name}”`);
		const ok = await confirm({
			title: `Delete “${name}”?`,
			description:
				'The database file and every table and record in it will be permanently deleted. This cannot be undone.' +
				(users.length ? ` ${users.join(', ')} ${users.length === 1 ? 'uses' : 'use'} it and will show no data.` : ''),
			action: 'Delete database',
			destructive: true
		});
		if (!ok) return;
		await attempt(() => workspaceService.deleteDatabase(name, true));
		await this.refresh();
	}

	/** True when another database, local or online, already uses `name` (case-insensitively). */
	nameTaken(name: string, except?: string): boolean {
		return [...this.databases, ...this.online].some((d) => d.name !== except && d.name.toLowerCase() === name.toLowerCase());
	}

	// ----- Online databases -----

	/** Saves a new connection; returns true on success. */
	async createOnline(name: string, settings: ConnectionSettings, secret: string): Promise<boolean> {
		const ok = (await attempt(() => onlineService.create(name, settings, secret || undefined))) !== undefined;
		await this.refresh();
		return ok;
	}

	/** Changes a connection; `secret` undefined keeps the saved one. Returns true on success. */
	async updateOnline(name: string, newName: string, settings: ConnectionSettings, secret?: string): Promise<boolean> {
		const ok = (await attempt(() => onlineService.update(name, newName, settings, secret))) !== undefined;
		await this.refresh();
		return ok;
	}

	/** Tries a connection; errors are thrown, not toasted, so the caller can show them in place. */
	testOnline(settings: ConnectionSettings, secret?: string, name?: string): Promise<ConnectionTest> {
		return onlineService.test(settings, secret, name);
	}

	async removeOnline(name: string) {
		const ok = await confirm({
			title: `Remove “${name}”?`,
			description: 'The saved connection and its password are removed from this computer. The online database and its data are not touched.',
			action: 'Remove connection',
			destructive: true
		});
		if (!ok) return;
		await attempt(() => onlineService.delete(name, true));
		await this.refresh();
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

	/** Creates an app over `databases`; with `build`, with a page for each of their tables. Returns true on success. */
	async createApp(name: string, databases: string[], build = false): Promise<boolean> {
		const ok = !!(await attempt(async () => {
			const app = newApp(databases);
			if (build) {
				try {
					for (const s of app.sources) {
						const group = app.sources.length > 1 ? s.database : '';
						generatePages(app, s.alias, s.database, await tableService.listDefinitions(s.database), group);
						s.auto = true;
					}
				} finally {
					await databaseService.release();
				}
			}
			await appService.create(name, app);
			return true;
		}));
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

	/** Saves app `name` with copies of its databases as one file to give to someone else. */
	async shareApp(name: string): Promise<void> {
		const file = await attempt(() => appService.exportPackage(name));
		if (!file) return;
		const online = this.apps.find((a) => a.name === name)?.databases.filter((d) => this.online.some((o) => o.name === d)) ?? [];
		toast.success(`Saved “${file}”`, {
			description: online.length
				? `Online databases (${online.join(', ')}) aren’t included; whoever opens it needs their own connection with the same name.`
				: 'Open it in SlateDB with “Open a shared app…” on the home screen.'
		});
	}

	/** Opens a shared app package into this workspace; the new app's name, or null. */
	async openSharedApp(): Promise<string | null> {
		const got = await attempt(() => appService.importPackage());
		await this.refresh();
		if (!got) return null;
		const renamed = got.databases.filter(([from, to]) => from !== to).map(([from, to]) => `“${from}” is “${to}” here`);
		toast.success(`Added the app “${got.app}”`, {
			description: [
				got.databases.length ? `With ${got.databases.length === 1 ? 'its database' : `${got.databases.length} databases`}.` : '',
				renamed.length ? `${renamed.join(', ')}, as the name was taken.` : '',
				got.online.length ? `It also uses online databases (${got.online.join(', ')}); add connections with those names.` : ''
			]
				.filter(Boolean)
				.join(' ')
		});
		return got.app;
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
