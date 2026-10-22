<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import CloudAlertIcon from '@lucide/svelte/icons/cloud-alert';
	import InfoIcon from '@lucide/svelte/icons/info';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlayIcon from '@lucide/svelte/icons/play';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import Redo2Icon from '@lucide/svelte/icons/redo-2';
	import SearchIcon from '@lucide/svelte/icons/search';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import Undo2Icon from '@lucide/svelte/icons/undo-2';
	import AppCanvas from '#lib/components/app/builder/AppCanvas.svelte';
	import AppCommandPalette from '#lib/components/app/builder/AppCommandPalette.svelte';
	import BuilderSidebar from '#lib/components/app/builder/BuilderSidebar.svelte';
	import WidgetProperties from '#lib/components/app/builder/WidgetProperties.svelte';
	import NameDialog from '#lib/components/app/NameDialog.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Kbd } from '#lib/components/ui/kbd/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as Tabs from '#lib/components/ui/tabs/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import { addPage } from '#lib/logic/app-def.js';
	import { appHref } from '#lib/logic/routes.js';
	import { appSession, type AppMode } from '#lib/state/app.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	const name = $derived(page.url.searchParams.get('name') ?? '');
	let paletteOpen = $state(false);
	let addingPage = $state(false);

	// Only the route's name should reopen the app, not state read inside open().
	$effect(() => {
		const n = name;
		if (n) untrack(() => appSession.open(n));
	});
	if (!workspace.loaded) workspace.load();

	const def = $derived(appSession.def);
	const editing = $derived(appSession.mode === 'edit');
	const ready = $derived(!!def && appSession.name === name);

	async function back() {
		if (await appSession.close()) goto(resolve('/'));
	}

	async function switchTo(app: string) {
		if (await appSession.close()) goto(appHref(resolve('/app'), app));
	}

	async function about() {
		if (await appSession.close()) goto(resolve('/about'));
	}

	function inTextBox(e: KeyboardEvent): boolean {
		const t = e.target as HTMLElement | null;
		return !!t && (t.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName));
	}

	function onKey(e: KeyboardEvent) {
		const mod = e.metaKey || e.ctrlKey;
		if (mod && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
			return;
		}
		if (!editing || inTextBox(e)) return;
		if (mod && e.key.toLowerCase() === 'z') {
			e.preventDefault();
			if (e.shiftKey) appSession.redo();
			else appSession.undo();
		} else if (mod && e.key.toLowerCase() === 'y') {
			e.preventDefault();
			appSession.redo();
		} else if (e.key === 'Escape' && appSession.selected) {
			appSession.selected = null;
		} else if ((e.key === 'Delete' || e.key === 'Backspace') && appSession.selected) {
			// Undo brings it back.
			e.preventDefault();
			appSession.deleteWidget(appSession.selected);
		}
	}

	const pageTitleProblem = (v: string) =>
		!v.trim() ? 'Enter a name' : def?.pages.some((p) => p.title.toLowerCase() === v.trim().toLowerCase()) ? 'A page with this name already exists' : null;
</script>

<svelte:window onkeydown={onKey} onbeforeunload={() => appSession.flush()} />
<svelte:head><title>{name} · SlateDB</title></svelte:head>

