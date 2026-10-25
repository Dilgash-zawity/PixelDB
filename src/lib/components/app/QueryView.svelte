<script lang="ts">
	// Services
	import { queryService } from '#lib/services/queries.js';
	import { tableService } from '#lib/services/tables.js';

	// Types
	import type { FilterOp, TableDef, TableInfo } from '#lib/api/types.js';

	// Utils
	import { toast } from 'svelte-sonner';
	import { FILTER_OPS, needsValue } from '#lib/logic/filters.js';
	import {
		AGGREGATES,
		buildSql,
		hasUnsavedChanges,
		joinableTables,
		markClean,
		newColumn,
		newQueryDraft,
		parseVisualQuery,
		pruneQuery,
		queryTables,
		removeTable,
		type Aggregate,
		type JoinKind,
		type SortOrder
	} from '#lib/logic/query-builder.js';
	import { displayValue, identifierProblem, jsonClone } from '#lib/logic/values.js';
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
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import NameDialog from './NameDialog.svelte';
	import Shortcut from './Shortcut.svelte';

	// Icons
	import {
		Check as CheckIcon,
		Code as CodeIcon,
		Eye as EyeIcon,
		FilePlus as FilePlusIcon,
		FolderOpen as FolderOpenIcon,
		Search as SearchIcon,
		KeyRound as KeyRoundIcon,
		Play as PlayIcon,
		Plus as PlusIcon,
		Save as SaveIcon,
		SearchCode as SearchCodeIcon,
		Table2 as TableIcon,
		Trash2 as TrashIcon,
		TriangleAlert as TriangleAlertIcon,
		Workflow as WorkflowIcon,
		X as XIcon,
	} from '@lucide/svelte';

	// Script
	const draft = $derived(session.query);
	let defs = $state<TableDef[]>([]);
	let running = $state(false);
	let saving = $state(false);
	let filter = $state('');

	const savedQueries = $derived(
		session.views.filter((v) => v.name.toLowerCase().includes(filter.trim().toLowerCase()))
	);

	let loads = 0;
	$effect(() => {
		void session.revision;
		const names = session.tables.map((t) => t.name);
		const n = ++loads;
		Promise.all(names.map((n) => tableService.get(n).catch(() => null))).then((infos) => {
			if (n !== loads) return; // a newer load is under way
			defs = infos.filter((i): i is TableInfo => i !== null).map((i) => i.definition);
			// Drop tables and fields deleted since the design was made (unless some failed to load).
			if (defs.length === names.length) {
				const pruned = pruneQuery(defs, session.query.visual);
				if (JSON.stringify(pruned) !== JSON.stringify(session.query.visual)) session.updateQueryDesign(() => pruned);
			}
		});
	});

	const built = $derived(buildSql(defs, draft.visual));
	const tablesInQuery = $derived(queryTables(draft.visual));
	const addable = $derived(joinableTables(defs, draft.visual));
	const currentSql = $derived(draft.mode === 'visual' ? built.sql : draft.sql);

	const tableItems = $derived(defs.map((d) => ({ value: d.name, label: d.name })));
	const opItems = FILTER_OPS.map((o) => ({ value: o.op, label: o.label }));
	const sortItems: { value: SortOrder; label: string }[] = [
		{ value: 'none', label: '—' },
		{ value: 'asc', label: 'Ascending' },
		{ value: 'desc', label: 'Descending' }
	];
	const joinItems: { value: JoinKind; label: string }[] = [
		{ value: 'inner', label: 'Only matching rows' },
		{ value: 'left', label: 'Keep rows without a match' }
	];

	function def(name: string) {
		return defs.find((d) => d.name === name);
	}

	function hasColumn(table: string, field: string) {
		return draft.visual.columns.some((c) => c.table === table && c.field === field);
	}

	function toggleColumn(table: string, field: string) {
		const v = draft.visual;
		v.columns = hasColumn(table, field)
			? v.columns.filter((c) => !(c.table === table && c.field === field))
			: [...v.columns, newColumn(table, field)];
	}

	function addTable(name: string) {
		const v = draft.visual;
		if (!v.from) v.from = name;
		else v.joins = [...v.joins, { table: name, kind: 'inner' }];
	}

	function dropTable(name: string) {
		draft.visual = removeTable(defs, draft.visual, name);
	}

	/** Switching to SQL shows the designer's query unless the SQL was edited by hand. */
	async function setMode(mode: 'visual' | 'sql') {
		if (mode === draft.mode) return;
		if (mode === 'sql' && built.sql && built.sql !== draft.sql) {
			const edited = draft.sql.trim() !== '' && draft.sql !== draft.copied;
			if (
				!edited ||
				(await confirm({
					title: 'Replace the SQL?',
					description: 'The SQL editor has changes of its own. Replace them with the query from the designer?',
					action: 'Replace'
				}))
			) {
				draft.sql = built.sql;
				draft.copied = built.sql;
			}
		}
		draft.mode = mode;
	}

	/** Number of the latest run; results of an earlier one that finishes later are dropped. */
	let runId = 0;

	/**
	 * Runs the current query, or `text` (a saved query's SQL, before the
	 * designer's tables have loaded). Opening a saved query runs it even while
	 * another query is still running; only the latest results are shown.
	 */
	async function run(text?: string) {
		const sql = (text ?? currentSql).trim();
		if (!sql || (running && text === undefined)) return;
		const id = ++runId;
		if (text === undefined && draft.mode === 'visual' && built.problems.length) {
			toast.warning('Some criteria were skipped', { description: built.problems.join('. ') });
		}
		running = true;
		const designed = text === undefined && draft.mode === 'visual' ? built.types : [];
		const result = await attempt(() => queryService.run(sql));
		if (id !== runId) return;
		running = false;
		if (result) {
			draft.result = {
				columns: result.columns,
				rows: result.rows,
				truncated: result.truncated,
				elapsedMs: result.elapsed_ms,
				// The designer knows totals' types; the backend knows plain columns'.
				types: result.columns.map((_, i) => designed[i] ?? result.types[i] ?? undefined)
			};
		}
	}

	function onKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			run();
		}
	}

	/** Asks before replacing a query with unsaved changes. */
	async function mayDiscard(): Promise<boolean> {
		return (
			!hasUnsavedChanges(draft) ||
			confirm({
				title: 'Discard changes?',
				description: draft.saved
					? `Your changes to “${draft.saved}” haven't been saved.`
					: "This query hasn't been saved.",
				action: 'Discard',
				destructive: true
			})
		);
	}

	async function openSaved(name: string) {
		if (!(await mayDiscard())) return;
		const saved = await attempt(() => queryService.get(name));
		if (saved === undefined) return;
		// Queries built in the designer reopen there; others open as SQL.
		const visual = parseVisualQuery(saved.design);
		session.query = markClean(
			visual
				? { ...newQueryDraft(), mode: 'visual', visual, sql: saved.sql, copied: saved.sql, saved: name }
				: { ...newQueryDraft(), mode: 'sql', sql: saved.sql, copied: '', saved: name }
		);
		run(saved.sql);
	}

	async function newQuery() {
		if (await mayDiscard()) session.query = newQueryDraft();
	}

	function suggestedName(): string {
		if (draft.saved) return draft.saved;
		let n = 1;
		while (session.objects.some((o) => o.name.toLowerCase() === `query ${n}`)) n++;
		return `Query ${n}`;
	}

	function nameProblem(name: string): string | null {
		return (
			identifierProblem(name, 'Name') ??
			(session.tables.some((t) => t.name.toLowerCase() === name.toLowerCase())
				? 'A table already has this name'
				: null)
		);
	}

	async function save(name: string): Promise<boolean> {
		const existing = session.views.find((v) => v.name.toLowerCase() === name.toLowerCase());
		const replace = !!existing;
		if (
			existing &&
			existing.name !== draft.saved &&
			!(await confirm({
				title: `Replace “${existing.name}”?`,
				description: 'A saved query with this name already exists. It will show the results of this query instead.',
				action: 'Replace'
			}))
		) {
			return false;
		}
		const design = draft.mode === 'visual' ? jsonClone(draft.visual) : null;
		const ok = (await attempt(() => queryService.save(name, currentSql, design, replace))) !== undefined;
		if (!ok) return false;
		await session.refresh();
		// Replacing a query recreates it under the name as typed.
		draft.saved = name;
		markClean(draft);
		session.revision++;
		toast.success(`Saved “${draft.saved}”`, { description: 'It also opens in the Datasheet, under Views.' });
		return true;
	}

	async function deleteSaved(name = draft.saved) {
		if (!name) return;
		const ok = await confirm({
			title: `Delete “${name}”?`,
			description: 'The saved query is removed. Your tables and records are not affected.',
			action: 'Delete query',
			destructive: true
		});
		if (!ok || (await attempt(() => queryService.drop(name, true))) === undefined) return;
		await session.refresh();
		if (session.table === name) session.table = session.tables[0]?.name ?? null;
		if (draft.saved === name) draft.saved = null;
		toast.success(`Deleted “${name}”`);
	}

	function numeric(v: unknown) {
		return typeof v === 'number';
	}
