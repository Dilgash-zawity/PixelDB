<script lang="ts">
	// Utils
	import { toggleMode } from 'mode-watcher';
	import { widgetLabel, WIDGET_TYPES, type WidgetType } from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';

	// Components
	import * as Command from '#lib/components/ui/command/index.js';
	import AppIcon from './AppIcon.svelte';
	import WidgetTypeIcon from './WidgetTypeIcon.svelte';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		ClipboardPaste as ClipboardPasteIcon,
		FilePlus as FilePlusIcon,
		PanelLeft as PanelLeftIcon,
		Settings as SettingsIcon,
		Pencil as PencilIcon,
		Play as PlayIcon,
		Printer as PrinterIcon,
		RefreshCw as RefreshCwIcon,
		SunMoon as SunMoonIcon,
	} from '@lucide/svelte';

	// Script
	let {
		open = $bindable(false),
		onAdd,
		onNewPage,
		onBack,
		onSettings
	}: {
		open?: boolean;
		onAdd: (type: WidgetType) => void;
		onNewPage: () => void;
		onBack: () => void;
		onSettings: () => void;
	} = $props();

	const editing = $derived(appSession.mode === 'edit');

	function run(action: () => void) {
		open = false;
		action();
	}
</script>

<Command.Dialog bind:open title="Search" description="Jump to a page or run an action">
	<Command.Input placeholder="Search pages and actions…" />
	<Command.List>
		<Command.Empty>Nothing found.</Command.Empty>
		<Command.Group heading="Pages">
			{#each (appSession.def?.pages ?? []).filter((p) => editing || !p.hidden) as p (p.id)}
				<Command.Item value={`page ${p.group} ${p.title}`} onSelect={() => run(() => appSession.showPage(p.id))}>
					<AppIcon name={p.icon} /> {p.title}
					{#if p.group}<span class="ms-auto text-xs text-muted-foreground">{p.group}</span>{/if}
				</Command.Item>
			{/each}
			{#if editing}
				<Command.Item value="new page" onSelect={() => run(onNewPage)}><FilePlusIcon /> New page…</Command.Item>
			{/if}
		</Command.Group>
		{#if !editing && appSession.def?.commands.length}
			<Command.Group heading="Actions">
				{#each appSession.def.commands as c (c.id)}
					<Command.Item value={`action ${c.label}`} onSelect={() => run(() => appSession.run(c.action))}>
						<AppIcon name={c.icon} /> {c.label}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		{#if editing}
			<Command.Group heading="Add to this page">
				{#each WIDGET_TYPES as t (t.type)}
					<Command.Item value={`add ${t.label}`} onSelect={() => run(() => onAdd(t.type))}>
						<WidgetTypeIcon type={t.type} /> Add {t.label.toLowerCase()}
					</Command.Item>
				{/each}
				{#if appSession.clipboard}
					<Command.Item value="paste widget" onSelect={() => run(() => appSession.pasteWidget())}>
						<ClipboardPasteIcon /> Paste “{appSession.clipboard.widget.title || widgetLabel(appSession.clipboard.widget.type)}”
					</Command.Item>
				{/if}
			</Command.Group>
		{/if}
		<Command.Separator />
		<Command.Group heading="General">
			{#if editing}
				<Command.Item onSelect={() => run(() => appSession.setMode('run'))}><PlayIcon /> Run the app</Command.Item>
			{:else}
				<Command.Item onSelect={() => run(() => appSession.setMode('edit'))}><PencilIcon /> Edit the app</Command.Item>
			{/if}
			<Command.Item onSelect={() => run(() => appSession.toggleSidebar())}><PanelLeftIcon /> Toggle the sidebar</Command.Item>
			<Command.Item onSelect={() => run(() => appSession.run({ do: 'refresh' }))}><RefreshCwIcon /> Reload the data</Command.Item>
			<Command.Item onSelect={() => run(() => setTimeout(() => window.print(), 100))}><PrinterIcon /> Print this page</Command.Item>
			<Command.Item onSelect={() => run(toggleMode)}><SunMoonIcon /> Toggle light / dark</Command.Item>
			<Command.Item onSelect={() => run(onBack)}><ArrowLeftIcon /> Home</Command.Item>
			<Command.Item onSelect={() => run(onSettings)}><SettingsIcon /> Settings</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Dialog>
