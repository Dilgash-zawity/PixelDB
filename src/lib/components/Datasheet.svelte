<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import type { FieldDef, FieldTypeName, FilterOp, FilterSpec, JsonValue, RecordKey, RecordPage, SortSpec, TableDef } from '../api/types';
	import { FIELD_TYPES, isReadOnlyType, makeFieldType } from '../logic/field-types';
	import * as rb from '../logic/record-buffer';
	import { emptySelection, pageOffsetFor, select, type Selection } from '../logic/selection';
	import { nextFreeName } from '../logic/table-draft';
	import { displayValue, fileToAttachment, formatBytes, identifierProblem, isBlobSummary, placeholderFor } from '../logic/values';
	import { recordService } from '../services/records';
	import { tableService } from '../services/tables';
	import { session, type Doc } from '../state/session.svelte';
	import { ui } from '../state/ui.svelte';
	import Icon from './Icon.svelte';

	let { doc }: { doc: Doc } = $props();

	const PAGE_SIZE = 100;
	const OPS: { op: FilterOp; label: string }[] = [
		{ op: 'eq', label: 'equals' },
		{ op: 'ne', label: 'does not equal' },
		{ op: 'contains', label: 'contains' },
		{ op: 'starts_with', label: 'begins with' },
		{ op: 'ends_with', label: 'ends with' },
		{ op: 'lt', label: 'less than' },
		{ op: 'le', label: 'at most' },
		{ op: 'gt', label: 'greater than' },
		{ op: 'ge', label: 'at least' },
		{ op: 'is_null', label: 'is blank' },
		{ op: 'is_not_null', label: 'is not blank' },
		{ op: 'in', label: 'is one of' }
	];
	const NUMERIC: FieldTypeName[] = ['number', 'large_number', 'currency', 'auto_number'];

	type Row = number | 'new';
	interface FilterRow {
		column: string;
		op: FilterOp;
		value: string;
	}
	interface LookupOption {
		value: JsonValue;
		label: string;
	}

	// svelte-ignore state_referenced_locally
	const name = doc.name as string;
	// svelte-ignore state_referenced_locally
	const docId = doc.id;
	const isView = session.isView(name);

	let def = $state<TableDef | null>(null);
	let page = $state<RecordPage | null>(null);
	let loaded = $state(false);
	let offset = $state(0);
	let sort = $state<SortSpec | null>(null);
	let filterRows = $state<FilterRow[]>([]);
	let appliedFilters = $state<FilterSpec[]>([]);
	let showFilter = $state(false);
	let search = $state('');
	let appliedSearch = $state('');
	let current = $state<Row | null>(null);
	let currentCol = $state<string | null>(null);
	let editRow = $state<Row | null>(null);
	let buf = $state<rb.RecordBuffer | null>(null);
	let selection = $state<Selection>(emptySelection());
	let lookups = $state<Record<string, LookupOption[]>>({});
	let jump = $state('');
	let gridEl = $state<HTMLDivElement | null>(null);
	let searchEl = $state<HTMLInputElement | null>(null);
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	const columns = $derived(page?.columns ?? []);
	const dirty = $derived(buf ? rb.isDirty(buf) : false);
	const total = $derived(page?.total ?? 0);
	const position = $derived(typeof current === 'number' ? offset + current : null);

	$effect(() => session.setDirty(docId, dirty));

	const keyId = (k: RecordKey | null | undefined) => JSON.stringify(k ?? null);
	const fieldFor = (col: string): FieldDef | undefined => def?.fields.find((f) => f.name === col);
	const isNumeric = (f?: FieldDef) => !!f && NUMERIC.includes(f.field_type.type);
	const editable = (col: string) => {
		const f = fieldFor(col);
		return !!f && !isReadOnlyType(f.field_type);
	};
	const firstEditable = () => columns.find(editable) ?? columns[0] ?? null;

	function cellText(col: string, value: JsonValue | undefined): string {
		const f = fieldFor(col);
		if (f?.field_type.type === 'lookup' && value !== null && value !== undefined) {
			const opt = lookups[col]?.find((o) => JSON.stringify(o.value) === JSON.stringify(value));
			if (opt) return opt.label;
		}
		return displayValue(value, f?.field_type);
	}

	async function loadLookups(table: TableDef) {
		const result: Record<string, LookupOption[]> = {};
		for (const f of table.fields) {
			if (f.field_type.type !== 'lookup') continue;
			const fk = table.foreign_keys?.find((k) => k.columns.length === 1 && k.columns[0] === f.name);
			const refCol = fk?.ref_columns?.[0];
			if (!fk || !refCol) continue;
			const shown = f.field_type.display_column || refCol;
			try {
				const rows = await recordService.list(fk.ref_table, { limit: 1000 });
				const vi = rows.columns.indexOf(refCol);
				const li = rows.columns.indexOf(shown);
				if (vi < 0) continue;
				result[f.name] = rows.rows.map((r) => ({
					value: r.values[vi],
					label: displayValue(r.values[li >= 0 ? li : vi])
				}));
			} catch {
				// A broken lookup source only loses the friendly labels.
			}
		}
		lookups = result;
	}

	async function load() {
		await ui.attempt(async () => {
			if (!isView) {
				def = (await tableService.get(name)).definition;
				await loadLookups(def);
			}
			page = await recordService.list(name, {
				offset,
				limit: PAGE_SIZE,
				sort: sort ? [sort] : [],
				filters: appliedFilters,
				search: appliedSearch || null
			});
			if (page.rows.length === 0 && page.total > 0 && offset > 0) {
				offset = pageOffsetFor(page.total - 1, PAGE_SIZE);
				page = await recordService.list(name, { offset, limit: PAGE_SIZE, sort: sort ? [sort] : [], filters: appliedFilters, search: appliedSearch || null });
			}
		});
		selection = emptySelection();
		if (typeof current === 'number' && page && current >= page.rows.length) current = page.rows.length ? page.rows.length - 1 : null;
		loaded = true;
	}

	// ----- editing -----

	function focusCell(col: string | null = currentCol) {
		if (!gridEl || !col) return;
		const row = gridEl.querySelector('tr.editing');
		const el = [...(row?.querySelectorAll<HTMLElement>('[data-col]') ?? [])].find((e) => e.dataset.col === col);
		el?.focus();
		if (el instanceof HTMLInputElement && el.type === 'text') el.select();
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
			ui.notify('error', 'Required field', `Enter a value in ${missing.join(', ')}.`);
			currentCol = missing[0];
			focusCell();
			return false;
		}
		const b = buf;
		const values = rb.submission(b, def);
		const ok = await ui.attempt(async () => {
			if (b.key === null) await recordService.insert(name, values);
			else if (Object.keys(values).length) await recordService.update(name, b.key, values);
			return true;
		});
		if (!ok) return false;
		buf = null;
		editRow = null;
		await load();
		return true;
	}

	/** Makes a cell current, saving the previous row first when it changes. */
	async function activate(row: Row, col: string | null) {
		if (!page) return;
		if (isView || !def) {
			current = row;
			currentCol = col;
			return;
		}
		if (editRow === row && buf) {
			currentCol = col;
			// Clicks inside the focused editor must not re-select its text.
			if ((document.activeElement as HTMLElement | null)?.dataset?.col !== col) focusCell();
			return;
		}
		const targetKey = row === 'new' ? null : page.rows[row]?.key;
		if (!(await commitRow())) return;
		let index: Row = row;
		if (targetKey) {
			const i = page.rows.findIndex((r) => keyId(r.key) === keyId(targetKey));
			if (i < 0) return;
			index = i;
		}
		current = index;
		currentCol = col ?? firstEditable();
		editRow = index;
		buf = rb.newBuffer(def, index === 'new' ? null : page.rows[index], page.columns);
		await tick();
		focusCell();
	}

	async function toggleCheckbox(row: Row, col: string) {
		await activate(row, col);
		if (buf && editRow !== null) rb.setValue(buf, col, buf.current[col] !== true);
	}

	function changed(col: string): boolean {
		return !!buf && JSON.stringify(buf.current[col]) !== JSON.stringify(buf.original[col]);
	}

	function editableCols(): string[] {
		const row = gridEl?.querySelector('tr.editing');
		return [...(row?.querySelectorAll<HTMLElement>('[data-col]') ?? [])].map((e) => e.dataset.col as string);
	}

	function moveHorizontal(col: string, delta: number) {
		const cols = editableCols();
		const i = cols.indexOf(col) + delta;
		if (i >= 0 && i < cols.length) {
			currentCol = cols[i];
			focusCell();
		} else if (i >= cols.length) {
			moveVertical(1, cols[0] ?? col);
		}
	}

	function moveVertical(delta: number, col: string | null) {
		if (!page) return;
		const n = page.rows.length;
		const from = editRow ?? current;
		const index = from === 'new' ? n : (from ?? -1);
		const target = index + delta;
		if (target < 0) return;
		if (target >= n) {
			if (isView) return;
			// Moving down from the new-record row saves it and starts another.
			if (editRow === 'new') {
				commitRow().then((ok) => {
					if (ok) activate('new', col);
				});
			} else {
				activate('new', col);
			}
			return;
		}
		activate(target, col);
	}

	function onGridKey(e: KeyboardEvent) {
		const target = e.target as HTMLElement;
		const col = target.dataset?.col;
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
		if (e.key === 'Enter' && col) {
			e.preventDefault();
			moveHorizontal(col, 1);
			return;
		}
		if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && col && target.tagName !== 'SELECT') {
			e.preventDefault();
			moveVertical(e.key === 'ArrowDown' ? 1 : -1, col);
			return;
		}
		if ((e.key === 'Delete' || e.key === 'Backspace') && !col && selection.rows.length) {
			e.preventDefault();
			deleteRecords();
		}
	}

	async function pickFile(col: string, input: HTMLInputElement) {
		const file = input.files?.[0];
		if (buf && file) rb.setValue(buf, col, await fileToAttachment(file));
		input.value = '';
	}

	// ----- records -----

	async function goToPosition(p: number) {
		if (!page || total === 0) return;
		const clamped = Math.max(0, Math.min(p, total - 1));
		if (!(await commitRow())) return;
		const target = pageOffsetFor(clamped, PAGE_SIZE);
		if (target !== offset) {
			offset = target;
			await load();
		}
		await activate(clamped - offset, currentCol ?? firstEditable());
	}

	function selectRow(i: number, e: MouseEvent) {
		selection = select(selection, i, { shift: e.shiftKey, toggle: e.metaKey || e.ctrlKey });
		if (editRow !== i) {
			commitRow().then((ok) => {
				if (ok) current = i;
			});
		}
		gridEl?.focus();
	}

	async function deleteRecords() {
		if (!page || isView) return;
		const rows = selection.rows.length ? selection.rows : typeof current === 'number' ? [current] : [];
		const keys = rows.map((i) => page!.rows[i]?.key).filter((k): k is RecordKey => !!k);
		if (!keys.length) {
			ui.notify('info', 'Nothing selected', 'Select records with the record selector on the left.');
			return;
		}
		const ok = await ui.confirm({
			title: `Delete ${keys.length} record${keys.length === 1 ? '' : 's'}?`,
			message: 'If you click Delete, you will not be able to undo this operation.',
			confirmLabel: 'Delete',
			danger: true
		});
		if (!ok) return;
		const deleted = await ui.attempt(() => recordService.delete(name, keys, true));
		if (deleted === undefined) return;
		buf = null;
		editRow = null;
		current = null;
		await load();
		ui.notify('success', `Deleted ${deleted} record${deleted === 1 ? '' : 's'}`);
	}

	async function applySort(column: string | null, descending = false) {
		if (!(await commitRow())) return;
		sort = column ? { column, descending } : null;
		offset = 0;
		current = null;
		await load();
	}

	function cycleSort(col: string) {
		if (sort?.column !== col) applySort(col, false);
		else if (!sort.descending) applySort(col, true);
		else applySort(null);
	}

	async function applyFilters() {
		if (!(await commitRow())) return;
		appliedFilters = filterRows.map((f) => ({
			column: f.column,
			op: f.op,
			value: f.op === 'is_null' || f.op === 'is_not_null' ? null : f.op === 'in' ? f.value.split(',').map((v) => v.trim()) : f.value
		}));
		offset = 0;
		current = null;
		await load();
	}

	async function clearFilters() {
		filterRows = [];
		await applyFilters();
	}

	function onSearchInput() {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(async () => {
			if (!(await commitRow())) return;
			appliedSearch = search.trim();
			offset = 0;
			current = null;
			await load();
		}, 300);
	}

	// ----- columns -----

	async function addColumn(type: FieldTypeName) {
		if (!(await commitRow())) return;
		const label = FIELD_TYPES.find((t) => t.name === type)?.label ?? 'Field';
		const fieldName = await ui.prompt({
			title: `New ${label} field`,
			label: 'Field name',
			value: nextFreeName('Field', columns),
			confirmLabel: 'Add field',
			validate: (v) =>
				identifierProblem(v, 'Field name') ??
				(columns.some((c) => c.toLowerCase() === v.toLowerCase()) ? 'A field with this name already exists' : null)
		});
		if (!fieldName) return;
		const result = await ui.attempt(() => tableService.addField(name, { name: fieldName, field_type: makeFieldType(type) }));
		if (result) await load();
	}

	function addColumnMenu(e: MouseEvent) {
		const quick: FieldTypeName[] = ['short_text', 'long_text', 'number', 'large_number', 'currency', 'date_time', 'yes_no', 'hyperlink', 'attachment'];
		ui.openMenu(
			e,
			[
				...quick.map((t) => ({ label: FIELD_TYPES.find((x) => x.name === t)!.label, action: () => addColumn(t) })),
				{ separator: true },
				{ label: 'More field types in Design View…', icon: 'design', action: () => session.switchView(docId, 'design') }
			]
		);
	}

	async function renameColumn(col: string) {
		if (!(await commitRow())) return;
		const newName = await ui.prompt({
			title: `Rename field "${col}"`,
			label: 'New name',
			value: col,
			confirmLabel: 'Rename',
			validate: (v) =>
				identifierProblem(v, 'Field name') ??
				(columns.some((c) => c !== col && c.toLowerCase() === v.toLowerCase()) ? 'A field with this name already exists' : null)
		});
		if (!newName || newName === col) return;
		if ((await ui.attempt(() => tableService.renameField(name, col, newName))) === undefined) return;
		if (sort?.column === col) sort = { ...sort, column: newName };
		await load();
	}

	async function deleteColumn(col: string) {
		if (!(await commitRow())) return;
		const ok = await ui.confirm({
			title: `Delete field "${col}"?`,
			message: 'The field and all the data in it will be permanently deleted.',
			confirmLabel: 'Delete field',
			danger: true
		});
		if (!ok) return;
		const result = await ui.attempt(() => tableService.dropField(name, col, true));
		if (!result) return;
		if (sort?.column === col) sort = null;
		appliedFilters = appliedFilters.filter((f) => f.column !== col);
		filterRows = filterRows.filter((f) => f.column !== col);
		await load();
		if (result.warnings.length) ui.notify('info', 'Field deleted', result.warnings.join(' '));
	}

	function headerMenu(e: MouseEvent, col: string) {
		ui.openMenu(e, [
			{ label: 'Sort A to Z', icon: 'sortAsc', action: () => applySort(col, false) },
			{ label: 'Sort Z to A', icon: 'sortDesc', action: () => applySort(col, true) },
			{ label: 'Remove Sort', icon: 'clearSort', disabled: !sort, action: () => applySort(null) },
			{ separator: true },
			{ label: 'Filter on this field…', icon: 'filter', action: () => { showFilter = true; filterRows.push({ column: col, op: 'eq', value: '' }); } },
			{ separator: true },
			{ label: 'Rename Field', icon: 'pencil', disabled: isView, action: () => renameColumn(col) },
			{ label: 'Delete Field', icon: 'trash', danger: true, disabled: isView || columns.length <= 1, action: () => deleteColumn(col) }
		]);
	}

	function rowMenu(e: MouseEvent, i: number) {
		if (!selection.rows.includes(i)) selection = select(selection, i);
		ui.openMenu(e, [
			{ label: 'New Record', icon: 'newRecord', disabled: isView, action: () => activate('new', firstEditable()) },
			{ label: selection.rows.length > 1 ? `Delete ${selection.rows.length} Records` : 'Delete Record', icon: 'trash', danger: true, disabled: isView, action: deleteRecords }
		]);
	}

	// ----- ribbon -----

	const sortColumn = () => currentCol ?? columns[0] ?? null;
	session.registerHandlers(docId, {
		refresh: async () => {
			if (await commitRow()) await load();
		},
		sortAscending: () => applySort(sortColumn(), false),
		sortDescending: () => applySort(sortColumn(), true),
		clearSort: () => applySort(null),
		toggleFilter: () => (showFilter = !showFilter),
		find: () => searchEl?.focus(),
		...(isView
			? {}
			: {
					save: commitRow,
					newRecord: () => activate('new', firstEditable()),
					deleteRecord: deleteRecords
				})
	});
	onDestroy(() => {
		clearTimeout(searchTimer);
		session.unregisterHandlers(docId);
	});

	load();
