<script lang="ts">
	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { databaseHref } from '#lib/logic/routes.js';
	import { matches, shortcutText } from '#lib/logic/shortcuts.js';
	import { session, type Tab } from '#lib/state/session.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import CommandPalette from '#lib/components/app/CommandPalette.svelte';
	import Logo from '#lib/components/app/Logo.svelte';
	import NewTableDrawer from '#lib/components/app/NewTableDrawer.svelte';
	import NewDatabaseDrawer from '#lib/components/app/NewDatabaseDrawer.svelte';
	import ProviderIcon from '#lib/components/app/ProviderIcon.svelte';
	import QueryView from '#lib/components/app/QueryView.svelte';
	import RecordsView from '#lib/components/app/RecordsView.svelte';
	import RelationshipsView from '#lib/components/app/RelationshipsView.svelte';
	import SearchButton from '#lib/components/app/SearchButton.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import * as Select from '#lib/components/ui/select/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Check as CheckIcon,
		ChevronDown as ChevronDownIcon,
		Database as DatabaseIcon,
		Settings as SettingsIcon,
		Network as NetworkIcon,
		Plus as PlusIcon,
		SearchCode as SearchCodeIcon,
		Table2 as TableIcon,
		TriangleAlert as TriangleAlertIcon,
	} from '@lucide/svelte';

	// Script
	const name = $derived(page.url.searchParams.get('name') ?? '');
	let paletteOpen = $state(false);
	let creatingDatabase = $state(false);
	let creatingTable = $state(false);

	const tableItems = $derived(session.objects.map((o) => ({ value: o.name, label: o.name })));

	$effect(() => {
		if (name) session.open(name);
	});
	if (!workspace.loaded) workspace.load();

	async function back() {
		if (await session.close()) goto(resolve('/'));
	}

	async function openSettings() {
		if (await session.close()) goto(resolve('/settings'));
	}

	async function openOnline(db: string) {
		if (await session.close()) goto(databaseHref(resolve('/online'), db));
	}

	async function switchTo(db: string) {
		if (!session.leaveGuard || (await session.leaveGuard())) goto(databaseHref(resolve('/db'), db));
	}



	function suggestedTableName(): string {
		let n = 1;
		while (session.objects.some((o) => o.name.toLowerCase() === `table ${n}`)) n++;
		return `Table ${n}`;
	}

	const TABS: Tab[] = ['diagram', 'datasheet', 'query'];

	function onKey(e: KeyboardEvent) {
		if (matches(e, 'mod+k')) {
			e.preventDefault();
			paletteOpen = !paletteOpen;
			return;
		}
		const n = TABS.findIndex((_, i) => matches(e, `mod+${i + 1}`));
		if (n >= 0 && session.info) {
			e.preventDefault();
			session.showTab(TABS[n]);
		}
	}
</script>

<svelte:window onkeydown={onKey} />
<svelte:head><title>{name} · SlateDB</title></svelte:head>

