<script lang="ts">
	import CopyIcon from '@lucide/svelte/icons/copy';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import GripVerticalIcon from '@lucide/svelte/icons/grip-vertical';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { widgetLabel, type Widget } from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { cn } from '#lib/utils.js';
	import WidgetTypeIcon from './WidgetTypeIcon.svelte';
	import WidgetView from './WidgetView.svelte';

	let {
		widget,
		editing,
		selected,
		onselect,
		ondragstart,
		onresizestart,
		onduplicate,
		onremove
	}: {
		widget: Widget;
		editing: boolean;
		selected: boolean;
		onselect: () => void;
		ondragstart: (e: PointerEvent) => void;
		onresizestart: (e: PointerEvent) => void;
		onduplicate: () => void;
		onremove: () => void;
	} = $props();

	const problems = $derived(appSession.problems(widget));
	/** Text, buttons and filters sit on the page without a card around them when running. */
	const bare = $derived(!editing && (widget.type === 'text' || widget.type === 'button' || widget.type === 'filter'));
	// Number cards show their caption instead; text and buttons have no title.
	const showTitle = $derived(!!widget.title && widget.type !== 'text' && widget.type !== 'button' && widget.type !== 'stat');
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div
	class={cn(
		'group/widget relative flex h-full min-h-0 flex-col',
		!bare && 'rounded-lg border bg-card text-card-foreground vivid:shadow-xs',
		editing && 'border-dashed',
		editing && selected && 'border-solid border-ring ring-2 ring-ring/40'
	)}
	onclick={(e) => {
		if (!editing) return;
		e.stopPropagation();
		onselect();
	}}
>
	{#if editing}
		<div class="flex h-8 shrink-0 items-center gap-1 border-b border-dashed ps-1 pe-1 text-xs text-muted-foreground">
			<button
				type="button"
				class="flex h-6 cursor-grab touch-none items-center px-0.5 active:cursor-grabbing"
				aria-label="Move {widget.title || widgetLabel(widget.type)}"
				onpointerdown={ondragstart}
			>
				<GripVerticalIcon class="size-4" />
			</button>
			<WidgetTypeIcon type={widget.type} class="size-3.5" />
			<span class="flex-1 truncate">{widget.title || widgetLabel(widget.type)}</span>
			{#if problems.length}
				<TriangleAlertIcon class="size-3.5 text-warning" aria-label="Needs attention" />
			{/if}
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-xs" aria-label="Widget options"><EllipsisIcon /></Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="end">
					<DropdownMenu.Group>
						<DropdownMenu.Item onclick={onduplicate}><CopyIcon /> Duplicate</DropdownMenu.Item>
					</DropdownMenu.Group>
					<DropdownMenu.Separator />
					<DropdownMenu.Group>
						<DropdownMenu.Item variant="destructive" onclick={onremove}><TrashIcon /> Delete</DropdownMenu.Item>
					</DropdownMenu.Group>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</div>
	{:else if showTitle && !bare}
		<div class="truncate px-3 pt-2.5 text-sm font-medium">{widget.title}</div>
	{:else if showTitle}
		<div class="truncate pb-1 text-xs font-medium text-muted-foreground">{widget.title}</div>
	{/if}

	<div class={cn('min-h-0 flex-1', !bare && 'p-3')} inert={editing}>
		{#if problems.length}
			<Alert.Root class="border-warning/40 bg-warning/10 text-warning">
				<TriangleAlertIcon />
				<Alert.Title>This {widgetLabel(widget.type).toLowerCase()} needs attention</Alert.Title>
				<Alert.Description>
					<ul class="list-disc ps-4">{#each problems as p (p)}<li>{p}</li>{/each}</ul>
				</Alert.Description>
			</Alert.Root>
		{:else}
			<WidgetView {widget} {editing} />
		{/if}
	</div>

	{#if editing}
		<button
			type="button"
			class="absolute end-0 bottom-0 size-4 cursor-se-resize touch-none opacity-0 group-hover/widget:opacity-100 focus-visible:opacity-100"
			class:opacity-100={selected}
			aria-label="Resize {widget.title || widgetLabel(widget.type)}"
			onpointerdown={onresizestart}
		>
			<svg viewBox="0 0 16 16" class="size-4 text-muted-foreground" aria-hidden="true">
				<path d="M15 6 6 15M15 11l-4 4" stroke="currentColor" stroke-width="1.5" fill="none" />
			</svg>
		</button>
	{/if}
</div>
