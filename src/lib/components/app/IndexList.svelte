<script lang="ts">
	// Services
	import { tableService } from '#lib/services/tables.js';

	// Types
	import type { IndexInfo, TableDef } from '#lib/api/types.js';

	// Utils
	import { toast } from 'svelte-sonner';
	import { attempt } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';

	// Icons
	import { Gauge as GaugeIcon, KeyRound as KeyRoundIcon, Plus as PlusIcon, ShieldCheck as ShieldCheckIcon, Trash2 as TrashIcon, X as XIcon } from '@lucide/svelte';

	// Script
	/**
	 * A table's indexes: the ones behind its key and "no duplicates" rules
	 * (shown, changed on the field) and ones added here to speed up searching
	 * and sorting or to keep a combination of fields unique.
	 */
	let {
		table,
		indexes,
		disabled = false,
		onChanged
	}: {
		table: TableDef;
		indexes: IndexInfo[];
		disabled?: boolean;
		onChanged: () => void;
	} = $props();

	const MAX_FIELDS = 8;

	let adding = $state(false);
	let picked = $state<string[]>([]);
	let unique = $state(false);
	let busy = $state(false);

	const indexable = $derived(table.fields.filter((f) => f.field_type.type !== 'attachment'));
	const sorted = $derived([...indexes].sort((a, b) => order(a) - order(b)));

	function order(i: IndexInfo) {
		return i.origin === 'pk' ? 0 : i.origin === 'u' ? 1 : 2;
	}

	function describe(i: IndexInfo): string {
		if (i.origin === 'pk') return 'The table’s key';
		if (i.origin === 'u') return 'No duplicates (a rule on the field)';
		return i.unique ? 'No duplicates of this combination' : 'Speeds up searching and sorting';
	}

	function toggle(name: string) {
		picked = picked.includes(name) ? picked.filter((p) => p !== name) : picked.length < MAX_FIELDS ? [...picked, name] : picked;
	}

	async function add() {
		if (!picked.length || busy) return;
		busy = true;
		try {
			const made = await attempt(() => tableService.createIndex(table.name, picked, unique));
			if (!made) return;
			toast.success(unique ? `No two records can share the same ${picked.join(' + ')} now` : `Added an index on ${picked.join(', ')}`);
			adding = false;
			picked = [];
			unique = false;
			onChanged();
		} finally {
			busy = false;
		}
	}

	async function remove(i: IndexInfo) {
		busy = true;
		try {
			if ((await attempt(() => tableService.dropIndex(table.name, i.name))) === undefined) return;
			toast.success(`Removed the index on ${i.columns.join(', ')}`);
			onChanged();
		} finally {
			busy = false;
		}
	}
</script>

<div class="flex flex-col gap-3">
	{#if sorted.length}
		<Item.Group class="gap-1.5">
			{#each sorted as i (i.name)}
				<Item.Root variant="outline" size="sm">
					<Item.Media>
						{#if i.origin === 'pk'}<KeyRoundIcon class="text-warning" />{:else if i.unique}<ShieldCheckIcon />{:else}<GaugeIcon />{/if}
					</Item.Media>
					<Item.Content>
						<Item.Title>{i.columns.join(' + ')}</Item.Title>
						<Item.Description>{describe(i)}{i.partial ? ' · only some records' : ''}</Item.Description>
					</Item.Content>
					{#if i.origin === 'c'}
						<Item.Actions>
							<Button variant="ghost" size="icon-sm" aria-label="Remove index on {i.columns.join(', ')}" disabled={disabled || busy} onclick={() => remove(i)}>
								<TrashIcon />
							</Button>
						</Item.Actions>
					{/if}
				</Item.Root>
			{/each}
		</Item.Group>
	{:else}
		<p class="text-sm text-muted-foreground">No indexes yet. Add one on fields you often search, sort or filter by.</p>
	{/if}

	{#if adding}
		<div class="flex flex-col gap-3 rounded-lg border p-3">
			<Field.Set>
				<Field.Legend variant="label">Fields, in order</Field.Legend>
				<Field.Description>Pick the fields you search or sort by together, most-used first.</Field.Description>
				<div class="flex flex-wrap gap-1.5" role="group" aria-label="Fields to index">
					{#each indexable as f (f.name)}
						{@const n = picked.indexOf(f.name)}
						<button
							type="button"
							aria-pressed={n >= 0}
							class={cn(
								'flex items-center gap-1.5 rounded-md border bg-card px-2 py-1 text-sm outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
								n >= 0 && 'border-primary bg-primary/5 ring-1 ring-primary'
							)}
							onclick={() => toggle(f.name)}
						>
							{#if n >= 0}<Badge class="size-4 rounded-full p-0 text-[10px]">{n + 1}</Badge>{/if}
							{f.name}
						</button>
					{/each}
				</div>
			</Field.Set>
			<Field.Field orientation="horizontal">
				<Field.Content>
					<Field.Label for="ix-unique">No duplicates</Field.Label>
					<Field.Description>Refuse two records with the same {picked.length > 1 ? 'combination' : 'value'}.</Field.Description>
				</Field.Content>
				<Switch id="ix-unique" checked={unique} onCheckedChange={(v) => (unique = v)} />
			</Field.Field>
			<div class="flex gap-2">
				<Button size="sm" disabled={!picked.length || busy || disabled} onclick={add}><PlusIcon data-icon="inline-start" /> Add index</Button>
				<Button size="sm" variant="ghost" onclick={() => ((adding = false), (picked = []))}><XIcon data-icon="inline-start" /> Cancel</Button>
			</div>
		</div>
	{:else}
		<Button variant="outline" class="self-start" {disabled} onclick={() => (adding = true)}><PlusIcon data-icon="inline-start" /> Add index</Button>
	{/if}
</div>
