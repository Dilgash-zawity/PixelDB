<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import type { DefaultValue, FieldDef, FkAction, TableDef, TableInfo } from '../api/types';
	import { FIELD_TYPES } from '../logic/field-types';
	import * as d from '../logic/table-draft';
	import { identifierProblem } from '../logic/values';
	import { tableService } from '../services/tables';
	import { session, type Doc } from '../state/session.svelte';
	import { ui } from '../state/ui.svelte';
	import Icon from './Icon.svelte';

	let { doc }: { doc: Doc } = $props();

	// svelte-ignore state_referenced_locally
	const docId = doc.id;

	const FK_ACTIONS: { value: FkAction; label: string }[] = [
		{ value: 'no_action', label: 'No action' },
		{ value: 'restrict', label: 'Restrict' },
		{ value: 'cascade', label: 'Cascade' },
		{ value: 'set_null', label: 'Set null' },
		{ value: 'set_default', label: 'Set default' }
	];
	const DEFAULT_KINDS = [
		{ value: 'none', label: '(none)' },
		{ value: 'literal', label: 'Value' },
		{ value: 'current_timestamp', label: 'Now (date and time)' },
		{ value: 'current_date', label: 'Today (date)' },
		{ value: 'current_time', label: 'Current time' },
		{ value: 'expression', label: 'Expression' }
	];
	const HELP: Record<string, string> = {
		name: 'A field name can be up to 128 characters long, including spaces.',
		type: 'The data type determines the kind of values that users can store in the field.',
		description: 'The field description is optional. It helps you describe the field and is shown as a tooltip in Datasheet View.',
		size_text: 'The maximum number of characters that can be entered in the field. Leave empty for no limit.',
		size_number: 'Long Integer stores whole numbers. Double stores numbers with decimal places.',
		scale: 'The number of decimal places. Amounts are stored exactly as whole numbers of the smallest unit (e.g. cents).',
		expression: 'The expression that calculates this field from other fields in the same record, for example: Quantity * UnitPrice',
		result: 'The type of value the expression produces.',
		stored: 'Stored values are computed when a record is saved; otherwise they are computed each time they are read.',
		default: 'A value that is automatically entered in this field for new records.',
		check: 'An expression that values must satisfy, using the field name, for example: Quantity >= 0',
		required: 'Require data entry in this field?',
		indexed: 'Yes (No Duplicates) prevents two records from having the same value in this field.',
		declared: 'The SQLite type declared for this column by the database that created it.',
		row_source: 'The table whose records this field refers to.',
		bound: 'The field in the related table whose value is stored. Usually its primary key.',
		display: 'The field shown instead of the stored value in Datasheet View.',
		key_type: 'Whether the stored value is a whole number or text.',
		on_delete: 'What happens to this record when the related record is deleted.',
		on_update: 'What happens to this record when the related key value changes.'
	};

	let info = $state<TableInfo | null>(null);
	let draft = $state(d.newDraft());
	let baseline = $state('');
	let selectedId = $state<number | null>(null);
	let tab = $state<'general' | 'lookup'>('general');
	let showSheet = $state(false);
	let help = $state(HELP.name);
	let newFieldName = $state('');
	let newUnique = $state('');
	let targetDefs = $state<Record<string, TableDef>>({});
	let gridEl = $state<HTMLDivElement | null>(null);

	const selected = $derived(draft.fields.find((f) => f.id === selectedId) ?? null);
	const dirty = $derived(baseline !== '' && JSON.stringify(d.toDesign(draft)) !== baseline);
	const blocked = $derived(!!info?.unsupported_features.length);

	$effect(() => session.setDirty(docId, dirty));

	// Fetch the related table's definition for the Lookup properties.
	$effect(() => {
		const fk = selected ? d.lookupTarget(draft, selected.field.name) : undefined;
		const target = fk?.ref_table;
		if (!target || targetDefs[target] || target.toLowerCase() === draft.name.toLowerCase()) return;
		tableService.get(target).then(
			(t) => (targetDefs[target] = t.definition),
			() => {}
		);
	});

	function reset(def: TableDef | null) {
		draft = def ? d.draftFromDefinition(def) : d.newDraft('');
		baseline = JSON.stringify(d.toDesign(draft));
		selectedId = draft.fields[0]?.id ?? null;
	}

	async function load() {
		if (doc.name === null) {
			reset(null);
			return;
		}
		const name = doc.name;
		await ui.attempt(async () => {
			info = await tableService.get(name);
			reset(info.definition);
		});
	}

	function targetFields(table: string | undefined): FieldDef[] {
		if (!table) return [];
		if (table.toLowerCase() === draft.name.toLowerCase()) return draft.fields.map((f) => f.field);
		return targetDefs[table]?.fields ?? [];
	}

	async function focusRow(id: number) {
		selectedId = id;
		await tick();
		gridEl?.querySelector<HTMLInputElement>(`tr[data-id="${id}"] input`)?.focus();
	}

	function addNamedField() {
		const name = newFieldName.trim();
		if (!name) return;
		const f = d.addField(draft, 'short_text', name);
		newFieldName = '';
		selectedId = f.id;
	}

	function insertRow() {
		const index = selected ? draft.fields.indexOf(selected) : draft.fields.length;
		focusRow(d.insertField(draft, index).id);
	}

	function deleteRow() {
		if (!selected) return;
		const index = draft.fields.indexOf(selected);
		d.removeField(draft, selected.id);
		selectedId = draft.fields[Math.min(index, draft.fields.length - 1)]?.id ?? null;
	}

	function rowMenu(e: MouseEvent, id: number) {
		selectedId = id;
		ui.openMenu(e, [
			{ label: 'Primary Key', icon: 'key', action: () => d.togglePrimaryKey(draft, id) },
			{ separator: true },
			{ label: 'Insert Rows', icon: 'insertRow', action: insertRow },
			{ label: 'Delete Rows', icon: 'deleteRow', danger: true, action: deleteRow }
		]);
	}

	function defaultKind(f: FieldDef): string {
		return f.default?.kind ?? 'none';
	}

	function setDefaultKind(f: FieldDef, kind: string) {
		if (kind === 'none') f.default = null;
		else if (kind === 'literal' || kind === 'expression') f.default = { kind, value: '' };
		else f.default = { kind } as DefaultValue;
	}

	function defaultText(f: FieldDef): string {
		return f.default && 'value' in f.default ? String(f.default.value ?? '') : '';
	}

	function setDefaultText(f: FieldDef, text: string) {
		if (f.default?.kind === 'literal') f.default = { kind: 'literal', value: text };
		else if (f.default?.kind === 'expression') f.default = { kind: 'expression', value: text };
	}

	function addUnique() {
		const cols = newUnique.split(',').map((c) => c.trim()).filter(Boolean);
		if (cols.length > 1) draft.unique_constraints.push(cols);
		else if (cols.length === 1) ui.notify('info', 'Use the Indexed property', 'For a single field, set Indexed to "Yes (No Duplicates)".');
		newUnique = '';
	}

	function reportProblems(problems: string[]): boolean {
		if (!problems.length) return false;
		ui.notify('error', 'The design has problems', problems.join(' · '));
		return true;
	}

	async function save(): Promise<boolean> {
		if (blocked) {
			ui.notify('error', 'Not supported', 'This table uses features the designer cannot preserve.');
			return false;
		}
		if (draft.isNew) {
			if (reportProblems(d.validateDraft({ ...draft, name: 'Table' }))) return false;
			const name = await ui.prompt({
				title: 'Save As',
				label: 'Table Name',
				value: d.nextFreeName('Table', session.objects.map((o) => o.name)),
				confirmLabel: 'Save',
				validate: (v) => session.tableNameProblem(v)
			});
			if (!name) return false;
			draft.name = name;
			const created = await ui.attempt(() => tableService.create(d.toDefinition(draft)));
			if (!created) {
				draft.name = '';
				return false;
			}
			info = created;
			reset(created.definition);
			session.adoptName(docId, created.definition.name);
			await session.refreshObjects();
			ui.notify('success', `Created table "${created.definition.name}"`);
			return true;
		}
		if (!info || reportProblems(d.validateDraft(draft))) return false;
		const table = info.definition.name;
		const removed = d.removedFields(draft, info.definition);
		let confirmed = false;
		if (removed.length) {
			confirmed = await ui.confirm({
				title: 'Delete fields?',
				message: `Saving permanently deletes the field${removed.length === 1 ? '' : 's'} ${removed.join(', ')} and all the data in ${removed.length === 1 ? 'it' : 'them'}.`,
				confirmLabel: 'Delete and save',
				danger: true
			});
			if (!confirmed) return false;
		}
		const result = await ui.attempt(() => tableService.saveDesign(table, d.toDesign(draft), confirmed));
		if (!result) return false;
		const selectedName = selected?.field.name;
		info = await tableService.get(table);
		reset(info.definition);
		selectedId = draft.fields.find((f) => f.field.name === selectedName)?.id ?? selectedId;
		await session.refreshObjects();
		ui.notify('success', 'Design saved', [result.rebuilt ? 'The table was rebuilt and your data was kept.' : '', ...result.warnings].filter(Boolean).join(' '));
		return true;
	}

	session.registerHandlers(docId, {
		save,
		refresh: async () => {
			if (
				dirty &&
				!(await ui.confirm({ title: 'Discard changes?', message: 'Reload the saved design and discard your changes?', confirmLabel: 'Discard', danger: true }))
			)
				return;
			await load();
		},
		primaryKey: () => selected && d.togglePrimaryKey(draft, selected.id),
		insertRow,
		deleteRow,
		propertySheet: () => (showSheet = !showSheet)
	});
	onDestroy(() => session.unregisterHandlers(docId));

	load();
