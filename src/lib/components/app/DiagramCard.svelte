<script lang="ts">
	// Types
	import type { FieldDef, TableDef, TablePosition } from '#lib/api/types.js';

	// Utils
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import { CARD_HEADER, CARD_PADDING, CARD_ROW, CARD_WIDTH } from '#lib/logic/relationships.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import PixelIcon from './PixelIcon.svelte';

	// Icons
	import {
		EllipsisVertical as EllipsisVerticalIcon,
		KeyRound as KeyRoundIcon,
		Link as LinkIcon,
		ListTree as ListTreeIcon,
		Plus as PlusIcon,
		Pencil as PencilIcon,
		Rows3 as Rows3Icon,
		Trash2 as TrashIcon,
	} from '@lucide/svelte';

	// Script
	let {
		def,
		position,
		rows,
		selected = false,
		/** Fields of this table used by the selected or hovered relationship. */
		highlight = [],
		/** Field (or '' for the whole card) a connection would be dropped on. */
		dropTarget = null,
		connecting = false,
		/** The field open in the side panel, if it belongs to this table. */
		selectedField = null,
		/** Online tables: no editing; the menu opens the rows or the structure. */
		readonly = false,
		onHeaderDown,
		onFieldClick,
		onAddField,
		onConnectStart,
		onOpen,
		onLink,
		onRename,
		onDelete,
		onHover,
		onStructure,
		onHeaderFocus,
		onHeaderKey
	}: {
		def: TableDef;
		position: TablePosition;
		rows: number | null;
		selected?: boolean;
		highlight?: string[];
		dropTarget?: string | null;
		connecting?: boolean;
		selectedField?: string | null;
		readonly?: boolean;
		onHeaderDown: (e: PointerEvent) => void;
		onFieldClick?: (field: string) => void;
		onAddField?: () => void;
		onConnectStart?: (e: PointerEvent, field: string, side: 1 | -1) => void;
		onOpen: () => void;
		onLink?: () => void;
		onRename?: () => void;
		onDelete?: () => void;
		onHover: (inside: boolean) => void;
		onStructure?: () => void;
		/** Read-only cards: the header is a focus stop (Enter opens the rows). */
		onHeaderFocus?: () => void;
		onHeaderKey?: (e: KeyboardEvent) => void;
	} = $props();

	const linked = $derived(new Set((def.foreign_keys ?? []).flatMap((fk) => fk.columns.map((c) => c.toLowerCase()))));
	const lit = $derived(new Set(highlight.map((c) => c.toLowerCase())));
	const SIDES = [-1, 1] as const;

</script>

