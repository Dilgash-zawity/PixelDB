<script lang="ts">
	import type { DefaultValue, FieldDef, FkAction, TableInfo } from '../api/types';
	import { FIELD_TYPES } from '../logic/field-types';
	import * as d from '../logic/table-draft';
	import { confirmAction } from '../services/dialogs';
	import { tableService } from '../services/tables';
	import { app } from '../state/app.svelte';

	let { name }: { name: string | null } = $props();

	let info = $state<TableInfo | null>(null);
	let draft = $state(d.newDraft());
	let problems = $state<string[]>([]);
	let newUnique = $state('');

	const FK_ACTIONS: FkAction[] = ['no_action', 'restrict', 'cascade', 'set_null', 'set_default'];
	const DEFAULT_KINDS = ['none', 'literal', 'current_timestamp', 'current_date', 'current_time', 'expression'];

	async function load() {
		problems = [];
		if (name === null) {
			info = null;
			draft = d.newDraft();
			return;
		}
		const table = name;
		await app.run(async () => {
			info = await tableService.get(table);
			draft = d.draftFromDefinition(info.definition);
		});
	}
	load();

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

	function optionalNumber(value: string): number | null {
		return value.trim() === '' ? null : Number(value);
	}

	function addUnique() {
		const cols = newUnique.split(',').map((c) => c.trim()).filter(Boolean);
		if (cols.length) draft.unique_constraints.push(cols);
		newUnique = '';
	}

	async function save() {
		problems = d.validateDraft(draft);
		if (problems.length) return;
		await app.run(async () => {
			if (draft.isNew || !info) {
				const created = await tableService.create(d.toDefinition(draft));
				await app.refresh();
				app.view = { kind: 'design', name: created.definition.name };
				app.message = `Created table ${created.definition.name}`;
				return;
			}
			const table = info.definition.name;
			const removed = d.removedFields(draft, info.definition);
			let confirmed = false;
			if (removed.length) {
				confirmed = await confirmAction(
					`Saving deletes the fields ${removed.join(', ')} and all data stored in them. Continue?`
				);
				if (!confirmed) return;
			}
			const result = await tableService.saveDesign(table, d.toDesign(draft), confirmed);
			info = await tableService.get(table);
			draft = d.draftFromDefinition(info.definition);
			await app.refresh();
			app.message = ['Design saved.', result.rebuilt ? 'The table was rebuilt.' : '', ...result.warnings]
				.filter(Boolean)
				.join(' ');
		});
	}
</script>