</script>

{#snippet editor(col: string, row: Row)}
	{@const f = fieldFor(col)}
	{@const b = buf!}
	{#if !f || isReadOnlyType(f.field_type)}
		<span class="ro" class:num={isNumeric(f)}>{row === 'new' && f?.field_type.type === 'auto_number' ? '(New)' : cellText(col, b.current[col] ?? (row === 'new' ? null : page!.rows[row as number].values[columns.indexOf(col)]))}</span>
	{:else if f.field_type.type === 'yes_no'}
		<span class="center"><input type="checkbox" data-col={col} checked={b.current[col] === true} onchange={(e) => rb.setValue(b, col, e.currentTarget.checked)} onfocus={() => (currentCol = col)} /></span>
	{:else if f.field_type.type === 'lookup' && lookups[col]}
		<select data-col={col} value={JSON.stringify(b.current[col] ?? null)} onchange={(e) => rb.setValue(b, col, JSON.parse(e.currentTarget.value))} onfocus={() => (currentCol = col)}>
			<option value="null"></option>
			{#each lookups[col] as o (JSON.stringify(o.value))}<option value={JSON.stringify(o.value)}>{o.label}</option>{/each}
		</select>
	{:else if f.field_type.type === 'attachment'}
		{@const v = b.current[col]}
		<span class="attach">
			<button data-col={col} class="mini" onclick={(e) => (e.currentTarget.nextElementSibling as HTMLInputElement).click()} onfocus={() => (currentCol = col)}>
				<Icon name="paperclip" size={13} />
				{v && typeof v === 'object' && 'base64' in v ? 'New file' : isBlobSummary(v) ? formatBytes(v.blob.size) : 'Attach…'}
			</button>
			<input type="file" hidden onchange={(e) => pickFile(col, e.currentTarget)} />
			{#if v !== null}<button class="mini icon" title="Remove attachment" aria-label="Remove attachment" onclick={() => rb.setValue(b, col, null)}><Icon name="close" size={12} /></button>{/if}
		</span>
	{:else}
		<input
			type="text"
			data-col={col}
			class:num={isNumeric(f)}
			value={String(b.current[col] ?? '')}
			placeholder={placeholderFor(f.field_type)}
			oninput={(e) => rb.setValue(b, col, e.currentTarget.value)}
			onfocus={() => (currentCol = col)}
		/>
	{/if}
{/snippet}

<div class="datasheet">
	{#if showFilter}
		<div class="filterbar">
			<span class="fb-title"><Icon name="filter" size={14} /> Filter</span>
			{#each filterRows as f, i (i)}
				<span class="cond">
					<select bind:value={f.column}>{#each columns as c (c)}<option value={c}>{c}</option>{/each}</select>
					<select bind:value={f.op}>{#each OPS as o (o.op)}<option value={o.op}>{o.label}</option>{/each}</select>
					{#if f.op !== 'is_null' && f.op !== 'is_not_null'}
						<input bind:value={f.value} placeholder={f.op === 'in' ? 'a, b, c' : 'value'} onkeydown={(e) => e.key === 'Enter' && applyFilters()} />
					{/if}
					<button class="icon" aria-label="Remove condition" onclick={() => filterRows.splice(i, 1)}><Icon name="close" size={13} /></button>
				</span>
			{/each}
			<button class="ghost" disabled={!columns.length} onclick={() => filterRows.push({ column: currentCol ?? columns[0], op: 'contains', value: '' })}><Icon name="plus" size={14} /> Condition</button>
			<span class="spacer"></span>
			<button class="primary" onclick={applyFilters}>Apply</button>
			<button onclick={clearFilters} disabled={!appliedFilters.length && !filterRows.length}>Clear</button>
			<button class="icon" aria-label="Close filter" onclick={() => (showFilter = false)}><Icon name="close" size={14} /></button>
		</div>
	{/if}

	<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
	<div class="grid-wrap" bind:this={gridEl} tabindex="0" role="grid" onkeydown={onGridKey}>
		{#if !loaded}
			<p class="muted pad">Loading…</p>
		{:else if page}
			<table class="grid">
				<thead>
					<tr>
						<th class="sel corner"></th>
						{#each columns as col (col)}
							{@const f = fieldFor(col)}
							<th
								class:sorted={sort?.column === col}
								class:num={isNumeric(f)}
								onclick={() => cycleSort(col)}
								oncontextmenu={(e) => headerMenu(e, col)}
								title={f?.description ?? col}
							>
								<span class="th">
									<span class="th-name">{#if f?.primary_key}<Icon name="key" size={12} />{/if}{col}</span>
									{#if sort?.column === col}<Icon name={sort.descending ? 'sortDesc' : 'sortAsc'} size={13} />{/if}
									<button class="hdr-menu icon" aria-label="Field options" onclick={(e) => { e.stopPropagation(); headerMenu(e, col); }}><Icon name="chevronDown" size={12} /></button>
								</span>
							</th>
						{/each}
						{#if !isView}
							<th class="add"><button class="ghost" onclick={addColumnMenu}>Click to Add <Icon name="chevronDown" size={12} /></button></th>
						{/if}
					</tr>
				</thead>
				<tbody>
					{#each page.rows as row, i (keyId(row.key) + ':' + i)}
						{@const editing = editRow === i && !!buf}
						<tr class:current={current === i} class:selected={selection.rows.includes(i)} class:editing class:alt={i % 2 === 1}>
							<td class="sel" onclick={(e) => selectRow(i, e)} oncontextmenu={(e) => rowMenu(e, i)}>
								{#if editing && dirty}<Icon name="pencil" size={12} />{:else if current === i}<Icon name="next" size={12} />{/if}
							</td>
							{#each columns as col, c (col)}
								{@const f = fieldFor(col)}
								<td class:num={isNumeric(f)} class:active-cell={current === i && currentCol === col} onclick={() => activate(i, col)}>
									{#if editing}
										{@render editor(col, i)}
									{:else if f?.field_type.type === 'yes_no'}
										<span class="center"><input type="checkbox" checked={row.values[c] === 1} tabindex="-1" disabled={isView} onclick={(e) => { e.preventDefault(); e.stopPropagation(); toggleCheckbox(i, col); }} /></span>
									{:else}
										<span class="text">{cellText(col, row.values[c])}</span>
									{/if}
								</td>
							{/each}
							{#if !isView}<td class="add"></td>{/if}
						</tr>
					{/each}
					{#if !isView}
						{@const editingNew = editRow === 'new' && !!buf}
						<tr class="new-row" class:editing={editingNew} class:current={current === 'new'}>
							<td class="sel" title="New record">{#if editingNew && dirty}<Icon name="pencil" size={12} />{:else}<span class="star">*</span>{/if}</td>
							{#each columns as col (col)}
								{@const f = fieldFor(col)}
								<td class:num={isNumeric(f)} onclick={() => activate('new', col)}>
									{#if editingNew}
										{@render editor(col, 'new')}
									{:else if f?.field_type.type === 'auto_number'}
										<span class="text muted">(New)</span>
									{/if}
								</td>
							{/each}
							<td class="add"></td>
						</tr>
					{/if}
				</tbody>
			</table>
			{#if page.rows.length === 0 && (appliedSearch || appliedFilters.length)}
				<p class="muted pad">No records match the current search or filter.</p>
			{/if}
		{/if}
	</div>

	<div class="navbar">
		<span class="nav-label">Record:</span>
		<button class="icon" title="First record" aria-label="First record" disabled={!total || position === 0} onclick={() => goToPosition(0)}><Icon name="first" size={14} /></button>
		<button class="icon" title="Previous record" aria-label="Previous record" disabled={!total || position === 0} onclick={() => goToPosition((position ?? 1) - 1)}><Icon name="prev" size={14} /></button>
		<input
			class="jump"
			aria-label="Current record"
			value={current === 'new' ? '(New)' : position !== null ? String(position + 1) : ''}
			oninput={(e) => (jump = e.currentTarget.value)}
			onkeydown={(e) => e.key === 'Enter' && Number(jump) > 0 && goToPosition(Number(jump) - 1)}
		/>
		<span class="of">of {total}</span>
		<button class="icon" title="Next record" aria-label="Next record" disabled={!total || (position ?? -1) >= total - 1} onclick={() => goToPosition((position ?? -1) + 1)}><Icon name="next" size={14} /></button>
		<button class="icon" title="Last record" aria-label="Last record" disabled={!total || position === total - 1} onclick={() => goToPosition(total - 1)}><Icon name="last" size={14} /></button>
		{#if !isView}
			<button class="icon" title="New (blank) record" aria-label="New record" onclick={() => activate('new', firstEditable())}><Icon name="newRecord" size={14} /></button>
		{/if}
		<span class="divider"></span>
		{#if appliedFilters.length}
			<button class="chip on" title="Remove filter" onclick={clearFilters}><Icon name="filter" size={12} /> Filtered</button>
		{:else}
			<button class="chip" onclick={() => (showFilter = !showFilter)}><Icon name="filter" size={12} /> No Filter</button>
		{/if}
		<label class="nav-search">
			<Icon name="search" size={13} />
			<input bind:this={searchEl} placeholder="Search" bind:value={search} oninput={onSearchInput} />
		</label>
		<span class="spacer"></span>
		{#if isView}<span class="muted">Read-only view</span>{:else if dirty}<span class="hint">Shift+Enter saves · Esc undoes</span>{/if}
	</div>
</div>

<style>
	.datasheet {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	.filterbar {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
		padding: 8px 10px;
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
	}
	.fb-title {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		font-weight: 600;
		color: var(--accent-text);
	}
	.cond {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 2px 2px 2px 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
	}
	.cond select,
	.cond input {
		padding: 3px 6px;
	}
	.spacer {
		flex: 1;
	}
	.grid-wrap {
		flex: 1;
		overflow: auto;
		min-height: 0;
		outline: none;
		background: var(--bg);
	}
	.pad {
		padding: 16px;
		margin: 0;
	}
	.grid {
		border-collapse: separate;
		border-spacing: 0;
		min-width: 100%;
		width: max-content;
	}
	th,
	td {
		height: 28px;
		padding: 0;
		border-right: 1px solid var(--border);
		border-bottom: 1px solid var(--border);
		text-align: left;
		white-space: nowrap;
	}
	th {
		position: sticky;
		top: 0;
		z-index: 2;
		min-width: 130px;
		max-width: 520px;
		background: var(--surface-2);
		font-weight: 600;
		cursor: pointer;
		resize: horizontal;
		overflow: hidden;
		user-select: none;
	}
	th:hover {
		background: var(--surface-3);
	}
	th.sorted {
		color: var(--accent-text);
	}
	.th {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 4px 0 10px;
	}
	.th-name {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		overflow: hidden;
		text-overflow: ellipsis;
		flex: 1;
	}
	.th-name :global(svg) {
		color: var(--warning);
	}
	.hdr-menu {
		opacity: 0;
		padding: 2px;
	}
	th:hover .hdr-menu {
		opacity: 1;
	}
	th.add {
		min-width: 120px;
		resize: none;
		cursor: default;
	}
	th.add button {
		width: 100%;
		height: 100%;
		border-radius: 0;
		color: var(--text-muted);
		font-weight: 400;
		justify-content: space-between;
	}
	.sel {
		position: sticky;
		left: 0;
		z-index: 1;
		width: 26px;
		min-width: 26px;
		background: var(--surface-2);
		color: var(--accent-text);
		cursor: pointer;
		text-align: center;
	}
	.sel :global(svg) {
		margin: 0 auto;
	}
	th.corner {
		z-index: 3;
		min-width: 26px;
		resize: none;
	}
	td {
		max-width: 520px;
		background: var(--bg);
		cursor: cell;
	}
	tr.alt td:not(.sel) {
		background: var(--grid-alt);
	}
	tr.selected td:not(.sel) {
		background: var(--accent-soft);
	}
	tr.selected .sel,
	tr.current .sel {
		background: var(--surface-4);
	}
	tr.editing td:not(.sel) {
		background: var(--surface);
	}
	td.active-cell {
		box-shadow: inset 0 0 0 1px var(--focus);
	}
	td.add {
		background: transparent !important;
		cursor: default;
	}
	.text,
	.ro {
		display: block;
		padding: 0 10px;
		overflow: hidden;
		text-overflow: ellipsis;
		line-height: 28px;
	}
	.ro {
		color: var(--text-muted);
	}
	.num,
	.num .text {
		text-align: right;
	}
	td input[type='text'],
	td select {
		width: 100%;
		height: 27px;
		padding: 0 9px;
		border: none;
		border-radius: 0;
		background: transparent;
		box-shadow: none;
	}
	td input[type='text']:focus,
	td select:focus {
		box-shadow: inset 0 0 0 2px var(--focus);
		background: var(--surface-2);
	}
	td input.num {
		text-align: right;
	}
	.center {
		display: flex;
		justify-content: center;
		align-items: center;
		height: 27px;
	}
	.attach {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 4px;
	}
	.mini {
		padding: 1px 7px;
		font-size: 12px;
		gap: 4px;
	}
	.new-row .star {
		font-weight: 700;
		color: var(--text-muted);
	}
	.new-row td:not(.sel) {
		background: var(--bg);
	}
	.navbar {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 8px;
		background: var(--surface);
		border-top: 1px solid var(--border);
		font-size: 12px;
	}
	.nav-label {
		color: var(--text-muted);
		margin-right: 2px;
	}
	.navbar button.icon {
		padding: 3px;
	}
	.jump {
		width: 60px;
		padding: 2px 6px;
		text-align: center;
	}
	.of {
		color: var(--text-muted);
		margin: 0 4px;
	}
	.divider {
		width: 1px;
		height: 18px;
		margin: 0 6px;
		background: var(--border);
	}
	.chip {
		padding: 2px 8px;
		border-radius: 999px;
		font-size: 12px;
		background: transparent;
		color: var(--text-muted);
	}
	.chip.on {
		border-color: var(--accent);
		color: var(--accent-text);
		background: var(--accent-soft);
	}
	.nav-search {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-left: 6px;
		padding-left: 6px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		color: var(--text-muted);
	}
	.nav-search input {
		width: 160px;
		padding: 2px 6px 2px 0;
		border: none;
		background: transparent;
		box-shadow: none;
	}
	.hint {
		color: var(--text-faint);
	}
</style>
