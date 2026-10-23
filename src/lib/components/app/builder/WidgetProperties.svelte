<script lang="ts">
	// Services
	import { queryService } from '#lib/services/queries.js';

	// Types
	import type { Aggregate, AggregateOrder, DateBucket } from '#lib/api/types.js';

	// Utils
	import {
		addVariable,
		findWidget,
		variableProblem,
		widgetLabel,
		type Binding,
		type ButtonVariant,
		type ChartKind,
		type ChartSeries,
		type FilterControl,
		type Widget,
		type WidgetFilter
	} from '#lib/logic/app-def.js';
	import { AGGREGATE_LABELS } from '#lib/logic/chart-data.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { attempt } from '#lib/state/feedback.svelte.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import ActionEditor from './ActionEditor.svelte';
	import BindingPicker from './BindingPicker.svelte';
	import ColumnPicker from './ColumnPicker.svelte';
	import FiltersEditor from './FiltersEditor.svelte';
	import PropSelect from './PropSelect.svelte';
	import WidgetTypeIcon from './WidgetTypeIcon.svelte';

	// Icons
	import { TriangleAlert as TriangleAlertIcon, X as XIcon } from '@lucide/svelte';

	// Script
	let { widget }: { widget: Widget } = $props();

	const uid = $props.id();
	const binding = $derived(
		widget.type === 'filter' ? widget.options.data : 'data' in widget ? widget.data : null
	);
	const schema = $derived(binding ? appSession.knownSchema(binding.source, binding.table) : undefined);
	const columns = $derived(schema?.columns.map((c) => c.name) ?? []);
	const columnOptions = $derived(columns.map((c) => ({ value: c, label: c })));
	const problems = $derived(appSession.problems(widget));

	$effect(() => {
		if (binding) appSession.objectSchema(binding.source, binding.table);
	});

	/** Edits this widget. `group` merges typing in one box into one undo step. */
	function edit<T extends Widget>(change: (w: T) => void, group?: string) {
		const id = widget.id;
		appSession.change((def) => {
			const found = findWidget(def, id);
			if (found) change(found.widget as T);
		}, group ? `${id}:${group}` : undefined);
	}

	/** Points a data widget at another table, dropping settings that named the old one's fields. */
	function rebind(b: Binding) {
		edit((w) => {
			if (w.type === 'filter') {
				w.options = { data: b, valueColumn: '', labelColumn: '' };
				return;
			}
			if (!('data' in w)) return;
			const oldTable = w.data?.table;
			w.data = b;
			if (!w.title || w.title === oldTable || w.title === widgetLabel(w.type)) w.title = b.table;
			w.filters = [];
			if (w.type === 'table') {
				w.columns = [];
				w.badges = [];
				w.sort = null;
				if (w.rowAction.do === 'set-variable' || w.rowAction.do === 'navigate') w.rowAction = { do: 'edit-record' };
			}
			if (w.type === 'form') w.fields = [];
			if (w.type === 'board') {
				w.groupBy = '';
				w.lanes = [];
				w.cardTitle = '';
				w.cardFields = [];
				if (w.cardAction.do === 'set-variable' || w.cardAction.do === 'navigate') w.cardAction = { do: 'edit-record' };
			}
			if (w.type === 'chart') {
				w.groupBy = null;
				w.series = [{ column: null, agg: 'count' }];
			}
			if (w.type === 'stat') w.series = { column: null, agg: 'count' };
		});
	}

	const editFilters = (change: (fs: WidgetFilter[]) => void, group?: string) =>
		edit((w) => 'filters' in w && change(w.filters), group);

	const aggOptions = Object.entries(AGGREGATE_LABELS).map(([value, label]) => ({ value, label }));
	const seriesColumnOptions = $derived([{ value: '', label: '(records)' }, ...columnOptions]);
	const typeOf = (c: string) => schema?.columns.find((x) => x.name === c)?.type ?? null;
	/** Fields a board can make columns of: not long text, files or calculations. */
	const laneColumnOptions = $derived(
		(schema?.columns ?? [])
			.filter((c) => !c.type || !['long_text', 'attachment', 'calculated', 'auto_number'].includes(c.type.type))
			.map((c) => ({ value: c.name, label: c.name }))
	);

	function setSeries(s: ChartSeries, part: { agg?: string; column?: string }) {
		if (part.agg !== undefined) s.agg = part.agg as Aggregate;
		if (part.column !== undefined) s.column = part.column || null;
		// Without a field only counting records makes sense.
		if (!s.column) s.agg = 'count';
	}

	const chartKinds: { value: ChartKind; label: string }[] = [
		{ value: 'bar', label: 'Bar' },
		{ value: 'horizontal-bar', label: 'Horizontal bar' },
		{ value: 'line', label: 'Line' },
		{ value: 'area', label: 'Area' },
		{ value: 'pie', label: 'Pie' }
	];
	const orders: { value: AggregateOrder; label: string }[] = [
		{ value: 'group', label: 'By group' },
		{ value: 'value_desc', label: 'Largest first' },
		{ value: 'value_asc', label: 'Smallest first' }
	];
	const buckets = [
		{ value: 'exact', label: 'Exact value' },
		{ value: 'day', label: 'By day' },
		{ value: 'month', label: 'By month' },
		{ value: 'year', label: 'By year' }
	];
	const controls: { value: FilterControl; label: string }[] = [
		{ value: 'text', label: 'Text box' },
		{ value: 'number', label: 'Number' },
		{ value: 'select', label: 'Dropdown' },
		{ value: 'date', label: 'Date' },
		{ value: 'toggle', label: 'On/off switch' }
	];
	const variants: { value: ButtonVariant; label: string }[] = [
		{ value: 'default', label: 'Primary' },
		{ value: 'secondary', label: 'Secondary' },
		{ value: 'outline', label: 'Outline' },
		{ value: 'destructive', label: 'Danger (red)' }
	];
	const NEW_VARIABLE = '__new__';
	const variableOptions = $derived([
		...(appSession.def?.variables ?? []).map((v) => ({ value: v.name, label: v.name })),
		{ value: NEW_VARIABLE, label: 'New variable…' }
	]);

	// ----- Query widgets -----

	const sourceOptions = $derived((appSession.def?.sources ?? []).map((s) => ({ value: s.alias, label: s.database })));
	const savedQueries = $derived(
		widget.type === 'query' && widget.source
			? (appSession.objects[widget.source] ?? []).filter((o) => o.kind === 'view').map((o) => ({ value: o.name, label: o.name }))
			: []
	);

	async function copySavedQuery(name: string) {
		if (widget.type !== 'query') return;
		const database = appSession.databaseOf(widget.source);
		const saved = await attempt(() => queryService.get(name, database));
		if (!saved) return;
		edit((x) => {
			if (x.type !== 'query') return;
			x.sql = saved.sql;
			if (!x.title || x.title === 'Query') x.title = name;
		});
	}

	/** Creates a variable named after the filter's title and uses it. */
	function newVariableFor() {
		const def = appSession.def;
		if (!def) return;
		const base = (widget.title || 'filter').replace(/[^A-Za-z0-9_]+/g, '_').replace(/^[^A-Za-z_]+/, '') || 'filter';
		let name = base.toLowerCase();
		for (let n = 2; variableProblem(def, name); n++) name = `${base.toLowerCase()}${n}`;
		const id = widget.id;
		appSession.change((d) => {
			addVariable(d, name, null);
			const found = findWidget(d, id);
			if (found?.widget.type === 'filter') found.widget.variable = name;
		});
	}
