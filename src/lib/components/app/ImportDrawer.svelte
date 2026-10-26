<script lang="ts">
	// Services
	import { tableService } from '#lib/services/tables.js';
	import { transferService } from '#lib/services/transfer.js';

	// Types
	import type { FieldDef, FieldType, FieldTypeName, ImportFile, TableDef } from '#lib/api/types.js';

	// Utils
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { ApiError } from '#lib/api/client.js';
	import { fieldTypeLabel, makeFieldType } from '#lib/logic/field-types.js';
	import {
		IMPORT_TYPES,
		buildRecords,
		columnCount,
		columnNames,
		fileRowMessage,
		guessDateOrder,
		guessType,
		importableFields,
		localDateOrder,
		looksLikeHeader,
		matchColumns,
		missingRequired,
		type ColumnTarget,
		type DateOrder
	} from '#lib/logic/import.js';
	import { fieldNameProblems, wizardTable } from '#lib/logic/table-wizard.js';
	import { attempt, cleanMessage, reportError } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		Columns3 as Columns3Icon,
		FileSpreadsheet as FileSpreadsheetIcon,
		FileUp as FileUpIcon,
		TriangleAlert as TriangleAlertIcon,
		Upload as UploadIcon
	} from '@lucide/svelte';

	// Script
	/**
	 * Imports a CSV or spreadsheet into a new table or an existing one: pick
	 * the file (sheet, header row, preview), then where each column goes.
	 * Nothing is written until Import; a new table is removed again if its
	 * rows can't all be added.
	 */
	let {
		open = $bindable(false),
		/** Starts with this table as the destination. */
		table = null,
		onImported
	}: {
		open?: boolean;
		table?: string | null;
		onImported?: (table: string) => void;
	} = $props();

	const STEPS = [
		{ label: 'File', icon: FileSpreadsheetIcon },
		{ label: 'Columns', icon: Columns3Icon }
	];
	const PREVIEW_ROWS = 8;
	/** Rows looked at to guess each column's type. */
	const SAMPLE_ROWS = 2000;
	const SKIP = '__skip__';

	interface Column {
		name: string;
		include: boolean;
		type: FieldType;
		/** The existing field it fills. */
		target: ColumnTarget;
	}

	let step = $state(0);
	let file = $state<ImportFile | null>(null);
	let picking = $state(false);
	let busy = $state(false);
	let sheetIndex = $state(0);
	let hasHeader = $state(true);
	let dateOrder = $state<DateOrder>('dmy');
	let dest = $state<'new' | 'existing'>('new');
	let newName = $state('');
	let existing = $state<string | null>(null);
	let existingDef = $state<TableDef | null>(null);
	let columns = $state<Column[]>([]);

	const sheet = $derived(file?.sheets[sheetIndex] ?? null);
	const count = $derived(sheet ? columnCount(sheet.rows) : 0);
	const header = $derived(sheet && hasHeader ? sheet.rows[0] : null);
	const dataRows = $derived(sheet ? sheet.rows.slice(hasHeader ? 1 : 0) : []);

	$effect(() => {
		if (!open) return;
		untrack(() => {
			step = 0;
			file = null;
			columns = [];
			dest = table && !session.isView(table) ? 'existing' : 'new';
			existing = table && !session.isView(table) ? table : (session.tables[0]?.name ?? null);
			existingDef = null;
		});
	});

	// The destination table's fields, when importing into one.
	$effect(() => {
		const name = existing;
		if (!open || !name) return;
		untrack(async () => {
			existingDef = null;
			const info = await attempt(() => tableService.get(name));
			if (info && existing === name) existingDef = info.definition;
		});
	});

	async function pick() {
		picking = true;
		try {
			const f = await attempt(() => transferService.pickImportFile());
			if (!f) return;
			file = f;
			sheetIndex = Math.max(0, f.sheets.findIndex((s) => s.rows.length > 0));
			const rows = f.sheets[sheetIndex]?.rows ?? [];
			hasHeader = looksLikeHeader(rows);
			const base = f.sheets.length > 1 ? f.sheets[sheetIndex].name : f.file_name.replace(/\.[^.]+$/, '');
			newName = freeTableName(base);
			resetColumns();
		} finally {
			picking = false;
		}
	}

	function freeTableName(base: string): string {
		const clean = columnNames([base], 1, [])[0];
		let name = clean;
		for (let n = 2; session.tableNameProblem(name); n++) name = `${clean} ${n}`;
		return name;
	}

	/** Names and guessed types from the sheet, matched to the destination's fields. */
	function resetColumns() {
		if (!sheet) return;
		const names = columnNames(header, count);
		const sample = dataRows.slice(0, SAMPLE_ROWS);
		const values = (c: number) => sample.map((r) => r[c] ?? '');
		dateOrder = guessDateOrder(names.flatMap((_, c) => values(c)), localDateOrder());
		const targets = existingDef ? matchColumns(names, existingDef.fields) : names.map(() => null);
		columns = names.map((name, c) => ({ name, include: true, type: guessType(values(c), dateOrder), target: targets[c] }));
	}

	function rematch() {
		if (!existingDef) return;
		const targets = matchColumns(columns.map((c) => c.name), existingDef.fields);
		columns.forEach((c, i) => (c.target = targets[i]));
	}

	$effect(() => {
		// Match again when the destination table's fields arrive.
		if (existingDef) untrack(rematch);
	});

	function setType(c: Column, v: FieldTypeName) {
		c.type = v === 'number' && c.type.type === 'number' ? c.type : makeFieldType(v);
	}

	const fields = $derived<FieldDef[]>(
		dest === 'new' ? columns.filter((c) => c.include).map((c) => ({ name: c.name.trim(), field_type: c.type })) : (existingDef?.fields ?? [])
	);
	const targets = $derived<ColumnTarget[]>(
		dest === 'new' ? columns.map((c) => (c.include ? c.name.trim() : null)) : columns.map((c) => c.target)
	);
	const used = $derived(targets.filter((t) => t !== null).length);
	const nameProblems = $derived(dest === 'new' ? fieldNameProblems(columns.filter((c) => c.include).map((c) => c.name)) : []);
	const tableProblem = $derived(dest === 'new' ? session.tableNameProblem(newName.trim()) : existing ? null : 'Choose a table');
	const built = $derived(step === 1 && (dest === 'new' || existingDef) ? buildRecords(dataRows, targets, fields, dateOrder, hasHeader ? 2 : 1) : null);
	const required = $derived(dest === 'existing' && existingDef ? missingRequired(existingDef.fields, targets) : []);
	const hasDates = $derived(fields.some((f) => f.field_type.type === 'date_time' && targets.includes(f.name)));
	const blocker = $derived.by(() => {
		if (tableProblem) return tableProblem;
		if (!used) return 'Choose at least one column to import';
		const n = nameProblems.find(Boolean);
		if (n) return n;
		if (required.length) return `Add a column for ${required.map((r) => `“${r}”`).join(', ')}, which every record needs`;
		if (built?.problems.length) return 'Fix the values below, or change those columns’ types';
		if (built && !built.records.length) return 'There are no rows to import';
		return null;
	});

	const targetItems = $derived([
		{ value: SKIP, label: 'Don’t import' },
		...importableFields(existingDef?.fields ?? []).map((f) => ({ value: f.name, label: f.name }))
	]);
	const typeItems = IMPORT_TYPES.map((t) => ({ value: t, label: fieldTypeLabel(makeFieldType(t)) }));

	function next(at: number) {
		if (at === 0 && !sheet?.rows.length) return false;
	}

	async function finish() {
		if (!built || blocker || busy) return;
		busy = true;
		let created: string | null = null;
		try {
			let target = existing!;
			if (dest === 'new') {
				const info = await attempt(() => tableService.create(wizardTable(newName, fields)));
				if (!info) return;
				created = target = info.definition.name;
			}
			try {
				const n = await transferService.importRecords(target, built.records);
				await session.refresh();
				open = false;
				toast.success(`Imported ${n} ${n === 1 ? 'record' : 'records'} into “${target}”`, {
					description: [
						built.skipped ? `${built.skipped} empty ${built.skipped === 1 ? 'row was' : 'rows were'} left out.` : '',
						sheet?.truncated ? 'The file had more rows than can be imported at once; the rest were left out.' : ''
					]
						.filter(Boolean)
						.join(' ') || undefined
				});
				onImported?.(target);
			} catch (e) {
				// Nothing was added; don't leave an empty new table behind.
				if (created) {
					const name = created;
					await attempt(() => tableService.drop(name, true));
					await session.refresh();
				}
				if (e instanceof ApiError) reportError(new ApiError(e.kind, fileRowMessage(cleanMessage(e.message), built.lines)));
				else reportError(e);
			}
		} finally {
			busy = false;
		}
	}
