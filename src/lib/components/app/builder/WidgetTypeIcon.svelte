<script lang="ts">
	import ChartColumnIcon from '@lucide/svelte/icons/chart-column';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import FunnelIcon from '@lucide/svelte/icons/funnel';
	import HashIcon from '@lucide/svelte/icons/hash';
	import MousePointerClickIcon from '@lucide/svelte/icons/mouse-pointer-click';
	import TableIcon from '@lucide/svelte/icons/table-2';
	import TypeIcon from '@lucide/svelte/icons/type';
	import type { WidgetType } from '#lib/logic/app-def.js';

	let { type, ...rest }: { type: WidgetType; class?: string; 'data-icon'?: string } = $props();

	const icons = {
		table: TableIcon,
		form: FileTextIcon,
		chart: ChartColumnIcon,
		stat: HashIcon,
		filter: FunnelIcon,
		button: MousePointerClickIcon,
		text: TypeIcon
	};
	const Icon = $derived(icons[type]);
</script>

<Icon {...rest} />
