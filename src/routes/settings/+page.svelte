<script lang="ts">
	// Utils
	import { getVersion } from '@tauri-apps/api/app';
	import { isTauri } from '@tauri-apps/api/core';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
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
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Separator } from '#lib/components/ui/separator/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';

	// Icons
	import {
		ArrowLeft as ArrowLeftIcon,
		FolderOpen as FolderOpenIcon,
		Info as InfoIcon,
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
		{ id: 'workspace', label: 'Workspace', icon: FolderOpenIcon, description: 'The folder on this computer where your databases and apps live.' },
		{ id: 'appearance', label: 'Appearance', icon: PaletteIcon, description: 'Make SlateDB look the way you like. Changes apply straight away.' },
		{ id: 'behaviour', label: 'Behaviour', icon: SlidersHorizontalIcon, description: 'How the canvases and animations feel.' },
		{ id: 'updates', label: 'Updates', icon: RefreshCwIcon, description: 'Get new features and fixes from the official SlateDB releases.' },
		{ id: 'about', label: 'About', icon: InfoIcon, description: 'The app and who made it.' }
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

<svelte:head><title>{current.label} · Settings · SlateDB</title></svelte:head>

<!-- One setting: what it does on the left, its control on the right. -->
<!-- `wide` controls (button groups) go under the text when there isn't room beside it. -->
{#snippet row(title: string, description: string, control: import('svelte').Snippet, id?: string, wide = false)}
	<Field.Field orientation={wide ? 'responsive' : 'horizontal'} class="px-4 py-3.5">
		<Field.Content>
			{#if id}<Field.Label for={id}>{title}</Field.Label>{:else}<Field.Title>{title}</Field.Title>{/if}
			<Field.Description>{description}</Field.Description>
		</Field.Content>
		{@render control()}
	</Field.Field>
{/snippet}

<div class="flex h-svh flex-col">
	<header data-bar class="app-header flex h-12 shrink-0 items-center gap-2 border-b bg-sidebar px-3">
		<Button variant="ghost" size="sm" onclick={() => history.back()}>
			<ArrowLeftIcon data-icon="inline-start" /> Back
		</Button>
		<span class="font-semibold">Settings</span>
	</header>

	<div class="flex min-h-0 flex-1">
		<nav class="flex w-48 shrink-0 flex-col gap-0.5 border-e bg-sidebar p-2 sm:w-56 sm:p-3" aria-label="Settings">
			{#each SECTIONS as s (s.id)}
				{@const active = current.id === s.id}
				<Button
					variant="ghost"
					size="sm"
					class={cn('justify-start', active && 'bg-accent text-accent-foreground vivid:shadow-[inset_2px_0_0_var(--map-accent)]')}
					aria-current={active ? 'page' : undefined}
					onclick={() => show(s.id)}
				>
					<s.icon data-icon="inline-start" />
					{s.label}
				</Button>
			{/each}
		</nav>

		<main bind:this={scroller} class="min-h-0 flex-1 overflow-y-auto">
			<div class="mx-auto flex max-w-2xl flex-col gap-6 px-6 py-10">
				<div class="flex flex-col gap-1">
					<h1 class="text-xl font-semibold tracking-tight">{current.label}</h1>
					<p class="text-sm text-muted-foreground">{current.description}</p>
				</div>

				<div class="flex flex-col rounded-lg border bg-card">
					{#if current.id === 'workspace'}
						{#snippet folder()}
							<Button variant="outline" onclick={() => workspace.chooseFolder()}>
								<FolderOpenIcon data-icon="inline-start" />
								{workspace.info ? 'Change folder…' : 'Choose folder…'}
							</Button>
						{/snippet}
						{#snippet layoutReset()}
							<Button variant="outline" disabled={!workspace.info} onclick={resetLayout}><RotateCcwIcon data-icon="inline-start" /> Reset layout</Button>
						{/snippet}
						<Field.Field orientation="horizontal" class="px-4 py-3.5">
							<Field.Content class="min-w-0">
								<Field.Title>Root folder</Field.Title>
								{#if workspace.info}
									<code class="truncate text-sm text-muted-foreground" title={workspace.info.path}>{workspace.info.path}</code>
									<Field.Description>
										{workspace.databases.length} database{workspace.databases.length === 1 ? '' : 's'} in its <code>DB</code> folder,
										{workspace.apps.length} app{workspace.apps.length === 1 ? '' : 's'} in <code>Apps</code>.
									</Field.Description>
								{:else}
									<Field.Description>No folder chosen yet.</Field.Description>
								{/if}
							</Field.Content>
							{@render folder()}
						</Field.Field>
						<Separator />
						{@render row('Home map layout', 'Forget where you moved the cards on the home screen and lay them out again.', layoutReset)}
					{:else if current.id === 'appearance'}
						{#snippet focus()}
							<Switch id="focus-mode" checked={settings.current.focusMode} onCheckedChange={(v) => settings.update({ focusMode: v })} />
						{/snippet}
						{#snippet theme()}
							<ToggleGroup.Root
								type="single"
								variant="outline"
								size="sm"
								value={userPrefersMode.current}
								onValueChange={(v) => v && chooseTheme(v as 'light' | 'dark' | 'system')}
							>
								{#each THEMES as t (t.value)}
									<ToggleGroup.Item value={t.value}><t.icon /> {t.label}</ToggleGroup.Item>
								{/each}
							</ToggleGroup.Root>
						{/snippet}
						{#snippet accent()}
							<div class="flex flex-wrap gap-1.5" role="radiogroup" aria-label="Accent color">
								{#each ACCENTS as a (a)}
									{@const selected = settings.current.accent === a}
									<button
										type="button"
										role="radio"
										aria-checked={selected}
										aria-label={label(a)}
										title={label(a)}
										class={cn(
											'size-7 rounded-full border-2 border-transparent outline-none transition-transform hover:scale-110 focus-visible:ring-2 focus-visible:ring-ring',
											selected && 'border-background ring-2 ring-foreground'
										)}
										style:background={SWATCH[a]}
										onclick={() => settings.update({ accent: a })}
									></button>
								{/each}
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
								{#each TEXT_SIZES as t, i (t)}
									<ToggleGroup.Item value={t}>
										<span class={['text-xs', 'text-sm', 'text-base'][i]}>Aa</span>
										{label(t)}
									</ToggleGroup.Item>
								{/each}
							</ToggleGroup.Root>
						{/snippet}
						{@render row('Theme', 'Light, dark, or follow your computer.', theme, undefined, true)}
						<Separator />
						{@render row('Focus mode', 'A calm grey look with square corners and no decoration.', focus, 'focus-mode')}
						<Separator />
						{@render row('Accent color', 'Used for main buttons, focus rings and highlighted lines.', accent, undefined, true)}
						<Separator />
						{@render row('Corners', 'Square like a spreadsheet, or softer.', corners, undefined, true)}
						<Separator />
						{@render row('Text size', 'Scales text and spacing throughout the app.', textSize, undefined, true)}
					{:else if current.id === 'behaviour'}
						{#snippet motion()}
							<Switch id="reduce-motion" checked={settings.current.reduceMotion} onCheckedChange={(v) => settings.update({ reduceMotion: v })} />
						{/snippet}
						{#snippet hints()}
							<Switch id="canvas-hints" checked={settings.current.canvasHints} onCheckedChange={(v) => settings.update({ canvasHints: v })} />
						{/snippet}
						{#snippet reset()}
							<Button variant="outline" onclick={resetAll}><RotateCcwIcon data-icon="inline-start" /> Reset</Button>
						{/snippet}
						{@render row('Reduce motion', 'Turn off animations, including the moving lines between apps and databases.', motion, 'reduce-motion')}
						<Separator />
						{@render row('Show canvas hints', 'The tips along the bottom of the home map and the relationships diagram.', hints, 'canvas-hints')}
						<Separator />
						{@render row('Reset all settings', "Go back to the dark theme and the default look. Your data isn't touched.", reset)}
					{:else if current.id === 'updates'}
						{#snippet actions()}
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
						{/snippet}
						{#snippet automatic()}
							<Switch id="automatic-updates" checked={settings.current.automaticUpdates} onCheckedChange={(value) => settings.update({ automaticUpdates: value })} />
						{/snippet}
						<Field.Field orientation="horizontal" class="px-4 py-3.5">
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
							{@render actions()}
						</Field.Field>
						<Separator />
						{@render row('Check automatically', 'On launch and every six hours while SlateDB is open. You choose when to install. Database contents are never sent.', automatic, 'automatic-updates')}
						{#if updates.current.notes}
							<Separator />
							<details class="px-4 py-3.5">
								<summary class="cursor-pointer text-sm font-medium">Release notes</summary>
								<p class="mt-3 whitespace-pre-wrap text-sm text-muted-foreground">{updates.current.notes}</p>
							</details>
						{/if}
					{:else}
						<div class="flex items-center gap-4 px-4 py-4">
							<Logo class="size-12" />
							<div class="flex min-w-0 flex-1 flex-col gap-0.5">
								<span class="font-semibold">SlateDB <span class="font-normal text-muted-foreground">{version ?? ''}</span></span>
								<p class="text-sm text-muted-foreground">An offline desktop database app in the spirit of Microsoft Access. Your data stays on this computer.</p>
							</div>
						</div>
						<Separator />
						{#snippet author()}
							<Button variant="outline" disabled={!native} onclick={() => attempt(() => openUrl(WEBSITE))}>dilgash.com</Button>
						{/snippet}
						{@render row('Made by Dilgash', 'Design and development.', author)}
					{/if}
				</div>

				{#if current.id === 'updates'}
					<Button variant="link" class="self-start px-0" disabled={!native} onclick={() => attempt(() => openUrl(RELEASES))}>View official downloads</Button>
				{/if}
			</div>
		</main>
	</div>
</div>
