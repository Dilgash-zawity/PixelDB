<script lang="ts">
	// Services
	import { connectionService } from '#lib/services/connections.js';

	// Types
	import type { RemoteResult, RemoteTable } from '#lib/api/types.js';

	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { ApiError } from '#lib/api/client.js';
	import { connectionSummary, providerInfo } from '#lib/logic/connections.js';
	import { humanizeError } from '#lib/logic/messages.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import Logo from '#lib/components/app/Logo.svelte';
	import ProviderIcon from '#lib/components/app/ProviderIcon.svelte';
	import RemoteResultTable from '#lib/components/app/RemoteResultTable.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { Textarea } from '#lib/components/ui/textarea/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Eye as EyeIcon,
		Play as PlayIcon,
		RefreshCw as RefreshIcon,
		Search as SearchIcon,
		SearchCode as SearchCodeIcon,
		Table2 as TableIcon,
		TriangleAlert as TriangleAlertIcon,
		Unplug as UnplugIcon,
	} from '@lucide/svelte';

	// Script
	const name = $derived(page.url.searchParams.get('name') ?? '');
	const conn = $derived(workspace.connection(name));
	const engine = $derived(conn ? providerInfo(conn.provider).engine : 'postgres');

	if (!workspace.loaded) workspace.load();

	type Loaded<T> = { status: 'loading' } | { status: 'ok'; value: T } | { status: 'error'; message: string };
	const failure = (e: unknown) => humanizeError(e instanceof ApiError ? e.message : String(e));

	let tables = $state<Loaded<RemoteTable[]>>({ status: 'loading' });
	let selected = $state<RemoteTable | null>(null);
	let preview = $state<Loaded<RemoteResult> | null>(null);
	let filter = $state('');
	let tab = $state<'data' | 'sql'>('data');
	let sql = $state('');
	let ran = $state<Loaded<RemoteResult> | null>(null);

	const keyOf = (t: RemoteTable) => `${t.schema ?? ''}\u0000${t.name}`;
	const label = (t: RemoteTable) => (t.schema ? `${t.schema}.${t.name}` : t.name);
	const groups = $derived.by(() => {
		if (tables.status !== 'ok') return [];
		const q = filter.trim().toLowerCase();
		const out = new Map<string, RemoteTable[]>();
		for (const t of tables.value) {
			if (q && !label(t).toLowerCase().includes(q)) continue;
			const g = t.schema ?? '';
			out.set(g, [...(out.get(g) ?? []), t]);
		}
		return [...out];
	});

	async function loadTables() {
		tables = { status: 'loading' };
		try {
			const list = await connectionService.tables(name);
			tables = { status: 'ok', value: list };
			if (!selected || !list.some((t) => keyOf(t) === keyOf(selected!))) {
				if (list[0]) choose(list[0]);
				else selected = null;
			} else {
				choose(selected);
			}
		} catch (e) {
			tables = { status: 'error', message: failure(e) };
		}
	}

	let previewFor = '';
	async function choose(t: RemoteTable) {
		selected = t;
		tab = 'data';
		const key = (previewFor = `${keyOf(t)}\u0001${Date.now()}`);
		preview = { status: 'loading' };
		try {
			const result = await connectionService.preview(name, t);
			if (previewFor === key) preview = { status: 'ok', value: result };
		} catch (e) {
			if (previewFor === key) preview = { status: 'error', message: failure(e) };
		}
	}

	/** A starter query for the selected table, quoted the way its server expects. */
	function starter(t: RemoteTable | null): string {
		if (!t) return 'SELECT 1';
		const q = (s: string) => (engine === 'mysql' ? `\`${s.replaceAll('`', '``')}\`` : engine === 'mssql' ? `[${s.replaceAll(']', ']]')}]` : `"${s.replaceAll('"', '""')}"`);
		const from = t.schema ? `${q(t.schema)}.${q(t.name)}` : q(t.name);
		return engine === 'mssql' ? `SELECT TOP (100) *\nFROM ${from}` : `SELECT *\nFROM ${from}\nLIMIT 100`;
	}

	async function run() {
		if (!sql.trim() || ran?.status === 'loading') return;
		ran = { status: 'loading' };
		try {
			ran = { status: 'ok', value: await connectionService.query(name, sql) };
		} catch (e) {
			ran = { status: 'error', message: failure(e) };
		}
	}

	function showSql() {
		if (!sql.trim()) sql = starter(selected);
		tab = 'sql';
	}

	// Load once the connection is known; again only when the page names another one.
	let loadedFor = '';
	$effect(() => {
		if (!conn || loadedFor === conn.name) return;
		loadedFor = conn.name;
		untrack(() => {
			selected = null;
			preview = null;
			ran = null;
			sql = '';
			loadTables();
		});
	});

	function onKey(e: KeyboardEvent) {
		if (tab === 'sql' && (e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			run();
		}
	}
</script>

<svelte:window onkeydown={onKey} />
<svelte:head><title>{name} · SlateDB</title></svelte:head>

{#if workspace.loaded && !conn}
	<Empty.Root class="min-h-svh">
		<Empty.Header>
			<Empty.Media variant="icon"><UnplugIcon /></Empty.Media>
			<Empty.Title>“{name}” isn’t saved</Empty.Title>
			<Empty.Description>There is no online database with this name. It may have been renamed or removed.</Empty.Description>
		</Empty.Header>
		<Empty.Content>
			<Button onclick={() => goto(resolve('/'))}><ArrowLeftIcon data-icon="inline-start" /> Back to databases</Button>
		</Empty.Content>
	</Empty.Root>
{:else}
	<div class="flex h-svh flex-col">
		<header class="app-header flex h-12 shrink-0 items-center gap-2 border-b px-3">
			<Button variant="ghost" size="sm" onclick={() => goto(resolve('/'))} aria-label="Back to databases">
				<ArrowLeftIcon data-icon="inline-start" />
				<Logo class="size-5" />
			</Button>
			<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />
			{#if conn}
				<ProviderIcon provider={conn.provider} />
				<span class="truncate font-semibold">{name}</span>
				<span class="hidden truncate text-xs text-muted-foreground md:inline">{connectionSummary(conn)}</span>
			{/if}
			<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning"><EyeIcon /> Read-only</Badge>

			<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />
			<Tabs.Root bind:value={() => tab, (v) => (v === 'sql' ? showSql() : (tab = 'data'))}>
				<Tabs.List>
					<Tabs.Trigger value="data"><TableIcon class="vivid:text-tone-blue" /> Data</Tabs.Trigger>
					<Tabs.Trigger value="sql"><SearchCodeIcon class="vivid:text-tone-amber" /> SQL</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>

			<span class="flex-1"></span>
			<Button variant="ghost" size="sm" disabled={tables.status === 'loading'} onclick={loadTables}>
				<RefreshIcon data-icon="inline-start" /> Refresh
			</Button>
			<ThemeToggle />
		</header>

		<div class="flex min-h-0 flex-1">
			<aside class="flex w-64 shrink-0 flex-col border-e">
				<div class="relative p-2">
					<SearchIcon class="pointer-events-none absolute start-4 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
					<Input bind:value={filter} placeholder="Find a table" class="h-8 ps-8" aria-label="Find a table" />
				</div>
				<nav class="min-h-0 flex-1 overflow-y-auto px-2 pb-2" aria-label="Tables">
					{#if tables.status === 'loading'}
						<div class="flex flex-col gap-1.5">{#each Array(6) as _, i (i)}<Skeleton class="h-7" />{/each}</div>
					{:else if tables.status === 'ok'}
						{#each groups as [schema, list] (schema)}
							{#if schema}<p class="px-2 pt-3 pb-1 text-xs font-medium text-muted-foreground">{schema}</p>{/if}
							{#each list as t (keyOf(t))}
								{@const active = !!selected && keyOf(selected) === keyOf(t)}
								<button
									type="button"
									class={cn(
										'flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-sm outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring/40',
										active && 'bg-accent font-medium'
									)}
									onclick={() => choose(t)}
								>
									{#if t.kind === 'view'}<EyeIcon class="size-4 shrink-0 text-muted-foreground" />{:else}<TableIcon class="size-4 shrink-0 text-muted-foreground vivid:text-tone-blue" />{/if}
									<span class="truncate">{t.name}</span>
								</button>
							{/each}
						{:else}
							<p class="px-2 py-3 text-sm text-muted-foreground">{filter ? 'No table matches.' : 'This database has no tables yet.'}</p>
						{/each}
					{/if}
				</nav>
			</aside>

			<main class="min-h-0 min-w-0 flex-1">
				{#if tables.status === 'error'}
					<Empty.Root class="h-full">
						<Empty.Header>
							<Empty.Media variant="icon"><TriangleAlertIcon /></Empty.Media>
							<Empty.Title>Couldn’t connect to “{name}”</Empty.Title>
							<Empty.Description>{tables.message}</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<Button onclick={loadTables}><RefreshIcon data-icon="inline-start" /> Try again</Button>
						</Empty.Content>
					</Empty.Root>
				{:else if tab === 'data'}
					{#if !preview || preview.status === 'loading'}
						<div class="flex flex-col gap-2 p-6">{#each Array(5) as _, i (i)}<Skeleton class="h-9" />{/each}</div>
					{:else if preview.status === 'error'}
						<Empty.Root class="h-full">
							<Empty.Header>
								<Empty.Media variant="icon"><TriangleAlertIcon /></Empty.Media>
								<Empty.Title>Couldn’t read “{selected ? label(selected) : ''}”</Empty.Title>
								<Empty.Description>{preview.message}</Empty.Description>
							</Empty.Header>
						</Empty.Root>
					{:else}
						<RemoteResultTable result={preview.value} emptyText="This table is empty" />
					{/if}
				{:else}
					<div class="flex h-full min-h-0 flex-col">
						<div class="flex shrink-0 flex-col gap-2 border-b p-3">
							<Textarea bind:value={sql} rows={6} spellcheck={false} class="font-mono text-sm" aria-label="SQL" placeholder={starter(selected)} />
							<div class="flex items-center gap-2">
								<Button size="sm" disabled={!sql.trim() || ran?.status === 'loading'} onclick={run}>
									{#if ran?.status === 'loading'}<Spinner data-icon="inline-start" />{:else}<PlayIcon data-icon="inline-start" />{/if}
									Run
								</Button>
								<span class="text-xs text-muted-foreground"><Kbd>⌘</Kbd> <Kbd>Enter</Kbd> · One statement that reads data; at most 1000 rows.</span>
							</div>
						</div>
						<div class="min-h-0 flex-1">
							{#if ran?.status === 'error'}
								<Empty.Root class="h-full">
									<Empty.Header>
										<Empty.Media variant="icon"><TriangleAlertIcon /></Empty.Media>
										<Empty.Title>The query didn’t run</Empty.Title>
										<Empty.Description>{ran.message}</Empty.Description>
									</Empty.Header>
								</Empty.Root>
							{:else if ran?.status === 'ok'}
								<RemoteResultTable result={ran.value} />
							{:else if !ran}
								<Empty.Root class="h-full">
									<Empty.Header>
										<Empty.Media variant="icon"><SearchCodeIcon /></Empty.Media>
										<Empty.Title>Ask the database a question</Empty.Title>
										<Empty.Description>Write a SELECT and run it. Nothing you run here can change data.</Empty.Description>
									</Empty.Header>
								</Empty.Root>
							{/if}
						</div>
					</div>
				{/if}
			</main>
		</div>
	</div>
{/if}
