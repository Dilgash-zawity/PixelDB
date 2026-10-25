<script lang="ts">
	// Types
	import type { OnlineDatabase, OnlineInfo } from '#lib/api/types.js';

	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { connectionSummary, engineFeatures, LOCAL_ONLY, provider, type EngineFeature } from '#lib/logic/online.js';
	import { appHref } from '#lib/logic/routes.js';
	import { formatBytes } from '#lib/logic/values.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import ProviderIcon from '../ProviderIcon.svelte';

	// Icons
	import {
		Braces as BracesIcon,
		CircleMinus as CircleMinusIcon,
		FolderTree as FolderTreeIcon,
		Globe as GlobeIcon,
		LayoutDashboard as LayoutDashboardIcon,
		PencilLine as PencilLineIcon,
		PlugZap as PlugZapIcon,
		Route as RouteIcon,
		Settings2 as SettingsIcon,
		Timer as TimerIcon,
		Users as UsersIcon
	} from '@lucide/svelte';

	// Script
	/** What an online database is and what it can do, compared with a local one. */
	let {
		connection,
		info,
		onEdit,
		onTest
	}: { connection: OnlineDatabase; info: OnlineInfo; onEdit: () => void; onTest: () => void } = $props();

	const settings = $derived(connection.settings!);
	const features = $derived(engineFeatures(info.engine));
	const usedBy = $derived(workspace.apps.filter((a) => a.valid && a.databases.includes(connection.name)));
	const ICONS: Record<EngineFeature['id'], typeof UsersIcon> = {
		shared: UsersIcon,
		edit: PencilLineIcon,
		schemas: FolderTreeIcon,
		'server-types': BracesIcon,
		plans: RouteIcon,
		'timed-plans': TimerIcon,
		edge: GlobeIcon
	};
	const TONES = ['vivid:bg-tone-blue/15 vivid:text-tone-blue', 'vivid:bg-tone-green/15 vivid:text-tone-green', 'vivid:bg-tone-violet/15 vivid:text-tone-violet', 'vivid:bg-tone-amber/15 vivid:text-tone-amber', 'vivid:bg-tone-cyan/15 vivid:text-tone-cyan', 'vivid:bg-tone-pink/15 vivid:text-tone-pink'];
</script>

<div class="h-full overflow-auto">
	<div class="mx-auto flex max-w-4xl flex-col gap-6 p-6">
		<Card.Root>
			<Card.Header>
				<div class="flex items-center gap-3">
					<span class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground vivid:bg-tone-blue/15 vivid:text-tone-blue">
						<ProviderIcon provider={settings.provider} class="size-5" />
					</span>
					<div class="flex min-w-0 flex-col">
						<Card.Title class="truncate">{connection.name}</Card.Title>
						<Card.Description class="truncate">{provider(settings.provider)?.name ?? 'Online'} · {connectionSummary(settings)}</Card.Description>
					</div>
				</div>
				<Card.Action class="flex gap-2">
					<Button size="sm" variant="outline" onclick={onTest}><PlugZapIcon data-icon="inline-start" /> Test</Button>
					<Button size="sm" variant="outline" onclick={onEdit}><SettingsIcon data-icon="inline-start" /> Edit connection</Button>
				</Card.Action>
			</Card.Header>
			<Card.Content>
				<dl class="grid grid-cols-2 gap-4 text-sm sm:grid-cols-4">
					<div class="flex flex-col gap-1">
						<dt class="text-muted-foreground">Server</dt>
						<dd class="truncate font-medium" title={info.version}>{info.version.split(' on ')[0].split(',')[0]}</dd>
					</div>
					<div class="flex flex-col gap-1">
						<dt class="text-muted-foreground">Size</dt>
						<dd class="font-medium tabular-nums">{info.size !== null ? formatBytes(info.size) : 'Not shown'}</dd>
					</div>
					<div class="flex flex-col gap-1">
						<dt class="text-muted-foreground">Tables</dt>
						<dd class="font-medium tabular-nums">{info.tables}{info.views ? ` · ${info.views} views` : ''}</dd>
					</div>
					<div class="flex flex-col gap-1">
						<dt class="text-muted-foreground">{info.engine === 'mysql' ? 'Databases' : 'Schemas'}</dt>
						<dd class="truncate font-medium" title={info.schemas.join(', ')}>{info.schemas.length ? info.schemas.join(', ') : '—'}</dd>
					</div>
				</dl>
			</Card.Content>
		</Card.Root>

		<section class="flex flex-col gap-3">
			<h2 class="text-sm font-semibold">What this database can do</h2>
			<Item.Group class="grid gap-2 sm:grid-cols-2">
				{#each features as f, i (f.id)}
					{@const Icon = ICONS[f.id]}
					<Item.Root variant="outline">
						<Item.Media variant="icon" class={TONES[i % TONES.length]}><Icon /></Item.Media>
						<Item.Content>
							<Item.Title>{f.label}</Item.Title>
							<Item.Description>{f.description}</Item.Description>
						</Item.Content>
					</Item.Root>
				{/each}
			</Item.Group>
			<div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
				<span>Only in local databases:</span>
				{#each LOCAL_ONLY as l (l)}<span class="inline-flex items-center gap-1"><CircleMinusIcon class="size-3" /> {l}</span>{/each}
			</div>
		</section>

		<section class="flex flex-col gap-3">
			<h2 class="text-sm font-semibold">Apps using it</h2>
			{#if usedBy.length}
				<Item.Group class="grid gap-2 sm:grid-cols-2">
					{#each usedBy as a (a.name)}
						<Item.Root variant="outline">
							{#snippet child({ props })}
								<button type="button" {...props} onclick={() => goto(appHref(resolve('/app'), a.name))}>
									<Item.Media variant="icon" class="vivid:bg-tone-violet/15 vivid:text-tone-violet"><LayoutDashboardIcon /></Item.Media>
									<Item.Content>
										<Item.Title>{a.name}</Item.Title>
										{#if a.description}<Item.Description>{a.description}</Item.Description>{/if}
									</Item.Content>
								</button>
							{/snippet}
						</Item.Root>
					{/each}
				</Item.Group>
			{:else}
				<p class="text-sm text-muted-foreground">No app uses it yet. On the home screen, drag this database's dot onto an app to connect them.</p>
			{/if}
		</section>
	</div>
</div>
