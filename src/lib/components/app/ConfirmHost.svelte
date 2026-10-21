<script lang="ts">
	import * as AlertDialog from '#lib/components/ui/alert-dialog/index.js';
	import { buttonVariants } from '#lib/components/ui/button/index.js';
	import { confirmation } from '#lib/state/feedback.svelte.js';

	let { scope }: { scope?: string } = $props();
	const mine = $derived(confirmation.options.scope === scope);
</script>

<AlertDialog.Root bind:open={() => confirmation.open && mine, (open) => !open && mine && confirmation.settle(false)}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{confirmation.options.title}</AlertDialog.Title>
			<AlertDialog.Description>{confirmation.options.description}</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel onclick={() => confirmation.settle(false)}>Cancel</AlertDialog.Cancel>
			<AlertDialog.Action
				class={buttonVariants({ variant: confirmation.options.destructive ? 'destructive' : 'default' })}
				onclick={() => confirmation.settle(true)}
			>
				{confirmation.options.action ?? 'Continue'}
			</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
