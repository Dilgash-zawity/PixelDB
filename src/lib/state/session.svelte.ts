import type { DatabaseInfo, ObjectSummary } from '../api/types';
import { nextFreeName } from '../logic/table-draft';
import { identifierProblem } from '../logic/values';
import { databaseService } from '../services/database';
import { tableService } from '../services/tables';
import { workspaceService } from '../services/workspace';
import { ui } from './ui.svelte';

export type DocKind = 'datasheet' | 'design';

/** An open document tab. `name` is null for a table design not yet saved. */
export interface Doc {
	id: string;
	kind: DocKind;
	name: string | null;
	dirty: boolean;
}

/** Ribbon commands a document supports; absent handlers disable the button. */
export interface DocHandlers {
	save?: () => Promise<boolean>;
	refresh?: () => void;
	newRecord?: () => void;
	deleteRecord?: () => void;
	sortAscending?: () => void;
	sortDescending?: () => void;
	clearSort?: () => void;
	toggleFilter?: () => void;
	find?: () => void;
	primaryKey?: () => void;
	insertRow?: () => void;
	deleteRow?: () => void;
	propertySheet?: () => void;
}

let untitled = 0;

function docId(kind: DocKind, name: string | null): string {
	return name === null ? `design:new-${++untitled}` : `${kind}:${name.toLowerCase()}`;
}

/** The database open on the database page, with its document tabs. */
class Session {
	name = $state<string | null>(null);
	info = $state<DatabaseInfo | null>(null);
	objects = $state<ObjectSummary[]>([]);
	docs = $state<Doc[]>([]);
	activeId = $state<string | null>(null);
	handlers = $state<Record<string, DocHandlers>>({});
	loadError = $state<string | null>(null);

	active = $derived(this.docs.find((d) => d.id === this.activeId) ?? null);
	activeHandlers = $derived<DocHandlers>((this.activeId && this.handlers[this.activeId]) || {});
	tables = $derived(this.objects.filter((o) => o.kind === 'table'));
	views = $derived(this.objects.filter((o) => o.kind === 'view'));

	async open(name: string) {
		this.reset();
		this.name = name;
		try {
			this.info = await workspaceService.openDatabase(name);
			await this.refreshObjects();
		} catch (e) {
			this.loadError = e instanceof Error ? e.message : String(e);
		}
	}

	reset() {
		this.name = null;
		this.info = null;
		this.objects = [];
		this.docs = [];
		this.activeId = null;
		this.handlers = {};
		this.loadError = null;
	}

	/** Asks about unsaved work, then closes the database. */
	async close(): Promise<boolean> {
		const dirty = this.docs.filter((d) => d.dirty);
		if (
			dirty.length &&
			!(await ui.confirm({
				title: 'Unsaved changes',
				message: `${dirty.length} open object(s) have unsaved changes that will be lost.`,
				confirmLabel: 'Discard and close',
				danger: true
			}))
		) {
			return false;
		}
		await ui.attempt(() => databaseService.close());
		this.reset();
		return true;
	}

	async refreshObjects() {
		await ui.attempt(async () => {
			this.objects = await databaseService.listObjects();
			this.info = await databaseService.info();
		});
	}

	isView(name: string): boolean {
		return this.views.some((v) => v.name.toLowerCase() === name.toLowerCase());
	}

	openDoc(kind: DocKind, name: string | null) {
		if (name !== null) {
			const same = this.docs.find((d) => d.name?.toLowerCase() === name.toLowerCase());
			if (same && same.kind === kind) {
				this.activeId = same.id;
				return;
			}
			if (same) {
				this.switchView(same.id, kind);
				return;
			}
		}
		const doc: Doc = { id: docId(kind, name), kind, name, dirty: false };
		this.docs.push(doc);
		this.activeId = doc.id;
	}

	/** Toggles a tab between Datasheet and Design view. */
	async switchView(id: string, kind: DocKind) {
		const doc = this.docs.find((d) => d.id === id);
		if (!doc || doc.kind === kind || doc.name === null) return;
		if (doc.dirty) {
			const handlers = this.handlers[id];
			const saved = await ui.confirm({
				title: 'Save changes?',
				message: `Save changes to "${doc.name}" before switching views?`,
				confirmLabel: 'Save'
			});
			if (!saved || !(await handlers?.save?.())) return;
		}
		const replacement: Doc = { id: docId(kind, doc.name), kind, name: doc.name, dirty: false };
		this.docs = this.docs.map((d) => (d.id === id ? replacement : d));
		this.activeId = replacement.id;
	}

