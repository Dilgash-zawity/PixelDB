<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import PencilRulerIcon from '@lucide/svelte/icons/pencil-ruler';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import TableIcon from '@lucide/svelte/icons/table-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import AppSidebar from '#lib/components/app/AppSidebar.svelte';
	import CommandPalette from '#lib/components/app/CommandPalette.svelte';
	import DesignView from '#lib/components/app/DesignView.svelte';
	import RecordsView from '#lib/components/app/RecordsView.svelte';
	import RelationshipsView from '#lib/components/app/RelationshipsView.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import * as Breadcrumb from '#lib/components/ui/breadcrumb/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import * as Sidebar from '#lib/components/ui/sidebar/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import { databaseHref } from '#lib/logic/routes.js';
	import { session, type TableView } from '#lib/state/session.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	const name = $derived(page.url.searchParams.get('name') ?? '');
	let paletteOpen = $state(false);
	const crumb = $derived(session.screen === 'relationships' ? 'Relationships' : session.table);
	let sidebar = $state<AppSidebar | null>(null);

	$effect(() => {
		if (name) session.open(name);
	});
	if (!workspace.loaded) workspace.load();

	async function back() {
		if (await session.close()) goto(resolve('/'));
	}

	async function switchTo(db: string) {
		if (!session.leaveGuard || (await session.leaveGuard())) goto(databaseHref(resolve('/db'), db));
	}

	function onKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}
</script>

<svelte:window onkeydown={onKey} />
<svelte:head><title>{name} · Base</title></svelte:head>

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
	<Sidebar.Provider>
		<AppSidebar bind:this={sidebar} onBack={back} onSwitch={switchTo} onSearch={() => (paletteOpen = true)} />
		<Sidebar.Inset class="h-svh overflow-hidden">
			<header class="flex h-14 shrink-0 items-center gap-2 border-b px-4">
				<Sidebar.Trigger class="-ms-1" />
				<Separator orientation="vertical" class="me-2 data-vertical:h-4 data-vertical:self-center" />
				<Breadcrumb.Root>
					<Breadcrumb.List>
						<Breadcrumb.Item class="hidden md:block">
							<Breadcrumb.Link href={resolve('/')} onclick={(e) => { e.preventDefault(); back(); }}>Databases</Breadcrumb.Link>
						</Breadcrumb.Item>
						<Breadcrumb.Separator class="hidden md:block" />
						<Breadcrumb.Item>
							{#if crumb}
								<span class="hidden md:inline">{name}</span>
							{:else}
								<Breadcrumb.Page>{name}</Breadcrumb.Page>
							{/if}
						</Breadcrumb.Item>
						{#if crumb}
							<Breadcrumb.Separator class="hidden md:block" />
							<Breadcrumb.Item><Breadcrumb.Page>{crumb}</Breadcrumb.Page></Breadcrumb.Item>
						{/if}
					</Breadcrumb.List>
				</Breadcrumb.Root>
				<span class="flex-1"></span>
				{#if session.screen === 'relationships'}
					<!-- The Relationships screen has its own toolbar. -->
				{:else if session.table && !session.isView(session.table)}
					<Tabs.Root bind:value={() => session.view, (v) => session.setView(v as TableView)}>
						<Tabs.List>
							<Tabs.Trigger value="records"><TableIcon /> Datasheet</Tabs.Trigger>
							<Tabs.Trigger value="fields"><PencilRulerIcon /> Design</Tabs.Trigger>
						</Tabs.List>
					</Tabs.Root>
				{:else if session.table}
					<Badge variant="secondary">Read-only view</Badge>
				{/if}
			</header>

			<div class="min-h-0 flex-1">
				{#if session.loading}
					<div class="flex flex-col gap-2 p-6">
						{#each Array(5) as _, i (i)}<Skeleton class="h-9" />{/each}
					</div>
				{:else if session.screen === 'relationships'}
					<RelationshipsView />
				{:else if !session.table}
					<Empty.Root class="h-full">
						<Empty.Header>
							<Empty.Media variant="icon"><TableIcon /></Empty.Media>
							<Empty.Title>Create your first table</Empty.Title>
							<Empty.Description>
								A table holds one kind of thing — like customers, products or orders. You'll add its fields next.
							</Empty.Description>
						</Empty.Header>
						<Empty.Content>
							<Button onclick={() => sidebar?.newTable()}><PlusIcon data-icon="inline-start" /> New table</Button>
						</Empty.Content>
					</Empty.Root>
				{:else}
					{#key `${session.table}:${session.view}:${session.revision}`}
						{#if session.view === 'fields' && !session.isView(session.table)}
							<DesignView table={session.table} />
						{:else}
							<RecordsView table={session.table} isView={session.isView(session.table)} />
						{/if}
					{/key}
				{/if}
			</div>
		</Sidebar.Inset>
	</Sidebar.Provider>
{/if}

<CommandPalette bind:open={paletteOpen} onNewTable={() => sidebar?.newTable()} onBack={back} />
