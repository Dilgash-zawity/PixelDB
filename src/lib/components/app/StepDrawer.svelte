<script lang="ts">
	// Types
	import type { Component, Snippet } from 'svelte';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Drawer from '#lib/components/ui/drawer/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import ConfirmHost from './ConfirmHost.svelte';
	import Stepper from './Stepper.svelte';

	// Icons
	import { ArrowLeft as ArrowLeftIcon, ArrowRight as ArrowRightIcon } from '@lucide/svelte';

	// Script
	/**
	 * The one drawer shape of the app: full screen, from the bottom, one step at a time.
	 * Next walks the steps (unless `onNext` says no); the last step's button calls `onFinish`.
	 */
	let {
		open = $bindable(false),
		step = $bindable(0),
		title,
		description,
		steps,
		finishLabel,
		finishIcon,
		nextLabel = 'Next',
		busy = false,
		/** Disables the main button (Next or the finish button). */
		disabled = false,
		/** Called on Next; return false to stay on this step. */
		onNext,
		onFinish,
		/** Renders a scoped ConfirmHost so confirms opened from inside show on top. */
		scope,
		/** Extra buttons on the footer's left, after Back/Cancel (e.g. Delete). */
		actions,
		/** Extra buttons just before the main button (e.g. "Save & add another"). */
		secondary,
		onkeydown,
		nested = false,
		children
	}: {
		open?: boolean;
		step?: number;
		title: string;
		description?: string;
		steps: { label: string; icon: Component }[];
		finishLabel: string;
		finishIcon?: Component;
		nextLabel?: string;
		busy?: boolean;
		disabled?: boolean;
		onNext?: (step: number) => boolean | void;
		onFinish: () => void;
		scope?: string;
		actions?: Snippet;
		secondary?: Snippet;
		onkeydown?: (e: KeyboardEvent) => void;
		/** Opens on top of another drawer, which it is rendered inside. */
		nested?: boolean;
		children: Snippet;
	} = $props();

	const last = $derived(step >= steps.length - 1);
	const Root = $derived(nested ? Drawer.NestedRoot : Drawer.Root);

	function submit(e: SubmitEvent) {
		e.preventDefault();
		if (busy || disabled) return;
		if (last) return onFinish();
		if (onNext?.(step) === false) return;
		step++;
	}
</script>

<Root bind:open>
	<Drawer.Content
		class="h-dvh data-[vaul-drawer-direction=bottom]:mt-0 data-[vaul-drawer-direction=bottom]:max-h-dvh"
		{onkeydown}
	>
		<form class="mx-auto flex min-h-0 w-full max-w-3xl flex-1 flex-col" onsubmit={submit}>
			<Drawer.Header class="gap-5">
				<div class="flex flex-col gap-1">
					<Drawer.Title>{title}</Drawer.Title>
					{#if description}<Drawer.Description>{description}</Drawer.Description>{/if}
				</div>
				<Stepper {steps} current={step} onjump={(i) => (step = i)} />
			</Drawer.Header>

			<div class="min-h-0 flex-1 overflow-y-auto px-4 pb-2">
				{@render children()}
			</div>

			<Drawer.Footer class="flex-row items-center gap-2">
				{#if step > 0}
					<Button variant="outline" onclick={() => step--}><ArrowLeftIcon data-icon="inline-start" /> Back</Button>
				{:else}
					<Drawer.Close>
						{#snippet child({ props })}<Button {...props} variant="outline">Cancel</Button>{/snippet}
					</Drawer.Close>
				{/if}
				{@render actions?.()}
				<span class="flex-1"></span>
				{#if last}{@render secondary?.()}{/if}
				<Button type="submit" disabled={busy || disabled}>
					{#if last}
						{#if busy}
							<Spinner data-icon="inline-start" />
						{:else if finishIcon}
							{@const Icon = finishIcon}
							<Icon data-icon="inline-start" />
						{/if}
						{finishLabel}
					{:else}
						{nextLabel} <ArrowRightIcon data-icon="inline-end" />
					{/if}
				</Button>
			</Drawer.Footer>
		</form>
		{#if scope}<ConfirmHost {scope} />{/if}
	</Drawer.Content>
</Root>