{#if session.loadError}
	<Empty.Root class="min-h-svh">
		<Empty.Header>
			<Empty.Media variant="icon"><TriangleAlertIcon /></Empty.Media>
			<Empty.Title>Couldn't open “{name}”</Empty.Title>
			<Empty.Description>{session.loadError}</Empty.Description>
		</Empty.Header>
		<Empty.Content>
			<Button onclick={back}><ArrowLeftIcon data-icon="inline-start" /> Back to databases</Button>
		</Empty.Content>
	</Empty.Root>
{:else}
	<div class="flex h-svh flex-col">
		<header data-bar class="app-header flex h-12 shrink-0 items-center gap-2 border-b px-3">
			<DropdownMenu.Root onOpenChange={(open) => open && workspace.refresh()}>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="sm" class="max-w-64">
							<Logo class="size-5" />
							<span class="truncate font-semibold">{name}</span>
							<ChevronDownIcon data-icon="inline-end" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content class="w-64" align="start">
					<DropdownMenu.Group>
						<DropdownMenu.Label>Databases</DropdownMenu.Label>
						{#each workspace.databases.filter((d) => d.valid) as db (db.file_name)}
							<DropdownMenu.Item onclick={() => db.name !== name && switchTo(db.name)}>
								<DatabaseIcon />
								<span class="truncate">{db.name}</span>
								{#if db.name === name}<CheckIcon class="ms-auto" />{/if}
							</DropdownMenu.Item>
						{/each}
						{#each workspace.online.filter((o) => o.settings) as db (db.name)}
							<DropdownMenu.Item onclick={() => openOnline(db.name)}>
								<ProviderIcon provider={db.settings?.provider} />
								<span class="truncate">{db.name}</span>
							</DropdownMenu.Item>
						{/each}
					</DropdownMenu.Group>
					<DropdownMenu.Separator />
					<DropdownMenu.Group>
						<DropdownMenu.Item onclick={() => (creatingDatabase = true)}><PlusIcon /> New database…</DropdownMenu.Item>
						<DropdownMenu.Item onclick={back}><ArrowLeftIcon /> All databases</DropdownMenu.Item>
						<DropdownMenu.Item onclick={openSettings}><SettingsIcon /> Settings</DropdownMenu.Item>
					</DropdownMenu.Group>
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />

			<Tabs.Root bind:value={() => session.tab, (v) => session.showTab(v as Tab)}>
				<Tabs.List variant="line">
					<Tabs.Trigger value="diagram" title="Diagram ({shortcutText('mod+1')})"><NetworkIcon /> Diagram</Tabs.Trigger>
					<Tabs.Trigger value="datasheet" title="Datasheet ({shortcutText('mod+2')})"><TableIcon /> Datasheet</Tabs.Trigger>
					<Tabs.Trigger value="query" title="Query ({shortcutText('mod+3')})"><SearchCodeIcon /> Query</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>

			{#if session.tab === 'datasheet' && tableItems.length}
				<Select.Root type="single" items={tableItems} bind:value={() => session.table ?? '', (v) => v && session.select(v)}>
					<Select.Trigger size="sm" class="w-48" aria-label="Table"><Select.Value placeholder="Choose a table" /></Select.Trigger>
					<Select.Content>
						<Select.Group>
							<Select.Label>Tables</Select.Label>
							{#each session.tables as t (t.name)}<Select.Item value={t.name} label={t.name}>{t.name}</Select.Item>{/each}
						</Select.Group>
						{#if session.views.length}
							<Select.Group>
								<Select.Label>Views</Select.Label>
								{#each session.views as v (v.name)}<Select.Item value={v.name} label={v.name}>{v.name}</Select.Item>{/each}
							</Select.Group>
						{/if}
					</Select.Content>
				</Select.Root>
				{#if session.isView(session.table)}<Badge variant="outline" class="border-warning/40 bg-warning/10 text-warning">Read-only view</Badge>{/if}
			{/if}

			<span class="flex-1"></span>
			<SearchButton onclick={() => (paletteOpen = true)} />
			<ThemeToggle />
		</header>

		<main class="min-h-0 flex-1">
			<!-- Views load data as soon as they mount, so wait until this database is the one open. -->
			{#if session.loading || session.name !== name || !session.info}
				<div class="flex flex-col gap-2 p-6">
					{#each Array(5) as _, i (i)}<Skeleton class="h-9" />{/each}
				</div>
			{:else if session.tab === 'diagram'}
				<RelationshipsView />
			{:else if session.tab === 'query'}
				<QueryView />
			{:else if !session.table}
				<Empty.Root class="h-full">
					<Empty.Header>
						<Empty.Media variant="icon"><TableIcon /></Empty.Media>
						<Empty.Title>No tables yet</Empty.Title>
						<Empty.Description>A table holds one kind of thing, like customers or orders.</Empty.Description>
					</Empty.Header>
					<Empty.Content>
						<Button onclick={() => (creatingTable = true)}><PlusIcon data-icon="inline-start" /> New table</Button>
					</Empty.Content>
				</Empty.Root>
			{:else}
				{#key `${session.table}:${session.revision}`}
					<RecordsView table={session.table} isView={session.isView(session.table)} />
				{/key}
			{/if}
		</main>
	</div>
{/if}

<CommandPalette bind:open={paletteOpen} onNewTable={() => (creatingTable = true)} onBack={back} onSettings={openSettings} />

<NewDatabaseDrawer
	bind:open={creatingDatabase}
	onCreated={(db, kind) => {
		if (kind === 'local') switchTo(db);
		else openOnline(db);
	}}
/>

<NewTableDrawer bind:open={creatingTable} initial={suggestedTableName()} onCreate={(t, fields) => session.createTable(t, true, fields)} />