</script>

<StepDrawer
	bind:open
	bind:step
	title="Import"
	description="From a CSV file or a spreadsheet (Excel, LibreOffice)"
	steps={STEPS}
	finishLabel={built?.records.length ? `Import ${built.records.length} ${built.records.length === 1 ? 'record' : 'records'}` : 'Import'}
	finishIcon={UploadIcon}
	{busy}
	disabled={(step === 0 && !sheet?.rows.length) || (step === 1 && !!blocker)}
	onNext={next}
	onFinish={finish}
>
	{#if step === 0}
		<div class="flex flex-col gap-5">
			<div class="flex flex-wrap items-center gap-3 rounded-lg border bg-card p-3">
				<span class="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground vivid:bg-tone-green/15 vivid:text-tone-green">
					<FileSpreadsheetIcon class="size-4" />
				</span>
				<div class="flex min-w-0 flex-1 flex-col">
					{#if file}
						<span class="truncate font-medium">{file.file_name}</span>
						<span class="text-xs text-muted-foreground">
							{dataRows.length} {dataRows.length === 1 ? 'row' : 'rows'} · {count} {count === 1 ? 'column' : 'columns'}
						</span>
					{:else}
						<span class="font-medium">No file chosen</span>
						<span class="text-xs text-muted-foreground">CSV, TSV, Excel (.xlsx, .xls) or OpenDocument (.ods)</span>
					{/if}
				</div>
				<Button variant={file ? 'outline' : 'default'} disabled={picking} onclick={pick}>
					<FileUpIcon data-icon="inline-start" />
					{file ? 'Choose another…' : 'Choose a file…'}
				</Button>
			</div>

			{#if file && sheet}
				{#if sheet.truncated}
					<Alert.Root class="border-warning/40 bg-warning/10 *:[svg]:text-warning">
						<TriangleAlertIcon />
						<Alert.Title class="text-warning">Large file</Alert.Title>
						<Alert.Description>Only the first {sheet.rows.length.toLocaleString()} rows can be imported at once.</Alert.Description>
					</Alert.Root>
				{/if}
				<div class="flex flex-wrap items-end gap-4">
					{#if file.sheets.length > 1}
						<Field.Field class="w-56">
							<Field.Label for="im-sheet">Sheet</Field.Label>
							<Select.Root
								type="single"
								items={file.sheets.map((s, i) => ({ value: String(i), label: s.name }))}
								value={String(sheetIndex)}
								onValueChange={(v) => {
									sheetIndex = Number(v);
									hasHeader = looksLikeHeader(sheet?.rows ?? []);
									newName = freeTableName(file!.sheets[sheetIndex].name);
									resetColumns();
								}}
							>
								<Select.Trigger id="im-sheet" class="w-full"><Select.Value /></Select.Trigger>
								<Select.Content>
									<Select.Group>
										{#each file.sheets as s, i (i)}<Select.Item value={String(i)} label={s.name}>{s.name}</Select.Item>{/each}
									</Select.Group>
								</Select.Content>
							</Select.Root>
						</Field.Field>
					{/if}
					<Field.Field orientation="horizontal" class="w-auto">
						<Switch
							id="im-header"
							checked={hasHeader}
							onCheckedChange={(v) => {
								hasHeader = v;
								resetColumns();
							}}
						/>
						<Field.Label for="im-header">The first row has the column names</Field.Label>
					</Field.Field>
				</div>
				<div class="overflow-x-auto rounded-lg border">
					<Table.Root>
						<Table.Header>
							<Table.Row>
								{#each columnNames(header, count) as name, i (i)}<Table.Head class="whitespace-nowrap">{name}</Table.Head>{/each}
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each dataRows.slice(0, PREVIEW_ROWS) as row, r (r)}
								<Table.Row>
									{#each Array(count) as _, c (c)}<Table.Cell class="max-w-56 truncate">{row[c] ?? ''}</Table.Cell>{/each}
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</div>
				{#if dataRows.length > PREVIEW_ROWS}
					<p class="text-xs text-muted-foreground">Showing the first {PREVIEW_ROWS} of {dataRows.length.toLocaleString()} rows.</p>
				{/if}
			{/if}
		</div>
	{:else}
		<div class="flex flex-col gap-6">
			<Field.Set>
				<Field.Legend variant="label">Import into</Field.Legend>
				<ToggleGroup.Root type="single" variant="outline" class="w-full sm:w-96" value={dest} onValueChange={(v) => v && (dest = v as 'new' | 'existing')}>
					<ToggleGroup.Item value="new" class="flex-1">A new table</ToggleGroup.Item>
					<ToggleGroup.Item value="existing" class="flex-1" disabled={!session.tables.length}>An existing table</ToggleGroup.Item>
				</ToggleGroup.Root>
				{#if dest === 'new'}
					<Field.Field data-invalid={!!tableProblem}>
						<Field.Label for="im-name">Table name</Field.Label>
						<Input id="im-name" class="sm:w-96" bind:value={newName} autocomplete="off" aria-invalid={!!tableProblem} />
						{#if tableProblem}<Field.Error>{tableProblem}</Field.Error>{:else}<Field.Description>An ID field is added for you.</Field.Description>{/if}
					</Field.Field>
				{:else}
					<Field.Field>
						<Field.Label for="im-table">Table</Field.Label>
						<Select.Root type="single" items={session.tables.map((t) => ({ value: t.name, label: t.name }))} value={existing ?? ''} onValueChange={(v) => (existing = v || null)}>
							<Select.Trigger id="im-table" class="sm:w-96"><Select.Value placeholder="Choose a table" /></Select.Trigger>
							<Select.Content>
								<Select.Group>
									{#each session.tables as t (t.name)}<Select.Item value={t.name} label={t.name}>{t.name}</Select.Item>{/each}
								</Select.Group>
							</Select.Content>
						</Select.Root>
						<Field.Description>The records are added to the ones already there.</Field.Description>
					</Field.Field>
				{/if}
			</Field.Set>

			<Field.Set>
				<Field.Legend variant="label">Columns</Field.Legend>
				<Field.Description>
					{dest === 'new' ? 'Untick a column to leave it out. Types are guessed from the values; change any that look wrong.' : 'Choose the field each column goes into.'}
				</Field.Description>
				<div class="overflow-x-auto rounded-lg border">
					<Table.Root>
						<Table.Header>
							<Table.Row>
								{#if dest === 'new'}<Table.Head class="w-10"></Table.Head>{/if}
								<Table.Head>{dest === 'new' ? 'Field name' : 'Column in the file'}</Table.Head>
								<Table.Head>{dest === 'new' ? 'Type' : 'Goes into'}</Table.Head>
								<Table.Head>First value</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each columns as c, i (i)}
								{@const sample = dataRows.find((r) => (r[i] ?? '').trim())?.[i] ?? ''}
								{@const problem = dest === 'new' && c.include ? nameProblems[columns.slice(0, i).filter((x) => x.include).length] : null}
								<Table.Row>
									{#if dest === 'new'}
										<Table.Cell><Checkbox checked={c.include} onCheckedChange={(v) => (c.include = v)} aria-label="Import {c.name}" /></Table.Cell>
										<Table.Cell class="min-w-48">
											<Input class="h-8" bind:value={c.name} disabled={!c.include} aria-invalid={!!problem} title={problem ?? undefined} aria-label="Field name" />
										</Table.Cell>
										<Table.Cell class="min-w-44">
											<Select.Root type="single" items={typeItems} value={c.type.type} disabled={!c.include} onValueChange={(v) => setType(c, v as FieldTypeName)}>
												<Select.Trigger size="sm" class="w-full" aria-label="Type">
													<span class="flex items-center gap-2"><FieldTypeIcon type={c.type.type} /><Select.Value /></span>
												</Select.Trigger>
												<Select.Content>
													<Select.Group>
														{#each IMPORT_TYPES as t (t)}<Select.Item value={t} label={fieldTypeLabel(makeFieldType(t))}><FieldTypeIcon type={t} /> {fieldTypeLabel(makeFieldType(t))}</Select.Item>{/each}
													</Select.Group>
												</Select.Content>
											</Select.Root>
										</Table.Cell>
									{:else}
										<Table.Cell class="font-medium">{c.name}</Table.Cell>
										<Table.Cell class="min-w-48">
											<Select.Root type="single" items={targetItems} value={c.target ?? SKIP} onValueChange={(v) => (c.target = v === SKIP ? null : v)}>
												<Select.Trigger size="sm" class="w-full" aria-label="Field"><Select.Value /></Select.Trigger>
												<Select.Content>
													<Select.Group>
														{#each targetItems as t (t.value)}
															<Select.Item value={t.value} label={t.label} disabled={t.value !== SKIP && t.value !== c.target && targets.includes(t.value)}>{t.label}</Select.Item>
														{/each}
													</Select.Group>
												</Select.Content>
											</Select.Root>
										</Table.Cell>
									{/if}
									<Table.Cell class="max-w-56 truncate text-muted-foreground">{sample}</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</div>
			</Field.Set>

			{#if hasDates}
				<Field.Field class="sm:w-96">
					<Field.Label for="im-dates">Dates like 03/04/2024 mean</Field.Label>
					<Select.Root
						type="single"
						items={[
							{ value: 'dmy', label: '3 April (day first)' },
							{ value: 'mdy', label: 'March 4 (month first)' }
						]}
						value={dateOrder}
						onValueChange={(v) => (dateOrder = v as DateOrder)}
					>
						<Select.Trigger id="im-dates" class="w-full"><Select.Value /></Select.Trigger>
						<Select.Content>
							<Select.Group>
								<Select.Item value="dmy" label="3 April (day first)">3 April (day first)</Select.Item>
								<Select.Item value="mdy" label="March 4 (month first)">March 4 (month first)</Select.Item>
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</Field.Field>
			{/if}

			{#if built?.problems.length}
				<Alert.Root variant="destructive">
					<TriangleAlertIcon />
					<Alert.Title>{built.problems.length} {built.problems.length === 1 ? 'value doesn’t' : 'values don’t'} fit</Alert.Title>
					<Alert.Description>
						<ul class="list-disc ps-4">
							{#each built.problems.slice(0, 6) as p, i (i)}<li>Row {p.row}, {p.column}: {p.message}</li>{/each}
						</ul>
						{#if built.problems.length > 6}<span>…and {built.problems.length - 6} more.</span>{/if}
					</Alert.Description>
				</Alert.Root>
			{:else if blocker}
				<p class="text-sm text-muted-foreground">{blocker}.</p>
			{:else if built}
				<p class="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
					<Badge variant="secondary">{built.records.length.toLocaleString()} records</Badge>
					<Badge variant="secondary">{used} {used === 1 ? 'field' : 'fields'}</Badge>
					{#if built.skipped}<Badge variant="outline">{built.skipped} empty rows left out</Badge>{/if}
					ready to import.
				</p>
			{/if}
		</div>
	{/if}
</StepDrawer>
