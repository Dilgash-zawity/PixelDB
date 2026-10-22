<script lang="ts">
	import { toggleMode } from 'mode-watcher';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import NetworkIcon from '@lucide/svelte/icons/network';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchCodeIcon from '@lucide/svelte/icons/search-code';
	import Settings2Icon from '@lucide/svelte/icons/settings-2';
	import SunMoonIcon from '@lucide/svelte/icons/sun-moon';
	import TableIcon from '@lucide/svelte/icons/table-2';
	import * as Command from '#lib/components/ui/command/index.js';
	import { session } from '#lib/state/session.svelte.js';

	let {
		open = $bindable(false),
		onNewTable,
		onBack,
		onSettings
	}: { open?: boolean; onNewTable: () => void; onBack: () => void; onSettings: () => void } = $props();

	const canEdit = $derived(session.tab === 'datasheet' && !!session.table && !session.isView(session.table));

	function run(action: () => void) {
		open = false;
		action();
	}
</script>

<Command.Dialog bind:open title="Search" description="Jump to a table or run an action">
	<Command.Input placeholder="Search tables and actions…" />
	<Command.List>
		<Command.Empty>Nothing found.</Command.Empty>
		{#if canEdit}
			<Command.Group heading={session.table ?? ''}>
				<Command.Item onSelect={() => run(() => session.request('new-record'))}>
					<PlusIcon /> New record
				</Command.Item>
				<Command.Item onSelect={() => run(() => session.request('new-field'))}>
					<Settings2Icon /> Add a field
				</Command.Item>
			</Command.Group>
		{/if}
		<Command.Group heading="Tables">
			{#each session.tables as t (t.name)}
				<Command.Item value={`table ${t.name}`} onSelect={() => run(() => session.select(t.name))}>
					<TableIcon /> {t.name}
				</Command.Item>
			{/each}
			{#each session.views as v (v.name)}
				<Command.Item value={`view ${v.name}`} onSelect={() => run(() => session.select(v.name))}>
					<EyeIcon /> {v.name}
				</Command.Item>
			{/each}
		</Command.Group>
		<Command.Separator />
		<Command.Group heading="General">
			<Command.Item onSelect={() => run(onNewTable)}><PlusIcon /> New table</Command.Item>
			<Command.Item onSelect={() => run(() => session.showTab('diagram'))}><NetworkIcon /> Diagram</Command.Item>
			<Command.Item onSelect={() => run(() => session.showTab('query'))}><SearchCodeIcon /> Query</Command.Item>
			<Command.Item onSelect={() => run(toggleMode)}><SunMoonIcon /> Toggle light / dark</Command.Item>
			<Command.Item onSelect={() => run(onBack)}><ArrowLeftIcon /> All databases</Command.Item>
			<Command.Item onSelect={() => run(onSettings)}><SettingsIcon /> Settings</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Dialog>
