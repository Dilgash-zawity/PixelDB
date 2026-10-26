<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';
	import { tableService } from '#lib/services/tables.js';
	import { transferService } from '#lib/services/transfer.js';

	// Types
	import type { ExportFormat, FieldDef, FieldTypeName, JsonValue, RecordKey, UndoState, RecordPage, RecordRow, SortSpec, TableDef } from '#lib/api/types.js';

	// Utils
	import { onDestroy, tick } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { MAX_BATCH, cellText, isBlock, parseClipboard, planFill, planPaste, toClipboard } from '#lib/logic/bulk-edit.js';
	import { FIELD_TYPES, NUMERIC_TYPES, isReadOnlyType, makeFieldType } from '#lib/logic/field-types.js';
	import { localDateOrder } from '#lib/logic/import.js';
	import { toFilterSpecs, type FilterRow } from '#lib/logic/filters.js';
	import { loadLookups, lookupLabel, type LookupOption } from '#lib/logic/lookups.js';
	import * as rb from '#lib/logic/record-buffer.js';
	import { matches, shortcutText, typingTarget } from '#lib/logic/shortcuts.js';
	import { putView, removeView, showsView, viewNameProblem, type SavedView } from '#lib/logic/saved-views.js';
	import { nextFreeName } from '#lib/logic/table-draft.js';
	import { displayValue, formatBytes, identifierProblem, isBlobSummary } from '#lib/logic/values.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as ButtonGroup from '#lib/components/ui/button-group/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import * as ContextMenu from '#lib/components/ui/context-menu/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import CellEditor from './CellEditor.svelte';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import FilterPopover from './FilterPopover.svelte';
	import NameDialog from './NameDialog.svelte';
	import FieldDrawer from './FieldDrawer.svelte';
	import ImportDrawer from './ImportDrawer.svelte';
	import RecordDrawer from './RecordDrawer.svelte';
	import ReplaceDialog from './ReplaceDialog.svelte';

	// Icons
	import {
		ArrowDown as ArrowDownIcon,
		ArrowDownWideNarrow as ArrowDownWideNarrowIcon,
		ArrowUp as ArrowUpIcon,
		ArrowUpNarrowWide as ArrowUpNarrowWideIcon,
		Asterisk as AsteriskIcon,
		Bookmark as BookmarkIcon,
		BookmarkPlus as BookmarkPlusIcon,
		Check as CheckIcon,
		ChevronDown as ChevronDownIcon,
		ChevronFirst as ChevronFirstIcon,
		ChevronLast as ChevronLastIcon,
		ChevronLeft as ChevronLeftIcon,
		ChevronRight as ChevronRightIcon,
		ClipboardPaste as ClipboardPasteIcon,
		Copy as CopyIcon,
		CopyCheck as CopyCheckIcon,
		Download as DownloadIcon,
		FileSpreadsheet as FileSpreadsheetIcon,
		FileText as FileTextIcon,
		ListFilter as FilterIcon,
		FunnelX as FilterXIcon,
		KeyRound as KeyRoundIcon,
		PanelBottomOpen as PanelBottomIcon,
		Paperclip as PaperclipIcon,
		Pencil as PencilIcon,
		Settings2 as Settings2Icon,
		Plus as PlusIcon,
		Redo2 as Redo2Icon,
		RefreshCw as RefreshCwIcon,
		Replace as ReplaceIcon,
		Save as SaveIcon,
		Search as SearchIcon,
		Trash2 as TrashIcon,
		Undo2 as Undo2Icon,
		Upload as UploadIcon,
		X as XIcon,
	} from '@lucide/svelte';

	// Script
	let { table, isView }: { table: string; isView: boolean } = $props();

	const PAGE_SIZE = 100;
	const QUICK_TYPES: FieldTypeName[] = ['short_text', 'long_text', 'number', 'currency', 'date_time', 'yes_no', 'hyperlink', 'attachment'];

	type Row = number | 'new';
	type MenuTarget = { kind: 'row'; index: number; col: string | null } | { kind: 'header'; col: string } | null;

	let def = $state<TableDef | null>(null);
	let data = $state<RecordPage | null>(null);
	let loading = $state(true);
	let offset = $state(0);
	let sort = $state<SortSpec | null>(null);
	let filterRows = $state<FilterRow[]>([]);
	let filterOpen = $state(false);
	let search = $state('');
	let appliedSearch = $state('');
	let current = $state<Row | null>(null);
	let currentCol = $state<string | null>(null);
	let editRow = $state<Row | null>(null);
	let buf = $state<rb.RecordBuffer | null>(null);
	let saving = $state(false);
	let selected = $state<Record<string, RecordKey>>({});
	let lookups = $state<Record<string, LookupOption[]>>({});
	let sheetOpen = $state(false);
	let importOpen = $state(false);
	let savingView = $state(false);
	let replacing = $state(false);
	/** The field Find and replace starts on (from a cell's menu). */
	let replaceField = $state<string | null>(null);
	/** The saved view last chosen or saved, while it is still what's shown. */
	let viewName = $state<string | null>(null);
	let exporting = $state(false);
	/** What Undo and Redo would do (saved record changes, from the backend's journal). */
	let history = $state<UndoState>({ undo: null, redo: null });
	/** The field drawer: which field (null for a new one) and whether it is open. */
	let fieldDrawer = $state<{ open: boolean; field: string | null }>({ open: false, field: null });
	let sheetRow = $state<RecordRow | null>(null);
	let menuTarget = $state<MenuTarget>(null);
	let addType = $state<FieldTypeName | null>(null);
	let renaming = $state<string | null>(null);
	let gridEl = $state<HTMLDivElement | null>(null);
	let searchEl = $state<HTMLInputElement | null>(null);
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	const columns = $derived(data?.columns ?? def?.fields.map((f) => f.name) ?? []);
	const filters = $derived(toFilterSpecs(filterRows));
	const total = $derived(data?.total ?? 0);
	const dirty = $derived(buf ? rb.isDirty(buf) : false);
	const position = $derived(typeof current === 'number' ? offset + current : null);
	const selectedKeys = $derived(Object.values(selected));
	const keyId = (k: RecordKey | null | undefined) => JSON.stringify(k ?? null);
	const fieldFor = (col: string): FieldDef | undefined => {
		if (def) return def.fields.find((f) => f.name === col);
		// Views: the type of the table column behind each column, for display only.
		const t = data?.types[data.columns.indexOf(col)];
		return t ? { name: col, field_type: t } : undefined;
	};
	const numeric = (f?: FieldDef) => !!f && NUMERIC_TYPES.includes(f.field_type.type);
	const editable = (col: string) => {
		const f = fieldFor(col);
		return !isView && !!f && !isReadOnlyType(f.field_type);
	};
	const firstEditable = () => columns.find(editable) ?? columns[0] ?? null;
	const allSelected = $derived(!!data?.rows.length && data.rows.every((r) => keyId(r.key) in selected));

	// ----- loading -----

	/** Number of the latest load; an earlier one finishing later is ignored. */
	let loadId = 0;

	/**
	 * Loads the shown page of records. `full` also reloads the table's design
	 * and the choices of its relation fields, which only change when fields
	 * do (or on Refresh), not on every saved row.
	 */
	async function load(full = false) {
		const id = ++loadId;
		const stale = () => id !== loadId;
		await attempt(async () => {
			if (!isView && (full || !def)) {
				const d = (await tableService.get(table)).definition;
				const l = await loadLookups(d);
				if (stale()) return;
				def = d;
				lookups = l;
			}
			const query = () => ({
				offset,
				limit: PAGE_SIZE,
				sort: sort ? [sort] : [],
				filters,
				search: appliedSearch || null
			});
			let page = await recordService.list(table, query());
			if (stale()) return;
			if (page.rows.length === 0 && page.total > 0 && offset > 0) {
				offset = Math.floor((page.total - 1) / PAGE_SIZE) * PAGE_SIZE;
				page = await recordService.list(table, query());
				if (stale()) return;
			}
			data = page;
		});
		if (stale()) return;
		if (!isView) history = (await recordService.undoState().catch(() => null)) ?? history;
		selected = {};
		if (typeof current === 'number' && data && current >= data.rows.length) current = data.rows.length ? data.rows.length - 1 : null;
		loading = false;
	}

	// ----- editing -----

	function focusCell(col: string | null = currentCol) {
		if (!gridEl || !col) return;
		const row = gridEl.querySelector('[data-editing]');
		const el = [...(row?.querySelectorAll<HTMLElement>('[data-col]') ?? [])].find((e) => e.dataset.col === col);
		el?.focus();
		if (el instanceof HTMLInputElement) el.select();
	}

	/** Saves the row being edited. Returns false (keeping the edits) on error. */
	async function commitRow(): Promise<boolean> {
		if (!buf || !def) {
			editRow = null;
			return true;
		}
		if (!rb.isDirty(buf)) {
			buf = null;
			editRow = null;
			return true;
		}
		const missing = rb.missingRequired(buf, def);
		if (missing.length) {
			toast.error('Required field', { description: `Enter a value in ${missing.map((m) => `“${m}”`).join(', ')}.` });
			currentCol = missing[0];
			await tick();
			focusCell();
			return false;
		}
		const b = buf;
		const values = rb.submission(b, def);
		saving = true;
		const ok = await attempt(async () => {
			if (b.key === null) await recordService.insert(table, values);
			else if (Object.keys(values).length) await recordService.update(table, b.key, values);
			return true;
		});
		saving = false;
		if (!ok) return false;
		buf = null;
		editRow = null;
		await load();
		return true;
	}

	/** Makes a cell current; leaving a row saves it first, like Access. */
	async function activate(row: Row, col: string | null) {
		if (!data) return;
		if (isView || !def) {
			current = row;
			currentCol = col;
			return;
		}
		if (editRow === row && buf) {
			currentCol = col;
			if ((document.activeElement as HTMLElement | null)?.dataset?.col !== col) focusCell();
			return;
		}
		const targetKey = row === 'new' ? null : data.rows[row]?.key;
		if (!(await commitRow())) return;
		let index: Row = row;
		if (targetKey) {
			const i = data.rows.findIndex((r) => keyId(r.key) === keyId(targetKey));
			if (i < 0) return;
			index = i;
		}
		current = index;
		currentCol = col && editable(col) ? col : firstEditable();
		editRow = index;
		buf = rb.newBuffer(def, index === 'new' ? null : data.rows[index], data.columns);
		await tick();
		focusCell();
	}

	function changed(col: string) {
		return !!buf && JSON.stringify(buf.current[col]) !== JSON.stringify(buf.original[col]);
	}

	function editorCols(): string[] {
		const row = gridEl?.querySelector('[data-editing]');
		return [...(row?.querySelectorAll<HTMLElement>('[data-col]') ?? [])].map((e) => e.dataset.col as string);
	}

	function moveHorizontal(col: string, delta: number) {
		const cols = editorCols();
		const i = cols.indexOf(col) + delta;
		if (i >= 0 && i < cols.length) {
			currentCol = cols[i];
			focusCell();
		} else if (i >= cols.length) {
			moveVertical(1, cols[0] ?? col);
		}
	}

	async function moveVertical(delta: number, col: string | null) {
		if (!data) return;
		const n = data.rows.length;
		const from = editRow ?? current;
		const index = from === 'new' ? n : (from ?? -1);
		const target = index + delta;
		if (target < 0) {
			if (offset > 0) await goToPosition(offset - 1);
			return;
		}
		if (target >= n) {
			if (offset + n < total) {
				await goToPosition(offset + n);
				return;
			}
			if (isView) return;
			// Moving down from the new-record row saves it and starts another.
			if (editRow === 'new') {
				if (await commitRow()) await activate('new', col);
			} else {
				await activate('new', col);
			}
			return;
		}
		await activate(target, col);
	}

	function onGridKey(e: KeyboardEvent) {
		const target = e.target as HTMLElement;
		const col = target.dataset?.col;
		const inMenu = target.closest('[role=listbox],[role=menu],[role=dialog]');
		if (inMenu) return;
		if (e.key === 'Escape' && buf) {
			e.preventDefault();
			e.stopPropagation();
			if (col && changed(col)) rb.undoField(buf, col);
			else if (rb.isDirty(buf)) rb.undoAll(buf);
			else {
				buf = null;
				editRow = null;
			}
			return;
		}
		if (e.key === 'Enter' && e.shiftKey && buf) {
			e.preventDefault();
			commitRow();
			return;
		}
		if (e.key === 'Enter' && col && target.tagName === 'INPUT') {
			e.preventDefault();
			moveHorizontal(col, 1);
			return;
		}
		if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && col && target.tagName === 'INPUT') {
			e.preventDefault();
			moveVertical(e.key === 'ArrowDown' ? 1 : -1, col);
			return;
		}
		if (e.key === 'Delete' && !col && selectedKeys.length) {
			e.preventDefault();
			deleteRecords();
			return;
		}
		if (typingTarget(e.target)) return;
		if (matches(e, 'mod+c')) {
			e.preventDefault();
			copyCells();
		} else if (matches(e, 'mod+v') && !isView) {
			// Let the paste event bring the text; read the clipboard only if it doesn't come.
			pasteSeen = false;
			setTimeout(async () => {
				if (pasteSeen) return;
				const text = await navigator.clipboard.readText().catch(() => '');
				if (text) pasteCells(text);
			}, 50);
		} else if (matches(e, 'mod+d') && typeof current === 'number' && currentCol) {
			e.preventDefault();
			fillSelected(current, currentCol);
		}
	}

	// ----- records -----

	async function goToPosition(p: number) {
		if (!data || total === 0) return;
		const clamped = Math.max(0, Math.min(p, total - 1));
		if (!(await commitRow())) return;
		const target = Math.floor(clamped / PAGE_SIZE) * PAGE_SIZE;
		if (target !== offset) {
			offset = target;
			current = null;
			await load();
		}
		await activate(clamped - offset, currentCol ?? firstEditable());
	}

	async function newRecord() {
		await activate('new', firstEditable());
	}

	async function deleteRecords(keys: RecordKey[] = selectedKeys) {
		if (isView || !data) return;
		if (!keys.length && typeof current === 'number' && data.rows[current]?.key) keys = [data.rows[current].key as RecordKey];
		if (!keys.length) {
			toast('Select records first', { description: 'Tick the boxes on the left, or click a record.' });
			return;
		}
		const ok = await confirm({
			title: `Delete ${keys.length === 1 ? 'this record' : `${keys.length} records`}?`,
			description: `You can bring ${keys.length === 1 ? 'it' : 'them'} back with Undo (${shortcutText('mod+z')}) until the table's fields change.`,
			action: 'Delete',
			destructive: true
		});
		if (!ok) return;
		const n = await attempt(() => recordService.delete(table, keys, true));
		if (n === undefined) return;
		buf = null;
		editRow = null;
		current = null;
		await load();
		toast.success(`Deleted ${n} ${n === 1 ? 'record' : 'records'}`);
	}

	async function toggleCheckbox(i: number, col: string) {
		if (!data || isView) return;
		if (editRow === i && buf) {
			rb.setValue(buf, col, buf.current[col] !== true);
			return;
		}
		if (!(await commitRow())) return;
		const row = data.rows[i];
		const c = data.columns.indexOf(col);
		if (!row?.key || c < 0) return;
		const key = row.key;
		if ((await attempt(() => recordService.update(table, key, { [col]: row.values[c] !== 1 }))) !== undefined) {
			current = i;
			currentCol = col;
			await load();
		}
	}

	async function applySort(column: string | null, descending = false) {
		if (!(await commitRow())) return;
		sort = column ? { column, descending } : null;
		offset = 0;
		current = null;
		await load();
	}

	async function applyFilters(rows: FilterRow[]) {
		if (!(await commitRow())) return;
		filterRows = rows;
		offset = 0;
		current = null;
		await load();
	}

	function onSearch() {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(async () => {
			if (!(await commitRow())) return;
			appliedSearch = search.trim();
			offset = 0;
			current = null;
			await load();
		}, 250);
	}

	async function openForm(index: number | null = typeof current === 'number' ? current : null) {
		if (!(await commitRow()) || !data) return;
		sheetRow = index === null ? null : data.rows[index];
		sheetOpen = true;
	}

	async function refresh() {
		if (await commitRow()) await load(true);
	}

	// ----- undo -----

	/** Undoes (or redoes) the latest saved change to this database's records. */
	async function undoRedo(redo = false) {
		if (isView || (redo ? !history.redo : !history.undo)) return;
		// Unsaved typing in a row is undone with Esc, not here.
		if (buf && rb.isDirty(buf)) {
			toast('Save or undo the row first', { description: 'Press Esc to undo what you typed in it.' });
			return;
		}
		buf = null;
		editRow = null;
		const label = await attempt(() => (redo ? recordService.redo() : recordService.undo()));
		await load();
		if (label) toast.success(`${redo ? 'Redid' : 'Undid'}: ${label}`);
	}

	// ----- many records at once -----

	/** Pastes a block of cells (from a spreadsheet or this datasheet) at the current cell: rows below become new records. */
	async function pasteCells(text: string) {
		if (isView || !def || !data) return;
		if (!(await commitRow())) return;
		if (!data) return;
		const cells = parseClipboard(text);
		const start = typeof current === 'number' ? current : data.rows.length;
		const col = currentCol ?? firstEditable() ?? columns[0];
		const plan = planPaste({
			cells,
			rows: data.rows,
			start,
			columns,
			startCol: Math.max(0, columns.indexOf(col)),
			fields: def.fields,
			editable,
			order: localDateOrder(),
			lookups
		});
		if (plan.problems.length) {
			toast.error(`${plan.problems.length} pasted ${plan.problems.length === 1 ? 'value doesn’t' : 'values don’t'} fit`, {
				description: plan.problems.slice(0, 3).map((p) => `Row ${p.row}, ${p.column}: ${p.message}`).join(' · ')
			});
			return;
		}
		if (!plan.ops.length) {
			toast('Nothing to paste here', { description: 'The cells landed on fields that can’t be edited.' });
			return;
		}
		if (plan.ops.length > MAX_BATCH) {
			toast.error('Too many rows', { description: `Paste at most ${MAX_BATCH.toLocaleString()} rows at a time, or use Import.` });
			return;
		}
		if (plan.ops.length > 1) {
			const parts = [plan.updated && `change ${plan.updated} ${plan.updated === 1 ? 'record' : 'records'}`, plan.added && `add ${plan.added}`].filter(Boolean);
			const skipped = plan.skippedColumns
				? ` ${plan.skippedColumns} ${plan.skippedColumns === 1 ? 'column fell on a field that can’t be edited and is' : 'columns fell on fields that can’t be edited and are'} left out.`
				: '';
			const ok = await confirm({
				title: `Paste ${cells.length} ${cells.length === 1 ? 'row' : 'rows'}?`,
				description: `This will ${parts.join(' and ')}.${skipped}`,
				action: 'Paste'
			});
			if (!ok) return;
		}
		const result = await attempt(() => recordService.batch(table, plan.ops, false, undefined, `Paste ${cells.length} ${cells.length === 1 ? 'row' : 'rows'}`));
		if (!result) return;
		await load();
		const done = [result.updated && `changed ${result.updated}`, result.inserted.length && `added ${result.inserted.length}`].filter(Boolean).join(' and ');
		toast.success(`Pasted: ${done} ${plan.ops.length === 1 ? 'record' : 'records'}`);
	}

	/** Copies the ticked records (with a header row), or else the current cell, as text spreadsheets understand. */
	async function copyCells() {
		if (!data) return;
		const rowsToCopy = selectedKeys.length ? data.rows.filter((r) => keyId(r.key) in selected) : [];
		let text: string;
		if (rowsToCopy.length) {
			text = toClipboard([columns, ...rowsToCopy.map((r) => columns.map((c, i) => cellText(r.values[i], fieldFor(c), lookups[c])))]);
		} else if (typeof current === 'number' && currentCol) {
			const i = columns.indexOf(currentCol);
			text = cellText(data.rows[current]?.values[i], fieldFor(currentCol), lookups[currentCol]);
		} else return;
		try {
			await navigator.clipboard.writeText(text);
			toast.success(rowsToCopy.length ? `Copied ${rowsToCopy.length} ${rowsToCopy.length === 1 ? 'record' : 'records'}` : 'Copied');
		} catch {
			toast.error('Couldn’t copy to the clipboard');
		}
	}

	/** Puts the value of field `col` in row `index` into every ticked record. */
	async function fillSelected(index: number, col: string) {
		if (!data || !editable(col) || !selectedKeys.length) return;
		if (fieldFor(col)?.field_type.type === 'attachment') return;
		if (!(await commitRow())) return;
		const value = data.rows[index]?.values[columns.indexOf(col)] ?? null;
		const keys = selectedKeys.filter((k) => keyId(k) !== keyId(data?.rows[index]?.key));
		if (!keys.length) return;
		const ok = await confirm({
			title: `Fill ${keys.length} ${keys.length === 1 ? 'record' : 'records'}?`,
			description: `“${col}” of the ticked records becomes “${cellText(value, fieldFor(col), lookups[col]) || 'empty'}”.`,
			action: 'Fill'
		});
		if (!ok) return;
		const result = await attempt(() => recordService.batch(table, planFill(keys, col, value), false, undefined, `Fill ${keys.length} ${col}`));
		if (!result) return;
		await load();
		toast.success(`Filled ${result.updated} ${result.updated === 1 ? 'record' : 'records'}`);
	}

	/** Set when a paste event reached the grid, so Ctrl+V on the grid itself doesn't read the clipboard a second time. */
	let pasteSeen = false;
	function onPaste(e: ClipboardEvent) {
		const text = e.clipboardData?.getData('text/plain') ?? '';
		const t = e.target as HTMLElement;
		const inField = t.tagName === 'INPUT' || t.tagName === 'TEXTAREA';
		pasteSeen = true;
		if (!text || (inField && !isBlock(text)) || t.closest('[role=listbox],[role=menu],[role=dialog]')) return;
		e.preventDefault();
		pasteCells(text);
	}

	/** Opens Find and replace on `field` (the current column by default). */
	async function openReplace(field: string | null = currentCol) {
		if (!(await commitRow())) return;
		replaceField = field;
		replacing = true;
	}

	// ----- saved views -----

	const views = $derived(session.savedViews[table] ?? []);
	const shownNow = $derived({ sort, filters: filterRows, search: appliedSearch });
	const activeView = $derived(views.find((v) => showsView(v, shownNow)) ?? null);
	const editedView = $derived(viewName && !activeView ? (views.find((v) => v.name === viewName) ?? null) : null);

	async function applyView(v: SavedView | null) {
		if (!(await commitRow())) return;
		// Conditions on fields that were since deleted are left out.
		filterRows = (v?.filters ?? []).filter((f) => columns.includes(f.column)).map((f) => ({ ...f }));
		sort = v?.sort && columns.includes(v.sort.column) ? { ...v.sort } : null;
		search = appliedSearch = v?.search ?? '';
		viewName = v?.name ?? null;
		offset = 0;
		current = null;
		await load();
	}

	async function saveView(name: string): Promise<boolean> {
		const view: SavedView = { name: name.trim(), sort, filters: $state.snapshot(filterRows), search: appliedSearch };
		if (!(await session.setSavedViews(putView(session.savedViews, table, view)))) return false;
		viewName = view.name;
		toast.success(`Saved the view “${view.name}”`);
		return true;
	}

	async function deleteView(name: string) {
		if (!(await session.setSavedViews(removeView(session.savedViews, table, name)))) return;
		if (viewName === name) viewName = null;
		toast.success(`Deleted the view “${name}”`);
	}

	/** Exports the records shown (with the filter, search and sort, on every page) to a file. */
	async function exportRecords(format: ExportFormat) {
		if (!(await commitRow())) return;
		exporting = true;
		try {
			const result = await attempt(() =>
				transferService.exportRecords(table, { sort: sort ? [sort] : [], filters, search: appliedSearch || null }, format)
			);
			if (!result) return;
			toast.success(`Exported ${result.rows} ${result.rows === 1 ? 'record' : 'records'}`, {
				description: result.truncated ? `Saved as ${result.file_name}. Only the first ${result.rows.toLocaleString()} fit in one file.` : `Saved as ${result.file_name}.`
			});
		} finally {
			exporting = false;
		}
	}

	// ----- fields -----

	function nameProblem(v: string, except?: string) {
		return (
			identifierProblem(v, 'Name') ??
			(columns.some((c) => c !== except && c.toLowerCase() === v.toLowerCase()) ? 'A field with this name already exists' : null)
		);
	}

	async function addColumn(name: string): Promise<boolean> {
		if (!addType || !(await commitRow())) return false;
		const type = addType;
		const result = await attempt(() => tableService.addField(table, { name, field_type: makeFieldType(type) }));
		if (!result) return false;
		await load(true);
		return true;
	}

	async function renameColumn(name: string): Promise<boolean> {
		const from = renaming;
		if (!from || name === from) return true;
		if (!(await commitRow())) return false;
		if ((await attempt(() => tableService.renameField(table, from, name))) === undefined) return false;
		session.renamedField(table, from, name);
		if (sort?.column === from) sort = { ...sort, column: name };
		filterRows = filterRows.map((f) => (f.column === from ? { ...f, column: name } : f));
		await load(true);
		return true;
	}

	async function deleteColumn(col: string) {
		if (!(await commitRow())) return;
		const ok = await confirm({
			title: `Delete the “${col}” field?`,
			description: `The field and the data in it will be permanently deleted from every record.`,
			action: 'Delete field',
			destructive: true
		});
		if (!ok) return;
		const result = await attempt(() => tableService.dropField(table, col, true));
		if (!result) return;
		if (sort?.column === col) sort = null;
		filterRows = filterRows.filter((f) => f.column !== col);
		await load(true);
		toast.success(`Deleted “${col}”`, { description: result.warnings.join(' ') || undefined });
	}

	function onContextMenu(e: MouseEvent) {
		const el = e.target as HTMLElement;
		const header = el.closest<HTMLElement>('[data-header]');
		const row = el.closest<HTMLElement>('[data-row]');
		const cell = el.closest<HTMLElement>('[data-cell]');
		if (header) menuTarget = { kind: 'header', col: header.dataset.header as string };
		else if (row) {
			const index = Number(row.dataset.row);
			menuTarget = { kind: 'row', index, col: cell?.dataset.cell ?? null };
			if (!(keyId(data?.rows[index]?.key) in selected) && current !== index) current = index;
		} else menuTarget = null;
	}

	// ----- lifecycle -----

	session.leaveGuard = commitRow;
	onDestroy(() => {
		clearTimeout(searchTimer);
		if (session.leaveGuard === commitRow) session.leaveGuard = null;
	});

	let lastIntent = session.intent.n;
	$effect(() => {
		const { kind, n } = session.intent;
		if (n === lastIntent) return;
		lastIntent = n;
		if (kind === 'new-record' && !isView) newRecord();
		if (kind === 'new-field' && !isView) fieldDrawer = { open: true, field: null };
		if (kind === 'export-xlsx') exportRecords('xlsx');
		if (kind === 'export-csv') exportRecords('csv');
	});

	function onWindowKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's' && !sheetOpen) {
			e.preventDefault();
			commitRow();
		} else if (matches(e, 'mod+h') && !isView && !sheetOpen && !e.defaultPrevented) {
			e.preventDefault();
			openReplace();
		} else if ((matches(e, 'mod+z') || matches(e, 'shift+mod+z')) && !isView && !sheetOpen && !e.defaultPrevented && !typingTarget(e.target)) {
			// Typing keeps its own undo; outside a text box this undoes saved changes.
			if ((e.target as HTMLElement).closest?.('[role=dialog],[role=menu]')) return;
			e.preventDefault();
			undoRedo(e.shiftKey);
		}
	}

	load(true);
