<script lang="ts">
	import { t } from '#lib/i18n/index.js';
	// Utils
	import { getVersion } from '@tauri-apps/api/app';
	import { isTauri } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { resetMode, setMode, userPrefersMode } from 'mode-watcher';
	import { ACCENTS, RADII, TEXT_SIZES, type Settings } from '#lib/logic/settings.js';
	import { nextRow, typingTarget } from '#lib/logic/shortcuts.js';
	import { homeLayoutKey } from '#lib/logic/workspace-map.js';
	import { diagnosticsService } from '#lib/services/diagnostics.js';
	import { attempt, confirm, diagnostics } from '#lib/state/feedback.svelte.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { updates } from '#lib/state/updates.svelte.js';
	import { tour } from '#lib/state/tour.svelte.js';
	import { cn } from '#lib/utils.js';
	import { toast } from 'svelte-sonner';

	// Components
	import Logo from '#lib/components/app/Logo.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Bug as BugIcon,
		CalendarClock as CalendarClockIcon,
		Check as CheckIcon,
		ChevronRight as ChevronRightIcon,
		CircleCheck as CircleCheckIcon,
		Copy as CopyIcon,
		Database as DatabaseIcon,
		Download as DownloadIcon,
		Droplet as DropletIcon,
		ExternalLink as ExternalLinkIcon,
		FileText as FileTextIcon,
		GraduationCap as GraduationCapIcon,
		Heart as HeartIcon,
		LayoutDashboard as LayoutDashboardIcon,
		Lightbulb as LightbulbIcon,
		Map as MapIcon,
		Package as PackageIcon,
		Scale as ScaleIcon,
		ShieldCheck as ShieldCheckIcon,
		Sparkles as SparklesIcon,
		Square as SquareIcon,
		TriangleAlert as TriangleAlertIcon,
		Type as TypeIcon,
		Wind as WindIcon,
		FolderOpen as FolderOpenIcon,
		Info as InfoIcon,
		Palette as PaletteIcon,
		RotateCcw as RotateCcwIcon,
		RefreshCw as RefreshCwIcon,
		SlidersHorizontal as SlidersHorizontalIcon,
	} from '@lucide/svelte';

	// Script
	const WEBSITE = 'https://dilgash.com';
	const RELEASES = 'https://github.com/Dilgash-zawity/PixelDB/releases/latest';
	const native = isTauri();
	const updateBusy = $derived(['checking', 'confirming', 'downloading', 'installing'].includes(updates.current.status));

	async function copySystemInfo() {
		const d = await diagnostics();
		if (!d) return;
		const text = `PixelDB ${d.version}\nSystem: ${d.os} ${d.arch}\nSQLite ${d.sqlite}`;
		try {
			await navigator.clipboard.writeText(text);
			toast.success('Copied system info');
		} catch {
			toast.info('System info', { description: text });
		}
	}

	async function installUpdate() {
		await updates.manager.install(() => confirm({
			title: `Install PixelDB ${updates.current.version}?`,
			description: 'Finish and save your work first. PixelDB will verify the download before installing. Windows closes and restarts during installation; on Mac and Linux you can restart when installation finishes.',
			action: 'Download and install'
		}));
	}

	let version = $state<string | null>(null);
	// Read from the running binary, so it always matches the installed build.
	getVersion().then((v) => (version = v), () => {});

	if (!workspace.loaded) workspace.load();

	const SECTIONS = [
		{ id: 'workspace', label: t('settings.workspace.label'), hint: t('settings.workspace.hint'), tone: 'vivid:bg-tone-amber/15 vivid:text-tone-amber', icon: FolderOpenIcon, description: t('settings.workspace.description') },
		{ id: 'appearance', label: t('settings.appearance.label'), hint: t('settings.appearance.hint'), tone: 'vivid:bg-tone-pink/15 vivid:text-tone-pink', icon: PaletteIcon, description: t('settings.appearance.description') },
		{ id: 'behaviour', label: t('settings.behaviour.label'), hint: t('settings.behaviour.hint'), tone: 'vivid:bg-tone-violet/15 vivid:text-tone-violet', icon: SlidersHorizontalIcon, description: t('settings.behaviour.description') },
		{ id: 'updates', label: t('settings.updates.label'), hint: t('settings.updates.hint'), tone: 'vivid:bg-tone-green/15 vivid:text-tone-green', icon: RefreshCwIcon, description: t('settings.updates.description') },
		{ id: 'about', label: t('settings.about.label'), hint: t('settings.about.hint'), tone: 'vivid:bg-tone-blue/15 vivid:text-tone-blue', icon: InfoIcon, description: t('settings.about.description') }
	] as const;
	type SectionId = (typeof SECTIONS)[number]['id'];
	/** The page shown, from `?section=` (links like "View update" open one directly). */
	const current = $derived(SECTIONS.find((x) => x.id === page.url.searchParams.get('section')) ?? SECTIONS[0]);

	let scroller = $state<HTMLElement | null>(null);

	/** Switches pages in place, so Back still leaves Settings. */
	function show(id: SectionId) {
		goto(`${resolve('/settings')}?section=${id}`, { replace: true });
		if (scroller) scroller.scrollTop = 0;
	}

	/** Escape goes back, unless a dialog or menu is open to take it. */
	function onKey(e: KeyboardEvent) {
		if (e.key !== 'Escape' || e.defaultPrevented || typingTarget(e.target)) return;
		if (document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"]')) return;
		history.back();
	}

	/** Up and Down walk the sections while the menu has focus. */
	function onNavKey(e: KeyboardEvent) {
		const i = SECTIONS.findIndex((x) => x.id === current.id);
		const next = nextRow(e.key, i, SECTIONS.length);
		if (next === null || next === i) return;
		e.preventDefault();
		show(SECTIONS[next].id);
		(e.currentTarget as HTMLElement).querySelectorAll('button')[next]?.focus();
	}

	function chooseTheme(value: 'light' | 'dark' | 'system') {
		if (value === 'system') resetMode();
		else setMode(value);
	}

	const SWATCH = $derived<Record<Settings['accent'], string>>({
		neutral: settings.current.focusMode ? 'var(--foreground)' : 'oklch(0.16 0 0)',
		blue: 'oklch(0.55 0.2 255)',
		green: 'oklch(0.56 0.15 150)',
		amber: 'oklch(0.75 0.16 75)',
		red: 'oklch(0.58 0.22 25)',
		violet: 'oklch(0.55 0.22 295)'
	});
	const RADIUS_PX = $derived<Record<Settings['radius'], string>>({ auto: settings.current.focusMode ? '0' : '8px', none: '0', small: '6px', medium: '10px', large: '14px' });
	const label = (v: string) => (v === 'neutral' ? 'Default' : v[0].toUpperCase() + v.slice(1));

	async function resetLayout() {
		if (!workspace.info) return;
		try {
			localStorage.removeItem(homeLayoutKey(workspace.info.path));
		} catch {
			// Nothing stored, nothing to reset.
		}
		toast.success('Home map layout reset');
	}

	async function resetAll() {
		const ok = await confirm({
			title: 'Reset all settings?',
			description: 'Focus mode, accent color, corners, text size and behaviour go back to their defaults. Your folder, databases and apps are not affected.',
			action: 'Reset settings',
			destructive: true
		});
		if (!ok) return;
		settings.reset();
		resetMode();
		setMode('dark');
	}

</script>

<svelte:head><title>{current.label} · Settings · PixelDB</title></svelte:head>

<!-- One setting: an icon, what it does, and its control on the right. -->
<!-- `wide` controls (button groups) go under the text when there isn't room beside it. -->
{#snippet row(title: string, description: string, control: import('svelte').Snippet, id?: string, wide = false, icon?: typeof InfoIcon)}
	<Field.Field orientation={wide ? 'responsive' : 'horizontal'} class="gap-4 px-5 py-4">
		<div class="flex min-w-0 flex-1 gap-3.5">
			{#if icon}
				{@const Icon = icon}
				<span class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"><Icon class="size-4" /></span>
			{/if}
			<Field.Content>
				{#if id}<Field.Label for={id}>{title}</Field.Label>{:else}<Field.Title>{title}</Field.Title>{/if}
				<Field.Description>{description}</Field.Description>
			</Field.Content>
		</div>
		{@render control()}
	</Field.Field>
{/snippet}

<!-- A titled card of rows. -->
{#snippet group(title: string, body: import('svelte').Snippet, tone = '')}
	<section class="flex flex-col gap-2.5">
		<h2 class="px-1 text-xs font-semibold tracking-wider text-muted-foreground uppercase">{title}</h2>
		<div class={cn('flex flex-col overflow-hidden rounded-xl border bg-card', tone)}>
			{@render body()}
		</div>
	</section>
{/snippet}

<!-- A small window drawn in fixed colors, previewing a theme or look. -->
{#snippet mock(bg: string, bar: string, line: string, accent: string, radius: string)}
	<span class="flex h-full w-full flex-col gap-1.5 p-2" style:background={bg}>
		<span class="h-2 w-full" style:background={bar} style:border-radius={radius}></span>
		<span class="flex flex-1 gap-1.5">
			<span class="w-1/4" style:background={line} style:border-radius={radius}></span>
			<span class="flex flex-1 flex-col gap-1">
				<span class="h-1.5 w-3/4" style:background={line} style:border-radius={radius}></span>
				<span class="h-1.5 w-1/2" style:background={line} style:border-radius={radius}></span>
				<span class="mt-auto h-2.5 w-8" style:background={accent} style:border-radius={radius}></span>
			</span>
		</span>
	</span>
{/snippet}

<!-- A picture card in a radio group. -->
{#snippet choice(selected: boolean, labelText: string, hint: string, onpick: () => void, picture: import('svelte').Snippet)}
	<button
		type="button"
		role="radio"
		aria-checked={selected}
		class={cn(
			'group/choice flex flex-col gap-2 rounded-xl border bg-background p-2 text-start outline-none transition-colors hover:border-foreground/30 focus-visible:ring-3 focus-visible:ring-ring/40',
			selected && 'border-primary ring-1 ring-primary vivid:border-(--map-accent) vivid:ring-(--map-accent)'
		)}
		onclick={onpick}
	>
		<span class="relative flex h-20 overflow-hidden rounded-lg border">
			{@render picture()}
			{#if selected}
				<span class="absolute end-1.5 top-1.5 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground vivid:bg-(--map-accent) vivid:text-white">
					<CheckIcon class="size-3" />
				</span>
			{/if}
		</span>
		<span class="flex flex-col px-1 pb-0.5">
			<span class="text-sm font-medium">{labelText}</span>
			<span class="text-xs text-muted-foreground">{hint}</span>
		</span>
	</button>
{/snippet}

<svelte:window onkeydown={onKey} />

<div class="flex h-svh flex-col">
	<header data-bar class="app-header flex h-12 shrink-0 items-center gap-2 border-b bg-sidebar px-3">
		<Button variant="ghost" size="sm" title="Back (Esc)" onclick={() => history.back()}>
			<ArrowLeftIcon data-icon="inline-start" /> Back
		</Button>
		<nav aria-label="Breadcrumb" class="flex items-center gap-1.5 text-sm">
			<span class="text-muted-foreground">{t('settings.title')}</span>
			<ChevronRightIcon class="size-3.5 text-muted-foreground" aria-hidden="true" />
			<span class="font-semibold" aria-current="page">{current.label}</span>
		</nav>
	</header>

	<div class="flex min-h-0 flex-1">
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions (arrow keys for the buttons inside) -->
		<nav class="flex w-64 shrink-0 flex-col gap-4 border-e bg-sidebar px-3 py-6 sm:w-72 sm:px-4" aria-label="Settings" onkeydown={onNavKey}>
			<div class="flex flex-col gap-0.5 px-2">
				<h2 class="text-lg font-semibold tracking-tight">{t('settings.title')}</h2>
				<p class="text-xs text-muted-foreground">{t('settings.subtitle')}</p>
			</div>
			<div class="flex flex-col gap-1">
				{#each SECTIONS as s (s.id)}
					{@const active = current.id === s.id}
					<button
						type="button"
						class={cn(
							'group/item relative flex items-center gap-3 rounded-lg border border-transparent px-2.5 py-2.5 text-start outline-none transition-colors hover:bg-sidebar-accent focus-visible:ring-3 focus-visible:ring-ring/40',
							active && 'border-border bg-background shadow-xs hover:bg-background'
						)}
						aria-current={active ? 'page' : undefined}
						onclick={() => show(s.id)}
					>
						{#if active}<span class="absolute inset-y-2.5 -start-px w-0.5 rounded-full bg-primary vivid:bg-(--map-accent)" aria-hidden="true"></span>{/if}
						<span class={cn('flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground', s.tone)}>
							<s.icon class="size-4.5" />
						</span>
						<span class="flex min-w-0 flex-1 flex-col">
							<span class={cn('truncate text-sm', active ? 'font-semibold' : 'font-medium')}>{s.label}</span>
							<span class="truncate text-xs text-muted-foreground">{s.hint}</span>
						</span>
						{#if s.id === 'updates' && ['available', 'ready'].includes(updates.current.status)}
							<span class="size-2 shrink-0 rounded-full bg-success" title={t('settings.updateReady')}></span>
						{/if}
					</button>
				{/each}
			</div>
			<div class="mt-auto flex items-center gap-2.5 px-2 text-xs text-muted-foreground">
				<Logo class="size-6" />
				<span class="flex flex-col">
					<span class="font-medium text-foreground">PixelDB</span>
					<span>{version ? `Version ${version}` : '\u00a0'}</span>
				</span>
			</div>
		</nav>

		<main bind:this={scroller} class="min-h-0 flex-1 overflow-y-auto">
			<div class="mx-auto flex max-w-3xl flex-col gap-8 px-6 py-10 sm:px-10">
				<div class="flex items-center gap-4">
					<span class={cn('flex size-14 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground', current.tone)}>
						<current.icon class="size-7" />
					</span>
					<div class="flex min-w-0 flex-col gap-1">
						<h1 class="text-2xl font-semibold tracking-tight">{current.label}</h1>
						<p class="text-sm text-muted-foreground">{current.description}</p>
					</div>
				</div>

				{#if current.id === 'workspace'}
					{#snippet folderCard()}
						<div class="flex flex-col gap-4 px-5 py-5">
							<div class="flex items-start gap-3.5">
								<span class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground vivid:bg-tone-amber/15 vivid:text-tone-amber"><FolderOpenIcon class="size-5" /></span>
								<div class="flex min-w-0 flex-1 flex-col gap-1">
									<span class="text-sm font-medium">Root folder</span>
									{#if workspace.info}
										<code class="truncate rounded-md bg-muted px-2 py-1 text-xs" title={workspace.info.path}>{workspace.info.path}</code>
									{:else}
										<span class="text-sm text-muted-foreground">No folder chosen yet.</span>
									{/if}
								</div>
								<Button variant="outline" onclick={() => workspace.chooseFolder()}>
									{workspace.info ? 'Change…' : 'Choose folder…'}
								</Button>
							</div>
							{#if workspace.info}
								<div class="grid grid-cols-2 gap-3">
									<div class="flex items-center gap-3 rounded-lg border bg-background px-3.5 py-3">
										<DatabaseIcon class="size-4 text-muted-foreground vivid:text-tone-blue" />
										<span class="flex flex-col">
											<span class="text-lg leading-tight font-semibold tabular-nums">{workspace.databases.length + workspace.online.length}</span>
											<span class="text-xs text-muted-foreground">{workspace.databases.length + workspace.online.length === 1 ? 'database' : 'databases'} in <code>DB</code></span>
										</span>
									</div>
									<div class="flex items-center gap-3 rounded-lg border bg-background px-3.5 py-3">
										<LayoutDashboardIcon class="size-4 text-muted-foreground vivid:text-tone-green" />
										<span class="flex flex-col">
											<span class="text-lg leading-tight font-semibold tabular-nums">{workspace.apps.length}</span>
											<span class="text-xs text-muted-foreground">{workspace.apps.length === 1 ? 'app' : 'apps'} in <code>Apps</code></span>
										</span>
									</div>
								</div>
							{/if}
						</div>
					{/snippet}
					{#snippet homeCard()}
						{#snippet layoutReset()}
							<Button variant="outline" disabled={!workspace.info} onclick={resetLayout}><RotateCcwIcon data-icon="inline-start" /> Reset layout</Button>
						{/snippet}
						{@render row('Card layout', 'Forget where you moved the cards on the home screen and lay them out again.', layoutReset, undefined, false, MapIcon)}
					{/snippet}
					{@render group('Folder', folderCard)}
					{@render group('Home map', homeCard)}
				{:else if current.id === 'appearance'}
					{#snippet themeCard()}
						<div class="grid grid-cols-3 gap-3 p-4" role="radiogroup" aria-label="Theme">
							{#snippet light()}{@render mock('oklch(0.985 0.022 95)', 'oklch(0.16 0 0)', 'oklch(0.9 0.02 90)', 'oklch(0.7 0.18 50)', '3px')}{/snippet}
							{#snippet dark()}{@render mock('oklch(0.17 0.01 90)', 'oklch(0.1 0 0)', 'oklch(0.3 0.01 90)', 'oklch(0.7 0.18 50)', '3px')}{/snippet}
							{#snippet system()}
								<span class="flex w-1/2 overflow-hidden">{@render light()}</span>
								<span class="flex w-1/2 overflow-hidden">{@render dark()}</span>
							{/snippet}
							{@render choice(userPrefersMode.current === 'light', 'Light', 'Bright and clear', () => chooseTheme('light'), light)}
							{@render choice(userPrefersMode.current === 'dark', 'Dark', 'Easy on the eyes', () => chooseTheme('dark'), dark)}
							{@render choice(userPrefersMode.current === 'system', 'System', 'Follow this computer', () => chooseTheme('system'), system)}
						</div>
					{/snippet}
					{#snippet lookCard()}
						<div class="grid grid-cols-2 gap-3 p-4" role="radiogroup" aria-label="Look">
							{#snippet colorful()}{@render mock('oklch(0.985 0.022 95)', 'oklch(0.16 0 0)', 'oklch(0.88 0.04 80)', 'oklch(0.7 0.18 50)', '4px')}{/snippet}
							{#snippet plain()}{@render mock('oklch(0.98 0 0)', 'oklch(0.92 0 0)', 'oklch(0.88 0 0)', 'oklch(0.3 0 0)', '0')}{/snippet}
							{@render choice(!settings.current.focusMode, 'Colorful', 'Paper colors, rounded tiles, an orange highlight', () => settings.update({ focusMode: false }), colorful)}
							{@render choice(settings.current.focusMode, 'Focus mode', 'A calm grey look with square corners', () => settings.update({ focusMode: true }), plain)}
						</div>
					{/snippet}
					{#snippet detailCard()}
						{#snippet accent()}
							<div class="flex items-center gap-3">
								<div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Accent color">
									{#each ACCENTS as a (a)}
										{@const selected = settings.current.accent === a}
										<button
											type="button"
											role="radio"
											aria-checked={selected}
											aria-label={label(a)}
											title={label(a)}
											class={cn(
												'flex size-8 items-center justify-center rounded-full border-2 border-transparent text-white outline-none transition-transform hover:scale-110 focus-visible:ring-3 focus-visible:ring-ring/50',
												selected && 'border-background ring-2 ring-foreground'
											)}
											style:background={SWATCH[a]}
											onclick={() => settings.update({ accent: a })}
										>
											{#if selected}<CheckIcon class={cn('size-4', a === 'neutral' && settings.current.focusMode && 'text-background')} />{/if}
										</button>
									{/each}
								</div>
							</div>
						{/snippet}
						{#snippet corners()}
							<ToggleGroup.Root
								type="single"
								variant="outline"
								size="sm"
								value={settings.current.radius}
								onValueChange={(v) => v && settings.update({ radius: v as Settings['radius'] })}
							>
								{#each RADII as r (r)}
									<ToggleGroup.Item value={r} aria-label={`${label(r)} corners`}>
										<span class="size-3 border-2 border-current" style:border-radius={RADIUS_PX[r]}></span>
										{label(r)}
									</ToggleGroup.Item>
								{/each}
							</ToggleGroup.Root>
						{/snippet}
						{#snippet textSize()}
							<ToggleGroup.Root
								type="single"
								variant="outline"
								size="sm"
								value={settings.current.textSize}
								onValueChange={(v) => v && settings.update({ textSize: v as Settings['textSize'] })}
							>
								{#each TEXT_SIZES as size, i (size)}
									<ToggleGroup.Item value={size}>
										<span class={['text-xs', 'text-sm', 'text-base'][i]}>Aa</span>
										{label(size)}
									</ToggleGroup.Item>
								{/each}
							</ToggleGroup.Root>
						{/snippet}
						{@render row('Accent color', 'Used for main buttons, focus rings and highlighted lines.', accent, undefined, true, DropletIcon)}
						<Separator />
						{@render row('Corners', 'Square like a spreadsheet, or softer.', corners, undefined, true, SquareIcon)}
						<Separator />
						{@render row('Text size', 'Scales text and spacing throughout the app.', textSize, undefined, true, TypeIcon)}
					{/snippet}
					{#snippet previewCard()}
						<div class="flex flex-col gap-4 p-5">
							<div class="flex flex-wrap items-center gap-2">
								<Button size="sm">Save record</Button>
								<Button size="sm" variant="outline">Cancel</Button>
								<Badge variant="secondary">Draft</Badge>
								<Badge class="border-warning/40 bg-warning/10 text-warning" variant="outline">Check before saving</Badge>
							</div>
							<div class="flex items-center gap-3">
								<Input class="max-w-64" placeholder="Search customers…" aria-label="Preview input" />
								<Switch checked aria-label="Preview switch" />
							</div>
						</div>
					{/snippet}
					{@render group('Theme', themeCard)}
					{@render group('Look', lookCard)}
					{@render group('Details', detailCard)}
					{@render group('Preview', previewCard)}
				{:else if current.id === 'behaviour'}
					{#snippet motionCard()}
						{#snippet motion()}
							<Switch id="reduce-motion" checked={settings.current.reduceMotion} onCheckedChange={(v) => settings.update({ reduceMotion: v })} />
						{/snippet}
						{#snippet hints()}
							<Switch id="canvas-hints" checked={settings.current.canvasHints} onCheckedChange={(v) => settings.update({ canvasHints: v })} />
						{/snippet}
						{@render row('Reduce motion', 'Turn off animations, including the moving lines between apps and databases. Your system setting is followed too.', motion, 'reduce-motion', false, WindIcon)}
						<Separator />
						{@render row('Show canvas hints', 'The tips along the bottom of the home map and the relationships diagram.', hints, 'canvas-hints', false, LightbulbIcon)}
					{/snippet}
					{#snippet helpCard()}
						{#snippet tourButton()}
							<Button variant="outline" onclick={() => (tour.open = true)}><GraduationCapIcon data-icon="inline-start" /> Show the tour</Button>
						{/snippet}
						{@render row('Getting started tour', 'A short look at databases, the Diagram, the Datasheet, queries and apps.', tourButton, undefined, false, SparklesIcon)}
					{/snippet}
					{#snippet resetCard()}
						{#snippet reset()}
							<Button variant="destructive" onclick={resetAll}><RotateCcwIcon data-icon="inline-start" /> Reset settings</Button>
						{/snippet}
						{@render row('Reset all settings', "Go back to the dark theme and the default look. Your folder, databases and apps aren't touched.", reset, undefined, false, TriangleAlertIcon)}
					{/snippet}
					{@render group('Motion and hints', motionCard)}
					{@render group('Help', helpCard)}
					{@render group('Start over', resetCard, 'border-destructive/30')}
				{:else if current.id === 'updates'}
					{@const available = !!updates.current.version && updates.current.status !== 'current'}
					{#snippet statusCard()}
						<div class="flex flex-wrap items-center gap-4 px-5 py-5">
							<span
								class={cn(
									'flex size-12 shrink-0 items-center justify-center rounded-xl',
									available ? 'bg-success/15 text-success' : 'bg-muted text-muted-foreground'
								)}
							>
								{#if updateBusy}<Spinner class="size-5" />{:else if available}<DownloadIcon class="size-6" />{:else}<CircleCheckIcon class="size-6" />{/if}
							</span>
							<div class="flex min-w-0 flex-1 flex-col gap-0.5">
								<span class="font-semibold">
									{#if updates.current.status === 'ready'}Ready to restart
									{:else if available}PixelDB {updates.current.version} is available
									{:else}PixelDB {version ?? '—'}{/if}
								</span>
								<span class="text-sm text-muted-foreground" role="status" aria-live="polite">
									{#if !native}Update checks are available in the desktop app.
									{:else if updates.current.status === 'checking'}Checking for updates…
									{:else if updates.current.status === 'downloading'}Downloading update{updates.current.progress === null ? '…' : `… ${updates.current.progress}%`}
									{:else}{updates.current.message || 'Updates are downloaded securely and verified before installation.'}{/if}
								</span>
								{#if updates.current.status === 'downloading' && updates.current.progress !== null}
									<span class="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-muted">
										<span class="block h-full bg-success transition-[width]" style:width="{updates.current.progress}%"></span>
									</span>
								{/if}
							</div>
							<div class="flex flex-wrap gap-2">
								{#if updates.current.status === 'ready'}
									<Button onclick={() => updates.manager.restart()}>Restart PixelDB</Button>
								{:else}
									<Button variant="outline" disabled={!native || updateBusy} onclick={() => updates.manager.check()}><RefreshCwIcon data-icon="inline-start" /> Check now</Button>
									{#if updates.current.version}
										<Button disabled={updateBusy} onclick={installUpdate}>Install {updates.current.version}</Button>
									{/if}
								{/if}
							</div>
						</div>
						{#if updates.current.notes}
							<Separator />
							<details class="group/notes px-5 py-4">
								<summary class="flex cursor-pointer items-center gap-2 text-sm font-medium">
									<ChevronRightIcon class="size-4 transition-transform group-open/notes:rotate-90" /> What’s new
								</summary>
								<p class="mt-3 text-sm whitespace-pre-wrap text-muted-foreground">{updates.current.notes}</p>
							</details>
						{/if}
					{/snippet}
					{#snippet prefCard()}
						{#snippet automatic()}
							<Switch id="automatic-updates" checked={settings.current.automaticUpdates} onCheckedChange={(value) => settings.update({ automaticUpdates: value })} />
						{/snippet}
						{@render row('Check automatically', 'On launch and every six hours while PixelDB is open. You choose when to install. Database contents are never sent.', automatic, 'automatic-updates', false, CalendarClockIcon)}
						<Separator />
						{#snippet downloads()}
							<Button variant="outline" disabled={!native} onclick={() => attempt(() => openUrl(RELEASES))}><ExternalLinkIcon data-icon="inline-start" /> Open</Button>
						{/snippet}
						{@render row('Official downloads', 'Every release with its installers, checksums and notes, on GitHub.', downloads, undefined, false, PackageIcon)}
					{/snippet}
					{@render group('Status', statusCard)}
					{@render group('Preferences', prefCard)}
				{:else}
					{#snippet aboutCard()}
						<div class="flex flex-col items-center gap-3 px-6 py-8 text-center">
							<Logo class="size-16" />
							<div class="flex flex-col items-center gap-1.5">
								<span class="text-xl font-semibold">PixelDB</span>
								{#if version}<Badge variant="secondary">Version {version}</Badge>{/if}
							</div>
							<p class="max-w-md text-sm text-balance text-muted-foreground">An offline desktop database app in the spirit of Microsoft Access. Your data stays on this computer.</p>
							<div class="mt-1 flex flex-wrap justify-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
								<span class="flex items-center gap-1.5"><ShieldCheckIcon class="size-3.5" /> Works offline</span>
								<span class="flex items-center gap-1.5"><ScaleIcon class="size-3.5" /> MIT license</span>
								<span class="flex items-center gap-1.5"><DatabaseIcon class="size-3.5" /> SQLite files</span>
							</div>
						</div>
						<Separator />
						{#snippet author()}
							<Button variant="outline" disabled={!native} onclick={() => attempt(() => openUrl(WEBSITE))}><ExternalLinkIcon data-icon="inline-start" /> dilgash.com</Button>
						{/snippet}
						{@render row('Made by Dilgash', 'Design and development.', author, undefined, false, HeartIcon)}
					{/snippet}
					{#snippet problemCard()}
						{#snippet copyInfo()}
							<Button variant="outline" disabled={!native} onclick={copySystemInfo}><CopyIcon data-icon="inline-start" /> Copy</Button>
						{/snippet}
						{#snippet logs()}
							<Button variant="outline" disabled={!native} onclick={() => attempt(() => diagnosticsService.openLogFolder())}><FolderOpenIcon data-icon="inline-start" /> Open</Button>
						{/snippet}
						{@render row('System info', 'Your PixelDB, system and SQLite versions, to paste into a bug report. Error messages also have a “Copy details” button.', copyInfo, undefined, false, BugIcon)}
						<Separator />
						{@render row('Log folder', 'Unexpected errors are kept in a log file on this computer; it is never sent anywhere.', logs, undefined, false, FileTextIcon)}
					{/snippet}
					{@render group('PixelDB', aboutCard)}
					{@render group('Reporting a problem', problemCard)}
				{/if}
			</div>
		</main>
	</div>
</div>
