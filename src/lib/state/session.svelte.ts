import { toast } from 'svelte-sonner';
import type { DatabaseInfo, ObjectSummary } from '../api/types';
import {
	hasUnsavedChanges,
	markClean,
	newQueryDraft,
	renameFieldInQuery,
	renameTableInQuery,
	type QueryDraft,
	type VisualQuery
} from '../logic/query-builder';
import { identifierProblem } from '../logic/values';
import { databaseService } from '../services/database';
import { tableService } from '../services/tables';
import { workspaceService } from '../services/workspace';
import { renameFieldInApps, renameTableInApps } from './app-sync';
import { attempt, confirm } from './feedback.svelte';

/** The tabs of the database page. */
export type Tab = 'diagram' | 'datasheet' | 'query';

/** The database open on the database page and the table being viewed. */
class Session {
	name = $state<string | null>(null);
	info = $state<DatabaseInfo | null>(null);
	objects = $state<ObjectSummary[]>([]);
	loading = $state(false);
	loadError = $state<string | null>(null);
	table = $state<string | null>(null);
	tab = $state<Tab>('diagram');
	/** The Query tab's work in progress, kept while switching tabs. */
	query = $state<QueryDraft>(newQueryDraft());
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
		this.tab = 'diagram';
		this.query = newQueryDraft();
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

	/** Opens a table in the Datasheet tab, letting the open datasheet save first. */
	async select(name: string): Promise<boolean> {
		if (name === this.table && this.tab === 'datasheet') return true;
		return this.navigate(() => {
			this.table = name;
			this.tab = 'datasheet';
		});
	}

	/** Switches tabs; leaving the Datasheet commits the row being edited first. */
	async showTab(tab: Tab): Promise<boolean> {
		if (tab === this.tab) return true;
		if (tab === 'datasheet' && !this.table) this.table = this.tables[0]?.name ?? this.views[0]?.name ?? null;
		return this.navigate(() => (this.tab = tab));
	}

	private async navigate(apply: () => void): Promise<boolean> {
		// One navigation at a time, so a guard's dialog is never opened twice.
		if (this.navigating) return false;
		this.navigating = true;
		try {
			if (this.leaveGuard && !(await this.leaveGuard())) return false;
			apply();
			return true;
		} finally {
			this.navigating = false;
		}
	}

	private navigating = false;

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

	/** Creates a table with an automatic ID and, unless `open` is false, opens it. */
	async createTable(name: string, open = true): Promise<boolean> {
		const created = await attempt(() =>
			tableService.create({
				name,
				fields: [{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true }]
			})
		);
		if (!created) return false;
		await this.refresh();
		if (!open) {
			toast.success(`Created “${created.definition.name}”`);
			return true;
		}
		await this.select(created.definition.name);
		toast.success(`Created “${created.definition.name}”`, {
			description: 'Use “Click to Add” to add fields.'
		});
		return true;
	}

	/** Applies a structure change to the Query tab's design, keeping it unchanged-looking if it was saved. */
	updateQueryDesign(change: (q: VisualQuery) => VisualQuery) {
		const clean = !hasUnsavedChanges(this.query);
		this.query.visual = change(this.query.visual);
		if (clean) markClean(this.query);
	}

	/** Called after field `from` of `table` was renamed. */
	renamedField(table: string, from: string, to: string) {
		this.updateQueryDesign((q) => renameFieldInQuery(q, table, from, to));
		if (this.name) renameFieldInApps(this.name, table, from, to);
	}

	async renameTable(name: string, newName: string): Promise<boolean> {
		if ((await attempt(() => tableService.rename(name, newName))) === undefined) return false;
		await this.refresh();
		this.updateQueryDesign((q) => renameTableInQuery(q, name, newName));
		if (this.name) renameTableInApps(this.name, name, newName);
		if (this.table === name) this.table = newName;
		this.revision++;
		return true;
	}

	/** Asks, then deletes a table. `scope` places the question above a drawer. */
	async deleteTable(name: string, scope?: string): Promise<boolean> {
		const ok = await confirm({
			title: `Delete “${name}”?`,
			description: 'The table and every record in it will be permanently deleted. This cannot be undone.',
			action: 'Delete table',
			destructive: true,
			scope
		});
		if (!ok) return false;
		const result = await attempt(() => tableService.drop(name, true));
		if (!result) return false;
		await this.refresh();
		if (this.table === name) this.table = this.tables[0]?.name ?? null;
		toast.success(`Deleted “${name}”`, { description: result.warnings.join(' ') || undefined });
		return true;
	}
}

export const session = new Session();
