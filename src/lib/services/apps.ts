import { call } from '../api/client';
import type { AppFile } from '../api/types';
import type { AppDef } from '../logic/app-def';

/** App definitions, stored as files in the workspace's Apps folder. */
export const appService = {
	list: () => call<AppFile[]>('list_apps'),
	create: (name: string, definition: AppDef) => call<void>('create_app', { name, definition }),
	load: (name: string) => call<unknown>('load_app', { name }),
	save: (name: string, definition: AppDef) => call<void>('save_app', { name, definition }),
	rename: (name: string, newName: string) => call<void>('rename_app', { name, newName }),
	delete: (name: string, confirm: boolean) => call<void>('delete_app', { name, confirm }),
	duplicate: (name: string, newName: string) => call<void>('duplicate_app', { name, newName })
};
