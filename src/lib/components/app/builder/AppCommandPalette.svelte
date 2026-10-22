<script lang="ts">
	import { toggleMode } from 'mode-watcher';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import FileIcon from '@lucide/svelte/icons/file';
	import InfoIcon from '@lucide/svelte/icons/info';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import PlayIcon from '@lucide/svelte/icons/play';
	import PrinterIcon from '@lucide/svelte/icons/printer';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import SunMoonIcon from '@lucide/svelte/icons/sun-moon';
	import * as Command from '#lib/components/ui/command/index.js';
	import { WIDGET_TYPES, type WidgetType } from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import WidgetTypeIcon from './WidgetTypeIcon.svelte';

	let {
		open = $bindable(false),
		onAdd,
		onBack,
		onAbout
	}: { open?: boolean; onAdd: (type: WidgetType) => void; onBack: () => void; onAbout: () => void } = $props();

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
			{#each appSession.def?.pages ?? [] as p (p.id)}
				<Command.Item value={`page ${p.title}`} onSelect={() => run(() => appSession.showPage(p.id))}>
					<FileIcon /> {p.title}
				</Command.Item>
			{/each}
		</Command.Group>
		{#if appSession.mode === 'edit'}
			<Command.Group heading="Add to this page">
				{#each WIDGET_TYPES as t (t.type)}
					<Command.Item value={`add ${t.label}`} onSelect={() => run(() => onAdd(t.type))}>
						<WidgetTypeIcon type={t.type} /> Add {t.label.toLowerCase()}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
		<Command.Separator />
		<Command.Group heading="General">
			{#if appSession.mode === 'edit'}
				<Command.Item onSelect={() => run(() => appSession.setMode('run'))}><PlayIcon /> Run the app</Command.Item>
			{:else}
				<Command.Item onSelect={() => run(() => appSession.setMode('edit'))}><PencilIcon /> Edit the app</Command.Item>
			{/if}
			<Command.Item onSelect={() => run(() => appSession.run({ do: 'refresh' }))}><RefreshCwIcon /> Reload the data</Command.Item>
			<Command.Item onSelect={() => run(() => setTimeout(() => window.print(), 100))}><PrinterIcon /> Print this page</Command.Item>
			<Command.Item onSelect={() => run(toggleMode)}><SunMoonIcon /> Toggle light / dark</Command.Item>
			<Command.Item onSelect={() => run(onBack)}><ArrowLeftIcon /> Home</Command.Item>
			<Command.Item onSelect={() => run(onAbout)}><InfoIcon /> About SlateDB</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Dialog>
