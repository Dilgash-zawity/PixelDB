<script lang="ts">
	// Types
	import type { JsonValue, RemoteResult } from '#lib/api/types.js';

	// Components
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Table from '#lib/components/ui/table/index.js';

	// Icons
	import { Rows3 as RowsIcon } from '@lucide/svelte';

	// Script
	/** Read-only rows from an online database. */
	let { result, emptyText = 'No rows' }: { result: RemoteResult; emptyText?: string } = $props();

	function show(v: JsonValue): string {
		if (typeof v === 'boolean') return v ? 'Yes' : 'No';
		if (typeof v === 'object' && v !== null) return JSON.stringify(v);
		return String(v);
	}
</script>

{#if !result.rows.length}
	<Empty.Root class="h-full">
		<Empty.Header>
			<Empty.Media variant="icon"><RowsIcon /></Empty.Media>
			<Empty.Title>{emptyText}</Empty.Title>
			<Empty.Description>Finished in {result.elapsed_ms} ms.</Empty.Description>
		</Empty.Header>
	</Empty.Root>
{:else}
	<div class="flex h-full min-h-0 flex-col">
		<div class="min-h-0 flex-1 overflow-auto">
			<Table.Root>
				<Table.Header class="sticky top-0 bg-background">
					<Table.Row>
						<Table.Head class="w-12 text-end text-muted-foreground">#</Table.Head>
						{#each result.columns as column, i (i)}<Table.Head>{column}</Table.Head>{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each result.rows as row, r (r)}
						<Table.Row>
							<Table.Cell class="text-end text-xs text-muted-foreground tabular-nums">{r + 1}</Table.Cell>
							{#each row as value, i (i)}
								{#if value === null}
									<Table.Cell class="text-muted-foreground/60 italic">null</Table.Cell>
								{:else}
									<Table.Cell class={typeof value === 'number' ? 'text-end tabular-nums' : 'max-w-96 truncate'} title={show(value)}>{show(value)}</Table.Cell>
								{/if}
							{/each}
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
		<p class="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">
			{result.rows.length}
			{result.rows.length === 1 ? 'row' : 'rows'}{result.truncated ? ' (showing the first ones only)' : ''} · {result.elapsed_ms} ms
		</p>
	</div>
{/if}
