<script lang="ts">
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import * as Empty from '#lib/components/ui/empty/index.js';
	import { duplicateWidget, findPage, type Layout, type Page } from '#lib/logic/app-def.js';
	import { cellsMoved, compact, findSpot, gridHeight, place, type Placed } from '#lib/logic/app-layout.js';
	import { appSession } from '#lib/state/app.svelte.js';
	import { confirm } from '#lib/state/feedback.svelte.js';
	import WidgetFrame from './WidgetFrame.svelte';

	let { page, editing }: { page: Page; editing: boolean } = $props();

	const ROW = 48;
	const GAP = 12;

	let width = $state(0);
	let drag = $state<{ id: string; mode: 'move' | 'resize'; x: number; y: number; origin: Layout; moved: boolean } | null>(null);
	let preview = $state<Placed[] | null>(null);

	const items = $derived(preview ?? page.widgets.map((w) => ({ id: w.id, layout: w.layout })));
	const layouts = $derived(new Map(items.map((i) => [i.id, i.layout])));
	const rows = $derived(Math.max(gridHeight(items) + (editing ? 4 : 0), editing ? 12 : 1));

	function start(e: PointerEvent, id: string, mode: 'move' | 'resize') {
		if (e.button !== 0) return;
		e.preventDefault();
		e.stopPropagation();
		const origin = page.widgets.find((w) => w.id === id)?.layout;
		if (!origin) return;
		appSession.selected = id;
		drag = { id, mode, x: e.clientX, y: e.clientY, origin: { ...origin }, moved: false };
	}

	function move(e: PointerEvent) {
		if (!drag) return;
		const colWidth = (width + GAP) / 12;
		const { dx, dy } = cellsMoved(e.clientX - drag.x, e.clientY - drag.y, colWidth, ROW + GAP);
		const o = drag.origin;
		const target =
			drag.mode === 'move' ? { ...o, x: o.x + dx, y: o.y + dy } : { ...o, w: Math.max(1, o.w + dx), h: Math.max(1, o.h + dy) };
		if (drag.mode === 'resize') target.w = Math.min(target.w, 12 - o.x);
		drag.moved ||= dx !== 0 || dy !== 0;
		const base = page.widgets.map((w) => ({ id: w.id, layout: w.layout }));
		preview = place(base, drag.id, target);
	}

	function end() {
		if (drag?.moved && preview) {
			const final = compact(preview);
			const pageId = page.id;
			appSession.change((def) => {
				const p = findPage(def, pageId);
				if (!p) return;
				for (const w of p.widgets) {
					const l = final.find((f) => f.id === w.id)?.layout;
					if (l) w.layout = l;
				}
			});
		}
		drag = null;
		preview = null;
	}

	function duplicate(id: string) {
		const w = page.widgets.find((x) => x.id === id);
		if (!w) return;
		const spot = findSpot(items, w.layout.w, w.layout.h);
		const pageId = page.id;
		let copyId: string | undefined;
		appSession.change((def) => {
			const p = findPage(def, pageId);
			if (p) copyId = duplicateWidget(p, id, spot)?.id;
		});
		if (copyId) appSession.selected = copyId;
	}

	async function remove(id: string) {
		const w = page.widgets.find((x) => x.id === id);
		if (!w) return;
		// Only widgets that took work to set up ask first; undo brings any of them back.
		const configured = w.type === 'form' || w.type === 'table' || w.type === 'chart';
		if (
			configured &&
			!(await confirm({
				title: `Delete “${w.title || 'this widget'}”?`,
				description: 'The widget is removed from the page. Its data stays in the database. You can undo this.',
				action: 'Delete widget',
				destructive: true
			}))
		) {
			return;
		}
		appSession.deleteWidget(id);
	}
</script>

<svelte:window onpointermove={move} onpointerup={end} onpointercancel={end} />

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="min-h-full p-4 print:p-0" onclick={() => editing && (appSession.selected = null)}>
	{#if !page.widgets.length}
		<Empty.Root class="min-h-80 border border-dashed">
			<Empty.Header>
				<Empty.Media variant="icon"><LayoutDashboardIcon /></Empty.Media>
				<Empty.Title>{editing ? 'Add your first widget' : 'This page is empty'}</Empty.Title>
				<Empty.Description>
					{editing
						? 'Pick a table, form, chart or another widget from the panel on the left.'
						: 'Switch to Edit to add tables, forms and charts.'}
				</Empty.Description>
			</Empty.Header>
		</Empty.Root>
	{:else}
		<div
			bind:clientWidth={width}
			class="relative grid"
			class:select-none={!!drag}
			style="grid-template-columns: repeat(12, minmax(0, 1fr)); grid-template-rows: repeat({rows}, {ROW}px); gap: {GAP}px;"
		>
			{#if editing}
				<!-- Column guides while editing. -->
				<div class="pointer-events-none absolute inset-0 grid" style="grid-template-columns: repeat(12, minmax(0, 1fr)); gap: {GAP}px;" aria-hidden="true">
					{#each Array(12) as _, i (i)}<div class="bg-muted/30"></div>{/each}
				</div>
			{/if}
			{#each page.widgets as w (w.id)}
				{@const l = layouts.get(w.id) ?? w.layout}
				<div class="relative min-w-0" style="grid-column: {l.x + 1} / span {l.w}; grid-row: {l.y + 1} / span {l.h};">
					<WidgetFrame
						widget={w}
						{editing}
						selected={appSession.selected === w.id}
						onselect={() => (appSession.selected = w.id)}
						ondragstart={(e) => start(e, w.id, 'move')}
						onresizestart={(e) => start(e, w.id, 'resize')}
						onduplicate={() => duplicate(w.id)}
						onremove={() => remove(w.id)}
					/>
				</div>
			{/each}
		</div>
	{/if}
</div>
