<script lang="ts">
	// Utils
	import {
		addCommand,
		dropPage,
		duplicatePage,
		findPage,
		pageGroups,
		removePage,
		type AppCommand,
		type Page
	} from '#lib/logic/app-def.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { confirm } from '#lib/state/feedback.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as DropdownMenu from '#lib/components/ui/dropdown-menu/index.js';
	import { ScrollArea } from '#lib/components/ui/scroll-area/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import * as Tooltip from '#lib/components/ui/tooltip/index.js';
	import AppIcon from './AppIcon.svelte';

	// Icons
	import {
		Copy as CopyIcon,
		Ellipsis as EllipsisIcon,
		Eye as EyeIcon,
		EyeOff as EyeOffIcon,
		GripVertical as GripVerticalIcon,
		House as HouseIcon,
		Plus as PlusIcon,
		Settings2 as Settings2Icon,
		Trash2 as TrashIcon,
		TriangleAlert as TriangleAlertIcon,
	} from '@lucide/svelte';

	// Script
	let { editing, onNewPage }: { editing: boolean; onNewPage: () => void } = $props();

	const def = $derived(appSession.def!);
	const collapsed = $derived(appSession.sidebarCollapsed);
	const groups = $derived(pageGroups(def.pages, editing));

	// ----- Pages -----

	function openPage(p: Page) {
		if (editing && p.id === appSession.page?.id) {
			appSession.selected = null;
			appSession.builderTab = 'page';
			return;
		}
		appSession.showPage(p.id);
	}

	function copyTitle(title: string): string {
		const taken = new Set(def.pages.map((p) => p.title.toLowerCase()));
		let name = `${title} copy`;
		for (let n = 2; taken.has(name.toLowerCase()); n++) name = `${title} copy ${n}`;
		return name;
	}

	function duplicate(p: Page) {
		let id = '';
		appSession.change((d) => (id = duplicatePage(d, p.id, copyTitle(p.title))?.id ?? ''));
		if (id) appSession.showPage(id);
	}

	async function deletePage(p: Page) {
		const count = findPage(def, p.id)?.widgets.length ?? 0;
		const ok = await confirm({
			title: `Delete page “${p.title}”?`,
			description: count
				? `Its ${count} widget${count === 1 ? '' : 's'} will be removed too. The data stays in the database. You can undo this.`
				: 'You can undo this.',
			action: 'Delete page',
			destructive: true
		});
		if (!ok) return;
		appSession.change((d) => removePage(d, p.id));
		if (appSession.pageId === p.id) appSession.showPage(def.startPage);
	}

	// ----- Dragging pages to reorder or regroup -----

	let dragging = $state<string | null>(null);
	let target = $state<{ before: string | null; group: string } | null>(null);

	function startDrag(e: PointerEvent, p: Page) {
		if (e.button !== 0) return;
		e.preventDefault();
		dragging = p.id;
		target = null;
	}

	function move(e: PointerEvent) {
		if (!dragging) return;
		const el = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-drop-group]');
		if (!el) {
			target = null;
			return;
		}
		const group = el.dataset.dropGroup ?? '';
		const row = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-page-id]');
		if (!row) {
			target = { before: null, group };
			return;
		}
		const pages = groups.find((g) => g.group === group)?.pages ?? [];
		const i = pages.findIndex((p) => p.id === row.dataset.pageId);
		const r = row.getBoundingClientRect();
		const after = e.clientY > r.top + r.height / 2;
		target = { before: after ? (pages[i + 1]?.id ?? null) : pages[i].id, group };
	}

	function end() {
		const id = dragging;
		const t = target;
		dragging = null;
		target = null;
		if (!id || !t) return;
		const page = findPage(def, id);
		if (!page || (t.before === id && page.group === t.group)) return;
		// Dropping at the end of a group puts the page after that group's last page.
		const groupPages = groups.find((g) => g.group === t.group)?.pages ?? [];
		const last = groupPages.at(-1);
		const before = t.before ?? (last ? (def.pages[def.pages.findIndex((x) => x.id === last.id) + 1]?.id ?? null) : null);
		appSession.change((d) => dropPage(d, id, before, t.group));
	}

	// ----- Commands -----

	function runCommand(c: AppCommand) {
		if (editing) {
			appSession.selected = c.id;
			return;
		}
		appSession.run(c.action);
	}

	function newCommand() {
		let id = '';
		appSession.change((d) => (id = addCommand(d, `Action ${d.commands.length + 1}`).id));
		appSession.selected = id;
	}
