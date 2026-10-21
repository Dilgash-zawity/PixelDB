import { toast } from 'svelte-sonner';
import type { DatabaseInfo, ObjectSummary } from '../api/types';
import { identifierProblem } from '../logic/values';
import { databaseService } from '../services/database';
import { tableService } from '../services/tables';
import { workspaceService } from '../services/workspace';
import { attempt, confirm } from './feedback.svelte';

export type TableView = 'records' | 'fields';

/** The database open on the database page and the table being viewed. */
class Session {
	name = $state<string | null>(null);
	info = $state<DatabaseInfo | null>(null);
	objects = $state<ObjectSummary[]>([]);
	loading = $state(false);
	loadError = $state<string | null>(null);
	table = $state<string | null>(null);
	view = $state<TableView>('records');
	/** Bumped whenever the open table's structure changes elsewhere. */
	revision = $state(0);
	/** One-shot requests from the command palette, read by the open view. */
	intent = $state<{ kind: 'none' | 'new-record' | 'new-field'; n: number }>({ kind: 'none', n: 0 });

	request(kind: 'new-record' | 'new-field') {
		this.intent = { kind, n: this.intent.n + 1 };
	}

	tables = $derived(this.objects.filter((o) => o.kind === 'table'));
	views = $derived(this.objects.filter((o) => o.kind === 'view'));
	current = $derived(this.objects.find((o) => o.name === this.table) ?? null);

	async open(name: string) {
		this.name = name;
		this.info = null;
		this.objects = [];
		this.table = null;
		this.loadError = null;
		this.loading = true;
		try {
			this.info = await workspaceService.openDatabase(name);
			await this.refresh();
			this.table = this.tables[0]?.name ?? this.views[0]?.name ?? null;
		} catch (e) {
			this.loadError = e instanceof Error ? e.message : String(e);
		} finally {
			this.loading = false;
		}
	}

	async close(): Promise<boolean> {
		if (this.leaveGuard && !(await this.leaveGuard())) return false;
		await attempt(() => databaseService.close());
		this.name = null;
		this.info = null;
		this.objects = [];
		this.table = null;
		return true;
	}

	async refresh() {
		await attempt(async () => {
			this.objects = await databaseService.listObjects();
			this.info = await databaseService.info();
		});
	}

	/**
	 * Called before leaving the open datasheet or design. Returns false to stay
	 * (e.g. a record that fails validation, or the user keeps their changes).
	 */
	leaveGuard: (() => Promise<boolean>) | null = null;

	/** Opens a table (and view), letting the current view save or cancel first. */
	async select(name: string, view: TableView = this.view): Promise<boolean> {
		const next = this.isView(name) ? 'records' : view;
		if (name === this.table && next === this.view) return true;
		// One navigation at a time, so a guard's dialog is never opened twice.
		if (this.navigating) return false;
		this.navigating = true;
		try {
			if (this.leaveGuard && !(await this.leaveGuard())) return false;
			this.table = name;
			this.view = next;
			return true;
		} finally {
			this.navigating = false;
		}
	}

	private navigating = false;

	async setView(view: TableView): Promise<boolean> {
		return this.table ? this.select(this.table, view) : false;
	}

	isView(name: string | null): boolean {
		return !!name && this.views.some((v) => v.name === name);
	}

	/** Problem with a proposed table name, or null when it is fine. */
	tableNameProblem(name: string, except?: string): string | null {
		return (
			identifierProblem(name, 'Name') ??
			(this.objects.some((o) => o.name !== except && o.name.toLowerCase() === name.toLowerCase())
				? 'A table with this name already exists'
				: null)
		);
	}

	/** Creates a table with an automatic ID and opens its fields. */
	async createTable(name: string): Promise<boolean> {
		const created = await attempt(() =>
			tableService.create({
				name,
				fields: [{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true }]
			})
		);
		if (!created) return false;
		await this.refresh();
		await this.select(created.definition.name, 'records');
		toast.success(`Created “${created.definition.name}”`, {
			description: 'Use “Click to Add” to add fields, or switch to Design view.'
		});
		return true;
	}

	async renameTable(name: string, newName: string): Promise<boolean> {
		if ((await attempt(() => tableService.rename(name, newName))) === undefined) return false;
		await this.refresh();
		if (this.table === name) this.table = newName;
		this.revision++;
		return true;
	}

	async deleteTable(name: string) {
		const ok = await confirm({
			title: `Delete “${name}”?`,
			description: 'The table and every record in it will be permanently deleted. This cannot be undone.',
			action: 'Delete table',
			destructive: true
		});
		if (!ok) return;
		const result = await attempt(() => tableService.drop(name, true));
		if (!result) return;
		await this.refresh();
		if (this.table === name) this.table = this.tables[0]?.name ?? null;
		toast.success(`Deleted “${name}”`, { description: result.warnings.join(' ') || undefined });
	}
}

export const session = new Session();
