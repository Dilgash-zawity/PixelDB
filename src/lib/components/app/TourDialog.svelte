<script lang="ts">
	// Utils
	import { t } from '#lib/i18n/index.js';
	import { shortcutText } from '#lib/logic/shortcuts.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Dialog from '#lib/components/ui/dialog/index.js';

	// Icons
	import {
		AppWindow as AppWindowIcon,
		Network as NetworkIcon,
		Rows3 as Rows3Icon,
		SearchCode as SearchCodeIcon,
		Sparkles as SparklesIcon
	} from '@lucide/svelte';

	// Script
	/** A short tour of how SlateDB fits together, a page per idea. Closing it marks it seen. */
	let { open = $bindable(false) }: { open?: boolean } = $props();

	const PARAMS = { undo: shortcutText('mod+z'), search: shortcutText('mod+k') };
	const PAGES = [
		{
			icon: SparklesIcon,
			tone: 'vivid:bg-tone-amber/15 vivid:text-tone-amber',
			title: t('tour.welcome.title'),
			text: t('tour.welcome.text', PARAMS)
		},
		{
			icon: NetworkIcon,
			tone: 'vivid:bg-tone-violet/15 vivid:text-tone-violet',
			title: t('tour.diagram.title'),
			text: t('tour.diagram.text', PARAMS)
		},
		{
			icon: Rows3Icon,
			tone: 'vivid:bg-tone-blue/15 vivid:text-tone-blue',
			title: t('tour.datasheet.title'),
			text: t('tour.datasheet.text', PARAMS)
		},
		{
			icon: SearchCodeIcon,
			tone: 'vivid:bg-tone-cyan/15 vivid:text-tone-cyan',
			title: t('tour.query.title'),
			text: t('tour.query.text', PARAMS)
		},
		{
			icon: AppWindowIcon,
			tone: 'vivid:bg-tone-green/15 vivid:text-tone-green',
			title: t('tour.apps.title'),
			text: t('tour.apps.text', PARAMS)
		}
	];

	let page = $state(0);
	const last = $derived(page === PAGES.length - 1);
	const current = $derived(PAGES[page]);

	$effect(() => {
		if (open) page = 0;
	});

	function close(next: boolean) {
		open = next;
		if (!next && !settings.current.tourSeen) settings.update({ tourSeen: true });
	}
</script>

<Dialog.Root bind:open={() => open, close}>
	<Dialog.Content class="sm:max-w-md">
		<div class="flex flex-col items-center gap-4 pt-2 text-center">
			<span class={cn('flex size-14 items-center justify-center rounded-xl bg-muted text-muted-foreground', current.tone)}>
				<current.icon class="size-7" />
			</span>
			<Dialog.Header class="items-center">
				<Dialog.Title>{current.title}</Dialog.Title>
				<Dialog.Description class="text-balance">{current.text}</Dialog.Description>
			</Dialog.Header>
			<div class="flex gap-1.5" aria-label="Page {page + 1} of {PAGES.length}">
				{#each PAGES as p, i (p.title)}
					<button
						type="button"
						class={cn('size-2 rounded-full bg-border transition-colors', i === page && 'bg-primary')}
						aria-label="Go to page {i + 1}"
						aria-current={i === page ? 'step' : undefined}
						onclick={() => (page = i)}
					></button>
				{/each}
			</div>
		</div>
		<Dialog.Footer class="flex-row items-center gap-2 sm:justify-between">
			<Button variant="ghost" onclick={() => close(false)}>{last ? t('common.done') : t('tour.skip')}</Button>
			<div class="flex gap-2">
				{#if page > 0}<Button variant="outline" onclick={() => page--}>{t('common.back')}</Button>{/if}
				{#if last}
					<Button onclick={() => close(false)}>{t('tour.start')}</Button>
				{:else}
					<Button onclick={() => page++}>{t('common.next')}</Button>
				{/if}
			</div>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