</script>

<svelte:window onpointermove={move} onpointerup={end} onpointercancel={() => ((dragging = null), (target = null))} />

{#snippet entry(icon: string, label: string, active: boolean, muted: boolean, onclick: () => void, extra?: import('svelte').Snippet)}
	{#if collapsed}
		<Tooltip.Root>
			<Tooltip.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						size="icon-sm"
						aria-label={label}
						aria-current={active ? 'page' : undefined}
						class={cn('mx-auto', active && 'bg-sidebar-accent text-sidebar-accent-foreground vivid:bg-(--app-accent)/15 vivid:text-(--app-accent)', muted && 'text-muted-foreground')}
						{onclick}
					>
						<AppIcon name={icon} />
					</Button>
				{/snippet}
			</Tooltip.Trigger>
			<Tooltip.Content side="right">{label}</Tooltip.Content>
		</Tooltip.Root>
	{:else}
		<button
			type="button"
			class={cn(
				'flex min-w-0 flex-1 items-center gap-2 px-2 py-1.5 text-start text-sm',
				active && 'font-medium',
				muted && 'text-muted-foreground'
			)}
			aria-current={active ? 'page' : undefined}
			{onclick}
		>
			<AppIcon name={icon} class={cn('size-4 shrink-0', active ? 'text-(--app-accent)' : 'text-muted-foreground')} />
			<span class="truncate">{label}</span>
			{@render extra?.()}
		</button>
	{/if}
{/snippet}

<aside
	class={cn('flex h-full shrink-0 flex-col border-e bg-sidebar text-sidebar-foreground print:hidden', collapsed ? 'w-12' : 'w-60')}
	aria-label="Pages"
>
	<ScrollArea class="min-h-0 flex-1">
		<nav class={cn('flex flex-col gap-3 py-2', collapsed ? 'px-1' : 'px-2')}>
			{#each groups as g (g.group)}
				<div class="flex flex-col gap-0.5" data-drop-group={g.group}>
					{#if g.group}
						{#if collapsed}
							<Separator class="my-1" />
						{:else}
							<div class="px-2 pt-1 pb-0.5 text-xs font-medium text-muted-foreground">{g.group}</div>
						{/if}
					{/if}
					{#each g.pages as p (p.id)}
						{@const active = p.id === appSession.page?.id}
						{#if target && target.before === p.id && target.group === g.group}
							<div class="h-0.5 bg-ring" aria-hidden="true"></div>
						{/if}
						<div
							data-page-id={p.id}
							class={cn(
								'group/page flex items-center',
								!collapsed && 'hover:bg-sidebar-accent',
								!collapsed && active && 'bg-sidebar-accent text-sidebar-accent-foreground vivid:bg-(--app-accent)/12 vivid:shadow-[inset_3px_0_0_var(--app-accent)]',
								'rounded-md',
								dragging === p.id && 'opacity-50'
							)}
						>
							{#if editing && !collapsed}
								<button
									type="button"
									class="flex h-7 w-4 shrink-0 cursor-grab touch-none items-center justify-center text-muted-foreground opacity-0 group-hover/page:opacity-100 focus-visible:opacity-100"
									aria-label="Drag to move {p.title}"
									onpointerdown={(e) => startDrag(e, p)}
								>
									<GripVerticalIcon class="size-3.5" />
								</button>
							{/if}
							{#snippet marks()}
								{#if editing && p.id === def.startPage}<HouseIcon class="ms-auto size-3.5 shrink-0 text-muted-foreground" aria-label="Opens first" />{/if}
								{#if editing && p.hidden}<EyeOffIcon class={cn('size-3.5 shrink-0 text-muted-foreground', p.id !== def.startPage && 'ms-auto')} aria-label="Hidden from the sidebar" />{/if}
							{/snippet}
							{@render entry(p.icon, p.title, active, p.hidden, () => openPage(p), marks)}
							{#if editing && !collapsed}
								<DropdownMenu.Root>
									<DropdownMenu.Trigger>
										{#snippet child({ props })}
											<Button {...props} size="icon-xs" variant="ghost" class="me-0.5 opacity-0 group-hover/page:opacity-100 data-[state=open]:opacity-100" aria-label="Page options">
												<EllipsisIcon />
											</Button>
										{/snippet}
									</DropdownMenu.Trigger>
									<DropdownMenu.Content align="start" side="right">
										<DropdownMenu.Group>
											<DropdownMenu.Item
												onclick={async () => {
													if (await appSession.showPage(p.id)) {
														appSession.selected = null;
														appSession.builderTab = 'page';
													}
												}}
											>
												<Settings2Icon /> Page settings
											</DropdownMenu.Item>
											<DropdownMenu.Item onclick={() => duplicate(p)}><CopyIcon /> Duplicate</DropdownMenu.Item>
											<DropdownMenu.Item
												onclick={() =>
													appSession.change((d) => {
														const x = findPage(d, p.id);
														if (x) x.hidden = !x.hidden;
													})}
											>
												{#if p.hidden}<EyeIcon /> Show in sidebar{:else}<EyeOffIcon /> Hide from sidebar{/if}
											</DropdownMenu.Item>
											<DropdownMenu.Item disabled={p.id === def.startPage} onclick={() => appSession.change((d) => (d.startPage = p.id))}>
												<HouseIcon /> Open first
											</DropdownMenu.Item>
										</DropdownMenu.Group>
										<DropdownMenu.Separator />
										<DropdownMenu.Group>
											<DropdownMenu.Item variant="destructive" disabled={def.pages.length === 1} onclick={() => deletePage(p)}>
												<TrashIcon /> Delete page
											</DropdownMenu.Item>
										</DropdownMenu.Group>
									</DropdownMenu.Content>
								</DropdownMenu.Root>
							{/if}
						</div>
					{/each}
					{#if target && target.before === null && target.group === g.group}
						<div class="h-0.5 bg-ring" aria-hidden="true"></div>
					{/if}
				</div>
			{/each}

			{#if editing}
				{#if collapsed}
					<Button variant="ghost" size="icon-sm" class="mx-auto" aria-label="New page" title="New page" onclick={onNewPage}><PlusIcon /></Button>
				{:else}
					<Button size="sm" variant="ghost" class="justify-start text-muted-foreground" onclick={onNewPage}>
						<PlusIcon data-icon="inline-start" /> New page
					</Button>
				{/if}
			{/if}

			{#if def.commands.length || editing}
				<div class="flex flex-col gap-0.5">
					{#if collapsed}
						<Separator class="my-1" />
					{:else}
						<div class="px-2 pt-1 pb-0.5 text-xs font-medium text-muted-foreground">Actions</div>
					{/if}
					{#each def.commands as c (c.id)}
						{@const problem = editing && appSession.commandProblems(c).length > 0}
						<div
							class={cn(
								'flex items-center',
								!collapsed && 'hover:bg-sidebar-accent',
								!collapsed && editing && appSession.selected === c.id && 'bg-sidebar-accent ring-1 ring-ring'
							)}
						>
							{#snippet warn()}
								{#if problem}<TriangleAlertIcon class="ms-auto size-3.5 shrink-0 text-warning" aria-label="Needs attention" />{/if}
							{/snippet}
							{@render entry(c.icon, c.label, false, false, () => runCommand(c), warn)}
						</div>
					{/each}
					{#if editing}
						{#if collapsed}
							<Button variant="ghost" size="icon-sm" class="mx-auto" aria-label="New action" title="New action" onclick={newCommand}><PlusIcon /></Button>
						{:else}
							<Button size="sm" variant="ghost" class="justify-start text-muted-foreground" onclick={newCommand}>
								<PlusIcon data-icon="inline-start" /> New action
							</Button>
						{/if}
					{/if}
				</div>
			{/if}
		</nav>
	</ScrollArea>
</aside>