</script>

<div class="design" class:with-sheet={showSheet}>
	<div class="main">
		{#if blocked && info}
			<div class="banner">
				<Icon name="warning" />
				<span>
					This table was created outside Database Builder and uses features the designer cannot preserve
					({info.unsupported_features.join(', ')}). You can still rename fields from the datasheet column menu.
				</span>
			</div>
		{/if}

		<div class="upper" bind:this={gridEl}>
			<table class="fields">
				<thead>
					<tr><th class="sel"></th><th>Field Name</th><th>Data Type</th><th>Description (Optional)</th></tr>
				</thead>
				<tbody>
					{#each draft.fields as df (df.id)}
						{@const f = df.field}
						<tr data-id={df.id} class:selected={selectedId === df.id} onclick={() => (selectedId = df.id)} oncontextmenu={(e) => rowMenu(e, df.id)}>
							<td class="sel">
								{#if f.primary_key}<span class="key" title="Primary key"><Icon name="key" size={14} /></span>{:else if selectedId === df.id}<Icon name="next" size={12} />{/if}
							</td>
							<td>
								<input
									value={f.name}
									class:invalid={!!identifierProblem(f.name)}
									oninput={(e) => d.renameField(draft, df.id, e.currentTarget.value)}
									onfocus={() => { selectedId = df.id; help = HELP.name; }}
								/>
								{#if df.originalName && df.originalName !== f.name}<span class="was">was {df.originalName}</span>{/if}
							</td>
							<td>
								<select
									value={f.field_type.type}
									onchange={(e) => d.setFieldType(draft, df.id, e.currentTarget.value as never)}
									onfocus={() => { selectedId = df.id; help = HELP.type; }}
								>
									{#each FIELD_TYPES as ft (ft.name)}<option value={ft.name}>{ft.label}</option>{/each}
									{#if f.field_type.type === 'unknown'}<option value="unknown">Other ({f.field_type.declared || 'untyped'})</option>{/if}
								</select>
							</td>
							<td>
								<input
									value={f.description ?? ''}
									oninput={(e) => (f.description = e.currentTarget.value || null)}
									onfocus={() => { selectedId = df.id; help = HELP.description; }}
								/>
							</td>
						</tr>
					{/each}
					<tr class="blank">
						<td class="sel"><Icon name="plus" size={12} /></td>
						<td>
							<input
								placeholder="Add a field…"
								bind:value={newFieldName}
								onkeydown={(e) => e.key === 'Enter' && addNamedField()}
								onblur={addNamedField}
								onfocus={() => (help = HELP.name)}
							/>
						</td>
						<td colspan="2" class="muted hint">Type a name and press Enter</td>
					</tr>
				</tbody>
			</table>
		</div>

		<div class="lower">
			<div class="props">
				<div class="props-head">
					<span class="title">Field Properties</span>
					<div class="ptabs">
						<button class:active={tab === 'general'} onclick={() => (tab = 'general')}>General</button>
						<button class:active={tab === 'lookup'} onclick={() => (tab = 'lookup')}>Lookup</button>
					</div>
				</div>
				{#if selected}
					{@const f = selected.field}
					{@const t = f.field_type}
					{@const readonlyType = t.type === 'auto_number' || t.type === 'calculated'}
					<div class="ptable">
						{#if tab === 'general'}
							{#if t.type === 'short_text'}
								<label for="p-size">Field Size</label>
								<input id="p-size" type="number" min="1" placeholder="No limit" value={t.max_length ?? ''} onfocus={() => (help = HELP.size_text)} oninput={(e) => (t.max_length = e.currentTarget.value === '' ? null : Number(e.currentTarget.value))} />
							{:else if t.type === 'number'}
								<label for="p-size">Field Size</label>
								<select id="p-size" value={t.real ? 'double' : 'long'} onfocus={() => (help = HELP.size_number)} onchange={(e) => (t.real = e.currentTarget.value === 'double')}>
									<option value="long">Long Integer</option><option value="double">Double</option>
								</select>
							{:else if t.type === 'currency'}
								<label for="p-scale">Decimal Places</label>
								<input id="p-scale" type="number" min="0" max="6" value={t.scale ?? 2} onfocus={() => (help = HELP.scale)} oninput={(e) => (t.scale = Number(e.currentTarget.value))} />
							{:else if t.type === 'calculated'}
								<label for="p-expr">Expression</label>
								<input id="p-expr" class="mono" bind:value={t.expression} placeholder="Quantity * UnitPrice" onfocus={() => (help = HELP.expression)} />
								<label for="p-result">Result Type</label>
								<select id="p-result" bind:value={t.result} onfocus={() => (help = HELP.result)}>
									<option value="any">Automatic</option><option value="text">Text</option>
									<option value="integer">Whole number</option><option value="real">Decimal number</option>
								</select>
								<label for="p-stored">Stored</label>
								<select id="p-stored" value={t.stored ? 'yes' : 'no'} onfocus={() => (help = HELP.stored)} onchange={(e) => (t.stored = e.currentTarget.value === 'yes')}>
									<option value="no">No</option><option value="yes">Yes</option>
								</select>
							{:else if t.type === 'unknown'}
								<label for="p-decl">Declared Type</label>
								<input id="p-decl" class="mono" bind:value={t.declared} onfocus={() => (help = HELP.declared)} />
							{/if}
							{#if !readonlyType}
								<label for="p-default">Default Value</label>
								<div class="pair">
									<select id="p-default" value={defaultKind(f)} onfocus={() => (help = HELP.default)} onchange={(e) => setDefaultKind(f, e.currentTarget.value)}>
										{#each DEFAULT_KINDS as k (k.value)}<option value={k.value}>{k.label}</option>{/each}
									</select>
									{#if f.default?.kind === 'literal' || f.default?.kind === 'expression'}
										<input class:mono={f.default.kind === 'expression'} aria-label="Default value" value={defaultText(f)} onfocus={() => (help = HELP.default)} oninput={(e) => setDefaultText(f, e.currentTarget.value)} />
									{/if}
								</div>
							{/if}
							<label for="p-check">Validation Rule</label>
							<input id="p-check" class="mono" value={f.check ?? ''} placeholder={`${f.name} >= 0`} onfocus={() => (help = HELP.check)} oninput={(e) => (f.check = e.currentTarget.value || null)} />
							{#if !readonlyType}
								<label for="p-req">Required</label>
								<select id="p-req" value={f.required ? 'yes' : 'no'} onfocus={() => (help = HELP.required)} onchange={(e) => (f.required = e.currentTarget.value === 'yes')}>
									<option value="no">No</option><option value="yes">Yes</option>
								</select>
							{/if}
							<label for="p-idx">Indexed</label>
							<select id="p-idx" value={f.unique || f.primary_key ? 'unique' : 'no'} disabled={f.primary_key} onfocus={() => (help = HELP.indexed)} onchange={(e) => (f.unique = e.currentTarget.value === 'unique')}>
								<option value="no">No</option><option value="unique">Yes (No Duplicates)</option>
							</select>
						{:else if t.type === 'lookup'}
							{@const fk = d.lookupTarget(draft, f.name)}
							<label for="p-src">Row Source</label>
							<select id="p-src" value={fk?.ref_table ?? ''} onfocus={() => (help = HELP.row_source)} onchange={(e) => d.setLookupTarget(draft, selected.id, e.currentTarget.value, '')}>
								<option value="">(choose a table)</option>
								{#each session.tables as tbl (tbl.name)}<option value={tbl.name}>{tbl.name}</option>{/each}
								{#if draft.name && !session.tables.some((x) => x.name === draft.name)}<option value={draft.name}>{draft.name}</option>{/if}
							</select>
							{#if fk}
								{@const fields = targetFields(fk.ref_table)}
								<label for="p-bound">Bound Column</label>
								<select id="p-bound" value={fk.ref_columns?.[0] ?? ''} onfocus={() => (help = HELP.bound)} onchange={(e) => (fk.ref_columns = e.currentTarget.value ? [e.currentTarget.value] : [])}>
									<option value="">(primary key)</option>
									{#each fields as tf (tf.name)}<option value={tf.name}>{tf.name}</option>{/each}
								</select>
								<label for="p-display">Display Column</label>
								<select id="p-display" value={t.display_column ?? ''} onfocus={() => (help = HELP.display)} onchange={(e) => (t.display_column = e.currentTarget.value || null)}>
									<option value="">(bound column)</option>
									{#each fields as tf (tf.name)}<option value={tf.name}>{tf.name}</option>{/each}
								</select>
								<label for="p-del">On Delete</label>
								<select id="p-del" bind:value={fk.on_delete} onfocus={() => (help = HELP.on_delete)}>
									{#each FK_ACTIONS as a (a.value)}<option value={a.value}>{a.label}</option>{/each}
								</select>
								<label for="p-upd">On Update</label>
								<select id="p-upd" bind:value={fk.on_update} onfocus={() => (help = HELP.on_update)}>
									{#each FK_ACTIONS as a (a.value)}<option value={a.value}>{a.label}</option>{/each}
								</select>
							{/if}
							<label for="p-kt">Key Type</label>
							<select id="p-kt" bind:value={t.value_type} onfocus={() => (help = HELP.key_type)}>
								<option value="integer">Whole number</option><option value="text">Text</option>
							</select>
						{:else}
							<p class="note">
								This field is not a lookup field.
								<button class="ghost" onclick={() => { d.setFieldType(draft, selected.id, 'lookup'); help = HELP.row_source; }}>Change to Lookup</button>
							</p>
						{/if}
					</div>
				{:else}
					<p class="note">Select a field to see its properties.</p>
				{/if}
			</div>
			<aside class="help">
				<Icon name="info" />
				<p>{help}</p>
			</aside>
		</div>
	</div>

	{#if showSheet}
		<aside class="sheet">
			<header>
				<span>Property Sheet</span>
				<button class="icon" aria-label="Close property sheet" onclick={() => (showSheet = false)}><Icon name="close" size={14} /></button>
			</header>
			<div class="sheet-body">
				<h4>Table Properties</h4>
				<label class="block">
					<span>Description</span>
					<textarea rows="3" bind:value={draft.description}></textarea>
				</label>

				<h4>Validation Rules</h4>
				{#each draft.checks as _, i (i)}
					<div class="line">
						<input class="mono" bind:value={draft.checks[i]} placeholder="EndDate >= StartDate" />
						<button class="icon" aria-label="Remove rule" onclick={() => draft.checks.splice(i, 1)}><Icon name="close" size={13} /></button>
					</div>
				{/each}
				<button class="ghost" onclick={() => draft.checks.push('')}><Icon name="plus" size={14} /> Add rule</button>

				<h4>Unique Field Combinations</h4>
				{#each draft.unique_constraints as u, i (i)}
					<div class="line"><span>{u.join(' + ')}</span><button class="icon" aria-label="Remove" onclick={() => draft.unique_constraints.splice(i, 1)}><Icon name="close" size={13} /></button></div>
				{/each}
				<div class="line">
					<input bind:value={newUnique} placeholder="FirstName, LastName" onkeydown={(e) => e.key === 'Enter' && addUnique()} />
					<button onclick={addUnique}>Add</button>
				</div>

				<h4>Relationships</h4>
				{#each draft.foreign_keys as fk, i (i)}
					<div class="line">
						<span>{fk.columns.join(', ')} → {fk.ref_table}{fk.ref_columns?.length ? `.${fk.ref_columns.join(', ')}` : ''}</span>
						<button class="icon" aria-label="Remove relationship" onclick={() => draft.foreign_keys.splice(i, 1)}><Icon name="close" size={13} /></button>
					</div>
				{:else}
					<p class="muted">None. Use a Lookup field to relate tables.</p>
				{/each}

				{#if info}
					<h4>Indexes</h4>
					{#each info.indexes as idx (idx.name)}
						<p class="muted">{idx.columns.join(', ')}{idx.unique ? ' · unique' : ''}</p>
					{:else}
						<p class="muted">None</p>
					{/each}
					<details>
						<summary>SQL</summary>
						<pre class="mono">{info.schema_sql}</pre>
					</details>
				{/if}
			</div>
		</aside>
	{/if}
</div>

<style>
	.design {
		display: grid;
		grid-template-columns: 1fr;
		height: 100%;
		min-height: 0;
	}
	.design.with-sheet {
		grid-template-columns: 1fr 300px;
	}
	.main {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
	}
	.banner {
		display: flex;
		gap: 10px;
		align-items: flex-start;
		padding: 10px 14px;
		background: rgba(229, 168, 59, 0.1);
		border-bottom: 1px solid rgba(229, 168, 59, 0.4);
		color: var(--warning);
	}
	.banner span {
		color: var(--text);
	}
	.upper {
		flex: 1 1 55%;
		overflow: auto;
		min-height: 120px;
	}
	.fields {
		width: 100%;
		border-collapse: separate;
		border-spacing: 0;
	}
	.fields th {
		position: sticky;
		top: 0;
		z-index: 1;
		padding: 6px 10px;
		background: var(--surface-2);
		border-bottom: 1px solid var(--border);
		border-right: 1px solid var(--border);
		text-align: left;
		font-weight: 600;
	}
	.fields th:nth-child(2) {
		width: 28%;
	}
	.fields th:nth-child(3) {
		width: 18%;
	}
	.fields td {
		padding: 0;
		height: 30px;
		border-bottom: 1px solid var(--border);
		border-right: 1px solid var(--border);
		position: relative;
	}
	.fields td input,
	.fields td select {
		width: 100%;
		height: 29px;
		border: none;
		border-radius: 0;
		background: transparent;
		box-shadow: none;
		padding: 0 10px;
	}
	.fields td input:focus,
	.fields td select:focus {
		background: var(--surface-2);
		box-shadow: inset 0 0 0 2px var(--focus);
	}
	.fields input.invalid {
		box-shadow: inset 0 -2px 0 var(--danger);
	}
	.sel {
		width: 28px;
		min-width: 28px;
		text-align: center;
		background: var(--surface-2);
		color: var(--accent-text);
	}
	.sel :global(svg) {
		margin: 0 auto;
	}
	.key {
		color: var(--warning);
	}
	tr.selected td:not(.sel) {
		background: var(--accent-softer);
	}
	tr.selected .sel {
		background: var(--surface-4);
	}
	.was {
		position: absolute;
		right: 8px;
		top: 7px;
		font-size: 11px;
		color: var(--text-faint);
		pointer-events: none;
	}
	.blank td {
		color: var(--text-faint);
	}
	.hint {
		padding: 0 10px !important;
		font-size: 12px;
	}
	.lower {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 260px;
		flex: 0 0 auto;
		height: 42%;
		min-height: 220px;
		border-top: 2px solid var(--border-strong);
		background: var(--surface);
	}
	.props {
		overflow: auto;
		min-width: 0;
	}
	.props-head {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 0 12px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-2);
		position: sticky;
		top: 0;
		z-index: 1;
	}
	.title {
		font-weight: 600;
		color: var(--text-muted);
		font-size: 12px;
	}
	.ptabs {
		display: flex;
	}
	.ptabs button {
		border: none;
		border-radius: 0;
		background: transparent;
		padding: 7px 14px;
		color: var(--text-muted);
		border-bottom: 2px solid transparent;
	}
	.ptabs button.active {
		color: var(--text);
		border-bottom-color: var(--accent);
	}
	.ptable {
		display: grid;
		grid-template-columns: 160px minmax(0, 1fr);
		align-items: center;
		gap: 6px 12px;
		padding: 12px;
		max-width: 720px;
	}
	.ptable label {
		color: var(--text-muted);
	}
	.pair {
		display: flex;
		gap: 6px;
	}
	.pair > * {
		flex: 1;
		min-width: 0;
	}
	.note {
		padding: 14px;
		margin: 0;
		color: var(--text-muted);
		grid-column: 1 / -1;
	}
	.help {
		display: flex;
		gap: 10px;
		padding: 14px;
		border-left: 1px solid var(--border);
		color: var(--text-muted);
		line-height: 1.5;
	}
	.help :global(svg) {
		flex: none;
		margin-top: 2px;
		color: var(--accent-text);
	}
	.help p {
		margin: 0;
	}
	.sheet {
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-left: 1px solid var(--border);
		background: var(--surface);
	}
	.sheet header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 8px 8px 12px;
		font-weight: 600;
		border-bottom: 1px solid var(--border);
	}
	.sheet-body {
		overflow: auto;
		padding: 4px 12px 16px;
	}
	.sheet h4 {
		margin: 16px 0 8px;
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--accent-text);
	}
	.block {
		display: grid;
		gap: 4px;
	}
	.block span {
		color: var(--text-muted);
	}
	.block textarea {
		resize: vertical;
	}
	.line {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 6px;
	}
	.line input {
		flex: 1;
		min-width: 0;
	}
	.line span {
		flex: 1;
	}
	.sheet p {
		margin: 4px 0;
	}
	pre {
		white-space: pre-wrap;
		font-size: 11px;
		padding: 8px;
		background: var(--bg);
		border-radius: var(--radius-sm);
		color: var(--text-muted);
	}
	summary {
		cursor: pointer;
		color: var(--text-muted);
		margin-top: 14px;
	}
</style>