</script>

<div class="flex h-full">
	<!-- Saved queries -->
	<aside class="flex w-60 shrink-0 flex-col border-e">
		<div class="flex items-center gap-2 border-b px-3 py-2">
			<span class="flex-1 text-sm font-semibold">Saved queries</span>
			<Button variant="ghost" size="icon-sm" aria-label="New query" onclick={newQuery}><PlusIcon /></Button>
		</div>
		<div class="border-b p-2">
			<InputGroup.Root class="h-8">
				<InputGroup.Addon><SearchIcon /></InputGroup.Addon>
				<InputGroup.Input bind:value={filter} placeholder="Filter" aria-label="Filter saved queries" />
			</InputGroup.Root>
		</div>
		<div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-1">
			{#each savedQueries as v (v.name)}
				<ContextMenu.Root>
					<ContextMenu.Trigger>
						{#snippet child({ props })}
							<Button
								{...props}
								variant="ghost"
								size="sm"
								class={cn('justify-start', v.name === draft.saved && 'bg-accent')}
								onclick={() => openSaved(v.name)}
							>
								<EyeIcon data-icon="inline-start" />
								<span class="flex-1 truncate text-left">{v.name}</span>
							</Button>
						{/snippet}
					</ContextMenu.Trigger>
					<ContextMenu.Content class="w-48">
						<ContextMenu.Group>
							<ContextMenu.Item onclick={() => openSaved(v.name)}><FolderOpenIcon /> Open</ContextMenu.Item>
						</ContextMenu.Group>
						<ContextMenu.Separator />
						<ContextMenu.Group>
							<ContextMenu.Item variant="destructive" onclick={() => deleteSaved(v.name)}><TrashIcon /> Delete…</ContextMenu.Item>
						</ContextMenu.Group>
					</ContextMenu.Content>
				</ContextMenu.Root>
			{:else}
				<p class="px-2 py-4 text-center text-sm text-muted-foreground">
					{session.views.length ? 'No matches.' : 'No saved queries yet. Use Save to keep a query here.'}
				</p>
			{/each}
		</div>
	</aside>

	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="flex h-full min-w-0 flex-1 flex-col" onkeydown={onKey}>
		<!-- Toolbar -->
		<div class="flex flex-wrap items-center gap-2 border-b px-4 py-2">
			<ToggleGroup.Root
				type="single"
				variant="outline"
				size="sm"
				value={draft.mode}
				onValueChange={(v) => v && setMode(v as 'visual' | 'sql')}
			>
				<ToggleGroup.Item value="visual" aria-label="Design visually"><WorkflowIcon /> Design</ToggleGroup.Item>
				<ToggleGroup.Item value="sql" aria-label="Write SQL"><CodeIcon /> SQL</ToggleGroup.Item>
			</ToggleGroup.Root>

			<Button size="sm" disabled={!currentSql.trim() || running} onclick={() => run()}>
				{#if running}<Spinner data-icon="inline-start" />{:else}<PlayIcon data-icon="inline-start" />{/if}
				Run <Shortcut keys="mod+enter" />
			</Button>

			<ButtonGroup.Root>
				<Button variant="outline" size="sm" onclick={newQuery}><FilePlusIcon data-icon="inline-start" /> New</Button>
				<Button variant="outline" size="sm" disabled={!currentSql.trim()} onclick={() => (saving = true)}>
					<SaveIcon data-icon="inline-start" /> Save
				</Button>
			</ButtonGroup.Root>
			{#if draft.saved}
				<Badge variant="secondary"><EyeIcon /> {draft.saved}</Badge>
				<Button variant="destructive" size="sm" onclick={() => deleteSaved()}><TrashIcon data-icon="inline-start" /> Delete</Button>
			{/if}

			<span class="flex-1"></span>
			{#if draft.result}
				<span class="text-sm text-muted-foreground">
					{draft.result.rows.length}{draft.result.truncated ? '+' : ''}
					{draft.result.rows.length === 1 ? 'row' : 'rows'} · {draft.result.elapsedMs} ms
				</span>
				{#if draft.result.truncated}
					<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning">First {draft.result.rows.length} rows</Badge>
				{/if}
			{/if}
		</div>

		<!-- Designer or editor -->
		<div class="flex max-h-[55%] min-h-48 shrink-0 flex-col overflow-auto border-b">
			{#if draft.mode === 'sql'}
				<Textarea
					bind:value={draft.sql}
					class="min-h-48 flex-1 resize-none rounded-none border-0 font-mono text-sm shadow-none focus-visible:ring-0"
					placeholder={'SELECT * FROM "Customers" WHERE …'}
					spellcheck={false}
					aria-label="SQL query"
				/>
			{:else if !draft.visual.from}
				<Empty.Root class="flex-1">
					<Empty.Header>
						<Empty.Media variant="icon"><SearchCodeIcon /></Empty.Media>
						<Empty.Title>Design a query</Empty.Title>
						<Empty.Description>Start from a table, add linked tables, then pick the fields to show.</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						{#if tableItems.length}
							<Select.Root type="single" items={tableItems} value="" onValueChange={(v) => v && addTable(v)}>
								<Select.Trigger class="w-56" aria-label="Start table"><Select.Value placeholder="Choose a table" /></Select.Trigger>
								<Select.Content>
									<Select.Group>
										{#each tableItems as t (t.value)}<Select.Item value={t.value} label={t.label}>{t.label}</Select.Item>{/each}
									</Select.Group>
								</Select.Content>
							</Select.Root>
						{:else}
							<p class="text-sm text-muted-foreground">Create a table on the Diagram first.</p>
						{/if}
					</Empty.Content>
				</Empty.Root>
			{:else}
				<!-- Tables in the query -->
				<div class="flex shrink-0 items-start gap-3 overflow-x-auto p-4">
					{#each tablesInQuery as name, i (name)}
						{@const d = def(name)}
						<div class="flex w-56 shrink-0 flex-col border bg-card">
							<div class="flex items-center gap-2 border-b px-3 py-2">
								<TableIcon class="size-4 text-muted-foreground" />
								<span class="flex-1 truncate text-sm font-semibold">{name}</span>
								<Button variant="ghost" size="icon-sm" aria-label={`Remove ${name}`} onclick={() => dropTable(name)}><XIcon /></Button>
							</div>
							{#if i > 0}
								{@const join = draft.visual.joins[i - 1]}
								<div class="border-b p-2">
									<Select.Root type="single" items={joinItems} value={join.kind} onValueChange={(v) => (join.kind = v as JoinKind)}>
										<Select.Trigger size="sm" class="w-full" aria-label="Join"><Select.Value /></Select.Trigger>
										<Select.Content>
											<Select.Group>
												{#each joinItems as j (j.value)}<Select.Item value={j.value} label={j.label}>{j.label}</Select.Item>{/each}
											</Select.Group>
										</Select.Content>
									</Select.Root>
								</div>
							{/if}
							<div class="flex max-h-48 flex-col overflow-y-auto p-1">
								{#each d?.fields ?? [] as f (f.name)}
									{@const on = hasColumn(name, f.name)}
									<Button
										variant="ghost"
										size="sm"
										class={cn('justify-start', on && 'bg-accent')}
										onclick={() => toggleColumn(name, f.name)}
									>
										{#if f.primary_key}<KeyRoundIcon data-icon="inline-start" class="text-warning" />{:else}<FieldTypeIcon type={f.field_type.type} data-icon="inline-start" />{/if}
										<span class="flex-1 truncate text-left">{f.name}</span>
										{#if on}<CheckIcon data-icon="inline-end" />{/if}
									</Button>
								{/each}
							</div>
						</div>
					{/each}
					{#if addable.length}
						<DropdownMenu.Root>
							<DropdownMenu.Trigger>
								{#snippet child({ props })}
									<Button {...props} variant="outline" size="sm"><PlusIcon data-icon="inline-start" /> Add table</Button>
								{/snippet}
							</DropdownMenu.Trigger>
							<DropdownMenu.Content align="start" class="w-56">
								<DropdownMenu.Group>
									<DropdownMenu.Label>Linked tables</DropdownMenu.Label>
									{#each addable as t (t)}
										<DropdownMenu.Item onclick={() => addTable(t)}><TableIcon /> {t}</DropdownMenu.Item>
									{/each}
								</DropdownMenu.Group>
							</DropdownMenu.Content>
						</DropdownMenu.Root>
					{/if}
				</div>

				<!-- Column grid -->
				{#if draft.visual.columns.length}
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head>Field</Table.Head>
								<Table.Head class="w-16 text-center">Show</Table.Head>
								<Table.Head class="w-36">Sort</Table.Head>
								<Table.Head class="w-36">Total</Table.Head>
								<Table.Head>Criteria</Table.Head>
								<Table.Head class="w-12"></Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each draft.visual.columns as col, i (`${col.table}.${col.field}`)}
								<Table.Row>
									<Table.Cell class="font-medium">
										<span class="text-muted-foreground">{col.table}.</span>{col.field}
									</Table.Cell>
									<Table.Cell class="text-center">
										<Checkbox checked={col.show} onCheckedChange={(v) => (col.show = !!v)} aria-label="Show" />
									</Table.Cell>
									<Table.Cell>
										<Select.Root type="single" items={sortItems} value={col.sort} onValueChange={(v) => (col.sort = v as SortOrder)}>
											<Select.Trigger size="sm" class="w-full" aria-label="Sort"><Select.Value /></Select.Trigger>
											<Select.Content>
												<Select.Group>
													{#each sortItems as s (s.value)}<Select.Item value={s.value} label={s.label}>{s.label}</Select.Item>{/each}
												</Select.Group>
											</Select.Content>
										</Select.Root>
									</Table.Cell>
									<Table.Cell>
										<Select.Root
											type="single"
											items={AGGREGATES}
											value={col.aggregate}
											onValueChange={(v) => (col.aggregate = v as Aggregate)}
										>
											<Select.Trigger size="sm" class="w-full" aria-label="Total"><Select.Value /></Select.Trigger>
											<Select.Content>
												<Select.Group>
													{#each AGGREGATES as a (a.value)}<Select.Item value={a.value} label={a.label}>{a.label}</Select.Item>{/each}
												</Select.Group>
											</Select.Content>
										</Select.Root>
									</Table.Cell>
									<Table.Cell>
										<div class="flex items-center gap-2">
											<Select.Root type="single" items={opItems} value={col.op} onValueChange={(v) => (col.op = v as FilterOp)}>
												<Select.Trigger size="sm" class="w-40" aria-label="Condition"><Select.Value /></Select.Trigger>
												<Select.Content>
													<Select.Group>
														{#each opItems as o (o.value)}<Select.Item value={o.value} label={o.label}>{o.label}</Select.Item>{/each}
													</Select.Group>
												</Select.Content>
											</Select.Root>
											{#if needsValue(col.op)}
												<Input
													class="h-8 min-w-32 flex-1"
													bind:value={col.value}
													placeholder={col.op === 'in' ? 'a, b, c' : 'Any value'}
													aria-label="Criteria value"
												/>
											{/if}
										</div>
									</Table.Cell>
									<Table.Cell>
										<Button
											variant="ghost"
											size="icon-sm"
											aria-label="Remove column"
											onclick={() => (draft.visual.columns = draft.visual.columns.filter((_, j) => j !== i))}
										>
											<XIcon />
										</Button>
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{:else}
					<p class="px-4 pb-4 text-sm text-muted-foreground">Click fields above to add them. With none picked, every field of {draft.visual.from} is shown.</p>
				{/if}
				{#if built.problems.length}
					<div class="m-4 mt-0 flex items-start gap-2 border border-warning/40 bg-warning/10 px-3 py-2 text-sm text-warning">
						<TriangleAlertIcon class="mt-0.5 size-4 shrink-0" />
						<span>{built.problems.join('. ')}. These criteria are skipped.</span>
					</div>
				{/if}
			{/if}
		</div>

		<!-- Results -->
		<div class="min-h-0 flex-1 overflow-auto">
			{#if !draft.result}
				<Empty.Root class="h-full">
					<Empty.Header>
						<Empty.Media variant="icon"><PlayIcon /></Empty.Media>
						<Empty.Title>No results yet</Empty.Title>
						<Empty.Description>Run the query to see its rows here. Queries only read data; they never change it.</Empty.Description>
					</Empty.Header>
				</Empty.Root>
			{:else if !draft.result.columns.length || !draft.result.rows.length}
				<Empty.Root class="h-full">
					<Empty.Header>
						<Empty.Title>No rows</Empty.Title>
						<Empty.Description>Nothing matches this query.</Empty.Description>
					</Empty.Header>
				</Empty.Root>
			{:else}
				{@const result = draft.result}
				<Table.Root class="w-auto">
					<Table.Header class="sticky top-0 z-10 bg-background">
						<Table.Row>
							<Table.Head class="w-14 border-e text-muted-foreground">#</Table.Head>
							{#each result.columns as c, ci (ci)}
								<Table.Head class={cn('min-w-32 border-e', numeric(result.rows[0][ci]) && 'text-right')}>{c}</Table.Head>
							{/each}
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each result.rows as row, ri (ri)}
							<Table.Row>
								<Table.Cell class="w-14 border-e text-muted-foreground">{ri + 1}</Table.Cell>
								{#each row as v, ci (ci)}
									<Table.Cell class={cn('max-w-96 border-e', numeric(v) && 'text-right tabular-nums')}>
										{#if v === null}
											<span class="text-muted-foreground"></span>
										{:else}
											<span class="block truncate">{displayValue(v, result.types[ci])}</span>
										{/if}
									</Table.Cell>
								{/each}
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{/if}
		</div>
	</div>
</div>

<NameDialog
	bind:open={saving}
	title="Save query"
	description="Saved queries keep their design and also open in the Datasheet as read-only views."
	initial={suggestedName()}
	placeholder="e.g. Active customers"
	submitLabel="Save query"
	validate={nameProblem}
	onSubmit={save}
/>
