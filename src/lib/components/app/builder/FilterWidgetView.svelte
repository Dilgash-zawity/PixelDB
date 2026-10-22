<script lang="ts">
	// Services
	import { recordService } from '#lib/services/records.js';

	// Types
	import type { JsonValue } from '#lib/api/types.js';

	// Utils
	import { isBlank, type FilterWidget } from '#lib/logic/app-def.js';
	import { displayValue } from '#lib/logic/values.js';
	import { appSession } from '#lib/state/app.svelte.js';

	// Components
	import * as InputGroup from '#lib/components/ui/input-group/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';

	// Icons
	import { X as XIcon } from '@lucide/svelte';

	// Script
	let { widget }: { widget: FilterWidget } = $props();

	const id = $props.id();
	const value = $derived(appSession.vars[widget.variable] ?? null);
	const text = $derived(isBlank(value) ? '' : String(value));

	/** Choices for a dropdown, as JSON-encoded values (Select values are strings). */
	let options = $state<{ value: string; label: string }[]>([]);
	const ANY = '__any__';

	$effect(() => {
		const o = widget.options;
		const database = o.data ? appSession.databaseOf(o.data.source) : undefined;
		void appSession.dataRevision;
		if (widget.control !== 'select' || !o.data || !database || !o.valueColumn) {
			options = [];
			return;
		}
		const table = o.data.table;
		const sort = [{ column: o.labelColumn || o.valueColumn }];
		recordService
			.list(table, { limit: 1000, sort }, database)
			.then((page) => {
				const vi = page.columns.indexOf(o.valueColumn);
				const li = page.columns.indexOf(o.labelColumn || o.valueColumn);
				if (vi < 0) return;
				const seen = new Set<string>();
				const out: { value: string; label: string }[] = [];
				for (const r of page.rows) {
					const v = r.values[vi];
					const key = JSON.stringify(v);
					if (v === null || seen.has(key)) continue;
					seen.add(key);
					out.push({ value: key, label: displayValue(r.values[li >= 0 ? li : vi], page.types[li >= 0 ? li : vi] ?? undefined) });
				}
				options = out;
			})
			.catch(() => (options = []));
	});

	// Typing waits a moment before filtering, so each keystroke doesn't reload everything.
	let timer: ReturnType<typeof setTimeout> | null = null;
	function setLater(v: JsonValue) {
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => appSession.setVariable(widget.variable, v), 300);
	}

	function set(v: JsonValue) {
		if (timer) clearTimeout(timer);
		appSession.setVariable(widget.variable, v);
	}

	const items = $derived([{ value: ANY, label: widget.placeholder || 'All' }, ...options]);
</script>

<div class="flex h-full flex-col justify-center gap-1.5">
	{#if widget.control === 'toggle'}
		<label class="flex items-center gap-2 text-sm" for={id}>
			<Switch {id} checked={value === true} onCheckedChange={(v) => set(v ? true : null)} />
			{widget.placeholder || 'Only matching records'}
		</label>
	{:else if widget.control === 'select'}
		<Select.Root
			type="single"
			{items}
			value={isBlank(value) ? ANY : JSON.stringify(value)}
			onValueChange={(v) => set(!v || v === ANY ? null : JSON.parse(v))}
		>
			<Select.Trigger {id} class="w-full" aria-label={widget.title || 'Filter'}>
				<Select.Value placeholder={widget.placeholder || 'All'} />
			</Select.Trigger>
			<Select.Content>
				<Select.Group>
					{#each items as item (item.value)}
						<Select.Item value={item.value} label={item.label}>{item.label}</Select.Item>
					{/each}
				</Select.Group>
			</Select.Content>
		</Select.Root>
	{:else}
		<InputGroup.Root>
			<InputGroup.Input
				{id}
				type={widget.control === 'date' ? 'date' : 'text'}
				inputmode={widget.control === 'number' ? 'decimal' : undefined}
				placeholder={widget.placeholder || (widget.control === 'number' ? 'Any number' : 'Type to filter')}
				value={text}
				aria-label={widget.title || 'Filter'}
				oninput={(e) => {
					const raw = e.currentTarget.value;
					if (widget.control === 'number') setLater(raw.trim() === '' || isNaN(Number(raw)) ? null : Number(raw));
					else if (widget.control === 'date') set(raw || null);
					else setLater(raw);
				}}
			/>
			{#if !isBlank(value)}
				<InputGroup.Addon align="inline-end">
					<InputGroup.Button size="icon-xs" aria-label="Clear" onclick={() => set(null)}><XIcon /></InputGroup.Button>
				</InputGroup.Addon>
			{/if}
		</InputGroup.Root>
	{/if}
</div>