</script>

<svelte:window onkeydown={onWindowKey} />

{#snippet toolButton(label: string, Icon: typeof SaveIcon, onclick: () => unknown, disabled = false)}
	<Tooltip.Root>
		<Tooltip.Trigger>
			{#snippet child({ props })}
				<Button {...props} variant="outline" size="icon-sm" aria-label={label} {disabled} {onclick}><Icon /></Button>
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content>{label}</Tooltip.Content>
	</Tooltip.Root>
{/snippet}

<div class="flex h-full flex-col">
	<!-- Toolbar -->
	<div class="flex flex-wrap items-center gap-2 border-b px-4 py-2">
		{#if !isView}
			<ButtonGroup.Root>
				<Button variant="outline" size="sm" onclick={newRecord}><PlusIcon data-icon="inline-start" /> New</Button>
				<Button variant="outline" size="sm" disabled={!dirty || saving} onclick={commitRow}><SaveIcon data-icon="inline-start" /> Save</Button>
				<Button
					variant="destructive"
					size="sm"
					disabled={!selectedKeys.length && typeof current !== 'number'}
					onclick={() => deleteRecords()}
				>
					<TrashIcon data-icon="inline-start" /> Delete
				</Button>
			</ButtonGroup.Root>
		{/if}
		<ButtonGroup.Root>
			{@render toolButton('Sort ascending', ArrowUpNarrowWideIcon, () => applySort(currentCol ?? columns[0], false), !columns.length)}
			{@render toolButton('Sort descending', ArrowDownWideNarrowIcon, () => applySort(currentCol ?? columns[0], true), !columns.length)}
			{@render toolButton('Remove sort', XIcon, () => applySort(null), !sort)}
		</ButtonGroup.Root>
		<FilterPopover bind:open={filterOpen} {columns} active={filters.length} initial={filterRows} onApply={applyFilters} />
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant={activeView ? 'secondary' : 'outline'} size="sm" class="max-w-56">
						<BookmarkIcon data-icon="inline-start" />
						<span class="truncate">{activeView?.name ?? (editedView ? `${editedView.name} (changed)` : 'Views')}</span>
						<ChevronDownIcon data-icon="inline-end" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="start" class="w-64">
				<DropdownMenu.Group>
					<DropdownMenu.Label>Saved views of {table}</DropdownMenu.Label>
					{#each views as v (v.name)}
						<DropdownMenu.Item onclick={() => applyView(v)}>
							{#if activeView?.name === v.name}<CheckIcon />{:else}<BookmarkIcon />{/if}
							<span class="truncate">{v.name}</span>
						</DropdownMenu.Item>
					{:else}
						<DropdownMenu.Item disabled>No saved views yet</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Group>
				<DropdownMenu.Separator />
				<DropdownMenu.Group>
					<DropdownMenu.Item disabled={!!activeView} onclick={() => (savingView = true)}><BookmarkPlusIcon /> Save as a new view…</DropdownMenu.Item>
					{#if editedView}
						{@const name = editedView.name}
						<DropdownMenu.Item onclick={() => saveView(name)}><SaveIcon /> Update “{name}”</DropdownMenu.Item>
					{/if}
					<DropdownMenu.Item disabled={!filterRows.length && !sort && !appliedSearch} onclick={() => applyView(null)}><FilterXIcon /> Show everything</DropdownMenu.Item>
				</DropdownMenu.Group>
				{#if activeView || editedView}
					{@const name = (activeView ?? editedView)!.name}
					<DropdownMenu.Separator />
					<DropdownMenu.Group>
						<DropdownMenu.Item variant="destructive" onclick={() => deleteView(name)}><TrashIcon /> Delete “{name}”</DropdownMenu.Item>
					</DropdownMenu.Group>
				{/if}
			</DropdownMenu.Content>
		</DropdownMenu.Root>
		{#if !isView}
			<ButtonGroup.Root>
				{@render toolButton(history.undo ? `Undo: ${history.undo} (${shortcutText('mod+z')})` : 'Nothing to undo', Undo2Icon, () => undoRedo(), !history.undo)}
				{@render toolButton(history.redo ? `Redo: ${history.redo} (${shortcutText('shift+mod+z')})` : 'Nothing to redo', Redo2Icon, () => undoRedo(true), !history.redo)}
			</ButtonGroup.Root>
		{/if}
		{@render toolButton('Refresh', RefreshCwIcon, refresh)}
	{#if !isView}{@render toolButton(`Find and replace (${shortcutText('mod+h')})`, ReplaceIcon, () => openReplace())}{/if}
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button {...props} variant="outline" size="sm" disabled={exporting}>
						<DownloadIcon data-icon="inline-start" /> Import / Export <ChevronDownIcon data-icon="inline-end" />
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="start">
				{#if !isView}
					<DropdownMenu.Group>
						<DropdownMenu.Item onclick={() => (importOpen = true)}><UploadIcon /> Import from a file…</DropdownMenu.Item>
					</DropdownMenu.Group>
					<DropdownMenu.Separator />
				{/if}
				<DropdownMenu.Group>
					<DropdownMenu.Label>{filters.length || appliedSearch ? 'Export the matching records' : 'Export all records'}</DropdownMenu.Label>
					<DropdownMenu.Item onclick={() => exportRecords('xlsx')}><FileSpreadsheetIcon /> Excel workbook (.xlsx)</DropdownMenu.Item>
					<DropdownMenu.Item onclick={() => exportRecords('csv')}><FileTextIcon /> CSV file (.csv)</DropdownMenu.Item>
				</DropdownMenu.Group>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
		<span class="flex-1"></span>
		{#if sort}
			<Badge variant="secondary">
				{#if sort.descending}<ArrowDownIcon />{:else}<ArrowUpIcon />{/if}
				{sort.column}
			</Badge>
		{/if}
		{#if !isView}
			<!-- Opens the selected record, or a new one when none is selected. -->
			<Button variant="outline" size="sm" onclick={() => openForm()}>
				<PanelBottomIcon data-icon="inline-start" /> Open as form
			</Button>
		{/if}
	</div>

	<!-- Datasheet -->
	<ContextMenu.Root>
		<ContextMenu.Trigger>
			{#snippet child({ props })}
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div
					{...props}
					bind:this={gridEl}
					class="min-h-0 flex-1 overflow-auto outline-none focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:ring-inset"
					role="grid"
					aria-label="{table} records"
					tabindex="0"
					onkeydown={onGridKey}
					oncontextmenucapture={onContextMenu}
					onpaste={onPaste}
				>
					{#if loading}
						<div class="flex flex-col gap-2 p-4">
							{#each Array(8) as _, i (i)}<Skeleton class="h-9" />{/each}
						</div>
					{:else if data}
						<Table.Root class="w-auto">
							<Table.Header class="sticky top-0 z-10 bg-background">
								<Table.Row>
									<Table.Head class="w-14 border-e">
										{#if !isView}
											<Checkbox
												checked={allSelected}
												onCheckedChange={(v) => (selected = v && data ? Object.fromEntries(data.rows.filter((r) => r.key).map((r) => [keyId(r.key), r.key as RecordKey])) : {})}
												aria-label="Select all"
											/>
										{/if}
									</Table.Head>
									{#each columns as col (col)}
										{@const f = fieldFor(col)}
										<Table.Head data-header={col} class={cn('min-w-40 border-e', numeric(f) && 'text-right')} title={f?.description ?? undefined}>
											<DropdownMenu.Root>
												<DropdownMenu.Trigger>
													{#snippet child({ props })}
														<Button {...props} variant="ghost" size="sm" class="-ms-2">
															{#if f?.primary_key}<KeyRoundIcon data-icon="inline-start" class="text-warning" />{:else if f}<FieldTypeIcon type={f.field_type.type} data-icon="inline-start" />{/if}
															{col}
															{#if sort?.column === col}
																{#if sort.descending}<ArrowDownIcon data-icon="inline-end" />{:else}<ArrowUpIcon data-icon="inline-end" />{/if}
															{:else}
																<ChevronDownIcon data-icon="inline-end" />
															{/if}
														</Button>
													{/snippet}
												</DropdownMenu.Trigger>
												<DropdownMenu.Content align="start">
													{@render headerItems(col, DropdownMenu)}
												</DropdownMenu.Content>
											</DropdownMenu.Root>
										</Table.Head>
									{/each}
									{#if !isView}
										<Table.Head class="min-w-36">
											<DropdownMenu.Root>
												<DropdownMenu.Trigger>
													{#snippet child({ props })}
														<Button {...props} variant="ghost" size="sm" class="-ms-2 text-muted-foreground">
															Click to Add <ChevronDownIcon data-icon="inline-end" />
														</Button>
													{/snippet}
												</DropdownMenu.Trigger>
												<DropdownMenu.Content align="start">
													<DropdownMenu.Group>
														<DropdownMenu.Label>Add a field</DropdownMenu.Label>
														{#each QUICK_TYPES as t (t)}
															<DropdownMenu.Item onclick={() => (addType = t)}>
																<FieldTypeIcon type={t} />
																{FIELD_TYPES.find((x) => x.name === t)?.label}
															</DropdownMenu.Item>
														{/each}
													</DropdownMenu.Group>
													<DropdownMenu.Separator />
													<DropdownMenu.Group>
														<DropdownMenu.Item onclick={() => (fieldDrawer = { open: true, field: null })}>
															<Settings2Icon /> More types and rules…
														</DropdownMenu.Item>
													</DropdownMenu.Group>
												</DropdownMenu.Content>
											</DropdownMenu.Root>
										</Table.Head>
									{/if}
								</Table.Row>
							</Table.Header>
							<Table.Body>
								{#each data.rows as row, i (keyId(row.key) + ':' + i)}
									{@const editing = editRow === i && !!buf}
									{@const isSelected = keyId(row.key) in selected}
									<Table.Row
										data-row={i}
										data-editing={editing || undefined}
										data-state={isSelected ? 'selected' : undefined}
										class={cn(current === i && !isSelected && 'bg-muted/50')}
									>
										<Table.Cell class="w-14 border-e">
											<div class="flex items-center gap-1.5">
												{#if !isView}
													<Checkbox
														checked={isSelected}
														onCheckedChange={(v) => {
															if (v && row.key) selected[keyId(row.key)] = row.key;
															else delete selected[keyId(row.key)];
														}}
														aria-label="Select record"
													/>
												{/if}
												{#if editing && dirty}<PencilIcon class="size-3.5 text-primary" />{:else if current === i}<ChevronRightIcon class="size-3.5 text-muted-foreground" />{/if}
											</div>
										</Table.Cell>
										{#each columns as col, c (col)}
											{@const f = fieldFor(col)}
											{@const v = row.values[c]}
											<Table.Cell
												data-cell={col}
												class={cn(
													'h-10 max-w-80 min-w-40 border-e py-1',
													numeric(f) && 'text-right tabular-nums',
													current === i && currentCol === col && !editing && 'ring-1 ring-ring ring-inset'
												)}
												onclick={() => activate(i, col)}
											>
												{#if editing && f && editable(col)}
													<CellEditor field={f} value={buf!.current[col]} options={lookups[col]} onchange={(nv) => rb.setValue(buf!, col, nv)} onfocus={() => (currentCol = col)} />
												{:else if f?.field_type.type === 'yes_no'}
													<div class="flex justify-center">
														<Checkbox
															checked={editing ? buf!.current[col] === true : v === 1}
															disabled={isView}
															tabindex={-1}
															onclick={(e) => {
																e.preventDefault();
																e.stopPropagation();
																toggleCheckbox(i, col);
															}}
															aria-label={col}
														/>
													</div>
												{:else if isBlobSummary(v)}
													<Badge variant="secondary"><PaperclipIcon /> {formatBytes(v.blob.size)}</Badge>
												{:else if v === null || v === ''}
													<span class="text-muted-foreground"></span>
												{:else}
													<span class="block truncate">
														{f?.field_type.type === 'lookup' ? (lookupLabel(lookups[col], v) ?? displayValue(v)) : displayValue(v, f?.field_type)}
													</span>
												{/if}
											</Table.Cell>
										{/each}
										{#if !isView}<Table.Cell></Table.Cell>{/if}
									</Table.Row>
								{/each}
								{#if !isView}
									{@const editingNew = editRow === 'new' && !!buf}
									<Table.Row data-editing={editingNew || undefined} class={cn(current === 'new' && 'bg-muted/50')}>
										<Table.Cell class="w-14 border-e" title="New record">
											{#if editingNew && dirty}<PencilIcon class="size-3.5 text-primary" />{:else}<AsteriskIcon class="size-3.5 text-muted-foreground" />{/if}
										</Table.Cell>
										{#each columns as col (col)}
											{@const f = fieldFor(col)}
											<Table.Cell data-cell={col} class={cn('h-10 min-w-40 border-e py-1', numeric(f) && 'text-right')} onclick={() => activate('new', col)}>
												{#if editingNew && f && editable(col)}
													<CellEditor field={f} value={buf!.current[col]} options={lookups[col]} onchange={(nv) => rb.setValue(buf!, col, nv)} onfocus={() => (currentCol = col)} />
												{:else if f?.field_type.type === 'auto_number'}
													<span class="text-muted-foreground">(New)</span>
												{/if}
											</Table.Cell>
										{/each}
										<Table.Cell></Table.Cell>
									</Table.Row>
								{/if}
							</Table.Body>
						</Table.Root>
						{#if data.rows.length === 0 && (filters.length || appliedSearch)}
							<p class="p-6 text-center text-sm text-muted-foreground">No records match your search or filter.</p>
						{/if}
					{/if}
				</div>
			{/snippet}
		</ContextMenu.Trigger>
		<ContextMenu.Content class="w-56">
			{#if menuTarget?.kind === 'header'}
				{@render headerItems(menuTarget.col, ContextMenu)}
			{:else if menuTarget?.kind === 'row' && !isView}
				{@const t = menuTarget}
				<ContextMenu.Group>
					<ContextMenu.Item onclick={() => openForm(t.index)}><PanelBottomIcon /> Open as form</ContextMenu.Item>
					<ContextMenu.Item onclick={newRecord}><PlusIcon /> New record</ContextMenu.Item>
				</ContextMenu.Group>
				<ContextMenu.Separator />
				<ContextMenu.Group>
					<ContextMenu.Item onclick={copyCells}>
						<CopyIcon /> {selectedKeys.length ? `Copy ${selectedKeys.length} ${selectedKeys.length === 1 ? 'record' : 'records'}` : 'Copy'}
						<ContextMenu.Shortcut>{shortcutText('mod+c')}</ContextMenu.Shortcut>
					</ContextMenu.Item>
					<ContextMenu.Item
						onclick={async () => {
							const text = await navigator.clipboard.readText().catch(() => '');
							if (text) pasteCells(text);
							else toast(`Press ${shortcutText('mod+v')} to paste`, { description: 'The clipboard can only be read from the keyboard here.' });
						}}
					>
						<ClipboardPasteIcon /> Paste here
						<ContextMenu.Shortcut>{shortcutText('mod+v')}</ContextMenu.Shortcut>
					</ContextMenu.Item>
					{#if t.col && editable(t.col) && selectedKeys.length}
						{@const col = t.col}
						<ContextMenu.Item onclick={() => fillSelected(t.index, col)}>
							<CopyCheckIcon /> Fill ticked records with this {col}
							<ContextMenu.Shortcut>{shortcutText('mod+d')}</ContextMenu.Shortcut>
						</ContextMenu.Item>
					{/if}
					{#if t.col}
						{@const col = t.col}
						<ContextMenu.Item
							onclick={() => openReplace(col)}
						>
							<ReplaceIcon /> Find and replace…
							<ContextMenu.Shortcut>{shortcutText('mod+h')}</ContextMenu.Shortcut>
						</ContextMenu.Item>
					{/if}
				</ContextMenu.Group>
				{#if t.col}
					<ContextMenu.Separator />
					<ContextMenu.Group>
						<ContextMenu.Item onclick={() => applySort(t.col, false)}><ArrowUpNarrowWideIcon /> Sort ascending</ContextMenu.Item>
						<ContextMenu.Item onclick={() => applySort(t.col, true)}><ArrowDownWideNarrowIcon /> Sort descending</ContextMenu.Item>
					</ContextMenu.Group>
				{/if}
				<ContextMenu.Separator />
				<ContextMenu.Group>
					<ContextMenu.Item
						variant="destructive"
						onclick={() => {
							const key = data?.rows[t.index]?.key;
							deleteRecords(selectedKeys.length ? selectedKeys : key ? [key] : []);
						}}
					>
						<TrashIcon /> {selectedKeys.length > 1 ? `Delete ${selectedKeys.length} records` : 'Delete record'}
						<ContextMenu.Shortcut>Del</ContextMenu.Shortcut>
					</ContextMenu.Item>
				</ContextMenu.Group>
			{:else}
				<ContextMenu.Group>
					{#if !isView}<ContextMenu.Item onclick={newRecord}><PlusIcon /> New record</ContextMenu.Item>{/if}
					<ContextMenu.Item onclick={refresh}><RefreshCwIcon /> Refresh</ContextMenu.Item>
				</ContextMenu.Group>
			{/if}
		</ContextMenu.Content>
	</ContextMenu.Root>

	<!-- Record navigation -->
	<div class="flex flex-wrap items-center gap-2 border-t px-4 py-1.5 text-sm">
		<span class="text-muted-foreground">Record</span>
		<ButtonGroup.Root>
			<Button variant="outline" size="icon-sm" aria-label="First record" disabled={!total || position === 0} onclick={() => goToPosition(0)}><ChevronFirstIcon /></Button>
			<Button variant="outline" size="icon-sm" aria-label="Previous record" disabled={!total || position === 0} onclick={() => goToPosition((position ?? 1) - 1)}><ChevronLeftIcon /></Button>
			<Input
				class="h-7 w-20 text-center"
				aria-label="Current record"
				value={current === 'new' ? 'New' : position !== null ? String(position + 1) : ''}
				onkeydown={(e) => {
					const n = Number(e.currentTarget.value);
					if (e.key === 'Enter' && n > 0) goToPosition(n - 1);
				}}
			/>
			<ButtonGroup.Text>of {total}</ButtonGroup.Text>
			<Button variant="outline" size="icon-sm" aria-label="Next record" disabled={!total || (position ?? -1) >= total - 1} onclick={() => goToPosition((position ?? -1) + 1)}><ChevronRightIcon /></Button>
			<Button variant="outline" size="icon-sm" aria-label="Last record" disabled={!total || position === total - 1} onclick={() => goToPosition(total - 1)}><ChevronLastIcon /></Button>
			{#if !isView}
				<Button variant="outline" size="icon-sm" aria-label="New record" onclick={newRecord}><AsteriskIcon /></Button>
			{/if}
		</ButtonGroup.Root>
		<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />
		{#if filters.length}
			<Button variant="secondary" size="sm" onclick={() => applyFilters([])}><FilterXIcon data-icon="inline-start" /> Filtered</Button>
		{:else}
			<Button variant="ghost" size="sm" onclick={() => (filterOpen = true)}><FilterIcon data-icon="inline-start" /> No filter</Button>
		{/if}
		<InputGroup.Root class="h-7 w-56">
			<InputGroup.Input bind:ref={searchEl} placeholder="Search" bind:value={search} oninput={onSearch} />
			<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
		</InputGroup.Root>
		<span class="flex-1"></span>
		{#if dirty}
			<span class="flex items-center gap-1 text-xs text-muted-foreground">
				<Kbd>Shift</Kbd><Kbd>Enter</Kbd> save · <Kbd>Esc</Kbd> undo
			</span>
		{:else if isView}
			<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning">Read-only</Badge>
		{/if}
	</div>
</div>

{#snippet headerItems(col: string, M: typeof DropdownMenu | typeof ContextMenu)}
	<M.Group>
		<M.Item onclick={() => applySort(col, false)}><ArrowUpNarrowWideIcon /> Sort A to Z</M.Item>
		<M.Item onclick={() => applySort(col, true)}><ArrowDownWideNarrowIcon /> Sort Z to A</M.Item>
		<M.Item disabled={!sort} onclick={() => applySort(null)}><XIcon /> Remove sort</M.Item>
		<M.Item
			onclick={() => {
				filterRows = [...filterRows, { column: col, op: 'contains', value: '' }];
				filterOpen = true;
			}}
		>
			<FilterIcon /> Filter by {col}
		</M.Item>
	</M.Group>
	{#if !isView}
		<M.Separator />
		<M.Group>
			<M.Item onclick={() => (renaming = col)}><PencilIcon /> Rename field</M.Item>
			<M.Item onclick={() => (fieldDrawer = { open: true, field: col })}><Settings2Icon /> Edit field…</M.Item>
			<M.Item variant="destructive" disabled={columns.length <= 1} onclick={() => deleteColumn(col)}><TrashIcon /> Delete field</M.Item>
		</M.Group>
	{/if}
{/snippet}

{#if def && !isView}
	<FieldDrawer bind:open={fieldDrawer.open} {table} field={fieldDrawer.field} onSaved={() => load(true)} />
	<ImportDrawer bind:open={importOpen} {table} onImported={() => load(true)} />
	<ReplaceDialog
		bind:open={replacing}
		{table}
		fields={def.fields}
		{filters}
		search={appliedSearch}
		{sort}
		initialField={replaceField}
		onReplaced={() => load()}
	/>
	<RecordDrawer bind:open={sheetOpen} {table} {def} columns={columns} row={sheetRow} {lookups} onSaved={() => load()} />
{/if}

<NameDialog
	bind:open={() => addType !== null, (open) => !open && (addType = null)}
	title={`New ${FIELD_TYPES.find((t) => t.name === addType)?.label ?? ''} field`}
	description="It's added as a new column at the end of the table."
	initial={nextFreeName('Field', columns)}
	submitLabel="Add field"
	validate={(v) => nameProblem(v)}
	onSubmit={addColumn}
/>

<NameDialog
	bind:open={savingView}
	title="Save view"
	description="Keeps this filter, search and sort under a name, in the database file."
	initial={nextFreeName('View', views.map((v) => v.name))}
	submitLabel="Save view"
	validate={(v) => viewNameProblem(session.savedViews, table, v)}
	onSubmit={saveView}
/>

<NameDialog
	bind:open={() => renaming !== null, (open) => !open && (renaming = null)}
	title="Rename field"
	initial={renaming ?? ''}
	submitLabel="Rename"
	validate={(v) => nameProblem(v, renaming ?? undefined)}
	onSubmit={renameColumn}
/>