{#if appSession.loadError && appSession.name === name}
	<Empty.Root class="min-h-svh">
		<Empty.Header>
			<Empty.Media variant="icon"><TriangleAlertIcon /></Empty.Media>
			<Empty.Title>Couldn't open “{name}”</Empty.Title>
			<Empty.Description>{appSession.loadError}</Empty.Description>
		</Empty.Header>
		<Empty.Content>
			<Button onclick={back}><ArrowLeftIcon data-icon="inline-start" /> Back to home</Button>
		</Empty.Content>
	</Empty.Root>
{:else}
	<div class="flex h-svh flex-col" data-accent={def?.accent ?? 'neutral'}>
		<header class="flex h-12 shrink-0 items-center gap-2 border-b px-3 print:hidden">
			<DropdownMenu.Root onOpenChange={(open) => open && workspace.refresh()}>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="sm" class="max-w-64">
							<LayoutDashboardIcon class="text-(--app-accent)" />
							<span class="truncate font-semibold">{name}</span>
							<ChevronDownIcon data-icon="inline-end" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content class="w-64" align="start">
					<DropdownMenu.Group>
						<DropdownMenu.Label>Apps</DropdownMenu.Label>
						{#each workspace.apps.filter((a) => a.valid) as app (app.name)}
							<DropdownMenu.Item onclick={() => app.name !== name && switchTo(app.name)}>
								<LayoutDashboardIcon />
								<span class="truncate">{app.name}</span>
								{#if app.name === name}<CheckIcon class="ms-auto" />{/if}
							</DropdownMenu.Item>
						{/each}
					</DropdownMenu.Group>
					<DropdownMenu.Separator />
					<DropdownMenu.Group>
						<DropdownMenu.Item onclick={back}><ArrowLeftIcon /> Home</DropdownMenu.Item>
						<DropdownMenu.Item onclick={about}><InfoIcon /> About SlateDB</DropdownMenu.Item>
					</DropdownMenu.Group>
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />

			{#if ready && def}
				<div class="flex min-w-0 items-center gap-1 overflow-x-auto [scrollbar-width:none]">
					<Tabs.Root value={appSession.page?.id ?? ''} onValueChange={(v) => v && appSession.showPage(v)}>
						<Tabs.List>
							{#each def.pages as p (p.id)}
								<Tabs.Trigger value={p.id}>{p.title}</Tabs.Trigger>
							{/each}
						</Tabs.List>
					</Tabs.Root>
					{#if editing}
						<Button variant="ghost" size="icon-sm" aria-label="New page" title="New page" onclick={() => (addingPage = true)}><PlusIcon /></Button>
					{/if}
				</div>
			{/if}

			<span class="flex-1"></span>

			{#if ready && editing}
				<span class="flex items-center gap-1 text-xs text-muted-foreground" aria-live="polite">
					{#if appSession.saveState === 'saving'}<Spinner class="size-3" /> Saving…
					{:else if appSession.saveState === 'error'}<CloudAlertIcon class="size-3.5 text-destructive" /> Not saved
					{:else if appSession.saveState === 'unsaved'}Editing…
					{:else}Saved{/if}
				</span>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon-sm" aria-label="Undo" disabled={!appSession.canUndo} onclick={() => appSession.undo()}><Undo2Icon /></Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content>Undo <Kbd>⌘Z</Kbd></Tooltip.Content>
				</Tooltip.Root>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon-sm" aria-label="Redo" disabled={!appSession.canRedo} onclick={() => appSession.redo()}><Redo2Icon /></Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content>Redo <Kbd>⇧⌘Z</Kbd></Tooltip.Content>
				</Tooltip.Root>
				<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />
			{/if}

			{#if ready}
				<ToggleGroup.Root
					type="single"
					size="sm"
					variant="outline"
					value={appSession.mode}
					onValueChange={(v) => v && appSession.setMode(v as AppMode)}
					aria-label="Mode"
				>
					<ToggleGroup.Item value="edit" aria-label="Edit"><PencilIcon /> Edit</ToggleGroup.Item>
					<ToggleGroup.Item value="run" aria-label="Run"><PlayIcon /> Run</ToggleGroup.Item>
				</ToggleGroup.Root>
			{/if}
			<Button variant="ghost" size="sm" class="text-muted-foreground" onclick={() => (paletteOpen = true)}>
				<SearchIcon data-icon="inline-start" /> Search <Kbd>⌘K</Kbd>
			</Button>
			<ThemeToggle />
		</header>

		<main class="flex min-h-0 flex-1">
			{#if !ready || !appSession.page}
				<div class="flex flex-1 flex-col gap-3 p-6">
					<Skeleton class="h-12" />
					<div class="grid grid-cols-3 gap-3">{#each Array(3) as _, i (i)}<Skeleton class="h-40" />{/each}</div>
				</div>
			{:else}
				{#if editing}<BuilderSidebar />{/if}
				<div class="min-h-0 min-w-0 flex-1 overflow-auto bg-background">
					{#key appSession.page.id}
						<AppCanvas page={appSession.page} {editing} />
					{/key}
				</div>
				{#if editing && appSession.selectedWidget}
					{#key appSession.selectedWidget.id}
						<WidgetProperties widget={appSession.selectedWidget} />
					{/key}
				{/if}
			{/if}
		</main>
	</div>
{/if}

<AppCommandPalette bind:open={paletteOpen} onAdd={(t) => appSession.addWidget(t)} onBack={back} onAbout={about} />

<NameDialog
	bind:open={addingPage}
	title="New page"
	initial={`Page ${(def?.pages.length ?? 0) + 1}`}
	submitLabel="Add page"
	validate={pageTitleProblem}
	onSubmit={async (title) => {
		let id = '';
		appSession.change((d) => (id = addPage(d, title.trim()).id));
		await appSession.showPage(id);
		return true;
	}}
/>