{#snippet fieldContent(f: FieldDef, isLit: boolean)}
	<span class="flex w-4 shrink-0 justify-center">
		{#if f.primary_key}
			<KeyRoundIcon class="size-3.5 text-warning" aria-label="Primary key" />
		{:else if linked.has(f.name.toLowerCase())}
			<LinkIcon class="size-3.5 text-muted-foreground" aria-label="Links to another table" />
		{/if}
	</span>
	<span class={cn('truncate', isLit && 'font-semibold')}>{f.name}</span>
	<span class="ms-auto shrink-0 text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
{/snippet}

<div
	data-card={def.name}
	role="group"
	aria-label={`Table ${def.name}`}
	class={cn(
		'absolute flex flex-col rounded-lg border border-(--map-line)/25 bg-card text-card-foreground transition-[box-shadow,border-color] select-none',
		selected && 'ring-2 ring-(--map-accent)',
		dropTarget === '' && 'ring-4 ring-(--map-accent)'
	)}
	style:left="{position.x}px"
	style:top="{position.y}px"
	style:width="{CARD_WIDTH}px"
	onpointerenter={() => onHover(true)}
	onpointerleave={() => onHover(false)}
>
	<!-- svelte-ignore a11y_no_noninteractive_tabindex (read-only cards: a focus stop that Enter opens; it holds the options button, so it can't be a button itself) -->
	<div
		class={cn(
			'-m-px flex shrink-0 cursor-grab items-center gap-2 rounded-t-[inherit] ps-3 pe-1.5 active:cursor-grabbing',
			selected ? 'bg-(--map-accent) text-(--map-accent-fg)' : 'bg-(--map-app) text-(--map-tile-fg)'
		)}
		style:height="{CARD_HEADER}px"
		data-card-header={def.name}
		role={readonly ? 'group' : 'presentation'}
		tabindex={readonly ? 0 : undefined}
		aria-label={readonly ? `Table ${def.name}, Enter opens its rows` : undefined}
		aria-keyshortcuts={readonly ? 'Enter' : undefined}
		onpointerdown={onHeaderDown}
		ondblclick={onOpen}
		onfocus={onHeaderFocus}
		onkeydown={(e) => {
			if (!readonly || e.target !== e.currentTarget) return;
			if (e.key === 'Enter') {
				e.preventDefault();
				onOpen();
			} else onHeaderKey?.(e);
		}}
	>
		<PixelIcon name="table" class="size-4 shrink-0" />
		<span class="truncate text-sm font-medium">{def.name}</span>
		{#if rows !== null}
			<span class="shrink-0 rounded-sm border border-current/30 px-1 text-[11px] tabular-nums opacity-80">{rows.toLocaleString()}</span>
		{/if}
		<span class="flex-1"></span>
		<span role="presentation" onpointerdown={(e) => e.stopPropagation()}>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-sm" class="text-current hover:bg-white/15 hover:text-current" aria-label={`${def.name} options`}>
							<EllipsisVerticalIcon />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="start" side="right">
					{#if readonly}
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={onOpen}><Rows3Icon /> Open data</DropdownMenu.Item>
							<DropdownMenu.Item onclick={onStructure}><ListTreeIcon /> Show structure</DropdownMenu.Item>
						</DropdownMenu.Group>
					{:else}
						<DropdownMenu.Group>
							<DropdownMenu.Item onclick={onOpen}><Rows3Icon /> Open datasheet</DropdownMenu.Item>
							<DropdownMenu.Item onclick={onAddField}><PlusIcon /> Add field</DropdownMenu.Item>
							<DropdownMenu.Item onclick={onLink}><LinkIcon /> Link to another table…</DropdownMenu.Item>
							<DropdownMenu.Item onclick={onRename}><PencilIcon /> Rename</DropdownMenu.Item>
						</DropdownMenu.Group>
						<DropdownMenu.Separator />
						<DropdownMenu.Group>
							<DropdownMenu.Item variant="destructive" onclick={onDelete}><TrashIcon /> Delete table</DropdownMenu.Item>
						</DropdownMenu.Group>
					{/if}
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</span>
	</div>
	<div style:padding-block="{CARD_PADDING}px">
		{#each def.fields as f (f.name)}
			{@const isLit = lit.has(f.name.toLowerCase())}
			{@const rowClass = cn(
				'group/row relative flex items-center gap-2 px-3 text-sm',
				isLit && 'bg-(--map-accent)/15',
				selectedField === f.name && 'bg-accent font-medium',
				dropTarget === f.name && 'bg-(--map-accent)/30',
				connecting && 'hover:bg-(--map-accent)/20'
			)}
			{#if readonly}
				<div data-row data-table={def.name} data-field={f.name} class={rowClass} style:height="{CARD_ROW}px">
					{@render fieldContent(f, isLit)}
				</div>
			{:else}
				<div
					role="button"
					tabindex="0"
					aria-label={`Edit field ${f.name}`}
					data-row
					data-table={def.name}
					data-field={f.name}
					class={cn(rowClass, 'cursor-pointer hover:bg-accent')}
					style:height="{CARD_ROW}px"
					onclick={() => onFieldClick?.(f.name)}
					onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), onFieldClick?.(f.name))}
				>
					{@render fieldContent(f, isLit)}
					{#each SIDES as side (side)}
						<button
							type="button"
							tabindex="-1"
							aria-label={`Link ${def.name}.${f.name} to another table`}
							title="Drag onto another table to link"
							class={cn(
								'absolute top-1/2 size-3 -translate-y-1/2 cursor-crosshair rounded-[2px] border-2 border-(--map-line) bg-(--map-paper) opacity-0 transition-opacity group-hover/row:opacity-100 hover:scale-125',
								side === 1 ? '-end-1.5' : '-start-1.5'
							)}
							onpointerdown={(e) => onConnectStart?.(e, f.name, side)}
							onclick={(e) => e.stopPropagation()}
						></button>
					{/each}
				</div>
			{/if}
		{/each}
		{#if !readonly}<div style:height="{CARD_ROW}px">
			<button
				type="button"
				class="flex size-full items-center gap-2 px-3 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
				onclick={onAddField}
			>
				<PlusIcon class="size-3.5" /> Add field
			</button>
		</div>{/if}
	</div>
</div>
