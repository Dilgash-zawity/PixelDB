import { DATABASE_EXTENSIONS } from './database';

const filters = [{ name: 'SQLite database', extensions: DATABASE_EXTENSIONS }];

/** Asks the user to confirm a destructive action. */
export async function confirmAction(message: string): Promise<boolean> {
	try {
		const { ask } = await import('@tauri-apps/plugin-dialog');
		return await ask(message, { title: 'Confirm', kind: 'warning' });
	} catch {
		return window.confirm(message);
	}
}

/** Native "save as" dialog for a new database. Returns null when cancelled. */
export async function pickNewDatabasePath(): Promise<string | null> {
	const { save } = await import('@tauri-apps/plugin-dialog');
	return save({ title: 'Create database', defaultPath: 'database.db', filters });
}

/** Native "open" dialog for an existing database. Returns null when cancelled. */
export async function pickExistingDatabasePath(): Promise<string | null> {
	const { open } = await import('@tauri-apps/plugin-dialog');
	const picked = await open({ title: 'Open database', multiple: false, directory: false, filters });
	return typeof picked === 'string' ? picked : null;
}
