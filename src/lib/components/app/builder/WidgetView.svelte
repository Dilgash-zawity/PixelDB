<script lang="ts">
	// Types
	import type { Widget } from '#lib/logic/app-def.js';

	// Components
	import BoardWidgetView from './BoardWidgetView.svelte';
	import ButtonWidgetView from './ButtonWidgetView.svelte';
	import ChartWidgetView from './ChartWidgetView.svelte';
	import FilterWidgetView from './FilterWidgetView.svelte';
	import FormWidgetView from './FormWidgetView.svelte';
	import QueryWidgetView from './QueryWidgetView.svelte';
	import StatWidgetView from './StatWidgetView.svelte';
	import TableWidgetView from './TableWidgetView.svelte';
	import TextWidgetView from './TextWidgetView.svelte';

	// Script
	let { widget, editing }: { widget: Widget; editing: boolean } = $props();
</script>

{#if widget.type === 'table'}
	<TableWidgetView {widget} {editing} />
{:else if widget.type === 'form'}
	<FormWidgetView {widget} {editing} />
{:else if widget.type === 'board'}
	<BoardWidgetView {widget} {editing} />
{:else if widget.type === 'chart'}
	<ChartWidgetView {widget} />
{:else if widget.type === 'stat'}
	<StatWidgetView {widget} />
{:else if widget.type === 'filter'}
	<FilterWidgetView {widget} />
{:else if widget.type === 'query'}
	<QueryWidgetView {widget} {editing} />
{:else if widget.type === 'button'}
	<ButtonWidgetView {widget} {editing} />
{:else}
	<TextWidgetView {widget} />
{/if}
