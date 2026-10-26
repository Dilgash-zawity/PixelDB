import { call } from '../api/client';
import type { AppFile, ImportedPackage } from '../api/types';
import type { AppDef } from '../logic/app-def';

/** App definitions, stored as files in the workspace's Apps folder. */
export const appService = {
	list: () => call<AppFile[]>('list_apps'),
	create: (name: string, definition: AppDef) => call<void>('create_app', { name, definition }),
	load: (name: string) => call<unknown>('load_app', { name }),
	save: (name: string, definition: AppDef) => call<void>('save_app', { name, definition }),
	rename: (name: string, newName: string) => call<void>('rename_app', { name, newName }),
	delete: (name: string, confirm: boolean) => call<void>('delete_app', { name, confirm }),
	duplicate: (name: string, newName: string) => call<void>('duplicate_app', { name, newName }),
	/** Saves the app and copies of its local databases as one package file (asks where); the file name, or null when cancelled. */
	exportPackage: (name: string) => call<string | null>('export_app_package', { name }),
	/** Asks for a package file and adds its app and databases under free names; null when cancelled. */
	importPackage: () => call<ImportedPackage | null>('import_app_package')
};
