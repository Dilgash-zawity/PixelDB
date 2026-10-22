<script lang="ts">
	// Types
	import type { Component } from 'svelte';

	// Utils
	import { cn } from '#lib/utils.js';

	// Icons
	import { Check as CheckIcon } from '@lucide/svelte';

	// Script
	/** The steps of a wizard as icons on a line; finished steps can be clicked to go back. */
	let {
		steps,
		current,
		onjump
	}: {
		steps: { label: string; icon: Component }[];
		current: number;
		onjump?: (index: number) => void;
	} = $props();
</script>

<ol class="flex items-start" aria-label="Steps">
	{#each steps as step, i (step.label)}
		{@const done = i < current}
		{@const active = i === current}
		{#if i > 0}
			<li class={cn('mt-4.5 h-0.5 flex-1 rounded-full', i <= current ? 'bg-primary' : 'bg-border')} aria-hidden="true"></li>
		{/if}
		<li class="flex w-16 flex-col items-center gap-1.5" aria-current={active ? 'step' : undefined}>
			<button
				type="button"
				class={cn(
					'flex size-9 shrink-0 items-center justify-center rounded-full border-2 transition-colors outline-none focus-visible:ring-3 focus-visible:ring-ring/40',
					done && 'border-primary bg-primary text-primary-foreground',
					active && 'border-primary bg-primary/10 text-primary',
					!done && !active && 'border-border bg-muted text-muted-foreground'
				)}
				disabled={!done}
				aria-label={`Step ${i + 1}: ${step.label}`}
				onclick={() => onjump?.(i)}
			>
				{#if done}<CheckIcon class="size-4" />{:else}<step.icon class="size-4" />{/if}
			</button>
			<span class={cn('text-center text-xs', active ? 'font-medium text-foreground' : 'text-muted-foreground')}>{step.label}</span>
		</li>
	{/each}
</ol>
