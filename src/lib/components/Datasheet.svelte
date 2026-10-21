<script lang="ts">
	import type { FieldDef, FilterOp, JsonValue, RecordKey, RecordPage, RecordRow, TableDef } from '../api/types';
	import { isReadOnlyType } from '../logic/field-types';
	import * as rb from '../logic/record-buffer';
	import { displayValue, fileToAttachment, inputKind, placeholderFor } from '../logic/values';
	import { confirmAction } from '../services/dialogs';
	import { recordService } from '../services/records';
	import { tableService } from '../services/tables';
	import { app } from '../state/app.svelte';

	let { name }: { name: string } = $props();

	interface FilterRow {
		column: string;
		op: FilterOp;
		value: string;
	}

	const OPS: FilterOp[] = ['eq', 'ne', 'lt', 'le', 'gt', 'ge', 'contains', 'starts_with', 'ends_with', 'is_null', 'is_not_null', 'in'];

	const isView = app.objects.some((o) => o.name === name && o.kind === 'view');
	let def = $state<TableDef | null>(null);
	let page = $state<RecordPage | null>(null);
	let search = $state('');
	let sortColumn = $state('');
	let sortDesc = $state(false);
	let filters = $state<FilterRow[]>([]);
	let pageSize = $state(50);
	let offset = $state(0);
	let selected = $state<Record<string, RecordKey>>({});
	let editor = $state<rb.RecordBuffer | null>(null);
	let attachmentLinks = $state<Record<string, string>>({});

	const keyId = (k: RecordKey | null) => JSON.stringify(k);
	const fieldFor = (col: string): FieldDef | undefined => def?.fields.find((f) => f.name === col);

	function filterValue(f: FilterRow): JsonValue {
		if (f.op === 'is_null' || f.op === 'is_not_null') return null;
		if (f.op === 'in') return f.value.split(',').map((v) => v.trim());
		return f.value;
	}

	async function reload() {
		await app.run(async () => {
			if (!isView && !def) def = (await tableService.get(name)).definition;
			page = await recordService.list(name, {
				offset,
				limit: pageSize,
				sort: sortColumn ? [{ column: sortColumn, descending: sortDesc }] : [],
				filters: filters.map((f) => ({ column: f.column, op: f.op, value: filterValue(f) })),
				search: search.trim() || null
			});
			selected = {};
		});
	}
	reload();

	function applyQuery() {
		offset = 0;
		reload();
	}

	function goTo(newOffset: number) {
		offset = Math.max(0, newOffset);
		reload();
	}

	async function leaveEditor(): Promise<boolean> {
		if (editor && rb.isDirty(editor) && !(await confirmAction('Discard unsaved changes to this record?'))) return false;
		editor = null;
		attachmentLinks = {};
		return true;
	}

	async function edit(row: RecordRow | null) {
		if (!def || !page || !(await leaveEditor())) return;
		editor = rb.newBuffer(def, row, page.columns);
	}

	async function saveRecord() {
		if (!editor || !def) return;
		const missing = rb.missingRequired(editor, def);
		if (missing.length) {
			app.error = `Required fields are empty: ${missing.join(', ')}`;
			return;
		}
		const buf = editor;
		const values = rb.submission(buf, def);
		await app.run(async () => {
			if (buf.key === null) {
				await recordService.insert(name, values);
			} else if (Object.keys(values).length) {
				await recordService.update(name, buf.key, values);
			}
			editor = null;
			await reload();
			app.message = buf.key === null ? 'Record added' : 'Record saved';
		});
	}

	async function deleteSelected() {
		const keys = Object.values(selected);
		if (!keys.length) return;
		if (!(await confirmAction(`Delete ${keys.length} record(s)? This cannot be undone.`))) return;
		await app.run(async () => {
			const n = await recordService.delete(name, keys, true);
			editor = null;
			await reload();
			app.message = `Deleted ${n} record(s)`;
		});
	}

	function toggle(key: RecordKey | null, on: boolean) {
		if (!key) return;
		if (on) selected[keyId(key)] = key;
		else delete selected[keyId(key)];
	}

	async function pickFile(column: string, input: HTMLInputElement) {
		const file = input.files?.[0];
		if (editor && file) rb.setValue(editor, column, await fileToAttachment(file));
	}

	async function loadAttachment(column: string) {
		if (!editor?.key) return;
		const key = editor.key;
		await app.run(async () => {
			const data = await recordService.getAttachment(name, key, column);
			if (data && typeof data === 'object' && 'base64' in data) {
				attachmentLinks[column] = `data:application/octet-stream;base64,${data.base64}`;
			}
		});
	}
</script>

