<script lang="ts">
	// Services
	import { onlineService } from '#lib/services/online.js';
	import { queryService } from '#lib/services/queries.js';

	// Types
	import type { Engine, FieldType, JsonValue } from '#lib/api/types.js';

	// Utils
	import { canTimePlans } from '#lib/logic/online.js';
	import { displayValue } from '#lib/logic/values.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as Table from '#lib/components/ui/table/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';
	import Shortcut from '../Shortcut.svelte';

	// Icons
	import { Play as PlayIcon, Route as RouteIcon, SearchCode as SearchCodeIcon, Timer as TimerIcon } from '@lucide/svelte';

	// Script
	/** Read-only SQL against an online database, with the server's query plans. */
	let { database, engine, initial }: { database: string; engine: Engine; initial: string } = $props();

	interface Shown {
		kind: 'rows' | 'plan';
		columns: string[];
		types: (FieldType | null)[];
		rows: JsonValue[][];
		truncated: boolean;
		elapsedMs: number;
	}

	// svelte-ignore state_referenced_locally
	let sql = $state(initial);
	let result = $state<Shown | null>(null);
	let error = $state<string | null>(null);
	let running = $state(false);

	async function go(kind: 'run' | 'explain' | 'analyze') {
		if (!sql.trim() || running) return;
		running = true;
		error = null;
		try {
			if (kind === 'run') {
				const r = await queryService.run(sql, database);
				result = { kind: 'rows', columns: r.columns, types: r.types, rows: r.rows, truncated: r.truncated, elapsedMs: r.elapsed_ms };
			} else {
				const r = await onlineService.explain(database, sql, kind === 'analyze');
				result = { kind: 'plan', columns: r.columns, types: r.columns.map(() => null), rows: r.rows, truncated: false, elapsedMs: r.elapsed_ms };
			}
		} catch (e) {
			error = cleanMessage(e instanceof Error ? e.message : String(e));
		} finally {
			running = false;
		}
	}

	function onKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			go('run');
		}
	}

	/** Postgres plans are one line of text per row; show them as text. */
	const textPlan = $derived(result?.kind === 'plan' && result.columns.length === 1);
	const numeric = (t: FieldType | null) => !!t && ['number', 'large_number', 'currency', 'auto_number'].includes(t.type);
</script>

<div class="flex h-full min-h-0 flex-col">
	<div class="flex shrink-0 flex-wrap items-center gap-2 border-b px-3 py-2">
		<Button size="sm" disabled={running || !sql.trim()} onclick={() => go('run')}>
			{#if running}<Spinner data-icon="inline-start" />{:else}<PlayIcon data-icon="inline-start" />{/if} Run <Shortcut keys="mod+enter" />
		</Button>
		<Button size="sm" variant="outline" disabled={running || !sql.trim()} onclick={() => go('explain')}>
			<RouteIcon data-icon="inline-start" /> Plan
		</Button>
		{#if canTimePlans(engine)}
			<Button size="sm" variant="outline" disabled={running || !sql.trim()} onclick={() => go('analyze')}>
				<TimerIcon data-icon="inline-start" /> Timed plan
			</Button>
		{/if}
		<span class="flex-1"></span>
		{#if result}
			<span class="text-sm text-muted-foreground tabular-nums">
				{#if result.kind === 'rows'}{result.rows.length}{result.truncated ? '+' : ''} {result.rows.length === 1 ? 'row' : 'rows'} ·{/if}
				{result.elapsedMs} ms
			</span>
			{#if result.truncated}<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning">First {result.rows.length} rows</Badge>{/if}
		{/if}
	</div>

	<div class="flex max-h-[45%] min-h-40 shrink-0 flex-col border-b">
		<Textarea
			bind:value={sql}
			onkeydown={onKey}
			class="min-h-40 flex-1 resize-none rounded-none border-0 font-mono text-sm shadow-none focus-visible:ring-0"
			placeholder="SELECT …"
			spellcheck={false}
			aria-label="SQL query"
		/>
	</div>

	<div class="min-h-0 flex-1 overflow-auto">
		{#if error}
			<p class="p-4 font-mono text-sm whitespace-pre-wrap text-destructive">{error}</p>
		{:else if !result}
			<Empty.Root class="h-full">
				<Empty.Header>
					<Empty.Media variant="icon"><SearchCodeIcon /></Empty.Media>
					<Empty.Title>Ask the server</Empty.Title>
					<Empty.Description>
						Queries only read: they run in a read-only transaction, so nothing on the server changes. Use Plan to see how the server would run one{canTimePlans(engine)
							? ', or Timed plan to run it and see where the time goes'
							: ''}.
					</Empty.Description>
				</Empty.Header>
			</Empty.Root>
		{:else if textPlan}
			<pre class="p-4 font-mono text-xs leading-relaxed">{result.rows.map((r) => String(r[0] ?? '')).join('\n')}</pre>
		{:else}
			<Table.Root>
				<Table.Header class="sticky top-0 bg-background">
					<Table.Row>
						{#each result.columns as col, i (i)}
							<Table.Head class={cn(numeric(result.types[i]) && 'text-right')}>{col}</Table.Head>
						{/each}
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each result.rows as row, r (r)}
						<Table.Row>
							{#each row as value, i (i)}
								<Table.Cell class={cn('max-w-80 truncate', numeric(result.types[i]) && 'text-right tabular-nums')}>
									{displayValue(value, result.types[i] ?? undefined)}
								</Table.Cell>
							{/each}
						</Table.Row>
					{:else}
						<Table.Row>
							<Table.Cell colspan={Math.max(1, result.columns.length)} class="py-8 text-center text-muted-foreground">No rows.</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		{/if}
	</div>
</div>
