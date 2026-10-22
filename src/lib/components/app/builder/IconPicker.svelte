<script lang="ts">
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Popover from '#lib/components/ui/popover/index.js';
	import { ICONS } from '#lib/logic/app-def.js';
	import { cn } from '#lib/utils.js';
	import AppIcon from './AppIcon.svelte';

	let { value, label = 'Icon', onchange }: { value: string; label?: string; onchange: (icon: string) => void } = $props();

	let open = $state(false);
</script>

<Popover.Root bind:open>
	<Popover.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="outline" size="icon-sm" aria-label={label} title={label}><AppIcon name={value} /></Button>
		{/snippet}
	</Popover.Trigger>
	<Popover.Content class="w-64 p-2" align="start">
		<div class="grid grid-cols-7 gap-1" role="listbox" aria-label={label}>
			{#each ICONS as icon (icon)}
				<Button
					variant="ghost"
					size="icon-sm"
					role="option"
					aria-selected={icon === value}
					aria-label={icon}
					title={icon}
					class={cn(icon === value && 'bg-accent')}
					onclick={() => {
						onchange(icon);
						open = false;
					}}
				>
					<AppIcon name={icon} />
				</Button>
			{/each}
		</div>
	</Popover.Content>
</Popover.Root>