<section>
	<h2>{isView ? 'View' : 'Table'}: {name}</h2>

	<p>
		<label>Search <input bind:value={search} onkeydown={(e) => e.key === 'Enter' && applyQuery()} /></label>
		<label>Sort by
			<select bind:value={sortColumn}>
				<option value="">(default)</option>
				{#each page?.columns ?? [] as c (c)}<option value={c}>{c}</option>{/each}
			</select>
		</label>
		<label><input type="checkbox" bind:checked={sortDesc} /> Descending</label>
		<label>Page size <input type="number" min="1" max="1000" bind:value={pageSize} /></label>
		<button onclick={applyQuery}>Apply</button>
	</p>

	<fieldset>
		<legend>Filters</legend>
		{#each filters as f, i (i)}
			<p>
				<select bind:value={f.column}>
					{#each page?.columns ?? [] as c (c)}<option value={c}>{c}</option>{/each}
				</select>
				<select bind:value={f.op}>{#each OPS as op (op)}<option value={op}>{op}</option>{/each}</select>
				{#if f.op !== 'is_null' && f.op !== 'is_not_null'}
					<input bind:value={f.value} placeholder={f.op === 'in' ? 'a, b, c' : 'value'} />
				{/if}
				<button onclick={() => filters.splice(i, 1)}>Remove</button>
			</p>
		{/each}
		<button disabled={!page?.columns.length} onclick={() => filters.push({ column: page!.columns[0], op: 'eq', value: '' })}>Add filter</button>
		<button onclick={applyQuery}>Apply filters</button>
	</fieldset>

	{#if page}
		<p>
			Records {page.total === 0 ? 0 : page.offset + 1}–{Math.min(page.offset + page.rows.length, page.total)} of {page.total}
			<button disabled={offset === 0} onclick={() => goTo(0)}>First</button>
			<button disabled={offset === 0} onclick={() => goTo(offset - pageSize)}>Previous</button>
			<button disabled={offset + pageSize >= page.total} onclick={() => goTo(offset + pageSize)}>Next</button>
			<button disabled={offset + pageSize >= page.total} onclick={() => goTo(Math.floor((page!.total - 1) / pageSize) * pageSize)}>Last</button>
		</p>
		{#if !isView}
			<p>
				<button disabled={app.busy} onclick={() => edit(null)}>New record</button>
				<button disabled={app.busy || Object.keys(selected).length === 0} onclick={deleteSelected}>
					Delete selected ({Object.keys(selected).length})
				</button>
			</p>
		{/if}
		<table>
			<thead>
				<tr>
					{#if !isView}<th>Select</th><th></th>{/if}
					{#each page.columns as c (c)}<th>{c}</th>{/each}
				</tr>
			</thead>
			<tbody>
				{#each page.rows as row (keyId(row.key) + row.values.join())}
					<tr>
						{#if !isView}
							<td><input type="checkbox" checked={keyId(row.key) in selected} onchange={(e) => toggle(row.key, e.currentTarget.checked)} /></td>
							<td><button onclick={() => edit(row)}>Edit</button></td>
						{/if}
						{#each row.values as v, i (i)}<td>{displayValue(v, fieldFor(page.columns[i])?.field_type)}</td>{/each}
					</tr>
				{:else}
					<tr><td colspan={page.columns.length + 2}>No records</td></tr>
				{/each}
			</tbody>
		</table>
	{/if}

	{#if editor && def}
		{@const buf = editor}
		<fieldset>
			<legend>{buf.key === null ? 'New record' : 'Edit record'}</legend>
			{#each def.fields as f (f.name)}
				{@const kind = inputKind(f.field_type)}
				{@const changed = JSON.stringify(buf.current[f.name]) !== JSON.stringify(buf.original[f.name])}
				<p>
					<label>
						{f.name}{f.required || (f.primary_key && f.field_type.type !== 'auto_number') ? ' *' : ''}
						{#if kind === 'readonly' || isReadOnlyType(f.field_type)}
							<output>{displayValue(buf.current[f.name], f.field_type) || placeholderFor(f.field_type) || '(calculated)'}</output>
						{:else if kind === 'checkbox'}
							<input type="checkbox" checked={buf.current[f.name] === true} onchange={(e) => rb.setValue(buf, f.name, e.currentTarget.checked)} />
						{:else if kind === 'textarea'}
							<textarea rows="3" cols="50" value={String(buf.current[f.name] ?? '')} oninput={(e) => rb.setValue(buf, f.name, e.currentTarget.value)}></textarea>
						{:else if kind === 'file'}
							<output>{displayValue(buf.current[f.name]) || '(empty)'}</output>
							<input type="file" onchange={(e) => pickFile(f.name, e.currentTarget)} />
							<button onclick={() => rb.setValue(buf, f.name, null)}>Clear</button>
							{#if buf.key !== null && buf.original[f.name] !== null}
								<button onclick={() => loadAttachment(f.name)}>Load file</button>
								{#if attachmentLinks[f.name]}<a href={attachmentLinks[f.name]} download={f.name}>Save file</a>{/if}
							{/if}
						{:else}
							<input value={String(buf.current[f.name] ?? '')} placeholder={placeholderFor(f.field_type)} oninput={(e) => rb.setValue(buf, f.name, e.currentTarget.value)} />
						{/if}
					</label>
					{#if changed}<button onclick={() => rb.undoField(buf, f.name)}>Undo</button>{/if}
					{#if f.description}<small>{f.description}</small>{/if}
				</p>
			{/each}
			<p>
				<button disabled={app.busy} onclick={saveRecord}>Save record</button>
				<button disabled={!rb.isDirty(buf)} onclick={() => rb.undoAll(buf)}>Undo all changes</button>
				<button onclick={leaveEditor}>Close</button>
			</p>
		</fieldset>
	{/if}
</section>
