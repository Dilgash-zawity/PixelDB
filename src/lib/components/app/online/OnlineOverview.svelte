<script lang="ts">
	// Types
	import type { ObjectSummary, OnlineDatabase, OnlineInfo } from '#lib/api/types.js';

	// Utils
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { connectionSummary, engineFeatures, groupBySchema, LOCAL_ONLY, provider, type EngineFeature } from '#lib/logic/online.js';
	import { appHref } from '#lib/logic/routes.js';
	import { formatBytes } from '#lib/logic/values.js';
	import { workspace } from '#lib/state/workspace.svelte.js';

	// Components
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Card from '#lib/components/ui/card/index.js';
	import * as Item from '#lib/components/ui/item/index.js';
	import ProviderIcon from '../ProviderIcon.svelte';

	// Icons
	import {
		Braces as BracesIcon,
		ChevronDown as ChevronDownIcon,
		Eye as EyeIcon,
		Network as NetworkIcon,
		Table2 as TableIcon,
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
		objects,
		onEdit,
		onTest,
		onOpen,
		onDiagram
	}: {
		connection: OnlineDatabase;
		info: OnlineInfo;
		/** Tables and views, as the table picker lists them. */
		objects: ObjectSummary[];
		onEdit: () => void;
		onTest: () => void;
		onOpen: (table: string, view: 'data' | 'structure') => void;
		onDiagram: () => void;
	} = $props();

	/** Tables shown before "Show all". */
	const FIRST = 12;
	let showAll = $state(false);
	const groups = $derived(groupBySchema(objects, info.schemas));
	const defaultGroup = $derived(info.engine === 'postgres' ? 'public' : 'Tables');
	const shownGroups = $derived.by(() => {
		if (showAll) return groups;
		let left = FIRST;
		return groups
			.map((g) => {
				const items = g.items.slice(0, Math.max(0, left));
				left -= items.length;
				return { ...g, items };
			})
			.filter((g) => g.items.length);
	});

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
				<Card.Action class="flex flex-wrap justify-end gap-2">
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
						<dd class="font-medium tabular-nums">{info.tables.toLocaleString()}</dd>
					</div>
					<div class="flex flex-col gap-1">
						<dt class="text-muted-foreground">Views</dt>
						<dd class="font-medium tabular-nums">{info.views.toLocaleString()}</dd>
					</div>
				</dl>
				{#if info.schemas.length > 1}
					<div class="mt-4 flex flex-col gap-1.5 text-sm">
						<span class="text-muted-foreground">{info.engine === 'mysql' ? 'Databases' : 'Schemas'} · {info.schemas.length}</span>
						<div class="flex flex-wrap gap-1">
							{#each info.schemas as schema (schema)}<Badge variant="outline">{schema}</Badge>{/each}
						</div>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>

		<section class="flex flex-col gap-3" aria-labelledby="overview-tables">
			<div class="flex items-center gap-2">
				<h2 id="overview-tables" class="text-sm font-semibold">Tables and views</h2>
				<span class="flex-1"></span>
				<Button size="sm" variant="ghost" disabled={!objects.length} onclick={onDiagram}><NetworkIcon data-icon="inline-start" /> See the diagram</Button>
			</div>
			{#if objects.length}
				{#each shownGroups as g (g.schema)}
					<div class="flex flex-col gap-1.5">
						{#if groups.length > 1}<h3 class="text-xs font-medium text-muted-foreground">{g.schema || defaultGroup}</h3>{/if}
						<Item.Group class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
							{#each g.items as { label, item } (item.name)}
								<Item.Root variant="outline" size="sm">
									{#snippet child({ props })}
										<button type="button" {...props} onclick={() => onOpen(item.name, 'data')} title="Open the rows of {item.name}">
											<Item.Media variant="icon" class={item.kind === 'view' ? 'vivid:bg-tone-cyan/15 vivid:text-tone-cyan' : 'vivid:bg-tone-blue/15 vivid:text-tone-blue'}>
												{#if item.kind === 'view'}<EyeIcon />{:else}<TableIcon />{/if}
											</Item.Media>
											<Item.Content class="min-w-0">
												<Item.Title class="w-full truncate">{label}</Item.Title>
												{#if item.kind === 'view'}<Item.Description>View</Item.Description>{/if}
											</Item.Content>
										</button>
									{/snippet}
								</Item.Root>
							{/each}
						</Item.Group>
					</div>
				{/each}
				{#if objects.length > FIRST}
					<Button size="sm" variant="ghost" class="self-start" onclick={() => (showAll = !showAll)}>
						<ChevronDownIcon data-icon="inline-start" class={showAll ? 'rotate-180' : ''} />
						{showAll ? 'Show fewer' : `Show all ${objects.length.toLocaleString()}`}
					</Button>
				{/if}
			{:else}
				<p class="text-sm text-muted-foreground">The server has no tables or views this user may read. Tables are created with the server's own tools.</p>
			{/if}
		</section>

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
