<script lang="ts">
	import { DateFormatter, getLocalTimeZone, parseDate, type DateValue } from '@internationalized/date';
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import XIcon from '@lucide/svelte/icons/x';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Calendar } from '#lib/components/ui/calendar/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Popover from '#lib/components/ui/popover/index.js';

	/** Stored as ISO 8601: `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`. */
	let {
		id,
		value,
		invalid = false,
		compact = false,
		onchange,
		...rest
	}: {
		id?: string;
		value: string | null;
		invalid?: boolean;
		/** Datasheet cells: date button only (an existing time is kept). */
		compact?: boolean;
		onchange: (value: string | null) => void;
		[key: `data-${string}`]: unknown;
	} = $props();

	const df = new DateFormatter(navigator.language, { dateStyle: 'medium' });
	let open = $state(false);

	const datePart = $derived(value?.slice(0, 10) ?? '');
	const timePart = $derived(value && value.length > 10 ? value.slice(11, 16) : '');
	const date = $derived.by((): DateValue | undefined => {
		try {
			return datePart ? parseDate(datePart) : undefined;
		} catch {
			return undefined;
		}
	});

	function emit(d: string, t: string) {
		onchange(d ? (t ? `${d}T${t}` : d) : null);
	}
</script>

<div class="flex gap-2">
	<Popover.Root bind:open>
		<Popover.Trigger {id}>
			{#snippet child({ props })}
				<Button
					{...props}
					{...rest}
					variant="outline"
					size={compact ? 'sm' : 'default'}
					class="flex-1 justify-start font-normal"
					aria-invalid={invalid || undefined}
				>
					<CalendarIcon data-icon="inline-start" />
					{#if date}{df.format(date.toDate(getLocalTimeZone()))}{:else if value}{value}{:else}<span class="text-muted-foreground">Pick a date</span>{/if}
				</Button>
			{/snippet}
		</Popover.Trigger>
		<Popover.Content class="w-auto p-0" align="start">
			<Calendar
				type="single"
				value={date}
				captionLayout="dropdown"
				onValueChange={(v) => {
					emit(v ? v.toString() : '', timePart);
					open = false;
				}}
			/>
		</Popover.Content>
	</Popover.Root>
	{#if !compact}
	<Input
		type="time"
		class="w-32"
		aria-label="Time (optional)"
		disabled={!datePart}
		value={timePart}
		oninput={(e) => emit(datePart, e.currentTarget.value)}
	/>
	{#if value}
		<Button variant="ghost" size="icon" aria-label="Clear date" onclick={() => onchange(null)}><XIcon /></Button>
	{/if}
	{/if}
</div>
