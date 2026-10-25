<script lang="ts">
	// Services
	import { connectionService } from '#lib/services/connections.js';

	// Types
	import type { ConnectionInfo, ConnectionInput, Provider, TlsMode } from '#lib/api/types.js';

	// Utils
	import { tick, untrack } from 'svelte';
	import { PROVIDERS, blankConnection, connectionProblem, parseConnectionString, providerInfo } from '#lib/logic/connections.js';
	import { databaseNameProblem } from '#lib/logic/values.js';
	import { ApiError } from '#lib/api/client.js';
	import { workspace } from '#lib/state/workspace.svelte.js';
	import { cn } from '#lib/utils.js';

	// Components
	import * as Alert from '#lib/components/ui/alert/index.js';
	import { Badge } from '#lib/components/ui/badge/index.js';
	import { Button } from '#lib/components/ui/button/index.js';
	import * as Field from '#lib/components/ui/field/index.js';
	import { Input } from '#lib/components/ui/input/index.js';
	import { Spinner } from '#lib/components/ui/spinner/index.js';
	import { Switch } from '#lib/components/ui/switch/index.js';
	import * as ToggleGroup from '#lib/components/ui/toggle-group/index.js';
	import ProviderIcon from './ProviderIcon.svelte';
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		Check as CheckIcon,
		CircleCheck as CircleCheckIcon,
		Cloud as CloudIcon,
		HardDrive as HardDriveIcon,
		Plug as PlugIcon,
		Server as ServerIcon,
		TextCursorInput as TextCursorInputIcon,
		TriangleAlert as TriangleAlertIcon,
		Waypoints as WaypointsIcon,
	} from '@lucide/svelte';

	// Script
	/**
	 * Creates a database step by step: local or online, then for an online one
	 * its provider and connection details, then its name. With `editing`, changes
	 * a saved online database's details and name.
	 */
	let {
		open = $bindable(false),
		editing = null,
		onLocal,
		onOnline
	}: {
		open?: boolean;
		editing?: ConnectionInfo | null;
		/** Creates the local database; returns true when it was created. */
		onLocal: (name: string) => Promise<boolean>;
		onOnline: (name: string) => void;
	} = $props();

	type StepKey = 'where' | 'provider' | 'connection' | 'name';
	const STEP_INFO: Record<StepKey, { label: string; icon: typeof PlugIcon }> = {
		where: { label: 'Where', icon: WaypointsIcon },
		provider: { label: 'Database', icon: ServerIcon },
		connection: { label: 'Connection', icon: PlugIcon },
		name: { label: 'Name', icon: TextCursorInputIcon }
	};

	let step = $state(0);
	let where = $state<'local' | 'online'>('local');
	let conn = $state<ConnectionInput>(blankConnection('postgres'));
	let pasted = $state('');
	let pasteState = $state<'none' | 'filled' | 'unreadable'>('none');
	let touched = $state(false);
	let renamed = $state(false);
	let busy = $state(false);
	let test = $state<{ status: 'idle' | 'testing' | 'ok' | 'failed'; message: string }>({ status: 'idle', message: '' });
	let nameInput = $state<HTMLInputElement | null>(null);

	const keys = $derived<StepKey[]>(
		editing ? ['connection', 'name'] : where === 'local' ? ['where', 'name'] : ['where', 'provider', 'connection', 'name']
	);
	const current = $derived(keys[Math.min(step, keys.length - 1)]);
	const steps = $derived(keys.map((k) => STEP_INFO[k]));
	const provider = $derived(providerInfo(conn.provider));
	const libsql = $derived(provider.engine === 'libsql');

	function freeName(base: string): string {
		if (!workspace.nameTaken(base, editing?.name)) return base;
		let n = 2;
		while (workspace.nameTaken(`${base} ${n}`, editing?.name)) n++;
		return `${base} ${n}`;
	}

	/** A name from the connection (the database's own name, else the provider). */
	function suggestedName(): string {
		if (where === 'local') return freeName('My database');
		const fromUrl = libsql ? conn.url.replace(/^[a-z]+:\/\//i, '').split(/[.:/]/)[0] : '';
		const own = conn.database && conn.database !== 'postgres' ? conn.database : fromUrl;
		return freeName(own ? own.charAt(0).toUpperCase() + own.slice(1) : provider.name);
	}

	// Start over each time it opens.
	$effect(() => {
		if (!open) return;
		untrack(() => {
			step = 0;
			touched = false;
			renamed = !!editing;
			pasted = '';
			pasteState = 'none';
			test = { status: 'idle', message: '' };
			if (editing) {
				where = 'online';
				const { has_password: _p, has_token: _t, ...rest } = editing;
				conn = { ...rest, password: '', token: '' };
			} else {
				where = 'local';
				conn = blankConnection('postgres', freeName('My database'));
			}
		});
	});

	// Ready to type on the name step.
	$effect(() => {
		if (open && current === 'name') tick().then(() => nameInput?.select());
	});

	const nameProblem = $derived(
		databaseNameProblem(conn.name.trim()) ?? (workspace.nameTaken(conn.name.trim(), editing?.name) ? 'A database with this name already exists' : null)
	);
	const hasSavedSecret = $derived(!!editing && (libsql ? editing.has_token : editing.has_password));
	const detailsProblem = $derived(connectionProblem(conn, hasSavedSecret));

	function chooseWhere(w: 'local' | 'online', andGo = false) {
		where = w;
		if (!renamed) conn.name = suggestedName();
		if (andGo) step = 1;
	}

	function chooseProvider(p: Provider, andGo = false) {
		if (p !== conn.provider) {
			conn = blankConnection(p, conn.name);
			pasted = '';
			pasteState = 'none';
			test = { status: 'idle', message: '' };
		}
		if (andGo) step = 2;
	}

	function onPaste() {
		const text = pasted.trim();
		if (!text) return (pasteState = 'none');
		const next = parseConnectionString(text, conn);
		if (!next) return (pasteState = 'unreadable');
		conn = next;
		pasteState = 'filled';
		test = { status: 'idle', message: '' };
	}

	/** Any change to the details makes an earlier test result stale. */
	function changed() {
		if (test.status !== 'testing') test = { status: 'idle', message: '' };
	}

	function payload(): ConnectionInput {
		return { ...conn, name: conn.name.trim() };
	}

	async function runTest(): Promise<boolean> {
		if (detailsProblem) {
			touched = true;
			return false;
		}
		test = { status: 'testing', message: '' };
		try {
			const info = await connectionService.test(payload(), editing?.name);
			test = { status: 'ok', message: info.version };
			return true;
		} catch (e) {
			test = { status: 'failed', message: e instanceof ApiError ? e.message : String(e) };
			return false;
		}
	}

	function next() {
		if (current === 'connection') {
			touched = true;
			if (detailsProblem) return false;
			if (!renamed && !editing) conn.name = suggestedName();
			touched = false;
		}
		if (current === 'name') {
			touched = true;
			if (nameProblem) return false;
		}
	}

	/** Online databases are tested before they're saved; `anyway` saves one that failed. */
	async function submit(anyway = false) {
		touched = true;
		if (nameProblem) return;
		if (busy) return;
		busy = true;
		try {
			const name = conn.name.trim();
			if (where === 'local') {
				if (await onLocal(name)) open = false;
				return;
			}
			if (detailsProblem) {
				step = keys.indexOf('connection');
				return;
			}
			if (!anyway && test.status !== 'ok' && !(await runTest())) return;
			const saved = await workspace.saveConnection(payload(), editing?.name);
			if (saved) {
				open = false;
				onOnline(saved.name);
			}
		} finally {
			busy = false;
		}
	}

	const card = (selected: boolean) =>
		cn(
			'flex items-start gap-3 rounded-lg border bg-card p-3 text-start outline-none transition-colors hover:bg-accent/50 focus-visible:ring-3 focus-visible:ring-ring/40',
			selected && 'border-primary bg-primary/5 ring-1 ring-primary'
		);
	const tile = (selected: boolean) =>
		cn('flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground', selected && 'bg-primary text-primary-foreground');

	const TLS_LABELS: Record<TlsMode, string> = { require: 'Required', prefer: 'If available', disable: 'Off' };
</script>

<StepDrawer
	bind:open
	bind:step
	title={editing ? `Edit “${editing.name}”` : 'New database'}
	description={editing ? 'Change how SlateDB connects. Nothing on the server changes.' : 'Keep it on this computer, or connect to one online.'}
	{steps}
	finishLabel={editing ? 'Save' : where === 'local' ? 'Create database' : 'Connect'}
	finishIcon={CheckIcon}
	{busy}
	onNext={next}
	onFinish={() => submit()}
>
	{#snippet secondary()}
		{#if where === 'online' && test.status === 'failed'}
			<Button variant="outline" disabled={busy} onclick={() => submit(true)}>Save anyway</Button>
		{/if}
	{/snippet}

	{#if current === 'where'}
		<Field.Set>
			<Field.Legend>Where does the database live?</Field.Legend>
			<Field.Description>Double-click a card to choose it and go on.</Field.Description>
			<div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label="Where">
				<button type="button" role="radio" aria-checked={where === 'local'} class={card(where === 'local')} onclick={() => chooseWhere('local')} ondblclick={() => chooseWhere('local', true)}>
					<span class={cn(tile(where === 'local'), where !== 'local' && 'vivid:bg-tone-cyan/15 vivid:text-tone-cyan')}><HardDriveIcon class="size-4" /></span>
					<span class="flex min-w-0 flex-col gap-1">
						<span class="text-sm font-medium">Local</span>
						<span class="text-xs text-muted-foreground">A file in your databases folder. Works offline, with tables you design, apps and queries.</span>
					</span>
				</button>
				<button type="button" role="radio" aria-checked={where === 'online'} class={card(where === 'online')} onclick={() => chooseWhere('online')} ondblclick={() => chooseWhere('online', true)}>
					<span class={cn(tile(where === 'online'), where !== 'online' && 'vivid:bg-tone-violet/15 vivid:text-tone-violet')}><CloudIcon class="size-4" /></span>
					<span class="flex min-w-0 flex-col gap-1">
						<span class="text-sm font-medium">Online</span>
						<span class="text-xs text-muted-foreground">A database on a server or a cloud service. Browse its tables and run queries that read data.</span>
					</span>
				</button>
			</div>
		</Field.Set>
	{:else if current === 'provider'}
		<div class="flex flex-col gap-5">
			{#each [{ title: 'Database servers', cloud: false }, { title: 'Cloud services', cloud: true }] as group (group.title)}
				<Field.Set>
					<Field.Legend>{group.title}</Field.Legend>
					<div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label={group.title}>
						{#each PROVIDERS.filter((p) => p.cloud === group.cloud) as p (p.id)}
							{@const selected = conn.provider === p.id}
							<button type="button" role="radio" aria-checked={selected} class={card(selected)} onclick={() => chooseProvider(p.id)} ondblclick={() => chooseProvider(p.id, true)}>
								<span class="flex size-9 shrink-0 items-center justify-center rounded-md border bg-background">
									<ProviderIcon provider={p.id} class="size-5" />
								</span>
								<span class="flex min-w-0 flex-col gap-1">
									<span class="flex items-center gap-2 text-sm font-medium">{p.name}{#if selected}<CheckIcon class="size-3.5 text-primary" />{/if}</span>
									<span class="text-xs text-muted-foreground">{p.description}</span>
								</span>
							</button>
						{/each}
					</div>
				</Field.Set>
			{/each}
		</div>
	{:else if current === 'connection'}
		<div class="flex flex-col gap-5">
			<div class="flex items-center gap-3 rounded-lg border bg-card p-3">
				<span class="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-background"><ProviderIcon provider={conn.provider} class="size-5" /></span>
				<div class="flex min-w-0 flex-col">
					<span class="font-semibold">{provider.name}</span>
					<span class="text-xs text-muted-foreground">{provider.hint}</span>
				</div>
			</div>

			<Field.Field data-invalid={pasteState === 'unreadable' || undefined}>
				<Field.Label for="nd-paste">Connection string <span class="font-normal text-muted-foreground">(optional)</span></Field.Label>
				<Input
					id="nd-paste"
					bind:value={pasted}
					autocomplete="off"
					spellcheck={false}
					placeholder={libsql ? 'libsql://my-db-name.turso.io?authToken=…' : provider.engine === 'mysql' ? 'mysql://user:password@host:3306/database' : provider.engine === 'mssql' ? 'sqlserver://user:password@host:1433?database=…' : 'postgresql://user:password@host:5432/database'}
					oninput={onPaste}
				/>
				{#if pasteState === 'filled'}
					<Field.Description class="text-success">Filled in the details below from it.</Field.Description>
				{:else if pasteState === 'unreadable'}
					<Field.Error>That doesn’t look like a {provider.name} connection string.</Field.Error>
				{:else}
					<Field.Description>Paste one to fill in the details below, or type them yourself.</Field.Description>
				{/if}
			</Field.Field>

			<Field.Group>
				{#if libsql}
					<Field.Field>
						<Field.Label for="nd-url">Database URL</Field.Label>
						<Input id="nd-url" bind:value={conn.url} oninput={changed} autocomplete="off" spellcheck={false} placeholder={conn.provider === 'turso' ? 'libsql://my-db-name.turso.io' : 'http://localhost:8080'} />
					</Field.Field>
					<Field.Field>
						<Field.Label for="nd-token">Access token{#if conn.provider === 'sqlite'} <span class="font-normal text-muted-foreground">(if the server needs one)</span>{/if}</Field.Label>
						<Input id="nd-token" type="password" bind:value={conn.token} oninput={changed} autocomplete="off" placeholder={editing?.has_token ? 'Saved — leave blank to keep it' : ''} />
					</Field.Field>
				{:else}
					<div class="grid gap-4 sm:grid-cols-[1fr_8rem]">
						<Field.Field>
							<Field.Label for="nd-host">Server address</Field.Label>
							<Input id="nd-host" bind:value={conn.host} oninput={changed} autocomplete="off" spellcheck={false} placeholder={provider.hostPlaceholder} />
						</Field.Field>
						<Field.Field>
							<Field.Label for="nd-port">Port</Field.Label>
							<Input
								id="nd-port"
								inputmode="numeric"
								value={conn.port ?? ''}
								oninput={(e) => {
									const v = e.currentTarget.value.trim();
									conn.port = v === '' ? null : Number(v);
									changed();
								}}
								placeholder={String(provider.port ?? '')}
							/>
						</Field.Field>
					</div>
					<Field.Field>
						<Field.Label for="nd-database">Database name{#if provider.engine !== 'postgres'} <span class="font-normal text-muted-foreground">(optional)</span>{/if}</Field.Label>
						<Input id="nd-database" bind:value={conn.database} oninput={changed} autocomplete="off" spellcheck={false} />
					</Field.Field>
					<div class="grid gap-4 sm:grid-cols-2">
						<Field.Field>
							<Field.Label for="nd-user">User</Field.Label>
							<Input id="nd-user" bind:value={conn.user} oninput={changed} autocomplete="off" spellcheck={false} />
						</Field.Field>
						<Field.Field>
							<Field.Label for="nd-password">Password</Field.Label>
							<Input id="nd-password" type="password" bind:value={conn.password} oninput={changed} autocomplete="off" placeholder={editing?.has_password ? 'Saved — leave blank to keep it' : ''} />
						</Field.Field>
					</div>
					<Field.Field>
						<Field.Label>Encryption</Field.Label>
						<ToggleGroup.Root
							type="single"
							variant="outline"
							size="sm"
							class="w-full sm:w-fit"
							value={conn.tls}
							onValueChange={(v) => {
								if (!v) return;
								conn.tls = v as TlsMode;
								changed();
							}}
						>
							{#each ['require', 'prefer', 'disable'] as const as mode (mode)}
								<ToggleGroup.Item value={mode}>{TLS_LABELS[mode]}</ToggleGroup.Item>
							{/each}
						</ToggleGroup.Root>
						<Field.Description>{provider.cloud ? 'Cloud services need an encrypted connection.' : 'Use Off only on a network you trust.'}</Field.Description>
					</Field.Field>
					{#if provider.engine === 'mssql'}
						<Field.Field orientation="horizontal" class="rounded-lg border p-3">
							<Field.Content>
								<Field.Label for="nd-trust">Trust the server’s certificate</Field.Label>
								<Field.Description>For servers with a self-signed certificate, like a local SQL Server.</Field.Description>
							</Field.Content>
							<Switch
								id="nd-trust"
								checked={conn.trust_certificate}
								onCheckedChange={(v) => {
									conn.trust_certificate = v;
									changed();
								}}
							/>
						</Field.Field>
					{/if}
				{/if}
			</Field.Group>

			{#if touched && detailsProblem}
				<Field.Error>{detailsProblem}</Field.Error>
			{/if}

			<div class="flex flex-col gap-3">
				<div>
					<Button variant="outline" disabled={test.status === 'testing'} onclick={runTest}>
						{#if test.status === 'testing'}<Spinner data-icon="inline-start" />{:else}<PlugIcon data-icon="inline-start" />{/if}
						Test connection
					</Button>
				</div>
				{#if test.status === 'ok'}
					<Alert.Root class="border-success/40 bg-success/10 *:[svg]:text-success">
						<CircleCheckIcon />
						<Alert.Title>Connected</Alert.Title>
						<Alert.Description>{test.message}</Alert.Description>
					</Alert.Root>
				{:else if test.status === 'failed'}
					<Alert.Root variant="destructive">
						<TriangleAlertIcon />
						<Alert.Title>Couldn’t connect</Alert.Title>
						<Alert.Description>{test.message}</Alert.Description>
					</Alert.Root>
				{/if}
			</div>
		</div>
	{:else}
		<div class="flex flex-col gap-5">
			<Field.Field data-invalid={touched && !!nameProblem}>
				<Field.Label for="nd-name">What should the database be called?</Field.Label>
				<Input id="nd-name" bind:ref={nameInput} bind:value={conn.name} autocomplete="off" aria-invalid={touched && !!nameProblem} oninput={() => (renamed = true)} />
				{#if touched && nameProblem}
					<Field.Error>{nameProblem}</Field.Error>
				{:else if where === 'local'}
					<Field.Description>It is saved as a single file in your databases folder. Double-click it on the map to add tables.</Field.Description>
				{:else}
					<Field.Description>The name SlateDB shows for it. The database on the server keeps its own name.</Field.Description>
				{/if}
			</Field.Field>
			{#if where === 'online'}
				<div class="flex items-center gap-3 rounded-lg border bg-card p-3">
					<span class="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-background"><ProviderIcon provider={conn.provider} class="size-5" /></span>
					<div class="flex min-w-0 flex-1 flex-col">
						<span class="truncate font-semibold">{conn.name.trim() || provider.name}</span>
						<span class="truncate text-xs text-muted-foreground">{provider.name} · {libsql ? conn.url : [conn.host, conn.database].filter(Boolean).join(' · ')}</span>
					</div>
					{#if test.status === 'ok'}<Badge variant="secondary" class="text-success"><CircleCheckIcon /> Tested</Badge>{/if}
				</div>
				{#if test.status === 'failed'}
					<Alert.Root variant="destructive">
						<TriangleAlertIcon />
						<Alert.Title>Couldn’t connect</Alert.Title>
						<Alert.Description>{test.message}</Alert.Description>
					</Alert.Root>
				{/if}
			{/if}
		</div>
	{/if}
</StepDrawer>
