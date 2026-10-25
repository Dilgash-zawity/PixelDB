<script lang="ts">
	// Utils
	import { untrack } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { pageGroups, removeCommand } from '#lib/logic/app-def.js';
	import { matches, shortcutText } from '#lib/logic/shortcuts.js';
	import { appHref } from '#lib/logic/routes.js';
	import { appSession, type AppMode } from '#lib/state/app.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import AppCanvas from '#lib/components/app/builder/AppCanvas.svelte';
	import AppCommandPalette from '#lib/components/app/builder/AppCommandPalette.svelte';
	import AppIcon from '#lib/components/app/builder/AppIcon.svelte';
	import AppSidebar from '#lib/components/app/builder/AppSidebar.svelte';
	import BuilderSidebar from '#lib/components/app/builder/BuilderSidebar.svelte';
	import CommandProperties from '#lib/components/app/builder/CommandProperties.svelte';
	import NewPageDialog from '#lib/components/app/builder/NewPageDialog.svelte';
	import WidgetProperties from '#lib/components/app/builder/WidgetProperties.svelte';
	import Shortcut from '#lib/components/app/Shortcut.svelte';
	import SearchButton from '#lib/components/app/SearchButton.svelte';
	import ThemeToggle from '#lib/components/app/ThemeToggle.svelte';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Skeleton } from '#lib/components/ui/skeleton/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Check as CheckIcon,
		ChevronDown as ChevronDownIcon,
		CloudAlert as CloudAlertIcon,
		LayoutDashboard as LayoutDashboardIcon,
		PanelLeft as PanelLeftIcon,
		Pencil as PencilIcon,
		Play as PlayIcon,
		Redo2 as Redo2Icon,
		TriangleAlert as TriangleAlertIcon,
		Undo2 as Undo2Icon,
	} from '@lucide/svelte';

	// Script
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

	async function openSettings() {
		if (await appSession.close()) goto(resolve('/settings'));
	}

	function inTextBox(e: KeyboardEvent): boolean {
		const t = e.target as HTMLElement | null;
		return !!t && (t.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(t.tagName));
	}

	/**
	 * Shortcuts for the selection (copy, arrows, Delete, Escape) only apply on
	 * the canvas or the page sidebar, not while a menu, dropdown or the
	 * properties panel has focus.
	 */
	function onCanvas(e: KeyboardEvent): boolean {
		const t = e.target as HTMLElement | null;
		return !e.defaultPrevented && (!t || t === document.body || !!t.closest('[data-canvas], [data-app-sidebar]'));
	}

	function onKey(e: KeyboardEvent) {
		const mod = e.metaKey || e.ctrlKey;
		if (mod && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
			return;
		}
		if (mod && e.key.toLowerCase() === 'b' && !inTextBox(e)) {
			e.preventDefault();
			appSession.toggleSidebar();
			return;
		}
		if (matches(e, 'mod+e') && appSession.def) {
			e.preventDefault();
			appSession.setMode(editing ? 'run' : 'edit');
			return;
		}
		if ((matches(e, 'alt+arrowdown') || matches(e, 'alt+arrowup')) && !inTextBox(e) && appSession.def) {
			e.preventDefault();
			// In sidebar order; hidden pages only while editing, as the sidebar shows them.
			const pages = pageGroups(appSession.def.pages, editing).flatMap((g) => g.pages);
			const i = pages.findIndex((p) => p.id === appSession.page?.id);
			const next = pages[i + (e.key === 'ArrowDown' ? 1 : -1)];
			if (next) appSession.showPage(next.id);
			return;
		}
		if (!editing || inTextBox(e)) return;
		const widgetId = onCanvas(e) ? appSession.selectedWidget?.id : undefined;
		const key = e.key.toLowerCase();
		if (mod && key === 'v' && appSession.clipboard && onCanvas(e)) {
			e.preventDefault();
			appSession.pasteWidget();
			return;
		}
		if (widgetId && mod && (key === 'c' || key === 'x')) {
			e.preventDefault();
			appSession.copyWidget(widgetId);
			if (key === 'x') appSession.deleteWidget(widgetId);
			else toast.success('Widget copied', { description: 'Paste it on any page with ⌘V.' });
			return;
		}
		if (widgetId && mod && key === 'd') {
			e.preventDefault();
			appSession.duplicateWidget(widgetId);
			return;
		}
		const arrows: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
		if (widgetId && !mod && e.key in arrows) {
			e.preventDefault();
			const [dx, dy] = arrows[e.key];
			appSession.nudgeWidget(widgetId, dx, dy, e.shiftKey);
			return;
		}
		if (mod && e.key.toLowerCase() === 'z') {
			e.preventDefault();
			if (e.shiftKey) appSession.redo();
			else appSession.undo();
		} else if (mod && e.key.toLowerCase() === 'y') {
			e.preventDefault();
			appSession.redo();
		} else if (e.key === 'Escape' && appSession.selected && onCanvas(e)) {
			appSession.selected = null;
		} else if ((e.key === 'Delete' || e.key === 'Backspace') && appSession.selected && onCanvas(e)) {
			// Undo brings it back.
			e.preventDefault();
			const id = appSession.selected;
			if (appSession.selectedCommand) {
				appSession.change((d) => removeCommand(d, id));
				appSession.selected = null;
			} else {
				appSession.deleteWidget(id);
			}
		}
	}
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
		<header data-bar class="app-header flex h-12 shrink-0 items-center gap-2 border-b px-3 print:hidden">
			<Tooltip.Root>
				<Tooltip.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							size="icon-sm"
							aria-label={appSession.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
							aria-pressed={!appSession.sidebarCollapsed}
							onclick={() => appSession.toggleSidebar()}
						>
							<PanelLeftIcon />
						</Button>
					{/snippet}
				</Tooltip.Trigger>
				<Tooltip.Content>{appSession.sidebarCollapsed ? 'Expand' : 'Collapse'} sidebar <Shortcut keys="mod+b" /></Tooltip.Content>
			</Tooltip.Root>
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
					</DropdownMenu.Group>
				</DropdownMenu.Content>
			</DropdownMenu.Root>

			{#if ready && appSession.page}
				<Separator orientation="vertical" class="data-vertical:h-5 data-vertical:self-center" />
				<span class="flex min-w-0 items-center gap-2 px-1 text-sm">
					<AppIcon name={appSession.page.icon} class="size-4 shrink-0 text-muted-foreground vivid:text-(--app-accent)" />
					<span class="truncate">{appSession.page.title}</span>
				</span>
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
					<Tooltip.Content>Undo <Shortcut keys="mod+z" /></Tooltip.Content>
				</Tooltip.Root>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}
							<Button {...props} variant="ghost" size="icon-sm" aria-label="Redo" disabled={!appSession.canRedo} onclick={() => appSession.redo()}><Redo2Icon /></Button>
						{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content>Redo <Shortcut keys="shift+mod+z" /></Tooltip.Content>
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
					title="Switch between Edit and Run ({shortcutText('mod+e')})"
				>
					<ToggleGroup.Item value="edit" aria-label="Edit"><PencilIcon /> Edit</ToggleGroup.Item>
					<ToggleGroup.Item value="run" aria-label="Run"><PlayIcon /> Run</ToggleGroup.Item>
				</ToggleGroup.Root>
			{/if}
			<SearchButton onclick={() => (paletteOpen = true)} />
			<ThemeToggle />
		</header>

		<main class="flex min-h-0 flex-1">
			{#if !ready || !appSession.page}
				<div class="flex flex-1 flex-col gap-3 p-6">
					<Skeleton class="h-12" />
					<div class="grid grid-cols-3 gap-3">{#each Array(3) as _, i (i)}<Skeleton class="h-40" />{/each}</div>
				</div>
			{:else}
				<AppSidebar {editing} onNewPage={() => (addingPage = true)} />
				<div class="min-h-0 min-w-0 flex-1 overflow-auto bg-background" data-canvas>
					{#key appSession.page.id}
						<AppCanvas page={appSession.page} {editing} />
					{/key}
				</div>
				{#if editing}
					{#if appSession.selectedWidget}
						{#key appSession.selectedWidget.id}
							<WidgetProperties widget={appSession.selectedWidget} />
						{/key}
					{:else if appSession.selectedCommand}
						{#key appSession.selectedCommand.id}
							<CommandProperties command={appSession.selectedCommand} />
						{/key}
					{:else}
						<BuilderSidebar />
					{/if}
				{/if}
			{/if}
		</main>
	</div>
{/if}

<AppCommandPalette
	bind:open={paletteOpen}
	onAdd={(t) => appSession.addWidget(t)}
	onNewPage={() => (addingPage = true)}
	onBack={back}
	onSettings={openSettings}
/>

<NewPageDialog bind:open={addingPage} />