	async closeDoc(id: string) {
		const doc = this.docs.find((d) => d.id === id);
		if (!doc) return;
		if (
			doc.dirty &&
			!(await ui.confirm({
				title: 'Unsaved changes',
				message: `Close "${doc.name ?? 'new table'}" and discard unsaved changes?`,
				confirmLabel: 'Discard',
				danger: true
			}))
		) {
			return;
		}
		const index = this.docs.indexOf(doc);
		this.docs = this.docs.filter((d) => d.id !== id);
		if (this.activeId === id) this.activeId = this.docs[Math.min(index, this.docs.length - 1)]?.id ?? null;
	}

	setDirty(id: string, dirty: boolean) {
		const doc = this.docs.find((d) => d.id === id);
		if (doc && doc.dirty !== dirty) doc.dirty = dirty;
	}

	/** After saving a new table design, the tab takes the table's name. */
	adoptName(id: string, name: string) {
		const doc = this.docs.find((d) => d.id === id);
		if (doc) doc.name = name;
	}

	registerHandlers(id: string, handlers: DocHandlers) {
		this.handlers[id] = handlers;
	}

	unregisterHandlers(id: string) {
		delete this.handlers[id];
	}

	private takenNames(except?: string): string[] {
		return this.objects.map((o) => o.name).filter((n) => n !== except);
	}

	/** Validates a new table name against the rules and existing objects. */
	tableNameProblem(name: string, except?: string): string | null {
		return (
			identifierProblem(name, 'Table name') ??
			(this.takenNames(except).some((n) => n.toLowerCase() === name.toLowerCase())
				? 'An object with this name already exists'
				: null)
		);
	}

	/** Create > Table: a new table with an AutoNumber ID, opened as a datasheet. */
	async createTable() {
		const name = await ui.prompt({
			title: 'New table',
			label: 'Table name',
			value: nextFreeName('Table', this.takenNames()),
			confirmLabel: 'Create',
			validate: (v) => this.tableNameProblem(v)
		});
		if (!name) return;
		const created = await ui.attempt(() =>
			tableService.create({
				name,
				fields: [{ name: 'ID', field_type: { type: 'auto_number' }, primary_key: true }]
			})
		);
		if (!created) return;
		await this.refreshObjects();
		this.openDoc('datasheet', name);
	}

	async renameObject(name: string) {
		const open = this.docs.find((d) => d.name?.toLowerCase() === name.toLowerCase());
		if (open?.dirty) {
			ui.notify('error', 'Save first', `Save or close "${name}" before renaming it.`);
			return;
		}
		const newName = await ui.prompt({
			title: `Rename "${name}"`,
			label: 'New name',
			value: name,
			confirmLabel: 'Rename',
			validate: (v) => this.tableNameProblem(v, name)
		});
		if (!newName || newName === name) return;
		const ok = await ui.attempt(() => tableService.rename(name, newName));
		if (ok === undefined) return;
		this.docs = this.docs.map((d) =>
			d.name?.toLowerCase() === name.toLowerCase() ? { ...d, id: docId(d.kind, newName), name: newName } : d
		);
		if (open && this.activeId === open.id) this.activeId = docId(open.kind, newName);
		await this.refreshObjects();
	}

	async deleteObject(name: string) {
		const confirmed = await ui.confirm({
			title: `Delete table "${name}"?`,
			message: 'The table and all of its records will be permanently deleted. This cannot be undone.',
			confirmLabel: 'Delete table',
			danger: true
		});
		if (!confirmed) return;
		const result = await ui.attempt(() => tableService.drop(name, true));
		if (!result) return;
		this.docs = this.docs.filter((d) => d.name?.toLowerCase() !== name.toLowerCase());
		if (!this.docs.some((d) => d.id === this.activeId)) this.activeId = this.docs[0]?.id ?? null;
		await this.refreshObjects();
		ui.notify('success', `Deleted "${name}"`, result.warnings.join(' '));
	}
}

export const session = new Session();
