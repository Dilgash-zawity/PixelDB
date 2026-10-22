<script lang="ts" module>
	// Types
	import type { Component } from 'svelte';
	import type { FieldTypeName } from '#lib/api/types.js';

	// Icons
	import {
		Type as TypeIcon,
		AlignLeft as AlignLeftIcon,
		Hash as HashIcon,
		Sigma as SigmaIcon,
		Coins as CoinsIcon,
		KeyRound as KeyRoundIcon,
		Calendar as CalendarIcon,
		SquareCheck as SquareCheckIcon,
		Paperclip as PaperclipIcon,
		Link as LinkIcon,
		Calculator as CalculatorIcon,
		Waypoints as WaypointsIcon,
		CircleHelp as CircleHelpIcon,
	} from '@lucide/svelte';

	// Script
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
	// Script
	let { type, ...rest }: { type: FieldTypeName } & Record<string, unknown> = $props();
	const Icon = $derived(FIELD_TYPE_ICONS[type] ?? CircleHelpIcon);
</script>

<Icon {...rest} />
