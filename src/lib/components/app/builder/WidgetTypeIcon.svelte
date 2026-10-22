<script lang="ts">
	// Types
	import type { WidgetType } from '#lib/logic/app-def.js';

	// Icons
	import {
		ChartColumn as ChartColumnIcon,
		FileText as FileTextIcon,
		Funnel as FunnelIcon,
		Hash as HashIcon,
		Code as CodeIcon,
		MousePointerClick as MousePointerClickIcon,
		Table2 as TableIcon,
		Type as TypeIcon,
	} from '@lucide/svelte';

	// Script
	let { type, ...rest }: { type: WidgetType; class?: string; 'data-icon'?: string } = $props();

	const icons = {
		table: TableIcon,
		form: FileTextIcon,
		chart: ChartColumnIcon,
		stat: HashIcon,
		filter: FunnelIcon,
		button: MousePointerClickIcon,
		text: TypeIcon,
		query: CodeIcon
	};
	const Icon = $derived(icons[type]);
</script>

<Icon {...rest} />
