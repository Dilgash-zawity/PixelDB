import { readApp, renameFieldInApp, renameTableInApp, type AppDef } from '../logic/app-def';
import { syncGeneratedPages } from '../logic/app-generate';
import { appService } from '../services/apps';
import { tableService } from '../services/tables';
import { reportError } from './feedback.svelte';

/** App files are changed one update at a time, so a rename and the sync after it can't overwrite each other. */
let queue: Promise<void> = Promise.resolve();
function enqueue(work: () => Promise<void>): Promise<void> {
	queue = queue.then(work, work);
	return queue;
}

/**
 * Applies `change` to every app that uses `database` and saves the ones it
 * changed. Used to carry table and field renames made on the database page
 * into apps, so their widgets keep working.
 */
export function updateApps(database: string, change: (app: AppDef) => boolean | Promise<boolean>): Promise<void> {
	return enqueue(async () => {
		try {
			for (const file of await appService.list()) {
				if (!file.valid || !file.databases.includes(database)) continue;
				const app = readApp(await appService.load(file.name));
				if (await change(app)) await appService.save(file.name, app);
			}
		} catch (e) {
			reportError(e);
		}
	});
}

/**
 * Brings the pages of every app that updates automatically from `database`
 * in step with its tables (see `syncGeneratedPages`). Returns the number of
 * apps changed.
 */
export async function syncAutoApps(database: string): Promise<number> {
	let schema: Awaited<ReturnType<typeof tableService.listSchema>> | null = null;
	let count = 0;
	await updateApps(database, async (app) => {
		const sources = app.sources.filter((s) => s.auto && s.database === database);
		if (!sources.length) return false;
		schema ??= await tableService.listSchema(database);
		let changed = false;
		for (const s of sources) {
			const group = app.sources.length > 1 ? s.database : '';
			changed = syncGeneratedPages(app, s.alias, s.database, schema.tables, schema.views, group) || changed;
		}
		if (changed) count++;
		return changed;
	});
	return count;
}

export const renameTableInApps = (database: string, from: string, to: string) =>
	updateApps(database, (app) => renameTableInApp(app, database, from, to));

export const renameFieldInApps = (database: string, table: string, from: string, to: string) =>
	updateApps(database, (app) => renameFieldInApp(app, database, table, from, to));
