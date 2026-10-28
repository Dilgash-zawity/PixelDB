<script lang="ts">
	// Services
	import { onlineService } from '#lib/services/online.js';

	// Types
	import type { OnlineTable } from '#lib/api/types.js';

	// Utils
	import { untrack } from 'svelte';
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import FieldTypeIcon from '../FieldTypeIcon.svelte';

	// Icons
	import { ArrowRight as ArrowRightIcon, Info as InfoIcon, KeyRound as KeyRoundIcon, Link as LinkIcon } from '@lucide/svelte';

	// Script
	/** The columns, keys and links of one online table or view, as the server describes them. Remount to show another. */
	let { database, table, onOpen }: { database: string; table: string; onOpen: (table: string) => void } = $props();

	let t = $state<OnlineTable | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		untrack(async () => {
			try {
				t = await onlineService.describe(database, table);
			} catch (e) {
				error = cleanMessage(e instanceof Error ? e.message : String(e));
			}
		});
	});

	const linked = (column: string) => !!t?.foreign_keys.some((k) => k.columns.includes(column));
	const ACTIONS: Record<string, string> = {
		no_action: 'nothing happens',
		restrict: 'it is refused',
		set_null: 'the link is cleared',
		set_default: 'the link is reset',
		cascade: 'these rows follow'
	};
</script>

<div class="h-full overflow-auto">
	<div class="mx-auto flex max-w-5xl flex-col gap-6 p-6">
		{#if error}
			<p class="text-sm text-destructive">{error}</p>
		{:else if !t}
			{#each Array(5) as _, i (i)}<Skeleton class="h-9" />{/each}
		{:else}
			<div class="flex flex-wrap items-center gap-2">
				<h2 class="text-lg font-semibold">{t.name}</h2>
				<Badge variant="secondary">{t.kind === 'view' ? 'View' : 'Table'}</Badge>
				{#if t.schema}<Badge variant="outline">Schema {t.schema}</Badge>{/if}
				{#if t.row_estimate !== null}<Badge variant="outline" class="tabular-nums">≈ {t.row_estimate.toLocaleString()} rows</Badge>{/if}
				{#if t.editable}
					<Badge variant="outline" class="border-success/40 bg-success/10 text-success">Editable</Badge>
				{:else}
					<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning">Read-only</Badge>
				{/if}
			</div>

			<div class="overflow-hidden rounded-lg border">
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>Field</Table.Head>
							<Table.Head>Server type</Table.Head>
							<Table.Head>Shown as</Table.Head>
							<Table.Head>Can be empty</Table.Head>
							<Table.Head>Default</Table.Head>
							<Table.Head>Notes</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each t.columns as c (c.name)}
							<Table.Row>
								<Table.Cell class="font-medium">
									<span class="inline-flex items-center gap-1.5">
										{#if c.primary_key}<KeyRoundIcon class="size-3.5 text-warning" aria-label="Primary key" />{/if}
										{#if linked(c.name)}<LinkIcon class="size-3.5 text-muted-foreground vivid:text-tone-violet" aria-label="Links to another table" />{/if}
										{c.name}
									</span>
									{#if c.description}<p class="text-xs font-normal text-muted-foreground">{c.description}</p>{/if}
								</Table.Cell>
								<Table.Cell class="font-mono text-xs">{c.server_type}</Table.Cell>
								<Table.Cell>
									<span class="inline-flex items-center gap-1.5 text-muted-foreground">
										<FieldTypeIcon type={c.field_type.type} class="size-3.5" />
										{fieldTypeLabel(c.field_type)}
									</span>
								</Table.Cell>
								<Table.Cell>{c.nullable ? 'Yes' : 'No'}</Table.Cell>
								<Table.Cell class="max-w-48 truncate font-mono text-xs" title={c.default ?? ''}>{c.default ?? ''}</Table.Cell>
								<Table.Cell>
									<span class="flex flex-wrap gap-1">
										{#if c.auto}<Badge variant="secondary">Automatic</Badge>{/if}
										{#if c.generated}<Badge variant="secondary">Calculated</Badge>{/if}
										{#if c.unique}<Badge variant="secondary">Unique</Badge>{/if}
									</span>
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</div>

			{#if t.foreign_keys.length}
				<section class="flex flex-col gap-2">
					<h3 class="text-sm font-semibold">Links</h3>
					<ul class="flex flex-col divide-y rounded-lg border">
						{#each t.foreign_keys as fk, i (i)}
							<li class="flex flex-wrap items-center gap-2 px-3 py-2 text-sm">
								<span class="font-medium">{fk.columns.join(', ')}</span>
								<ArrowRightIcon class="size-4 text-muted-foreground" />
								<Button variant="link" size="sm" class="h-auto p-0" onclick={() => onOpen(fk.ref_table)}>{fk.ref_table}</Button>
								{#if fk.ref_columns?.length}<span class="text-muted-foreground">({fk.ref_columns.join(', ')})</span>{/if}
								<span class="ms-auto text-xs text-muted-foreground">When the linked row is deleted, {ACTIONS[fk.on_delete ?? 'no_action'] ?? 'nothing happens'}.</span>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			<p class="flex items-start gap-2 text-xs text-muted-foreground">
				<InfoIcon class="mt-0.5 size-3.5 shrink-0" />
				The structure of an online database is changed with the server's own tools. PixelDB picks up changes within about 20 seconds, or when you reload.
			</p>
		{/if}
	</div>
</div>
