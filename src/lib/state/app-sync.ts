import { readApp, renameFieldInApp, renameTableInApp, type AppDef } from '../logic/app-def';
import { appService } from '../services/apps';
import { reportError } from './feedback.svelte';

/**
 * Applies `change` to every app that uses `database` and saves the ones it
 * changed. Used to carry table and field renames made on the database page
 * into apps, so their widgets keep working.
 */
export async function updateApps(database: string, change: (app: AppDef) => boolean): Promise<void> {
	try {
		for (const file of await appService.list()) {
			if (!file.valid || !file.databases.includes(database)) continue;
			const app = readApp(await appService.load(file.name));
			if (change(app)) await appService.save(file.name, app);
		}
	} catch (e) {
		reportError(e);
	}
}

export const renameTableInApps = (database: string, from: string, to: string) =>
	updateApps(database, (app) => renameTableInApp(app, database, from, to));

export const renameFieldInApps = (database: string, table: string, from: string, to: string) =>
	updateApps(database, (app) => renameFieldInApp(app, database, table, from, to));
