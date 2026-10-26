<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { BatchOp, FieldDef, FilterSpec, RecordRow, SortSpec } from '#lib/api/types.js';

	// Utils
	import { toast } from 'svelte-sonner';
	import { MAX_BATCH, planReplace, replaceableFields, type ReplaceOptions } from '#lib/logic/bulk-edit.js';
	import { attempt } from '#lib/state/feedback.svelte.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import { Checkbox } from '#lib/components/ui/checkbox/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';

	// Script
	/**
	 * Find & replace in the text fields of every record the Datasheet's filter
	 * and search show (not only the page on screen). Find counts first; Replace
	 * all changes them in one go, all or none.
	 */
	let {
		open = $bindable(false),
		table,
		fields,
		filters,
		search,
		sort,
		initialField = null,
		onReplaced
	}: {
		open?: boolean;
		table: string;
		fields: FieldDef[];
		filters: FilterSpec[];
		search: string;
		sort: SortSpec | null;
		/** Starts with this field chosen. */
		initialField?: string | null;
		onReplaced: () => void;
	} = $props();

	const ALL = '__all__';
	const PAGE = 1000;

	let field = $state(ALL);
	let options = $state<ReplaceOptions>({ find: '', replace: '', matchCase: false, wholeCell: false });
	let found = $state<{ ops: BatchOp[]; cells: number; records: number; capped: boolean } | null>(null);
	let busy = $state(false);
	let findInput = $state<HTMLInputElement | null>(null);

	const textFields = $derived(replaceableFields(fields));
	const items = $derived([{ value: ALL, label: 'All text fields' }, ...textFields.map((f) => ({ value: f.name, label: f.name }))]);

	$effect(() => {
		if (!open) return;
		field = initialField && textFields.some((f) => f.name === initialField) ? initialField : ALL;
		found = null;
	});

	// What was found no longer holds once the search changes.
	$effect(() => {
		void [field, options.find, options.replace, options.matchCase, options.wholeCell];
		found = null;
	});

	/** Every record matching the Datasheet's filter and search, up to what one batch can change. */
	async function rows(): Promise<{ rows: RecordRow[]; columns: string[]; capped: boolean }> {
		const out: RecordRow[] = [];
		let columns: string[] = [];
		for (let offset = 0; ; offset += PAGE) {
			const page = await recordService.list(table, { offset, limit: PAGE, filters, search: search || null, sort: sort ? [sort] : [] });
			columns = page.columns;
			out.push(...page.rows);
			if (out.length >= page.total || !page.rows.length) return { rows: out, columns, capped: false };
			if (out.length >= MAX_BATCH) return { rows: out, columns, capped: true };
		}
	}

	async function find() {
		if (!options.find || busy) return;
		busy = true;
		try {
			const r = await attempt(rows);
			if (!r) return;
			const cols = field === ALL ? textFields.map((f) => f.name) : [field];
			const plan = planReplace(r.rows, r.columns, cols, $state.snapshot(options));
			found = { ...plan, records: plan.ops.length, capped: r.capped };
		} finally {
			busy = false;
		}
	}

	async function replaceAll() {
		if (!found?.ops.length || busy) return;
		const ops = found.ops;
		busy = true;
		try {
			const result = await attempt(() => recordService.batch(table, ops, false, undefined, `Replace “${options.find}”`));
			if (!result) return;
			toast.success(`Replaced ${found.cells} ${found.cells === 1 ? 'value' : 'values'} in ${result.updated} ${result.updated === 1 ? 'record' : 'records'}`);
			open = false;
			onReplaced();
		} finally {
			busy = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content
		class="sm:max-w-lg"
		onOpenAutoFocus={(e) => {
			e.preventDefault();
			findInput?.focus();
		}}
	>
		<form
			class="flex flex-col gap-6"
			onsubmit={(e) => {
				e.preventDefault();
				if (found?.ops.length) replaceAll();
				else find();
			}}
		>
			<Dialog.Header>
				<Dialog.Title>Find and replace</Dialog.Title>
				<Dialog.Description>
					In {filters.length || search ? 'the records your filter and search show' : `every ${table} record`}.
				</Dialog.Description>
			</Dialog.Header>
			<Field.Group class="gap-4">
				<Field.Field>
					<Field.Label for="rp-field">Look in</Field.Label>
					<Select.Root type="single" {items} value={field} onValueChange={(v) => (field = v)}>
						<Select.Trigger id="rp-field" class="w-full"><Select.Value /></Select.Trigger>
						<Select.Content>
							<Select.Group>
								{#each items as i (i.value)}<Select.Item value={i.value} label={i.label}>{i.label}</Select.Item>{/each}
							</Select.Group>
						</Select.Content>
					</Select.Root>
				</Field.Field>
				<Field.Field>
					<Field.Label for="rp-find">Find</Field.Label>
					<Input id="rp-find" bind:ref={findInput} bind:value={options.find} autocomplete="off" />
				</Field.Field>
				<Field.Field>
					<Field.Label for="rp-replace">Replace with</Field.Label>
					<Input id="rp-replace" bind:value={options.replace} autocomplete="off" placeholder="Leave empty to remove the text" />
				</Field.Field>
				<div class="flex flex-wrap gap-x-6 gap-y-3">
					<Field.Field orientation="horizontal" class="w-auto">
						<Checkbox id="rp-case" checked={options.matchCase} onCheckedChange={(v) => (options.matchCase = v)} />
						<Field.Label for="rp-case">Match case</Field.Label>
					</Field.Field>
					<Field.Field orientation="horizontal" class="w-auto">
						<Checkbox id="rp-whole" checked={options.wholeCell} onCheckedChange={(v) => (options.wholeCell = v)} />
						<Field.Label for="rp-whole">Whole value only</Field.Label>
					</Field.Field>
				</div>
			</Field.Group>
			{#if found}
				<p class="text-sm" role="status">
					{#if found.cells}
						Found {found.cells} {found.cells === 1 ? 'value' : 'values'} in {found.records} {found.records === 1 ? 'record' : 'records'}.
					{:else}
						Nothing matches.
					{/if}
					{#if found.capped}<span class="text-warning"> Only the first {MAX_BATCH.toLocaleString()} records were searched; narrow it with a filter for more.</span>{/if}
				</p>
			{/if}
			<Dialog.Footer>
				<Button type="button" variant="outline" onclick={() => (open = false)}>Cancel</Button>
				{#if found?.ops.length}
					<Button type="submit" disabled={busy}>{#if busy}<Spinner data-icon="inline-start" />{/if} Replace all</Button>
				{:else}
					<Button type="submit" disabled={busy || !options.find || !textFields.length}>{#if busy}<Spinner data-icon="inline-start" />{/if} Find</Button>
				{/if}
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
