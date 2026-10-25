<script lang="ts">
	// Types
	import type { ConnectionSettings, ConnectionTest, OnlineDatabase } from '#lib/api/types.js';

	// Utils
	import { tick, untrack } from 'svelte';
	import { ApiError } from '#lib/api/client.js';
	import {
		ENGINE_NAMES,
		PROVIDERS,
		connectionProblems,
		newSettings,
		parseConnectionUrl,
		provider as providerOf,
		suggestedName
	} from '#lib/logic/online.js';
	import { databaseNameProblem } from '#lib/logic/values.js';
	import { cleanMessage } from '#lib/state/feedback.svelte.js';
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
	import StepDrawer from './StepDrawer.svelte';

	// Icons
	import {
		Check as CheckIcon,
		CircleCheck as CircleCheckIcon,
		Cloud as CloudIcon,
		Database as DatabaseIcon,
		HardDrive as HardDriveIcon,
		MapPin as MapPinIcon,
		PlugZap as PlugZapIcon,
		Server as ServerIcon,
		TextCursorInput as TextCursorInputIcon,
		TriangleAlert as TriangleAlertIcon
	} from '@lucide/svelte';

	// Script
	/**
	 * Creates a database step by step: where it lives (a local file in the
	 * workspace's DB folder, or an online server), then its name — or for
	 * online, the provider and connection first. With `editing`, changes a
	 * saved online connection instead.
	 */
	let {
		open = $bindable(false),
		editing = null,
		onCreated
	}: {
		open?: boolean;
		editing?: OnlineDatabase | null;
		onCreated?: (name: string, kind: 'local' | 'online') => void;
	} = $props();

	type StepId = 'where' | 'provider' | 'connection' | 'name';

	let step = $state(0);
	let kind = $state<'local' | 'online'>('local');
	let name = $state('');
	let renamed = $state(false);
	let settings = $state<ConnectionSettings>(newSettings('postgres'));
	let secret = $state('');
	let pasted = $state('');
	let pastedNote = $state('');
	let touched = $state(false);
	let busy = $state(false);
	let testing = $state(false);
	let tested = $state<{ ok: true; result: ConnectionTest } | { ok: false; message: string } | null>(null);
	let nameInput = $state<HTMLInputElement | null>(null);

	const STEP_META: Record<StepId, { label: string; icon: typeof DatabaseIcon }> = {
		where: { label: 'Where', icon: MapPinIcon },
		provider: { label: 'Provider', icon: ServerIcon },
		connection: { label: 'Connection', icon: PlugZapIcon },
		name: { label: 'Name', icon: TextCursorInputIcon }
	};
	const stepIds = $derived<StepId[]>(
		editing ? ['provider', 'connection', 'name'] : kind === 'local' ? ['where', 'name'] : ['where', 'provider', 'connection', 'name']
	);
	const steps = $derived(stepIds.map((id) => STEP_META[id]));
	const current = $derived(stepIds[step] ?? 'where');
	const chosen = $derived(providerOf(settings.provider) ?? PROVIDERS[0]);
	const problems = $derived(connectionProblems(settings));
	const connectionOk = $derived(Object.keys(problems).length === 0);

	function freeName(base: string): string {
		const clean = base.trim() || 'My database';
		if (!workspace.nameTaken(clean, editing?.name)) return clean;
		let n = 2;
		while (workspace.nameTaken(`${clean} ${n}`, editing?.name)) n++;
		return `${clean} ${n}`;
	}

	function localSuggestion(): string {
		let n = 1;
		while (workspace.nameTaken(`Database ${n}`)) n++;
		return n === 1 && !workspace.nameTaken('My database') ? 'My database' : `Database ${n}`;
	}

	// Start over each time it opens (not when the folder listing refreshes).
	$effect(() => {
		if (!open) return;
		untrack(() => {
			step = 0;
			touched = false;
			busy = false;
			tested = null;
			secret = '';
			pasted = '';
			pastedNote = '';
			if (editing?.settings) {
				kind = 'online';
				settings = { ...editing.settings };
				name = editing.name;
				renamed = true;
			} else {
				kind = 'local';
				settings = newSettings('postgres');
				name = localSuggestion();
				renamed = false;
			}
		});
	});

	// Ready to type on the name step.
	$effect(() => {
		if (open && current === 'name') tick().then(() => nameInput?.select());
	});

	// A test result describes the settings it ran with.
	$effect(() => {
		void [settings.host, settings.port, settings.database, settings.user, settings.ssl, settings.provider, secret];
		untrack(() => (tested = null));
	});

	const nameProblem = $derived(
		databaseNameProblem(name.trim()) ?? (workspace.nameTaken(name.trim(), editing?.name) ? 'A database with this name already exists' : null)
	);

	function chooseKind(k: 'local' | 'online', andGo = false) {
		kind = k;
		if (!renamed) name = k === 'local' ? localSuggestion() : '';
		if (andGo) step = 1;
	}

	function chooseProvider(id: string, andGo = false) {
		if (id !== settings.provider) {
			const fresh = newSettings(id);
			// Keep what was typed when switching between services of the same engine.
			settings =
				fresh.engine === settings.engine
					? { ...settings, provider: id, ssl: settings.ssl || fresh.ssl, database: settings.database || fresh.database }
					: fresh;
			if (fresh.engine !== settings.engine) secret = '';
		}
		if (andGo) step = stepIds.indexOf('connection');
	}

	/** Fills the fields from a pasted connection string. */
	function applyUrl(text: string): boolean {
		const parsed = parseConnectionUrl(text, settings.provider);
		if (!parsed) return false;
		settings = parsed.settings;
		if (parsed.secret) secret = parsed.secret;
		pastedNote = `Filled in from the connection string${parsed.secret ? `, including the ${chosen.secretLabel.toLowerCase()}` : ''}.`;
		return true;
	}

	function onPaste() {
		if (applyUrl(pasted)) pasted = '';
	}

	function onHost() {
		// People often paste the whole URL into the address field.
		if (settings.host.includes('://')) applyUrl(settings.host);
	}

	async function test() {
		touched = true;
		if (!connectionOk || testing) return;
		testing = true;
		try {
			const keep = !!editing?.has_secret && !secret;
			const result = await workspace.testOnline($state.snapshot(settings), keep ? undefined : secret, keep ? editing!.name : undefined);
			tested = { ok: true, result };
		} catch (e) {
			tested = { ok: false, message: e instanceof ApiError ? cleanMessage(e.message) : String(e) };
		} finally {
			testing = false;
		}
	}

	function next(at: number) {
		const id = stepIds[at];
		if (id === 'connection') {
			touched = true;
			if (!connectionOk) return false;
			if (!renamed) name = freeName(suggestedName(settings) || chosen.name);
			touched = false;
		}
		if (id === 'provider' || id === 'where') touched = false;
	}

	async function submit() {
		touched = true;
		if (nameProblem || busy) return;
		if (kind === 'online' && !connectionOk) {
			step = stepIds.indexOf('connection');
			return;
		}
		busy = true;
		const n = name.trim();
		const s = { ...$state.snapshot(settings), host: settings.host.trim(), user: settings.user.trim(), database: settings.database.trim() };
		try {
			const ok =
				kind === 'local'
					? await workspace.create(n)
					: editing
						? await workspace.updateOnline(editing.name, n, s, secret || undefined)
						: await workspace.createOnline(n, s, secret);
			if (ok) {
				open = false;
				onCreated?.(n, kind);
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
	const URL_PLACEHOLDER = {
		postgres: 'postgresql://user:password@host/database',
		mysql: 'mysql://user:password@host/database',
		libsql: 'libsql://your-db.turso.io?authToken=…'
	};
</script>

<StepDrawer
	bind:open
	bind:step
	title={editing ? `Edit “${editing.name}”` : 'New database'}
	description={editing
		? 'Change where this online database is and how to sign in.'
		: 'A file on this computer, or a database on a server you connect to.'}
	{steps}
	finishLabel={editing ? 'Save changes' : kind === 'local' ? 'Create database' : 'Save connection'}
	finishIcon={CheckIcon}
	{busy}
	onNext={next}
	onFinish={submit}
>
	{#if current === 'where'}
		<Field.Set>
			<Field.Legend>Where should the database live?</Field.Legend>
			<Field.Description>Double-click a card to choose it and go on.</Field.Description>
			<div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label="Where">
				{#each [{ id: 'local', title: 'Local', text: 'A file in the DB folder of your workspace. Works offline; only you can open it.', Icon: HardDriveIcon }, { id: 'online', title: 'Online', text: 'A database on a server: PostgreSQL, MySQL or a service like Neon, Supabase or Turso.', Icon: CloudIcon }] as const as o (o.id)}
					{@const selected = kind === o.id}
					<button type="button" role="radio" aria-checked={selected} class={card(selected)} onclick={() => chooseKind(o.id)} ondblclick={() => chooseKind(o.id, true)}>
						<span class={tile(selected)}><o.Icon class="size-4" /></span>
						<span class="flex min-w-0 flex-col gap-1">
							<span class="text-sm font-medium">{o.title}</span>
							<span class="text-xs text-muted-foreground">{o.text}</span>
						</span>
					</button>
				{/each}
			</div>
		</Field.Set>
	{:else if current === 'provider'}
		<div class="flex flex-col gap-6">
			{#each [{ group: 'database', legend: 'Databases', text: 'A server you or your team runs.' }, { group: 'service', legend: 'Hosting services', text: 'Managed databases in the cloud.' }] as const as g (g.group)}
				<Field.Set>
					<Field.Legend>{g.legend}</Field.Legend>
					<Field.Description>{g.text}</Field.Description>
					<div class="grid gap-2 sm:grid-cols-2" role="radiogroup" aria-label={g.legend}>
						{#each PROVIDERS.filter((p) => p.group === g.group) as p (p.id)}
							{@const selected = settings.provider === p.id}
							<button type="button" role="radio" aria-checked={selected} class={card(selected)} onclick={() => chooseProvider(p.id)} ondblclick={() => chooseProvider(p.id, true)}>
								<span class={tile(selected)}>
									{#if p.group === 'service'}<CloudIcon class="size-4" />{:else}<DatabaseIcon class="size-4" />{/if}
								</span>
								<span class="flex min-w-0 flex-col gap-1">
									<span class="text-sm font-medium">{p.name}</span>
									<span class="text-xs text-muted-foreground">{p.description}</span>
									{#if p.group === 'service'}<Badge variant="secondary" class="mt-0.5">{ENGINE_NAMES[p.engine]}</Badge>{/if}
								</span>
							</button>
						{/each}
					</div>
				</Field.Set>
			{/each}
		</div>
	{:else if current === 'connection'}
		<div class="flex flex-col gap-6">
			<p class="text-sm text-muted-foreground">{chosen.hint}</p>
			<Field.Field>
				<Field.Label for="nd-url">Connection string</Field.Label>
				<div class="flex gap-2">
					<Input
						id="nd-url"
						bind:value={pasted}
						placeholder={URL_PLACEHOLDER[chosen.engine]}
						autocomplete="off"
						spellcheck={false}
						onpaste={() => setTimeout(onPaste)}
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								e.preventDefault();
								onPaste();
							}
						}}
					/>
					<Button variant="outline" disabled={!pasted.trim()} onclick={onPaste}>Fill in</Button>
				</div>
				{#if pastedNote}
					<Field.Description class="text-success">{pastedNote}</Field.Description>
				{:else if pasted.trim() && !parseConnectionUrl(pasted)}
					<Field.Error>This doesn't look like a connection string.</Field.Error>
				{:else}
					<Field.Description>Optional: paste one to fill in the fields below.</Field.Description>
				{/if}
			</Field.Field>

			<Field.Group>
				<div class="grid gap-4 sm:grid-cols-[1fr_8rem]">
					<Field.Field data-invalid={touched && !!problems.host}>
						<Field.Label for="nd-host">Server address</Field.Label>
						<Input
							id="nd-host"
							bind:value={settings.host}
							placeholder={chosen.host}
							autocomplete="off"
							spellcheck={false}
							aria-invalid={touched && !!problems.host}
							oninput={onHost}
						/>
						{#if touched && problems.host}<Field.Error>{problems.host}</Field.Error>{/if}
					</Field.Field>
					<Field.Field data-invalid={touched && !!problems.port}>
						<Field.Label for="nd-port">Port</Field.Label>
						<Input
							id="nd-port"
							inputmode="numeric"
							value={settings.port ?? ''}
							placeholder={String(chosen.port)}
							autocomplete="off"
							aria-invalid={touched && !!problems.port}
							oninput={(e) => {
								const v = e.currentTarget.value.trim();
								settings.port = v ? Number(v) : null;
							}}
						/>
						{#if touched && problems.port}<Field.Error>{problems.port}</Field.Error>{/if}
					</Field.Field>
				</div>
				{#if chosen.engine !== 'libsql'}
					<div class="grid gap-4 sm:grid-cols-2">
						<Field.Field>
							<Field.Label for="nd-db">Database</Field.Label>
							<Input id="nd-db" bind:value={settings.database} placeholder={chosen.database || 'app'} autocomplete="off" spellcheck={false} />
						</Field.Field>
						<Field.Field data-invalid={touched && !!problems.user}>
							<Field.Label for="nd-user">User name</Field.Label>
							<Input
								id="nd-user"
								bind:value={settings.user}
								placeholder={chosen.user || 'user'}
								autocomplete="off"
								spellcheck={false}
								aria-invalid={touched && !!problems.user}
							/>
							{#if touched && problems.user}<Field.Error>{problems.user}</Field.Error>{/if}
						</Field.Field>
					</div>
				{/if}
				<Field.Field>
					<Field.Label for="nd-secret">{chosen.secretLabel}</Field.Label>
					<Input
						id="nd-secret"
						type="password"
						bind:value={secret}
						placeholder={editing?.has_secret ? 'Saved — leave empty to keep it' : ''}
						autocomplete="off"
					/>
					<Field.Description>Kept on this computer only, never in your workspace folder.</Field.Description>
				</Field.Field>
				{#if chosen.engine !== 'libsql'}
					<Field.Field orientation="horizontal" class="rounded-lg border p-3" data-disabled={chosen.sslRequired || undefined}>
						<Field.Content>
							<Field.Label for="nd-ssl">Require an encrypted connection</Field.Label>
							<Field.Description>
								{chosen.sslRequired ? `${chosen.name} only accepts encrypted connections.` : 'Off: encrypts when the server supports it.'}
							</Field.Description>
						</Field.Content>
						<Switch id="nd-ssl" checked={settings.ssl || chosen.sslRequired} disabled={chosen.sslRequired} onCheckedChange={(v) => (settings.ssl = v)} />
					</Field.Field>
				{/if}
			</Field.Group>

			<div class="flex flex-col gap-3">
				<div>
					<Button variant="outline" disabled={testing} onclick={test}>
						{#if testing}<Spinner data-icon="inline-start" />{:else}<PlugZapIcon data-icon="inline-start" />{/if}
						Test connection
					</Button>
				</div>
				{#if tested?.ok}
					<Alert.Root class="border-success/40 bg-success/10 *:[svg]:text-success">
						<CircleCheckIcon />
						<Alert.Title>Connected</Alert.Title>
						<Alert.Description>{tested.result.server} answered in {tested.result.millis} ms.</Alert.Description>
					</Alert.Root>
				{:else if tested}
					<Alert.Root variant="destructive">
						<TriangleAlertIcon />
						<Alert.Title>Couldn't connect</Alert.Title>
						<Alert.Description class="break-words">{tested.message}</Alert.Description>
					</Alert.Root>
				{/if}
			</div>
		</div>
	{:else}
		<Field.Field data-invalid={touched && !!nameProblem}>
			<Field.Label for="nd-name">What should the database be called?</Field.Label>
			<Input
				id="nd-name"
				bind:ref={nameInput}
				bind:value={name}
				autocomplete="off"
				aria-invalid={touched && !!nameProblem}
				oninput={() => (renamed = true)}
			/>
			{#if touched && nameProblem}
				<Field.Error>{nameProblem}</Field.Error>
			{:else if kind === 'local'}
				<Field.Description>Saved as “{name.trim() || '…'}.db” in the DB folder of your workspace. Double-click it on the map to add tables.</Field.Description>
			{:else}
				<Field.Description>The name SlateDB shows for it; the database on the server keeps its own.</Field.Description>
			{/if}
		</Field.Field>
	{/if}
</StepDrawer>
