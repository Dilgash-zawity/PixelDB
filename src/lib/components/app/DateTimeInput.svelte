<script lang="ts">
	// Utils
	import { DateFormatter, getLocalTimeZone, parseDate, today, type DateValue } from '@internationalized/date';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import { Calendar } from '#lib/components/ui/calendar/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import * as Popover from '#lib/components/ui/popover/index.js';

	// Icons
	import { Calendar as CalendarIcon, X as XIcon } from '@lucide/svelte';

	// Script
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

	/** Today's date (where the user is), keeping a time already set. */
	function setToday() {
		emit(today(getLocalTimeZone()).toString(), timePart);
		open = false;
	}
	const isToday = $derived(datePart === today(getLocalTimeZone()).toString());
</script>

<!-- Wraps in narrow forms: the date takes its own line, Today, time and clear go under it. -->
<div class={compact ? 'flex gap-2' : 'flex flex-wrap gap-2'}>
	<Popover.Root bind:open>
		<Popover.Trigger {id}>
			{#snippet child({ props })}
				<Button
					{...props}
					{...rest}
					variant="outline"
					size={compact ? 'sm' : 'default'}
					class={compact ? 'flex-1 justify-start font-normal' : 'min-w-40 flex-1 justify-start font-normal'}
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
			<div class="border-t p-2">
				<Button variant="ghost" size="sm" class="w-full" disabled={isToday} onclick={setToday}>Today</Button>
			</div>
		</Popover.Content>
	</Popover.Root>
	{#if !compact}
	<Button variant="outline" disabled={isToday} onclick={setToday}>Today</Button>
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
