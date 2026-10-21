<script lang="ts" module>
	import type { Component } from 'svelte';
	import type { FieldTypeName } from '#lib/api/types.js';
	import TypeIcon from '@lucide/svelte/icons/type';
	import AlignLeftIcon from '@lucide/svelte/icons/align-left';
	import HashIcon from '@lucide/svelte/icons/hash';
	import SigmaIcon from '@lucide/svelte/icons/sigma';
	import CoinsIcon from '@lucide/svelte/icons/coins';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import SquareCheckIcon from '@lucide/svelte/icons/square-check';
	import PaperclipIcon from '@lucide/svelte/icons/paperclip';
	import LinkIcon from '@lucide/svelte/icons/link';
	import CalculatorIcon from '@lucide/svelte/icons/calculator';
	import WaypointsIcon from '@lucide/svelte/icons/waypoints';
	import CircleHelpIcon from '@lucide/svelte/icons/circle-help';

	export const FIELD_TYPE_ICONS: Record<FieldTypeName, Component> = {
		short_text: TypeIcon,
		long_text: AlignLeftIcon,
		number: HashIcon,
		large_number: SigmaIcon,
		currency: CoinsIcon,
		auto_number: KeyRoundIcon,
		date_time: CalendarIcon,
		yes_no: SquareCheckIcon,
		attachment: PaperclipIcon,
		hyperlink: LinkIcon,
		calculated: CalculatorIcon,
		lookup: WaypointsIcon,
		unknown: CircleHelpIcon
	};
</script>

<script lang="ts">
	let { type, ...rest }: { type: FieldTypeName } & Record<string, unknown> = $props();
	const Icon = $derived(FIELD_TYPE_ICONS[type] ?? CircleHelpIcon);
</script>

<Icon {...rest} />
