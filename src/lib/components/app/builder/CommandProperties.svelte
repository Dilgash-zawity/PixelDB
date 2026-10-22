<script lang="ts">
	import ArrowDownIcon from '@lucide/svelte/icons/arrow-down';
	import ArrowUpIcon from '@lucide/svelte/icons/arrow-up';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import XIcon from '@lucide/svelte/icons/x';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.js';
	import { findCommand, moveCommand, removeCommand, type AppCommand } from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import ActionEditor from './ActionEditor.svelte';
	import AppIcon from './AppIcon.svelte';
	import IconPicker from './IconPicker.svelte';

	let { command }: { command: AppCommand } = $props();

	const uid = $props.id();
	const problems = $derived(appSession.commandProblems(command));
	const index = $derived(appSession.def?.commands.findIndex((c) => c.id === command.id) ?? -1);
	const count = $derived(appSession.def?.commands.length ?? 0);

	function edit(change: (c: AppCommand) => void, group?: string) {
		const id = command.id;
		appSession.change((d) => {
			const c = findCommand(d, id);
			if (c) change(c);
		}, group ? `${id}:${group}` : undefined);
	}

	function remove() {
		const id = command.id;
		// Undo brings it back.
		appSession.change((d) => removeCommand(d, id));
		appSession.selected = null;
	}
</script>

<aside class="flex h-full w-80 shrink-0 flex-col border-s bg-sidebar print:hidden">
	<div class="flex h-11 shrink-0 items-center gap-2 border-b px-3">
		<AppIcon name={command.icon} class="size-4 text-muted-foreground" />
		<span class="flex-1 truncate text-sm font-medium">Sidebar action</span>
		<Button size="icon-sm" variant="ghost" aria-label="Close properties" onclick={() => (appSession.selected = null)}><XIcon /></Button>
	</div>
	<ScrollArea class="min-h-0 flex-1">
		<div class="flex flex-col gap-5 p-3">
			{#if problems.length}
				<Alert.Root class="border-warning/40 bg-warning/10 text-warning">
					<TriangleAlertIcon />
					<Alert.Title>Needs attention</Alert.Title>
					<Alert.Description><ul class="list-disc ps-4">{#each problems as p (p)}<li>{p}</li>{/each}</ul></Alert.Description>
				</Alert.Root>
			{/if}
			<Field.Field>
				<Field.Label for="{uid}-label">Label</Field.Label>
				<div class="flex gap-1.5">
					<IconPicker value={command.icon} onchange={(icon) => edit((c) => (c.icon = icon))} />
					<Input
						id="{uid}-label"
						class="h-8"
						value={command.label}
						oninput={(e) => {
							const v = e.currentTarget.value;
							edit((c) => (c.label = v), 'label');
						}}
						onblur={() => appSession.seal()}
					/>
				</div>
			</Field.Field>
			<Field.Set>
				<Field.Legend variant="label">When clicked</Field.Legend>
				<ActionEditor action={command.action} context="button" onchange={(next, group) => edit((c) => (c.action = next), group)} />
			</Field.Set>
			<div class="flex gap-1.5">
				<Button size="sm" variant="outline" disabled={index <= 0} onclick={() => appSession.change((d) => moveCommand(d, command.id, index - 1))}>
					<ArrowUpIcon data-icon="inline-start" /> Up
				</Button>
				<Button size="sm" variant="outline" disabled={index < 0 || index >= count - 1} onclick={() => appSession.change((d) => moveCommand(d, command.id, index + 1))}>
					<ArrowDownIcon data-icon="inline-start" /> Down
				</Button>
				<span class="flex-1"></span>
				<Button size="sm" variant="destructive" onclick={remove}><TrashIcon data-icon="inline-start" /> Delete</Button>
			</div>
		</div>
	</ScrollArea>
</aside>
