import { ApiError } from '../api/client';
import type { DatabaseInfo, ObjectSummary } from '../api/types';
import { databaseService } from '../services/database';
import { confirmAction, pickExistingDatabasePath, pickNewDatabasePath } from '../services/dialogs';
import { tableService } from '../services/tables';

export type View =
	| { kind: 'none' }
	| { kind: 'datasheet'; name: string }
	| { kind: 'design'; name: string | null };

export function describeError(e: unknown): string {
	if (e instanceof ApiError) return `[${e.kind}] ${e.message}`;
	return e instanceof Error ? e.message : String(e);
}

/** Global application state shared by all components. */
class AppState {
	info = $state<DatabaseInfo | null>(null);
	objects = $state<ObjectSummary[]>([]);
	view = $state<View>({ kind: 'none' });
	error = $state<string | null>(null);
	message = $state<string | null>(null);
	busy = $state(false);

	get tables(): ObjectSummary[] {
		return this.objects.filter((o) => o.kind === 'table');
	}

	/** Runs an action, recording any error for display. */
	async run<T>(action: () => Promise<T>): Promise<T | undefined> {
		this.busy = true;
		this.error = null;
		this.message = null;
		try {
			return await action();
		} catch (e) {
			this.error = describeError(e);
			return undefined;
		} finally {
			this.busy = false;
		}
	}

	async refresh(): Promise<void> {
		this.info = await databaseService.info();
		this.objects = this.info ? await databaseService.listObjects() : [];
	}

	async createDatabase(path?: string) {
		await this.run(async () => {
			const target = path?.trim() || (await pickNewDatabasePath());
			if (!target) return;
			await databaseService.create(target);
			this.view = { kind: 'none' };
			await this.refresh();
			this.message = `Created ${target}`;
		});
	}

	async openDatabase(path?: string) {
		await this.run(async () => {
			const target = path?.trim() || (await pickExistingDatabasePath());
			if (!target) return;
			await databaseService.open(target);
			this.view = { kind: 'none' };
			await this.refresh();
			this.message = `Opened ${target}`;
		});
	}

	async closeDatabase() {
		await this.run(async () => {
			await databaseService.close();
			this.view = { kind: 'none' };
			await this.refresh();
			this.message = 'Database closed';
		});
	}

	async renameTable(name: string, newName: string) {
		await this.run(async () => {
			await tableService.rename(name, newName);
			if (this.view.kind !== 'none' && this.view.name === name) this.view = { ...this.view, name: newName };
			await this.refresh();
			this.message = `Renamed ${name} to ${newName}`;
		});
	}

	async dropTable(name: string) {
		if (!(await confirmAction(`Delete table "${name}" and all of its records? This cannot be undone.`))) return;
		await this.run(async () => {
			const result = await tableService.drop(name, true);
			if (this.view.kind !== 'none' && this.view.name === name) this.view = { kind: 'none' };
			await this.refresh();
			this.message = [`Deleted table ${name}`, ...result.warnings].join('. ');
		});
	}
}

export const app = new AppState();
