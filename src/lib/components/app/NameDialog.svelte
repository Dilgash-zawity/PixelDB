<script lang="ts">
	// Components
	import * as Dialog from '#lib/components/ui/dialog/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';

	// Script
	let {
		open = $bindable(false),
		title,
		description,
		label = 'Name',
		initial = '',
		placeholder = '',
		submitLabel = 'Save',
		validate,
		onSubmit
	}: {
		open?: boolean;
		title: string;
		description?: string;
		label?: string;
		initial?: string;
		placeholder?: string;
		submitLabel?: string;
		validate?: (value: string) => string | null;
		/** Returns true when the dialog should close. */
		onSubmit: (value: string) => Promise<boolean>;
	} = $props();

	let value = $state('');
	let touched = $state(false);
	let busy = $state(false);
	let input = $state<HTMLInputElement | null>(null);

	// Reset each time the dialog opens.
	$effect(() => {
		if (open) {
			value = initial;
			touched = false;
		}
	});

	const problem = $derived(validate?.(value) ?? null);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		touched = true;
		if (problem || busy) return;
		busy = true;
		try {
			if (await onSubmit(value)) open = false;
		} finally {
			busy = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<!-- Select the suggested name so typing replaces it. -->
	<Dialog.Content
		class="sm:max-w-md"
		onOpenAutoFocus={(e) => {
			e.preventDefault();
			input?.select();
		}}
	>
		<form onsubmit={submit} class="flex flex-col gap-6">
			<Dialog.Header>
				<Dialog.Title>{title}</Dialog.Title>
				{#if description}<Dialog.Description>{description}</Dialog.Description>{/if}
			</Dialog.Header>
			<Field.Group>
				<Field.Field data-invalid={touched && !!problem ? true : undefined}>
					<Field.Label for="name-dialog-input">{label}</Field.Label>
					<Input
						id="name-dialog-input"
						bind:ref={input}
						bind:value
						{placeholder}
						autocomplete="off"
						aria-invalid={touched && !!problem ? true : undefined}
						oninput={() => (touched = true)}
					/>
					{#if touched && problem}<Field.Error>{problem}</Field.Error>{/if}
				</Field.Field>
			</Field.Group>
			<Dialog.Footer>
				<Button type="button" variant="outline" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={busy}>
					{#if busy}<Spinner data-icon="inline-start" />{/if}
					{submitLabel}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