</script>

<aside class="flex h-full w-80 shrink-0 flex-col border-s bg-sidebar print:hidden">
	<div class="flex h-11 shrink-0 items-center gap-2 border-b px-3">
		<WidgetTypeIcon type={widget.type} class="size-4 text-muted-foreground" />
		<span class="flex-1 truncate text-sm font-medium">{widgetLabel(widget.type)}</span>
		<Button size="icon-sm" variant="ghost" aria-label="Close properties" onclick={() => (appSession.selected = null)}><XIcon /></Button>
	</div>
	<ScrollArea class="min-h-0 flex-1">
		<div class="flex flex-col gap-5 p-3">
			{#if problems.length}
				<Alert.Root class="border-warning/40 bg-warning/10 text-warning">
					<TriangleAlertIcon />
					<Alert.Title>Needs attention</Alert.Title>
					<Alert.Description><ul class="list-disc ps-4">{#each problems as p (p)}<li>{p}</li>{/each}</ul></Alert.Description>
				</Alert.Root>
			{/if}

			<Field.Group class="gap-4">
				{#if widget.type !== 'text'}
					<Field.Field>
						<Field.Label for="{uid}-title">{widget.type === 'button' || widget.type === 'stat' ? 'Name (only shown while editing)' : 'Title'}</Field.Label>
						<Input
							id="{uid}-title"
							class="h-8"
							value={widget.title}
							oninput={(e) => {
								const v = e.currentTarget.value;
								edit((w) => (w.title = v), 'title');
							}}
							onblur={() => appSession.seal()}
						/>
					</Field.Field>
				{/if}

				{#if widget.type === 'table' || widget.type === 'form' || widget.type === 'board' || widget.type === 'chart' || widget.type === 'stat'}
					<Field.Field>
						<Field.Label for="{uid}-data">Data</Field.Label>
						<BindingPicker id="{uid}-data" binding={widget.data} tablesOnly={widget.type === 'form' || widget.type === 'board'} onchange={rebind} />
						{#if widget.type === 'form' || widget.type === 'board'}<Field.Description>{widget.type === 'form' ? 'Forms' : 'Boards'} edit a table; saved queries are read-only.</Field.Description>{/if}
					</Field.Field>
				{/if}
			</Field.Group>

			{#if widget.type === 'table'}
				{@const w = widget}
				<Separator />
				<Field.Set>
					<Field.Legend variant="label">Columns</Field.Legend>
					<ColumnPicker {columns} selected={w.columns} onchange={(next) => edit((x) => x.type === 'table' && (x.columns = next))} />
				</Field.Set>
				<Field.Set>
					<Field.Legend variant="label">Colored labels</Field.Legend>
					<Field.Description>Show values of these columns as colored labels, like a status.</Field.Description>
					<ColumnPicker {columns} selected={w.badges} showAll={false} orderable={false} onchange={(next) => edit((x) => x.type === 'table' && (x.badges = next))} />
				</Field.Set>
				<Field.Set>
					<Field.Legend variant="label">Only show records where</Field.Legend>
					<FiltersEditor filters={w.filters} {columns} onchange={editFilters} />
				</Field.Set>
				<Field.Group class="gap-4">
					<Field.Field>
						<Field.Label>Sort by</Field.Label>
						<div class="flex gap-1.5">
							<div class="min-w-0 flex-1">
								<PropSelect
									label="Sort by"
									value={w.sort?.column ?? ''}
									options={[{ value: '', label: 'Record order' }, ...columnOptions]}
									onchange={(v) => edit((x) => x.type === 'table' && (x.sort = v ? { column: v, descending: x.sort?.descending ?? false } : null))}
								/>
							</div>
							{#if w.sort}
								<div class="w-28">
									<PropSelect
										label="Direction"
										value={w.sort.descending ? 'desc' : 'asc'}
										options={[{ value: 'asc', label: 'A → Z' }, { value: 'desc', label: 'Z → A' }]}
										onchange={(v) => edit((x) => x.type === 'table' && x.sort && (x.sort.descending = v === 'desc'))}
									/>
								</div>
							{/if}
						</div>
					</Field.Field>
					<Field.Field>
						<Field.Label for="{uid}-size">Rows per page</Field.Label>
						<Input
							id="{uid}-size"
							class="h-8"
							type="number"
							min="5"
							max="500"
							value={w.pageSize}
							onchange={(e) => {
								const n = Math.max(5, Math.min(500, Number(e.currentTarget.value) || 25));
								edit((x) => x.type === 'table' && (x.pageSize = n));
							}}
						/>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-search" checked={w.search} onCheckedChange={(v) => edit((x) => x.type === 'table' && (x.search = v))} />
						<Field.Label for="{uid}-search" class="font-normal">Search box</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-new" checked={w.allowNew} onCheckedChange={(v) => edit((x) => x.type === 'table' && (x.allowNew = v))} />
						<Field.Label for="{uid}-new" class="font-normal">“New” button</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-tdel" checked={w.allowDelete} onCheckedChange={(v) => edit((x) => x.type === 'table' && (x.allowDelete = v))} />
						<Field.Label for="{uid}-tdel" class="font-normal">Delete records</Field.Label>
					</Field.Field>
				</Field.Group>
				<Field.Set>
					<Field.Legend variant="label">When a row is clicked</Field.Legend>
					<ActionEditor
						action={w.rowAction}
						context="row"
						{columns}
						onchange={(next, group) => edit((x) => x.type === 'table' && (x.rowAction = next), group)}
					/>
				</Field.Set>
			{:else if widget.type === 'form'}
				{@const w = widget}
				<Separator />
				<Field.Set>
					<Field.Legend variant="label">Fields</Field.Legend>
					<ColumnPicker {columns} selected={w.fields} onchange={(next) => edit((x) => x.type === 'form' && (x.fields = next))} />
				</Field.Set>
				<Field.Set>
					<Field.Legend variant="label">Step through records where</Field.Legend>
					<FiltersEditor filters={w.filters} {columns} onchange={editFilters} />
					<Field.Description>New records start with the values these filters pin down.</Field.Description>
				</Field.Set>
				<Field.Group class="gap-3">
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-fnew" checked={w.allowNew} onCheckedChange={(v) => edit((x) => x.type === 'form' && (x.allowNew = v))} />
						<Field.Label for="{uid}-fnew" class="font-normal">Add records</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-fedit" checked={w.allowEdit} onCheckedChange={(v) => edit((x) => x.type === 'form' && (x.allowEdit = v))} />
						<Field.Label for="{uid}-fedit" class="font-normal">Edit records</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-fdel" checked={w.allowDelete} onCheckedChange={(v) => edit((x) => x.type === 'form' && (x.allowDelete = v))} />
						<Field.Label for="{uid}-fdel" class="font-normal">Delete records</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch
							id="{uid}-fquick"
							checked={w.quickEntry}
							disabled={!w.allowNew}
							onCheckedChange={(v) => edit((x) => x.type === 'form' && (x.quickEntry = v))}
						/>
						<Field.Content>
							<Field.Label for="{uid}-fquick" class="font-normal">Quick entry</Field.Label>
							<Field.Description>Starts blank; Enter adds the record and clears the form for the next one. Click a row in a linked table to edit it.</Field.Description>
						</Field.Content>
					</Field.Field>
				</Field.Group>
			{:else if widget.type === 'board'}
				{@const w = widget}
				{@const groupType = typeOf(w.groupBy)}
				<Separator />
				<Field.Group class="gap-4">
					<Field.Field>
						<Field.Label>Columns come from</Field.Label>
						<PropSelect
							label="Columns come from"
							value={w.groupBy}
							options={laneColumnOptions}
							placeholder="Choose a field, like Status"
							onchange={(v) => edit((x) => x.type === 'board' && (x.groupBy = v))}
						/>
						<Field.Description>Each value gets a column. Drag a card to another column to change it.</Field.Description>
					</Field.Field>
					{#if w.groupBy && groupType?.type !== 'lookup' && groupType?.type !== 'yes_no'}
						<Field.Field>
							<Field.Label for="{uid}-lanes">Columns to always show</Field.Label>
							<Textarea
								id="{uid}-lanes"
								rows={4}
								placeholder={'To do\nDoing\nDone'}
								value={w.lanes.join('\n')}
								oninput={(e) => {
									const lanes = e.currentTarget.value.split('\n').map((l) => l.trim()).filter(Boolean);
									edit((x) => x.type === 'board' && (x.lanes = lanes), 'lanes');
								}}
								onblur={() => appSession.seal()}
							/>
							<Field.Description>One per line, in order, even while empty. Other values get columns after them.</Field.Description>
						</Field.Field>
					{/if}
					<Field.Field>
						<Field.Label>Card heading</Field.Label>
						<PropSelect
							label="Card heading"
							value={w.cardTitle}
							options={[{ value: '', label: 'First text field' }, ...columnOptions]}
							onchange={(v) => edit((x) => x.type === 'board' && (x.cardTitle = v))}
						/>
					</Field.Field>
				</Field.Group>
				<Field.Set>
					<Field.Legend variant="label">Also show on cards</Field.Legend>
					<ColumnPicker
						columns={columns.filter((c) => c !== w.groupBy)}
						selected={w.cardFields}
						showAll={false}
						onchange={(next) => edit((x) => x.type === 'board' && (x.cardFields = next))}
					/>
				</Field.Set>
				<Field.Set>
					<Field.Legend variant="label">Only show records where</Field.Legend>
					<FiltersEditor filters={w.filters} {columns} onchange={editFilters} />
				</Field.Set>
				<Field.Field orientation="horizontal">
					<Switch id="{uid}-bnew" checked={w.allowNew} onCheckedChange={(v) => edit((x) => x.type === 'board' && (x.allowNew = v))} />
					<Field.Label for="{uid}-bnew" class="font-normal">Add records from each column</Field.Label>
				</Field.Field>
				<Field.Field orientation="horizontal">
					<Switch id="{uid}-bdel" checked={w.allowDelete} onCheckedChange={(v) => edit((x) => x.type === 'board' && (x.allowDelete = v))} />
					<Field.Label for="{uid}-bdel" class="font-normal">Delete records</Field.Label>
				</Field.Field>
				<Field.Set>
					<Field.Legend variant="label">When a card is clicked</Field.Legend>
					<ActionEditor
						action={w.cardAction}
						context="row"
						{columns}
						onchange={(next, group) => edit((x) => x.type === 'board' && (x.cardAction = next), group)}
					/>
				</Field.Set>
			{:else if widget.type === 'chart'}
				{@const w = widget}
				<Separator />
				<Field.Group class="gap-4">
					<Field.Field>
						<Field.Label>Chart</Field.Label>
						<PropSelect label="Chart" value={w.chart} options={chartKinds} onchange={(v) => edit((x) => x.type === 'chart' && (x.chart = v as ChartKind))} />
					</Field.Field>
					<Field.Field>
						<Field.Label>{w.chart === 'pie' ? 'Slices' : 'Group by'}</Field.Label>
						<PropSelect
							label="Group by"
							value={w.groupBy?.column ?? ''}
							options={[{ value: '', label: 'No grouping (one total)' }, ...columnOptions]}
							onchange={(v) =>
								edit((x) => {
									if (x.type !== 'chart') return;
									x.groupBy = v ? { column: v, bucket: typeOf(v)?.type === 'date_time' ? 'month' : null } : null;
								})}
						/>
						{#if w.groupBy && typeOf(w.groupBy.column)?.type === 'date_time'}
							<PropSelect
								label="Dates"
								value={w.groupBy.bucket ?? 'exact'}
								options={buckets}
								onchange={(v) => edit((x) => x.type === 'chart' && x.groupBy && (x.groupBy.bucket = v === 'exact' ? null : (v as DateBucket)))}
							/>
						{/if}
					</Field.Field>
				</Field.Group>
				<Field.Set>
					<Field.Legend variant="label">{w.chart === 'pie' ? 'Size of each slice' : 'Values'}</Field.Legend>
					<div class="flex flex-col gap-2">
						{#each w.chart === 'pie' ? w.series.slice(0, 1) : w.series as s, i (i)}
							<div class="flex items-center gap-1">
								<div class="w-28 shrink-0">
									<PropSelect label="Total" value={s.agg} options={aggOptions} disabled={!s.column} onchange={(v) => edit((x) => x.type === 'chart' && setSeries(x.series[i], { agg: v }))} />
								</div>
								<div class="min-w-0 flex-1">
									<PropSelect label="Of field" value={s.column ?? ''} options={seriesColumnOptions} onchange={(v) => edit((x) => x.type === 'chart' && setSeries(x.series[i], { column: v }))} />
								</div>
								{#if w.series.length > 1}
									<Button size="icon-xs" variant="ghost" aria-label="Remove value" onclick={() => edit((x) => x.type === 'chart' && x.series.splice(i, 1))}><XIcon /></Button>
								{/if}
							</div>
						{/each}
						{#if w.chart !== 'pie'}
							<Button size="sm" variant="outline" onclick={() => edit((x) => x.type === 'chart' && x.series.push({ column: null, agg: 'count' }))}>Add a value</Button>
						{/if}
					</div>
				</Field.Set>
				{#if w.groupBy}
					<Field.Group class="gap-4">
						<Field.Field>
							<Field.Label>Order</Field.Label>
							<PropSelect label="Order" value={w.order} options={orders} onchange={(v) => edit((x) => x.type === 'chart' && (x.order = v as AggregateOrder))} />
						</Field.Field>
						<Field.Field>
							<Field.Label for="{uid}-limit">At most this many groups</Field.Label>
							<Input
								id="{uid}-limit"
								class="h-8"
								type="number"
								min="1"
								max="1000"
								value={w.limit}
								onchange={(e) => {
									const n = Math.max(1, Math.min(1000, Number(e.currentTarget.value) || 50));
									edit((x) => x.type === 'chart' && (x.limit = n));
								}}
							/>
						</Field.Field>
					</Field.Group>
				{/if}
				<Field.Set>
					<Field.Legend variant="label">Only count records where</Field.Legend>
					<FiltersEditor filters={w.filters} {columns} onchange={editFilters} />
				</Field.Set>
			{:else if widget.type === 'stat'}
				{@const w = widget}
				<Separator />
				<Field.Field>
					<Field.Label>Show</Field.Label>
					<div class="flex items-center gap-1">
						<div class="w-28 shrink-0">
							<PropSelect label="Total" value={w.series.agg} options={aggOptions} disabled={!w.series.column} onchange={(v) => edit((x) => x.type === 'stat' && setSeries(x.series, { agg: v }))} />
						</div>
						<div class="min-w-0 flex-1">
							<PropSelect label="Of field" value={w.series.column ?? ''} options={seriesColumnOptions} onchange={(v) => edit((x) => x.type === 'stat' && setSeries(x.series, { column: v }))} />
						</div>
					</div>
				</Field.Field>
				<Field.Field>
					<Field.Label for="{uid}-caption">Caption</Field.Label>
					<Input
						id="{uid}-caption"
						class="h-8"
						placeholder="e.g. Revenue this month"
						value={w.caption}
						oninput={(e) => {
							const v = e.currentTarget.value;
							edit((x) => x.type === 'stat' && (x.caption = v), 'caption');
						}}
						onblur={() => appSession.seal()}
					/>
				</Field.Field>
				<Field.Set>
					<Field.Legend variant="label">Only count records where</Field.Legend>
					<FiltersEditor filters={w.filters} {columns} onchange={editFilters} />
				</Field.Set>
			{:else if widget.type === 'filter'}
				{@const w = widget}
				<Separator />
				<Field.Group class="gap-4">
					<Field.Field>
						<Field.Label>Sets variable</Field.Label>
						<PropSelect
							label="Sets variable"
							value={w.variable}
							options={variableOptions}
							placeholder="Choose a variable"
							onchange={(v) => (v === NEW_VARIABLE ? newVariableFor() : edit((x) => x.type === 'filter' && (x.variable = v)))}
						/>
						<Field.Description>Use the variable in other widgets' filters to make them follow this control.</Field.Description>
					</Field.Field>
					<Field.Field>
						<Field.Label>Control</Field.Label>
						<PropSelect label="Control" value={w.control} options={controls} onchange={(v) => edit((x) => x.type === 'filter' && (x.control = v as FilterControl))} />
					</Field.Field>
					<Field.Field>
						<Field.Label for="{uid}-ph">{w.control === 'toggle' ? 'Label' : w.control === 'select' ? 'Label for “all”' : 'Placeholder'}</Field.Label>
						<Input
							id="{uid}-ph"
							class="h-8"
							value={w.placeholder}
							oninput={(e) => {
								const v = e.currentTarget.value;
								edit((x) => x.type === 'filter' && (x.placeholder = v), 'placeholder');
							}}
							onblur={() => appSession.seal()}
						/>
					</Field.Field>
					{#if w.control === 'select'}
						<Field.Field>
							<Field.Label for="{uid}-opts">Choices from</Field.Label>
							<BindingPicker id="{uid}-opts" binding={w.options.data} onchange={rebind} />
						</Field.Field>
						{#if w.options.data}
							<Field.Field>
								<Field.Label>Value</Field.Label>
								<PropSelect label="Value" value={w.options.valueColumn} options={columnOptions} onchange={(v) => edit((x) => x.type === 'filter' && (x.options.valueColumn = v))} />
								<Field.Description>What the variable is set to, e.g. the ID.</Field.Description>
							</Field.Field>
							<Field.Field>
								<Field.Label>Shown as</Field.Label>
								<PropSelect
									label="Shown as"
									value={w.options.labelColumn}
									options={[{ value: '', label: 'The value itself' }, ...columnOptions]}
									onchange={(v) => edit((x) => x.type === 'filter' && (x.options.labelColumn = v))}
								/>
							</Field.Field>
						{/if}
					{/if}
				</Field.Group>
			{:else if widget.type === 'button'}
				{@const w = widget}
				<Separator />
				<Field.Group class="gap-4">
					<Field.Field>
						<Field.Label for="{uid}-label">Label</Field.Label>
						<Input
							id="{uid}-label"
							class="h-8"
							value={w.label}
							oninput={(e) => {
								const v = e.currentTarget.value;
								edit((x) => x.type === 'button' && (x.label = v), 'label');
							}}
							onblur={() => appSession.seal()}
						/>
					</Field.Field>
					<Field.Field>
						<Field.Label>Style</Field.Label>
						<PropSelect label="Style" value={w.variant} options={variants} onchange={(v) => edit((x) => x.type === 'button' && (x.variant = v as ButtonVariant))} />
					</Field.Field>
				</Field.Group>
				<Field.Set>
					<Field.Legend variant="label">When clicked</Field.Legend>
					<ActionEditor action={w.action} context="button" onchange={(next, group) => edit((x) => x.type === 'button' && (x.action = next), group)} />
				</Field.Set>
			{:else if widget.type === 'query'}
				{@const w = widget}
				<Separator />
				<Field.Group class="gap-4">
					<Field.Field>
						<Field.Label>Database</Field.Label>
						<PropSelect
							label="Database"
							value={w.source}
							options={sourceOptions}
							placeholder={sourceOptions.length ? 'Choose a database' : 'Add a database first'}
							onchange={(v) => edit((x) => x.type === 'query' && (x.source = v))}
						/>
					</Field.Field>
					{#if savedQueries.length}
						<Field.Field>
							<Field.Label>Start from a saved query</Field.Label>
							<PropSelect label="Saved query" value="" options={savedQueries} placeholder="Copy its SQL…" onchange={copySavedQuery} />
						</Field.Field>
					{/if}
					<Field.Field>
						<Field.Label for="{uid}-sql">SQL</Field.Label>
						<Textarea
							id="{uid}-sql"
							rows={10}
							class="font-mono text-xs"
							spellcheck={false}
							placeholder="SELECT …"
							value={w.sql}
							oninput={(e) => {
								const v = e.currentTarget.value;
								edit((x) => x.type === 'query' && (x.sql = v), 'sql');
							}}
							onblur={() => appSession.seal()}
						/>
						<Field.Description>
							One query that only reads data. Write <code>:name</code> to use an app variable, e.g.
							<code>WHERE Company = :company</code>; an empty variable is NULL.
						</Field.Description>
						{#if appSession.def?.variables.length}
							<div class="flex flex-wrap gap-1">
								{#each appSession.def.variables as v (v.name)}
									<Button
										size="xs"
										variant="outline"
										class="font-mono"
										onclick={() => edit((x) => x.type === 'query' && (x.sql = `${x.sql}${/\s$/.test(x.sql) || !x.sql ? '' : ' '}:${v.name}`))}
									>
										:{v.name}
									</Button>
								{/each}
							</div>
						{/if}
					</Field.Field>
					<Field.Field orientation="horizontal">
						<Switch id="{uid}-qsearch" checked={w.search} onCheckedChange={(v) => edit((x) => x.type === 'query' && (x.search = v))} />
						<Field.Label for="{uid}-qsearch" class="font-normal">Search box</Field.Label>
					</Field.Field>
				</Field.Group>
				<Field.Set>
					<Field.Legend variant="label">When a row is clicked</Field.Legend>
					<ActionEditor
						action={w.rowAction}
						context="query"
						columns={appSession.queryColumns[w.id] ?? []}
						onchange={(next, group) => edit((x) => x.type === 'query' && (x.rowAction = next), group)}
					/>
				</Field.Set>
			{:else if widget.type === 'text'}
				{@const w = widget}
				<Field.Field>
					<Field.Label for="{uid}-md">Text</Field.Label>
					<Textarea
						id="{uid}-md"
						rows={12}
						class="font-mono text-xs"
						value={w.markdown}
						oninput={(e) => {
							const v = e.currentTarget.value;
							edit((x) => x.type === 'text' && (x.markdown = v), 'markdown');
						}}
						onblur={() => appSession.seal()}
					/>
					<Field.Description># Heading, **bold**, *italic*, `code`, - lists, &gt; quotes and --- lines.</Field.Description>
				</Field.Field>
			{/if}
		</div>
	</ScrollArea>
</aside>
