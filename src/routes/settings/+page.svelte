<script lang="ts">
	// Utils
	import { getVersion } from '@tauri-apps/api/app';
	import { isTauri } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { resetMode, setMode, userPrefersMode } from 'mode-watcher';
	import { ACCENTS, RADII, TEXT_SIZES, type Settings } from '#lib/logic/settings.js';
	import { homeLayoutKey } from '#lib/logic/workspace-map.js';
	import { attempt, confirm } from '#lib/state/feedback.svelte.js';
	import { settings } from '#lib/state/settings.svelte.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { updates } from '#lib/state/updates.svelte.js';
	import { cn } from '#lib/utils.js';
	import { toast } from 'svelte-sonner';

	// Components
	import Logo from '#lib/components/app/Logo.svelte';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		Check as CheckIcon,
		Database as DatabaseIcon,
		Focus as FocusIcon,
		FolderOpen as FolderOpenIcon,
		Info as InfoIcon,
		LayoutDashboard as LayoutDashboardIcon,
		Monitor as MonitorIcon,
		Moon as MoonIcon,
		Palette as PaletteIcon,
		RotateCcw as RotateCcwIcon,
		RefreshCw as RefreshCwIcon,
		SlidersHorizontal as SlidersHorizontalIcon,
		Sun as SunIcon,
	} from '@lucide/svelte';

	// Script
	const WEBSITE = 'https://dilgash.com';
	const RELEASES = 'https://github.com/Dilgash-zawity/SlateDB/releases/latest';
	const native = isTauri();
	const updateBusy = $derived(['checking', 'confirming', 'downloading', 'installing'].includes(updates.current.status));

	onMount(() => {
		if (location.hash === '#updates') jump('updates');
	});

	async function installUpdate() {
		await updates.manager.install(() => confirm({
			title: `Install SlateDB ${updates.current.version}?`,
			description: 'Finish and save your work first. SlateDB will verify the download before installing. Windows closes and restarts during installation; on Mac and Linux you can restart when installation finishes.',
			action: 'Download and install'
		}));
	}

	let version = $state<string | null>(null);
	// Read from the running binary, so it always matches the installed build.
	getVersion().then((v) => (version = v), () => {});

	if (!workspace.loaded) workspace.load();

	const SECTIONS = [
		{ id: 'workspace', label: 'Workspace', icon: FolderOpenIcon, tone: 'vivid:text-tone-blue' },
		{ id: 'appearance', label: 'Appearance', icon: PaletteIcon, tone: 'vivid:text-tone-pink' },
		{ id: 'behaviour', label: 'Behaviour', icon: SlidersHorizontalIcon, tone: 'vivid:text-tone-green' },
		{ id: 'updates', label: 'Updates', icon: RefreshCwIcon, tone: 'vivid:text-tone-blue' },
		{ id: 'about', label: 'About', icon: InfoIcon, tone: 'vivid:text-tone-amber' }
	];
	let active = $state('workspace');

	const THEMES = [
		{ value: 'light', label: 'Light', icon: SunIcon },
		{ value: 'dark', label: 'Dark', icon: MoonIcon },
		{ value: 'system', label: 'System', icon: MonitorIcon }
	] as const;
	function chooseTheme(value: 'light' | 'dark' | 'system') {
		if (value === 'system') resetMode();
		else setMode(value);
	}

	const SWATCH = $derived<Record<Settings['accent'], string>>({
		neutral: settings.current.focusMode ? 'var(--foreground)' : 'oklch(0.53 0.17 260)',
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

	function jump(id: string) {
		active = id;
		document.getElementById(id)?.scrollIntoView({ behavior: settings.current.reduceMotion ? 'auto' : 'smooth', block: 'start' });
	}

	/** Highlights the section being read in the side menu. */
	function spy(node: HTMLElement) {
		const io = new IntersectionObserver(
			(entries) => {
				const top = entries.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
				if (top) active = top.target.id;
			},
			{ root: node, rootMargin: '0px 0px -60% 0px' }
		);
		for (const s of node.querySelectorAll('section[id]')) io.observe(s);
		return () => io.disconnect();
	}
</script>

<svelte:head><title>Settings · SlateDB</title></svelte:head>

{#snippet sectionHeading(title: string, description: string)}
	<div class="flex flex-col gap-1">
		<h2 class="text-lg font-semibold tracking-tight">{title}</h2>
		<p class="text-sm text-muted-foreground">{description}</p>
	</div>
{/snippet}

<div class="flex h-svh flex-col">
	<header class="app-header flex h-12 shrink-0 items-center gap-2 border-b bg-sidebar px-3">
		<Button variant="ghost" size="sm" onclick={() => history.back()}>
			<ArrowLeftIcon data-icon="inline-start" /> Back
		</Button>
		<span class="font-semibold">Settings</span>
	</header>

	<div class="flex min-h-0 flex-1">
		<nav class="hidden w-56 shrink-0 flex-col gap-1 border-e bg-sidebar p-3 md:flex" aria-label="Settings sections">
			{#each SECTIONS as s (s.id)}
				<Button
					variant="ghost"
					size="sm"
					class={cn('justify-start', active === s.id && 'bg-accent text-accent-foreground')}
					aria-current={active === s.id ? 'true' : undefined}
					onclick={() => jump(s.id)}
				>
					<s.icon data-icon="inline-start" class={s.tone} />
					{s.label}
				</Button>
			{/each}
		</nav>

		<main class="min-h-0 flex-1 overflow-y-auto" {@attach spy}>
			<div class="mx-auto flex max-w-3xl flex-col gap-12 px-6 py-10">
				<!-- Workspace -->
				<section id="workspace" class="flex scroll-mt-6 flex-col gap-6">
					{@render sectionHeading('Workspace', 'The folder on this computer where your databases and apps live.')}
					<div class="flex flex-col gap-4 border bg-card p-4">
						<div class="flex flex-wrap items-center gap-3">
							<span class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground vivid:bg-tone-blue/15 vivid:text-tone-blue">
								<FolderOpenIcon class="size-5" />
							</span>
							<div class="flex min-w-0 flex-1 flex-col gap-0.5">
								<span class="text-sm font-medium">Root folder</span>
								{#if workspace.info}
									<code class="truncate text-sm text-muted-foreground" title={workspace.info.path}>{workspace.info.path}</code>
								{:else}
									<span class="text-sm text-muted-foreground">No folder chosen yet</span>
								{/if}
							</div>
							<Button variant="outline" onclick={() => workspace.chooseFolder()}>
								<FolderOpenIcon data-icon="inline-start" />
								{workspace.info ? 'Change folder…' : 'Choose folder…'}
							</Button>
						</div>
						{#if workspace.info}
							<div class="flex flex-wrap gap-2 border-t pt-4 text-sm text-muted-foreground">
								<Badge variant="secondary"><DatabaseIcon /> {workspace.databases.length} database{workspace.databases.length === 1 ? '' : 's'}</Badge>
								<Badge variant="secondary"><LayoutDashboardIcon /> {workspace.apps.length} app{workspace.apps.length === 1 ? '' : 's'}</Badge>
								<span>Apps are stored in the <code>Apps</code> folder inside it.</span>
							</div>
						{/if}
					</div>
					<Field.Group>
						<Field.Field orientation="horizontal">
							<Field.Content>
								<Field.Title>Home map layout</Field.Title>
								<Field.Description>Forget where you moved the cards on the home screen and lay them out again.</Field.Description>
							</Field.Content>
							<Button variant="outline" disabled={!workspace.info} onclick={resetLayout}><RotateCcwIcon data-icon="inline-start" /> Reset layout</Button>
						</Field.Field>
					</Field.Group>
				</section>

				<!-- Appearance -->
				<section id="appearance" class="flex scroll-mt-6 flex-col gap-6">
					{@render sectionHeading('Appearance', 'Make SlateDB look the way you like. Changes apply straight away.')}
					<Field.Group>
						<Field.Field orientation="horizontal" class="rounded-lg border bg-card p-4">
							<span class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground vivid:bg-primary/10 vivid:text-primary">
								<FocusIcon class="size-5" />
							</span>
							<Field.Content>
								<Field.Label for="focus-mode">Focus mode</Field.Label>
								<Field.Description>A calm grey look with square corners and no decoration. Turn it off for the colorful look.</Field.Description>
							</Field.Content>
							<Switch id="focus-mode" checked={settings.current.focusMode} onCheckedChange={(v) => settings.update({ focusMode: v })} />
						</Field.Field>

						<Field.Separator />

						<Field.Set>
							<Field.Legend variant="label">Theme</Field.Legend>
							<div class="grid grid-cols-3 gap-3">
								{#each THEMES as t (t.value)}
									{@const selected = userPrefersMode.current === t.value}
									<button
										type="button"
										aria-pressed={selected}
										class={cn(
											'flex flex-col gap-2 border bg-card p-2 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring',
											'rounded-[var(--radius)]',
											selected && 'border-primary ring-1 ring-primary'
										)}
										onclick={() => chooseTheme(t.value)}
									>
										<!-- A tiny window in that theme. -->
										<span
											class={cn(
												'flex h-16 flex-col gap-1 overflow-hidden border p-1.5',
												t.value === 'light' && 'border-neutral-200 bg-white',
												t.value === 'dark' && 'border-neutral-700 bg-neutral-900',
												t.value === 'system' && 'border-neutral-400 bg-[linear-gradient(135deg,white_50%,var(--color-neutral-900)_50%)]'
											)}
											aria-hidden="true"
										>
											<span class={cn('h-1.5 w-10', t.value === 'dark' ? 'bg-neutral-600' : 'bg-neutral-300')}></span>
											<span class="flex flex-1 gap-1">
												<span class={cn('w-1/3', t.value === 'dark' ? 'bg-neutral-800' : 'bg-neutral-100')}></span>
												<span class="flex-1 border border-dashed border-neutral-400/60"></span>
											</span>
										</span>
										<span class="flex items-center gap-2 text-sm font-medium">
											<t.icon class="size-4" />
											{t.label}
											{#if selected}<CheckIcon class="ms-auto size-4 text-primary" />{/if}
										</span>
									</button>
								{/each}
							</div>
						</Field.Set>

						<Field.Separator />

						<Field.Set>
							<Field.Legend variant="label">Accent color</Field.Legend>
							<Field.Description>Used for main buttons, focus rings and highlighted lines.</Field.Description>
							<div class="flex flex-wrap gap-2">
								{#each ACCENTS as a (a)}
									{@const selected = settings.current.accent === a}
									<button
										type="button"
										aria-pressed={selected}
										class={cn(
											'flex items-center gap-2 border bg-card py-1.5 ps-1.5 pe-3 text-sm outline-none transition-colors hover:bg-accent/50 focus-visible:ring-2 focus-visible:ring-ring',
											'rounded-[var(--radius)]',
											selected && 'border-primary ring-1 ring-primary'
										)}
										onclick={() => settings.update({ accent: a })}
									>
										<span class="flex size-6 items-center justify-center rounded-full" style:background={SWATCH[a]}>
											{#if selected}<CheckIcon class="size-3.5 text-background" />{/if}
										</span>
										{label(a)}
									</button>
								{/each}
							</div>
						</Field.Set>

						<Field.Separator />

						<Field.Field orientation="responsive">
							<Field.Content>
								<Field.Title>Corners</Field.Title>
								<Field.Description>Square like a spreadsheet, or softer.</Field.Description>
							</Field.Content>
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
						</Field.Field>

						<Field.Separator />

						<Field.Field orientation="responsive">
							<Field.Content>
								<Field.Title>Text size</Field.Title>
								<Field.Description>Scales text and spacing throughout the app.</Field.Description>
							</Field.Content>
							<ToggleGroup.Root
								type="single"
								variant="outline"
								size="sm"
								value={settings.current.textSize}
								onValueChange={(v) => v && settings.update({ textSize: v as Settings['textSize'] })}
							>
								{#each TEXT_SIZES as t, i (t)}
									<ToggleGroup.Item value={t}>
										<span class={['text-xs', 'text-sm', 'text-base'][i]}>Aa</span>
										{label(t)}
									</ToggleGroup.Item>
								{/each}
							</ToggleGroup.Root>
						</Field.Field>
					</Field.Group>
				</section>

				<!-- Behaviour -->
				<section id="behaviour" class="flex scroll-mt-6 flex-col gap-6">
					{@render sectionHeading('Behaviour', 'How the canvases and animations feel.')}
					<Field.Group>
						<Field.Field orientation="horizontal">
							<Field.Content>
								<Field.Label for="reduce-motion">Reduce motion</Field.Label>
								<Field.Description>Turn off animations, including the moving lines between apps and databases.</Field.Description>
							</Field.Content>
							<Switch
								id="reduce-motion"
								checked={settings.current.reduceMotion}
								onCheckedChange={(v) => settings.update({ reduceMotion: v })}
							/>
						</Field.Field>
						<Field.Separator />
						<Field.Field orientation="horizontal">
							<Field.Content>
								<Field.Label for="canvas-hints">Show canvas hints</Field.Label>
								<Field.Description>The tips along the bottom of the home map and the relationships diagram.</Field.Description>
							</Field.Content>
							<Switch
								id="canvas-hints"
								checked={settings.current.canvasHints}
								onCheckedChange={(v) => settings.update({ canvasHints: v })}
							/>
						</Field.Field>
						<Field.Separator />
						<Field.Field orientation="horizontal">
							<Field.Content>
								<Field.Title>Reset all settings</Field.Title>
								<Field.Description>Go back to the dark theme and the colorful look. Your data isn't touched.</Field.Description>
							</Field.Content>
							<Button variant="outline" onclick={resetAll}><RotateCcwIcon data-icon="inline-start" /> Reset</Button>
						</Field.Field>
					</Field.Group>
				</section>

				<section id="updates" class="flex scroll-mt-6 flex-col gap-6">
					{@render sectionHeading('Updates', 'Get new features and fixes from the official SlateDB releases.')}
					<Field.Group>
						<Field.Field orientation="horizontal">
							<Field.Content>
								<Field.Label for="automatic-updates">Check for updates automatically</Field.Label>
								<Field.Description>Check on launch and every six hours while SlateDB is open. You choose when to install. Database contents are never sent.</Field.Description>
							</Field.Content>
							<Switch id="automatic-updates" checked={settings.current.automaticUpdates} onCheckedChange={(value) => settings.update({ automaticUpdates: value })} />
						</Field.Field>
						<Field.Separator />
						<Field.Field orientation="responsive">
							<Field.Content>
								<Field.Title>SlateDB {version ?? '—'}</Field.Title>
								<Field.Description>
									<span role="status" aria-live="polite">
										{#if !native}Update checks are available in the desktop app.
										{:else if updates.current.status === 'checking'}Checking for updates…
										{:else if updates.current.status === 'downloading'}Downloading update{updates.current.progress === null ? '…' : `… ${updates.current.progress}%`}
										{:else}{updates.current.message || 'Updates are downloaded securely and verified before installation.'}{/if}
									</span>
								</Field.Description>
							</Field.Content>
							<div class="flex flex-wrap gap-2">
								{#if updates.current.status === 'ready'}
									<Button onclick={() => updates.manager.restart()}>Restart SlateDB</Button>
								{:else}
									<Button variant="outline" disabled={!native || updateBusy} onclick={() => updates.manager.check()}>Check for updates</Button>
									{#if updates.current.version}
										<Button disabled={updateBusy} onclick={installUpdate}>Install {updates.current.version}</Button>
									{/if}
								{/if}
							</div>
						</Field.Field>
					</Field.Group>
					{#if updates.current.notes}
						<details class="rounded-lg border p-4">
							<summary class="cursor-pointer text-sm font-medium">Release notes</summary>
							<p class="mt-3 whitespace-pre-wrap text-sm text-muted-foreground">{updates.current.notes}</p>
						</details>
					{/if}
					<Button variant="link" class="self-start" disabled={!native} onclick={() => attempt(() => openUrl(RELEASES))}>View official downloads</Button>
				</section>

				<!-- About -->
				<section id="about" class="flex scroll-mt-6 flex-col gap-6">
					{@render sectionHeading('About', 'The app and who made it.')}
					<div class="flex flex-wrap items-center gap-4 border bg-card p-4">
						<Logo class="size-14" />
						<div class="flex min-w-0 flex-1 flex-col gap-1">
							<div class="flex items-center gap-2">
								<span class="text-lg font-semibold">SlateDB</span>
								<Badge variant="secondary">Version {version ?? '—'}</Badge>
							</div>
							<p class="text-sm text-muted-foreground">
								An offline desktop database app in the spirit of Microsoft Access. Your data stays on this computer.
							</p>
							<p class="text-sm text-muted-foreground">
								Made by
								<Button variant="link" class="h-auto cursor-pointer p-0" onclick={() => attempt(() => openUrl(WEBSITE))}>Dilgash</Button>
							</p>
						</div>
					</div>
				</section>
			</div>
		</main>
	</div>
</div>
