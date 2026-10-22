<script lang="ts">
	import EllipsisVerticalIcon from '@lucide/svelte/icons/ellipsis-vertical';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import LinkIcon from '@lucide/svelte/icons/link';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Table2Icon from '@lucide/svelte/icons/table-2';
	import Rows3Icon from '@lucide/svelte/icons/rows-3';
	import TrashIcon from '@lucide/svelte/icons/trash-2';
	import type { TableDef, TablePosition } from '#lib/api/types.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { fieldTypeLabel } from '#lib/logic/field-types.js';
	import { CARD_HEADER, CARD_PADDING, CARD_ROW, CARD_WIDTH } from '#lib/logic/relationships.js';
	import { cn } from '#lib/utils.js';

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
		onHeaderDown,
		onFieldClick,
		onAddField,
		onConnectStart,
		onOpen,
		onLink,
		onRename,
		onDelete,
		onHover
	}: {
		def: TableDef;
		position: TablePosition;
		rows: number | null;
		selected?: boolean;
		highlight?: string[];
		dropTarget?: string | null;
		connecting?: boolean;
		selectedField?: string | null;
		onHeaderDown: (e: PointerEvent) => void;
		onFieldClick: (field: string) => void;
		onAddField: () => void;
		onConnectStart: (e: PointerEvent, field: string, side: 1 | -1) => void;
		onOpen: () => void;
		onLink: () => void;
		onRename: () => void;
		onDelete: () => void;
		onHover: (inside: boolean) => void;
	} = $props();

	const linked = $derived(new Set((def.foreign_keys ?? []).flatMap((fk) => fk.columns.map((c) => c.toLowerCase()))));
	const lit = $derived(new Set(highlight.map((c) => c.toLowerCase())));
</script>

<div
	data-card={def.name}
	role="group"
	aria-label={`Table ${def.name}`}
	class={cn(
		'absolute flex flex-col rounded-lg border bg-card text-card-foreground shadow-md shadow-black/5 transition-[box-shadow,border-color] select-none',
		selected && 'border-primary ring-2 ring-primary/30',
		dropTarget === '' && 'border-primary ring-2 ring-primary/40'
	)}
	style:left="{position.x}px"
	style:top="{position.y}px"
	style:width="{CARD_WIDTH}px"
	onpointerenter={() => onHover(true)}
	onpointerleave={() => onHover(false)}
>
	<div
		class="flex shrink-0 cursor-grab items-center gap-2 border-b ps-3 pe-1.5 active:cursor-grabbing"
		style:height="{CARD_HEADER}px"
		role="presentation"
		onpointerdown={onHeaderDown}
		ondblclick={onOpen}
	>
		<Table2Icon class="size-4 shrink-0 text-muted-foreground" />
		<span class="truncate text-sm font-semibold">{def.name}</span>
		{#if rows !== null}
			<span class="shrink-0 text-xs text-muted-foreground tabular-nums">{rows.toLocaleString()}</span>
		{/if}
		<span class="flex-1"></span>
		<span role="presentation" onpointerdown={(e) => e.stopPropagation()}>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon-sm" aria-label={`${def.name} options`}>
							<EllipsisVerticalIcon />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content align="start" side="right">
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
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</span>
	</div>
	<div style:padding-block="{CARD_PADDING}px">
		{#each def.fields as f (f.name)}
			{@const isLit = lit.has(f.name.toLowerCase())}
			<div
				role="button"
				tabindex="0"
				aria-label={`Edit field ${f.name}`}
				data-row
				data-table={def.name}
				data-field={f.name}
				class={cn(
					'group/row relative flex cursor-pointer items-center gap-2 px-3 text-sm hover:bg-accent',
					isLit && 'bg-primary/10',
					selectedField === f.name && 'bg-accent font-medium',
					dropTarget === f.name && 'bg-primary/20',
					connecting && 'hover:bg-primary/15'
				)}
				style:height="{CARD_ROW}px"
				onclick={() => onFieldClick(f.name)}
				onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), onFieldClick(f.name))}
			>
				<span class="flex w-4 shrink-0 justify-center">
					{#if f.primary_key}
						<KeyRoundIcon class="size-3.5 text-warning" aria-label="Primary key" />
					{:else if linked.has(f.name.toLowerCase())}
						<LinkIcon class="size-3.5 text-muted-foreground" aria-label="Links to another table" />
					{/if}
				</span>
				<span class={cn('truncate', isLit && 'font-semibold')}>{f.name}</span>
				<span class="ms-auto shrink-0 text-xs text-muted-foreground">{fieldTypeLabel(f.field_type)}</span>
				{#each [-1, 1] as const as side (side)}
					<button
						type="button"
						tabindex="-1"
						aria-label={`Link ${def.name}.${f.name} to another table`}
						title="Drag onto another table to link"
						class={cn(
							'absolute top-1/2 size-3 -translate-y-1/2 cursor-crosshair rounded-full border-2 border-primary bg-background opacity-0 transition-opacity group-hover/row:opacity-100 hover:scale-125',
							side === 1 ? '-end-1.5' : '-start-1.5'
						)}
						onpointerdown={(e) => onConnectStart(e, f.name, side)}
						onclick={(e) => e.stopPropagation()}
					></button>
				{/each}
			</div>
		{/each}
		<div style:height="{CARD_ROW}px">
			<button
				type="button"
				class="flex size-full items-center gap-2 px-3 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
				onclick={onAddField}
			>
				<PlusIcon class="size-3.5" /> Add field
			</button>
		</div>
	</div>
</div>