<section>
	<h2>{draft.isNew ? 'New table' : `Design: ${info?.definition.name ?? name}`}</h2>

	{#if info}
		<p>
			{info.row_count} records.
			{info.managed ? 'Definition stored in application metadata.' : 'Definition inferred from the SQLite schema.'}
		</p>
		{#if info.unsupported_features.length}
			<p role="alert">
				This table uses features the designer cannot preserve ({info.unsupported_features.join(', ')}).
				Only renaming fields is possible.
			</p>
		{/if}
	{/if}

	<p>
		<label>Table name <input bind:value={draft.name} disabled={!draft.isNew} /></label>
		{#if !draft.isNew}(rename from the table list){/if}
	</p>
	<p><label>Description <input bind:value={draft.description} size="60" /></label></p>

	<h3>Fields</h3>
	<table>
		<thead>
			<tr>
				<th>Order</th><th>Name</th><th>Type</th><th>Type options</th><th>Key</th><th>Required</th>
				<th>Unique</th><th>Default</th><th>Check</th><th>Description</th><th></th>
			</tr>
		</thead>
		<tbody>
			{#each draft.fields as df (df.id)}
				{@const f = df.field}
				{@const t = f.field_type}
				<tr>
					<td>
						<button onclick={() => d.moveField(draft, df.id, -1)}>Up</button>
						<button onclick={() => d.moveField(draft, df.id, 1)}>Down</button>
					</td>
					<td>
						<input value={f.name} oninput={(e) => d.renameField(draft, df.id, e.currentTarget.value)} />
						{#if df.originalName && df.originalName !== f.name}(was {df.originalName}){/if}
					</td>
					<td>
						<select value={t.type} onchange={(e) => d.setFieldType(draft, df.id, e.currentTarget.value as never)}>
							{#each FIELD_TYPES as ft (ft.name)}
								<option value={ft.name} title={ft.storage}>{ft.label}</option>
							{/each}
							{#if t.type === 'unknown'}<option value="unknown">Other</option>{/if}
						</select>
					</td>
					<td>
						{#if t.type === 'short_text'}
							<label>Max length
								<input type="number" min="1" value={t.max_length ?? ''} oninput={(e) => (t.max_length = optionalNumber(e.currentTarget.value))} />
							</label>
						{:else if t.type === 'number'}
							<label><input type="checkbox" bind:checked={t.real} /> Decimal (REAL)</label>
						{:else if t.type === 'currency'}
							<label>Decimals
								<input type="number" min="0" max="6" value={t.scale ?? 2} oninput={(e) => (t.scale = Number(e.currentTarget.value))} />
							</label>
						{:else if t.type === 'calculated'}
							<label>Expression <input bind:value={t.expression} placeholder="price * qty" /></label>
							<label><input type="checkbox" bind:checked={t.stored} /> Stored</label>
							<select bind:value={t.result}>
								<option value="any">any</option><option value="text">text</option>
								<option value="integer">integer</option><option value="real">real</option>
							</select>
						{:else if t.type === 'lookup'}
							{@const fk = d.lookupTarget(draft, f.name)}
							<label>Related table
								<select value={fk?.ref_table ?? ''} onchange={(e) => d.setLookupTarget(draft, df.id, e.currentTarget.value, '')}>
									<option value="">(choose)</option>
									{#each app.tables as tbl (tbl.name)}<option value={tbl.name}>{tbl.name}</option>{/each}
									{#if draft.name && !app.tables.some((x) => x.name === draft.name)}<option value={draft.name}>{draft.name} (this table)</option>{/if}
								</select>
							</label>
							{#if fk}
								<label>Related field
									<input value={fk.ref_columns?.[0] ?? ''} placeholder="(primary key)" oninput={(e) => (fk.ref_columns = e.currentTarget.value ? [e.currentTarget.value] : [])} />
								</label>
								<label>On delete
									<select bind:value={fk.on_delete}>{#each FK_ACTIONS as a (a)}<option value={a}>{a}</option>{/each}</select>
								</label>
								<label>On update
									<select bind:value={fk.on_update}>{#each FK_ACTIONS as a (a)}<option value={a}>{a}</option>{/each}</select>
								</label>
							{/if}
							<label>Display field <input value={t.display_column ?? ''} oninput={(e) => (t.display_column = e.currentTarget.value || null)} /></label>
							<select bind:value={t.value_type}>
								<option value="integer">integer key</option><option value="text">text key</option>
							</select>
						{:else if t.type === 'unknown'}
							<label>Declared type <input bind:value={t.declared} /></label>
						{/if}
					</td>
					<td><input type="checkbox" bind:checked={f.primary_key} disabled={t.type === 'calculated'} /></td>
					<td><input type="checkbox" bind:checked={f.required} /></td>
					<td><input type="checkbox" bind:checked={f.unique} /></td>
					<td>
						<select value={f.default?.kind ?? 'none'} onchange={(e) => setDefaultKind(f, e.currentTarget.value)}>
							{#each DEFAULT_KINDS as k (k)}<option value={k}>{k}</option>{/each}
						</select>
						{#if f.default?.kind === 'literal' || f.default?.kind === 'expression'}
							<input value={defaultText(f)} oninput={(e) => setDefaultText(f, e.currentTarget.value)} />
						{/if}
					</td>
					<td><input value={f.check ?? ''} oninput={(e) => (f.check = e.currentTarget.value || null)} placeholder="e.g. qty >= 0" /></td>
					<td><input value={f.description ?? ''} oninput={(e) => (f.description = e.currentTarget.value || null)} /></td>
					<td><button onclick={() => d.removeField(draft, df.id)}>Remove</button></td>
				</tr>
			{/each}
		</tbody>
	</table>
	<p>
		Add field:
		{#each FIELD_TYPES as ft (ft.name)}
			<button onclick={() => d.addField(draft, ft.name)}>{ft.label}</button>{' '}
		{/each}
	</p>

	<h3>Relationships</h3>
	{#if draft.foreign_keys.length === 0}
		<p>None. Use a Lookup field to relate this table to another.</p>
	{:else}
		<ul>
			{#each draft.foreign_keys as fk, i (i)}
				<li>
					({fk.columns.join(', ')}) → {fk.ref_table}({(fk.ref_columns ?? []).join(', ') || 'primary key'}),
					on delete {fk.on_delete ?? 'no_action'}, on update {fk.on_update ?? 'no_action'}
					<button onclick={() => draft.foreign_keys.splice(i, 1)}>Remove</button>
				</li>
			{/each}
		</ul>
	{/if}

	<h3>Multi-field unique constraints</h3>
	<ul>
		{#each draft.unique_constraints as u, i (i)}
			<li>{u.join(', ')} <button onclick={() => draft.unique_constraints.splice(i, 1)}>Remove</button></li>
		{/each}
	</ul>
	<p>
		<label>Fields (comma separated) <input bind:value={newUnique} /></label>
		<button onclick={addUnique}>Add unique constraint</button>
	</p>

	<h3>Table validation rules (CHECK)</h3>
	<ul>
		{#each draft.checks as _, i (i)}
			<li><input bind:value={draft.checks[i]} size="50" /> <button onclick={() => draft.checks.splice(i, 1)}>Remove</button></li>
		{/each}
	</ul>
	<p><button onclick={() => draft.checks.push('')}>Add rule</button></p>

	{#if problems.length}
		<ul role="alert">
			{#each problems as p (p)}<li>{p}</li>{/each}
		</ul>
	{/if}
	<p>
		<button disabled={app.busy} onclick={save}>{draft.isNew ? 'Create table' : 'Save design'}</button>
		<button disabled={app.busy} onclick={load}>Discard changes</button>
		{#if !draft.isNew && info}
			<button onclick={() => (app.view = { kind: 'datasheet', name: info!.definition.name })}>Open records</button>
		{/if}
	</p>

	{#if info}
		<details>
			<summary>Schema</summary>
			<pre>{info.schema_sql}</pre>
			<p>Indexes:</p>
			<ul>
				{#each info.indexes as idx (idx.name)}
					<li>{idx.name}: {idx.columns.join(', ')}{idx.unique ? ' (unique)' : ''}{idx.partial ? ' (partial)' : ''}</li>
				{:else}
					<li>none</li>
				{/each}
			</ul>
		</details>
	{/if}
</section>
