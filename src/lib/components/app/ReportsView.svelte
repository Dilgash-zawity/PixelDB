<script lang="ts">
	// Services
	import { databaseService } from '#lib/services/database.js';
	import { recordService } from '#lib/services/records.js';
	import { tableService } from '#lib/services/tables.js';

	// Types
	import type { FieldType, JsonValue } from '#lib/api/types.js';

	// Utils
	import { onDestroy, tick, untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { t } from '#lib/i18n/index.js';
	import { toFilterSpecs, type FilterRow } from '#lib/logic/filters.js';
	import { loadLookups, type LookupOption } from '#lib/logic/lookups.js';
	import {
		MAX_REPORT_ROWS,
		TOTALS,
		buildReport,
		newReport,
		reportNameProblem,
		reportSort,
		totalsFor,
		type ReportDef,
		type TotalKind
	} from '#lib/logic/reports.js';
	import { jsonClone } from '#lib/logic/values.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { session } from '#lib/state/session.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import FieldTypeIcon from './FieldTypeIcon.svelte';
	import FilterPopover from './FilterPopover.svelte';
	import ReportPaper from './ReportPaper.svelte';

	// Icons
	import {
		ArrowDown as ArrowDownIcon,
		ArrowUp as ArrowUpIcon,
		FileText as FileTextIcon,
		Plus as PlusIcon,
		Printer as PrinterIcon,
		Trash2 as TrashIcon
	} from '@lucide/svelte';

	// Script
	/**
	 * The Reports tab: saved reports on the left, the report as it will print
	 * in the middle, and its design on the right. Changes save by themselves
	 * into the database file; Print uses the system dialog (which can also
	 * save a PDF).
	 */

	const NONE = '__none__';
	const SAVE_DELAY = 500;

	const names = $derived(Object.keys(session.reports).sort((a, b) => a.localeCompare(b)));
	const def = $derived(session.report ? (session.reports[session.report] ?? null) : null);
	const sources = $derived([...session.tables, ...session.views].map((o) => o.name));

	// Pick a report when none is chosen (or it was deleted).
	$effect(() => {
		if (!session.report || !session.reports[session.report]) session.report = names[0] ?? null;
	});

	// ----- changes -----

	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	let nameDraft = $state('');
	$effect(() => {
		nameDraft = session.report ?? '';
	});

	/** Applies a change to the open report and saves soon after. */
	function change(edit: (r: ReportDef) => void) {
		if (!def) return;
		const next = jsonClone(def);
		edit(next);
		session.reports = { ...session.reports, [next.name]: next };
		clearTimeout(saveTimer);
		saveTimer = setTimeout(saveNow, SAVE_DELAY);
	}

	function saveNow() {
		clearTimeout(saveTimer);
		saveTimer = undefined;
		void attempt(() => databaseService.saveDocument('reports', $state.snapshot(session.reports)));
	}

	onDestroy(() => {
		if (saveTimer !== undefined) saveNow();
	});

	function rename() {
		const from = session.report;
		const to = nameDraft.trim();
		if (!from || !def || to === from) return;
		const problem = reportNameProblem(session.reports, to, from);
		if (problem) {
			toast.error(problem);
			nameDraft = from;
			return;
		}
		const { [from]: old, ...rest } = session.reports;
		session.report = to;
		void session.setReports({ ...rest, [to]: { ...old, name: to, title: old.title === from ? to : old.title } });
	}

	async function create() {
		const source = (session.tab === 'reports' && def?.source) || session.table || sources[0];
		if (!source) {
			toast('Add a table first', { description: 'A report lists the records of a table or saved query.' });
			return;
		}
		let n = 1;
		while (session.reports[`Report ${n}`]) n++;
		const name = `Report ${n}`;
		const info = await attempt(() => tableService.get(source).catch(() => null));
		const fields = info
			? info.definition.fields.map((f) => ({ name: f.name, type: f.field_type }))
			: ((await attempt(() => recordService.list(source, { limit: 1 })))?.columns ?? []).map((c) => ({ name: c, type: null }));
		if (saveTimer !== undefined) saveNow();
		if (await session.setReports({ ...session.reports, [name]: newReport(name, source, fields) })) session.report = name;
	}

	async function remove() {
		const name = session.report;
		if (!name) return;
		const ok = await confirm({ title: t('reports.deleteTitle', { name }), description: t('reports.deleteText'), action: t('reports.deleteAction'), destructive: true });
		if (!ok) return;
		const { [name]: _gone, ...rest } = session.reports;
		clearTimeout(saveTimer);
		saveTimer = undefined;
		if (await session.setReports(rest)) session.report = null;
	}

	// ----- data -----

	interface Loaded {
		key: string;
		columns: string[];
		types: (FieldType | null)[];
		rows: JsonValue[][];
		total: number;
		lookups: Record<string, LookupOption[]>;
	}

	let data = $state<Loaded | null>(null);
	let loading = $state(false);
	let loadError = $state<string | null>(null);

	const request = $derived(def ? JSON.stringify({ source: def.source, filters: toFilterSpecs(def.filters), sort: reportSort(def), r: session.revision }) : '');

	$effect(() => {
		const key = request;
		if (!key || !def) return;
		const source = def.source;
		const filters = toFilterSpecs(def.filters);
		const sort = reportSort(def);
		untrack(() => load(key, source, filters, sort));
	});

	async function load(key: string, source: string, filters: ReturnType<typeof toFilterSpecs>, sort: ReturnType<typeof reportSort>) {
		loading = true;
		try {
			const rows: JsonValue[][] = [];
			let columns: string[] = [];
			let types: (FieldType | null)[] = [];
			let total = 0;
			for (let offset = 0; rows.length < MAX_REPORT_ROWS; offset += 1000) {
				const page = await recordService.list(source, { offset, limit: 1000, filters, sort });
				if (key !== request) return;
				columns = page.columns;
				types = page.types;
				total = page.total;
				rows.push(...page.rows.map((r) => r.values));
				if (!page.rows.length || rows.length >= page.total) break;
			}
			const info = await tableService.get(source).catch(() => null);
			const lookups = info ? await loadLookups(info.definition) : {};
			if (key !== request) return;
			data = { key, columns, types, rows: rows.slice(0, MAX_REPORT_ROWS), total, lookups };
			loadError = null;
		} catch (e) {
			if (key === request) loadError = e instanceof Error ? e.message : String(e);
		} finally {
			if (key === request) loading = false;
		}
	}

	const built = $derived(def && data ? buildReport(def, data, data.lookups) : null);
	const fieldType = (f: string) => (data ? (data.types[data.columns.indexOf(f)] ?? null) : null);
	/** Every field of the source, chosen ones first in their order. */
	const fieldRows = $derived.by(() => {
		if (!def || !data) return [];
		const chosen = def.columns.filter((c) => data!.columns.includes(c.field));
		const rest = data.columns.filter((c) => !chosen.some((x) => x.field === c));
		return [...chosen.map((c) => ({ field: c.field, column: c })), ...rest.map((field) => ({ field, column: null }))];
	});

	function toggleField(field: string, on: boolean) {
		change((r) => {
			if (on && !r.columns.some((c) => c.field === field)) r.columns.push({ field, label: '', total: 'none' });
			if (!on) r.columns = r.columns.filter((c) => c.field !== field);
		});
	}

	function move(field: string, by: number) {
		change((r) => {
			const i = r.columns.findIndex((c) => c.field === field);
			const j = i + by;
			if (i < 0 || j < 0 || j >= r.columns.length) return;
			[r.columns[i], r.columns[j]] = [r.columns[j], r.columns[i]];
		});
	}

	// ----- printing -----

	let printing = $state(false);

	/** Prints just the report: the rest of the window is hidden while the system dialog is open. */
	async function print() {
		if (!def || !built) return;
		if (saveTimer !== undefined) saveNow();
		const style = document.createElement('style');
		style.textContent = `@page { size: A4 ${def.landscape ? 'landscape' : 'portrait'}; margin: 14mm; }`;
		document.head.append(style);
		document.documentElement.dataset.printing = 'report';
		printing = true;
		await tick();
		const done = () => {
			printing = false;
			delete document.documentElement.dataset.printing;
			style.remove();
			window.removeEventListener('afterprint', done);
		};
		window.addEventListener('afterprint', done);
		window.print();
		// Some webviews don't send afterprint; print() has returned by then anyway.
		setTimeout(done, 1000);
	}

	/** Moves the print copy to the end of <body>, outside the app, so print CSS can show only it. */
	function portal(node: HTMLElement) {
		document.body.append(node);
		return () => node.remove();
	}

	const sourceItems = $derived(sources.map((s) => ({ value: s, label: s })));
	const groupItems = $derived([{ value: NONE, label: 'No grouping' }, ...(data?.columns ?? []).map((c) => ({ value: c, label: c }))]);
	const sortItems = $derived([{ value: NONE, label: 'Table order' }, ...(data?.columns ?? []).map((c) => ({ value: c, label: c }))]);
</script>

{#snippet select(id: string, items: { value: string; label: string }[], value: string, onChange: (v: string) => void)}
	<Select.Root type="single" {items} {value} onValueChange={onChange}>
		<Select.Trigger {id} size="sm" class="w-full min-w-0"><Select.Value /></Select.Trigger>
		<Select.Content>
			<Select.Group>
				{#each items as item (item.value)}<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>{/each}
			</Select.Group>
		</Select.Content>
	</Select.Root>
{/snippet}

<div class="flex h-full min-h-0">
	<!-- Saved reports -->
	<aside class="flex w-56 shrink-0 flex-col gap-1 border-e bg-sidebar p-2">
		<Button variant="outline" size="sm" class="mb-1 justify-start" onclick={create}><PlusIcon data-icon="inline-start" /> {t('reports.newReport')}</Button>
		{#each names as n (n)}
			<button
				type="button"
				class={cn(
					'flex items-center gap-2 rounded-md px-2 py-1.5 text-start text-sm outline-none hover:bg-accent focus-visible:ring-3 focus-visible:ring-ring/40',
					n === session.report && 'bg-accent font-medium'
				)}
				onclick={() => {
					if (saveTimer !== undefined) saveNow();
					session.report = n;
				}}
			>
				<FileTextIcon class="size-4 shrink-0 text-muted-foreground" />
				<span class="truncate">{n}</span>
			</button>
		{:else}
			<p class="px-2 py-1 text-xs text-muted-foreground">{t('reports.none')}</p>
		{/each}
	</aside>

	{#if !def}
		<Empty.Root class="flex-1">
			<Empty.Header>
				<Empty.Media variant="icon"><FileTextIcon /></Empty.Media>
				<Empty.Title>{t('reports.emptyTitle')}</Empty.Title>
				<Empty.Description>{t('reports.emptyText')}</Empty.Description>
			</Empty.Header>
			<Empty.Content>
				<Button onclick={create}><PlusIcon data-icon="inline-start" /> {t('reports.newReport')}</Button>
			</Empty.Content>
		</Empty.Root>
	{:else}
		<!-- Preview -->
		<div class="flex min-w-0 flex-1 flex-col">
			<div class="flex items-center gap-2 border-b px-4 py-2">
				<span class="truncate text-sm font-medium">{def.name}</span>
				{#if data && data.total > data.rows.length}<span class="text-xs text-warning">{t('reports.firstOf', { shown: data.rows.length.toLocaleString(), total: data.total.toLocaleString() })}</span>{/if}
				<span class="flex-1"></span>
				<Button size="sm" disabled={!built || loading} onclick={print}><PrinterIcon data-icon="inline-start" /> {t('reports.print')}</Button>
			</div>
			<div class="min-h-0 flex-1 overflow-auto bg-muted/40 p-6">
				{#if loadError}
					<p class="text-sm text-destructive">Couldn’t read “{def.source}”: {loadError}</p>
				{:else if !built}
					<div class="mx-auto flex max-w-3xl flex-col gap-2">{#each Array(6) as _, i (i)}<Skeleton class="h-8" />{/each}</div>
				{:else}
					<ReportPaper {def} {built} database={session.name ?? ''} truncated={!!data && data.total > data.rows.length} class={cn('mx-auto rounded-lg border', def.landscape ? 'max-w-5xl' : 'max-w-3xl', loading && 'opacity-60')} />
				{/if}
			</div>
		</div>

		<!-- Design -->
		<aside class="flex w-80 shrink-0 flex-col overflow-y-auto border-s bg-sidebar">
			<div class="flex flex-col gap-4 p-4">
				<Field.Field>
					<Field.Label for="rp-name">Report name</Field.Label>
					<Input id="rp-name" bind:value={nameDraft} onblur={rename} onkeydown={(e) => e.key === 'Enter' && rename()} autocomplete="off" />
				</Field.Field>
				<Field.Field>
					<Field.Label for="rp-source">Records from</Field.Label>
					{@render select('rp-source', sourceItems, def.source, (v) => change((r) => { r.source = v; r.columns = []; r.groupBy = null; r.sort = null; r.filters = []; }))}
					<Field.Description>A table or a saved query. Changing it starts the fields over.</Field.Description>
				</Field.Field>
				<Field.Field>
					<Field.Label for="rp-title">Title</Field.Label>
					<Input id="rp-title" value={def.title} oninput={(e) => change((r) => (r.title = e.currentTarget.value))} autocomplete="off" />
				</Field.Field>
				<Field.Field>
					<Field.Label for="rp-subtitle">Subtitle</Field.Label>
					<Input id="rp-subtitle" value={def.subtitle} placeholder="Optional" oninput={(e) => change((r) => (r.subtitle = e.currentTarget.value))} autocomplete="off" />
				</Field.Field>
			</div>
			<Separator />
			<div class="flex flex-col gap-3 p-4">
				<span class="text-sm font-medium">Fields</span>
				{#if !data}
					<Skeleton class="h-24" />
				{:else}
					<ul class="flex flex-col gap-1.5">
						{#each fieldRows as { field, column } (field)}
							{@const t = fieldType(field)}
							{@const ci = def.columns.findIndex((c) => c.field === field)}
							<li class={cn('flex flex-col gap-1.5 rounded-md border bg-background p-2', !column && 'border-dashed')}>
								<div class="flex items-center gap-2">
									<Checkbox checked={!!column} onCheckedChange={(v) => toggleField(field, v)} aria-label="Show {field}" />
									{#if t}<FieldTypeIcon type={t.type} class="size-3.5 text-muted-foreground" />{/if}
									<span class="min-w-0 flex-1 truncate text-sm">{field}</span>
									{#if column}
										<Button variant="ghost" size="icon-xs" aria-label="Move {field} up" disabled={ci === 0} onclick={() => move(field, -1)}><ArrowUpIcon /></Button>
										<Button variant="ghost" size="icon-xs" aria-label="Move {field} down" disabled={ci === def.columns.length - 1} onclick={() => move(field, 1)}><ArrowDownIcon /></Button>
									{/if}
								</div>
								{#if column}
									<div class="flex gap-1.5">
										<Input
											class="h-7 min-w-0 flex-1 text-xs"
											value={column.label}
											placeholder={field}
											aria-label="Heading for {field}"
											oninput={(e) => {
												const v = e.currentTarget.value;
												change((r) => {
													const c = r.columns.find((x) => x.field === field);
													if (c) c.label = v;
												});
											}}
										/>
										<Select.Root
											type="single"
											items={TOTALS.filter((x) => totalsFor(t).includes(x.value))}
											value={column.total}
											onValueChange={(v) =>
												change((r) => {
													const c = r.columns.find((x) => x.field === field);
													if (c) c.total = v as TotalKind;
												})}
										>
											<Select.Trigger size="sm" class="h-7 w-28 text-xs" aria-label="Total for {field}"><Select.Value /></Select.Trigger>
											<Select.Content>
												<Select.Group>
													{#each TOTALS.filter((x) => totalsFor(t).includes(x.value)) as x (x.value)}<Select.Item value={x.value} label={x.label}>{x.label}</Select.Item>{/each}
												</Select.Group>
											</Select.Content>
										</Select.Root>
									</div>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</div>
			<Separator />
			<div class="flex flex-col gap-4 p-4">
				<Field.Field>
					<Field.Label for="rp-group">Group by</Field.Label>
					{@render select('rp-group', groupItems, def.groupBy ?? NONE, (v) => change((r) => (r.groupBy = v === NONE ? null : v)))}
					<Field.Description>Each value gets a heading and its own totals.</Field.Description>
				</Field.Field>
				<Field.Field>
					<Field.Label for="rp-sort">Sort by</Field.Label>
					<div class="flex gap-2">
						{@render select('rp-sort', sortItems, def.sort?.column ?? NONE, (v) => change((r) => (r.sort = v === NONE ? null : { column: v, descending: r.sort?.descending ?? false })))}
						<Button
							variant="outline"
							size="icon-sm"
							disabled={!def.sort}
							aria-label={def.sort?.descending ? 'Sorted Z to A' : 'Sorted A to Z'}
							title={def.sort?.descending ? 'Z to A' : 'A to Z'}
							onclick={() => change((r) => r.sort && (r.sort.descending = !r.sort.descending))}
						>
							{#if def.sort?.descending}<ArrowDownIcon />{:else}<ArrowUpIcon />{/if}
						</Button>
					</div>
				</Field.Field>
				<Field.Field>
					<Field.Label>Only records where…</Field.Label>
					<div>
						<FilterPopover
							columns={data?.columns ?? []}
							active={toFilterSpecs(def.filters).length}
							initial={def.filters}
							onApply={(rows: FilterRow[]) => change((r) => (r.filters = jsonClone(rows)))}
						/>
					</div>
				</Field.Field>
				<Field.Field orientation="horizontal">
					<Field.Label for="rp-landscape">Landscape page</Field.Label>
					<Switch id="rp-landscape" checked={def.landscape} onCheckedChange={(v) => change((r) => (r.landscape = v))} />
				</Field.Field>
				<Field.Field orientation="horizontal">
					<Field.Label for="rp-count">Show record counts</Field.Label>
					<Switch id="rp-count" checked={def.showCount} onCheckedChange={(v) => change((r) => (r.showCount = v))} />
				</Field.Field>
			</div>
			<Separator />
			<div class="p-4">
				<Button variant="destructive" size="sm" onclick={remove}><TrashIcon data-icon="inline-start" /> Delete report</Button>
			</div>
		</aside>
	{/if}
</div>

{#if printing && def && built}
	<div data-print-area {@attach portal}>
		<ReportPaper {def} {built} database={session.name ?? ''} truncated={!!data && data.total > data.rows.length} />
	</div>
{/if}
